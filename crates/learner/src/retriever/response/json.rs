use resource::{chrono_to_toml_datetime, FieldDefinition, TypeDefinition};
use serde_json;
use toml::{self, Value as TomlValue};

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  pub field_maps: HashMap<String, FieldMap>,
}

// TODO: Refactor this
impl ResponseProcessor for JsonConfig {
  fn process_response(&self, data: &[u8], resource_config: &ResourceConfig) -> Result<Resource> {
    // Parse raw JSON data
    let json: serde_json::Value = serde_json::from_slice(data)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    trace!("Processing JSON response: {}", serde_json::to_string_pretty(&json).unwrap());

    let mut resource = BTreeMap::new();

    // Process each field according to resource configuration
    for field_def in &resource_config.fields {
      if let Some(field_map) = self.field_maps.get(&field_def.name) {
        // Extract raw value if present, now passing the full field definition
        if let Some(value) = self.extract_value(&json, field_map, field_def)? {
          resource.insert(field_def.name.clone(), value);
        } else if field_def.required {
          return Err(LearnerError::ApiError(format!(
            "Required field '{}' not found in response",
            field_def.name
          )));
        } else if let Some(default) = &field_def.default {
          resource.insert(field_def.name.clone(), default.clone());
        }
      }
    }

    Ok(resource)
  }
}

impl JsonConfig {
  /// Converts a JSON value into a TOML value, respecting type definitions
  fn json_to_toml_value(
    &self,
    value: &serde_json::Value,
    field_type: &str,
    type_definition: Option<&TypeDefinition>,
  ) -> Result<Option<TomlValue>> {
    match field_type {
      // Handle array types with potential element type definitions
      "array" => {
        let array =
          value.as_array().ok_or_else(|| LearnerError::ApiError("Expected array value".into()))?;

        // Get element type definition if available
        let element_def = type_definition.and_then(|def| def.element_type.as_ref());

        // Convert each array element according to its type definition
        let values: Result<Vec<_>> = array
          .iter()
          .map(|item| {
            if let Some(def) = element_def {
              self.json_to_toml_value(item, &def.field_type, def.type_definition.as_ref())
            } else {
              // For simple arrays without type definitions, do basic conversion
              Ok(convert_simple_value(item))
            }
          })
          .filter_map(|r| r.transpose())
          .collect();

        Ok(Some(TomlValue::Array(values?)))
      },

      // Handle table types with field definitions
      "table" => {
        let mut map = toml::map::Map::new();

        // If we have field definitions, follow them for the table structure
        if let Some(type_def) = type_definition {
          if let Some(fields) = &type_def.fields {
            for field_def in fields {
              if let Some(field_map) = self.field_maps.get(&field_def.name) {
                if let Some(field_value) = get_path_value(value, &field_map.path) {
                  if let Some(converted) = self.json_to_toml_value(
                    field_value,
                    &field_def.field_type,
                    field_def.type_definition.as_ref(),
                  )? {
                    map.insert(field_def.name.clone(), converted);
                  }
                }
              }
            }
          }
        } else {
          // For tables without type definitions, convert all fields
          let obj = value
            .as_object()
            .ok_or_else(|| LearnerError::ApiError("Expected object value".into()))?;
          for (key, val) in obj {
            if let Some(converted) = convert_simple_value(val) {
              map.insert(key.clone(), converted);
            }
          }
        }

        Ok(Some(TomlValue::Table(map)))
      },

      // Handle primitive types
      "string" | "datetime" | "boolean" => convert_primitive_value(value, field_type),

      // Handle unsupported types
      unsupported =>
        Err(LearnerError::ApiError(format!("Unsupported field type: {}", unsupported))),
    }
  }

  /// Updates extract_value to use the full field definition
  fn extract_value(
    &self,
    json: &serde_json::Value,
    field_map: &FieldMap,
    field_def: &FieldDefinition,
  ) -> Result<Option<TomlValue>> {
    if let Some(value) = get_path_value(json, &field_map.path) {
      // Apply transformations if configured
      let transformed_value = if let Some(transform) = &field_map.transform {
        serde_json::from_str(&apply_transform(&serde_json::to_string(&value)?, transform)?)?
      } else {
        value.clone()
      };

      // Convert using type definition
      self.json_to_toml_value(
        &transformed_value,
        &field_def.field_type,
        field_def.type_definition.as_ref(),
      )
    } else {
      Ok(None)
    }
  }
}

/// Converts a primitive JSON value to a TOML value
fn convert_primitive_value(
  value: &serde_json::Value,
  field_type: &str,
) -> Result<Option<TomlValue>> {
  match field_type {
    "string" => value
      .as_str()
      .map(|s| TomlValue::String(s.to_string()))
      .ok_or_else(|| LearnerError::ApiError("Expected string value".into()))
      .map(Some),

    "datetime" => value
      .as_str()
      .ok_or_else(|| LearnerError::ApiError("Expected string for datetime".into()))
      .and_then(|s| {
        DateTime::parse_from_rfc3339(s)
          .map_err(|e| LearnerError::ApiError(format!("Invalid datetime: {}", e)))
      })
      .map(|dt| Some(TomlValue::Datetime(chrono_to_toml_datetime(dt.with_timezone(&Utc))))),

    "boolean" => value
      .as_bool()
      .map(TomlValue::Boolean)
      .ok_or_else(|| LearnerError::ApiError("Expected boolean value".into()))
      .map(Some),

    _ => Ok(convert_simple_value(value)),
  }
}

pub fn get_path_value<'a>(
  json: &'a serde_json::Value,
  path: &str,
) -> Option<&'a serde_json::Value> {
  let mut current = json;
  for part in path.split('/') {
    current = if let Ok(index) = part.parse::<usize>() {
      current.as_array()?.get(index)?
    } else {
      current.get(part)?
    };
  }
  Some(current)
}

/// Basic conversion for simple JSON values
fn convert_simple_value(value: &serde_json::Value) -> Option<TomlValue> {
  match value {
    serde_json::Value::String(s) => Some(TomlValue::String(s.clone())),
    serde_json::Value::Number(n) =>
      if n.is_i64() {
        n.as_i64().map(TomlValue::Integer)
      } else {
        n.as_f64().map(TomlValue::Float)
      },
    serde_json::Value::Bool(b) => Some(TomlValue::Boolean(*b)),
    serde_json::Value::Array(arr) => {
      let values: Vec<_> = arr.iter().filter_map(|item| convert_simple_value(item)).collect();
      Some(TomlValue::Array(values))
    },
    serde_json::Value::Object(obj) => {
      let map = obj
        .iter()
        .filter_map(|(k, v)| convert_simple_value(v).map(|val| (k.clone(), val)))
        .collect();
      Some(TomlValue::Table(map))
    },
    serde_json::Value::Null => None,
  }
}
