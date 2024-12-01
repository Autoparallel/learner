//! JSON response parser implementation.
//!
//! This module handles parsing of JSON API responses into resources using
//! configurable field mappings. It supports path-based field extraction
//! with optional transformations.
//!
//! # Example Configuration
//!
//! ```toml
//! [response_format]
//! type = "json"
//!
//! [response_format.field_maps]
//! title = { path = "message/title/0" }
//! summary = { path = "message/abstract" }
//! created_at = { path = "message/published/date-time" }
//! contributors = { path = "message/contributors" }
//! ```

use resource::chrono_to_toml_datetime;
use serde_json;
use toml::{self, Value as TomlValue};

use super::*;

/// Configuration for processing JSON API responses.
///
/// Provides field mapping rules to extract resource fields from JSON responses
/// using path-based access patterns.
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
        // Extract raw value if present
        if let Some(value) = self.extract_value(&json, field_map, &field_def.field_type)? {
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
  /// Recursively converts a JSON value into a TOML value
  fn json_to_toml_value(&self, value: &serde_json::Value) -> Option<TomlValue> {
    match value {
      // For JSON objects, recursively convert all their fields
      serde_json::Value::Object(obj) => {
        let mut map = toml::map::Map::new();
        for (key, val) in obj {
          if let Some(converted) = self.json_to_toml_value(val) {
            map.insert(key.clone(), converted);
          }
        }
        Some(TomlValue::Table(map))
      },

      // For arrays, recursively convert all elements
      serde_json::Value::Array(arr) => {
        let values: Vec<_> = arr.iter().filter_map(|item| self.json_to_toml_value(item)).collect();
        Some(TomlValue::Array(values))
      },

      // Direct conversions for primitive types
      serde_json::Value::String(s) => Some(TomlValue::String(s.clone())),
      serde_json::Value::Number(n) =>
        if n.is_i64() {
          n.as_i64().map(TomlValue::Integer)
        } else {
          n.as_f64().map(TomlValue::Float)
        },
      serde_json::Value::Bool(b) => Some(TomlValue::Boolean(*b)),
      serde_json::Value::Null => None,
    }
  }

  /// Extracts and converts a value from the JSON response according to the field type
  fn extract_value(
    &self,
    json: &serde_json::Value,
    field_map: &FieldMap,
    field_type: &str,
  ) -> Result<Option<TomlValue>> {
    // Get the value at the specified path
    if let Some(value) = self.get_path_value(json, &field_map.path) {
      // Apply any transformations if it's a string
      let transformed_value = if let Some(transform) = &field_map.transform {
        if let Some(str_val) = value.as_str() {
          let transformed = apply_transform(str_val, transform)?;
          serde_json::Value::String(transformed)
        } else {
          value.clone()
        }
      } else {
        value.clone()
      };

      // Convert the value based on the expected field type
      match field_type {
        "string" => transformed_value
          .as_str()
          .map(|s| TomlValue::String(s.to_string()))
          .ok_or_else(|| {
            LearnerError::ApiError(format!("Expected string value for field type 'string'"))
          })
          .map(Some),
        "datetime" => transformed_value
          .as_str()
          .ok_or_else(|| LearnerError::ApiError("Expected string for datetime".into()))
          .and_then(|s| {
            DateTime::parse_from_rfc3339(s)
              .map_err(|e| LearnerError::ApiError(format!("Invalid datetime format: {}", e)))
          })
          .map(|dt| Some(TomlValue::Datetime(chrono_to_toml_datetime(dt.with_timezone(&Utc))))),
        "array" => Ok(self.json_to_toml_value(&transformed_value)),
        "table" => Ok(self.json_to_toml_value(&transformed_value)),
        unsupported =>
          Err(LearnerError::ApiError(format!("Unsupported field type: {}", unsupported))),
      }
    } else {
      Ok(None)
    }
  }

  /// Gets a string value from JSON using a path
  fn get_by_path(&self, json: &serde_json::Value, path: &str) -> Option<String> {
    self.get_path_value(json, path).and_then(|value| match value {
      serde_json::Value::String(s) => Some(s.clone()),
      serde_json::Value::Number(n) => Some(n.to_string()),
      serde_json::Value::Array(arr) if !arr.is_empty() => arr[0].as_str().map(String::from),
      _ => value.as_str().map(String::from),
    })
  }

  /// Navigates JSON structure using a path
  fn get_path_value<'a>(
    &self,
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
}
