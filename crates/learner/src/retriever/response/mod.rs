use resource::FieldDefinition;
use serde_json::Map;

use super::*;

pub mod json;
pub mod xml;

/// Available response format handlers.
///
/// Specifies how to parse and extract paper metadata from API responses
/// in different formats.
///
/// # Examples
///
/// XML configuration:
/// ```toml
/// [response_format]
/// type = "xml"
/// strip_namespaces = true
///
/// [response_format.field_maps]
/// title = { path = "entry/title" }
/// ```
///
/// JSON configuration:
/// ```toml
/// [response_format]
/// type = "json"
///
/// [response_format.field_maps]
/// title = { path = "message/title/0" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  /// XML response parser configuration
  #[serde(rename = "xml")]
  Xml(xml::XmlConfig),
  /// JSON response parser configuration
  #[serde(rename = "json")]
  Json(json::JsonConfig),
}

/// Field mapping configuration.
///
/// Defines how to extract and transform specific fields from API responses.
///
/// # Examples
///
/// ```toml
/// [field_maps.title]
/// path = "entry/title"
/// transform = { type = "replace", pattern = "\\s+", replacement = " " }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct FieldMap {
  /// Path to field in response (e.g., JSON path or XPath)
  pub path:      String,
  /// Optional transformation to apply to extracted value
  #[serde(default)]
  pub transform: Option<Transform>,
}

/// Available field value transformations.
///
/// Transformations that can be applied to extracted field values
/// before constructing the final Paper object.
///
/// # Examples
///
/// ```toml
/// # Clean up whitespace
/// transform = { type = "replace", pattern = "\\s+", replacement = " " }
///
/// # Convert date format
/// transform = { type = "date", from_format = "%Y-%m-%d", to_format = "%Y-%m-%dT00:00:00Z" }
///
/// # Construct full URL
/// transform = { type = "url", base = "https://example.com/", suffix = ".pdf" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Transform {
  /// Replace text using regex pattern
  Replace {
    /// Regular expression pattern to match
    pattern:     String,
    /// Text to replace matched patterns with
    replacement: String,
  },
  /// Convert between date formats
  Date {
    /// Source date format string using chrono syntax (e.g., "%Y-%m-%d")
    from_format: String,
    /// Target date format string using chrono syntax (e.g., "%Y-%m-%dT%H:%M:%SZ")
    to_format:   String,
  },
  /// Construct URL from parts
  Url {
    /// Base URL template, may contain {value} placeholder
    base:   String,
    /// Optional suffix to append to the URL (e.g., ".pdf")
    suffix: Option<String>,
  },
  Compose {
    /// List of field paths or direct values to combine
    sources: Vec<Source>,
    /// How to format the combined result
    format:  ComposeFormat,
  },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Source {
  /// Path to a field to extract
  #[serde(rename = "path")]
  Path(String),
  /// A literal string value
  #[serde(rename = "literal")]
  Literal(String),
  /// A field mapping with a new key name
  #[serde(rename = "key_value")]
  KeyValue { key: String, path: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ComposeFormat {
  /// Join fields with a delimiter
  Join { delimiter: String },
  /// Create an object with key-value pairs
  Object,
  /// Create an array of objects with specified structure
  ArrayOfObjects {
    /// How to structure each object
    template: BTreeMap<String, String>,
  },
}

/// Trait for processing API responses into Paper objects.
///
/// Implementors of this trait handle the conversion of raw API response data
/// into structured Paper metadata. The trait is implemented separately for
/// different response formats (XML, JSON) to provide a unified interface for
/// paper retrieval.
///
/// # Examples
///
/// ```no_run
/// # use learner::{retriever::ResponseProcessor, resource::Paper};
/// # use learner::error::LearnerError;
/// struct CustomProcessor;
///
/// #[async_trait::async_trait]
/// impl ResponseProcessor for CustomProcessor {
///   async fn process_response(&self, data: &[u8]) -> Result<Paper, LearnerError> {
///     // Parse response data and construct Paper
///     todo!()
///   }
/// }
/// ```
// #[async_trait]
pub trait ResponseProcessor: Send + Sync {
  /// Process raw response data into a Paper object.
  ///
  /// # Arguments
  ///
  /// * `data` - Raw bytes from the API response
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - A fully populated Paper object
  /// - A LearnerError if parsing fails
  fn process_response(&self, data: &[u8], resource_config: &ResourceConfig) -> Result<Resource>;
}

/// Process a JSON value according to field mappings and resource configuration
fn process_json_value(
  json: &Value,
  field_maps: &BTreeMap<String, FieldMap>,
  resource_config: &ResourceConfig,
) -> Result<Resource> {
  let mut resource = Resource::new();

  for field_def in &resource_config.fields {
    if let Some(field_map) = field_maps.get(&field_def.name) {
      if let Some(value) = extract_mapped_value(json, field_map, field_def)? {
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

/// Extract and transform a value from JSON using a field mapping
fn extract_mapped_value(
  json: &Value,
  field_map: &FieldMap,
  field_def: &FieldDefinition,
) -> Result<Option<Value>> {
  let path_components: Vec<&str> = field_map.path.split('/').collect();

  // Extract raw value using path
  let raw_value = get_path_value(json, &path_components)?;

  // If no value found, return None
  let Some(raw_value) = raw_value else {
    return Ok(None);
  };

  // First apply any explicit transforms
  let value = if let Some(transform) = &field_map.transform {
    apply_transform(&raw_value, transform)?
  } else {
    raw_value.clone()
  };

  // Then attempt type coercion based on field definition
  let coerced = coerce_to_type(&value, field_def)?;
  Ok(Some(coerced))
}

fn coerce_to_type(value: &Value, field_def: &FieldDefinition) -> Result<Value> {
  match field_def.field_type.as_str() {
    "array" => {
      let arr = match value {
        // Single value -> wrap in array
        Value::String(_) | Value::Object(_) | Value::Number(_) => vec![value.clone()],
        // Already an array
        Value::Array(arr) => arr.clone(),
        _ => return Ok(value.clone()), // Can't coerce, return as-is
      };

      // If we have inner type info, try to coerce each element
      if let Some(ref type_def) = field_def.type_definition {
        if let Some(ref element_def) = type_def.element_type {
          let coerced: Vec<Value> =
            arr.into_iter().map(|v| coerce_to_type(&v, element_def)).collect::<Result<_>>()?;
          Ok(Value::Array(coerced))
        } else {
          Ok(Value::Array(arr))
        }
      } else {
        Ok(Value::Array(arr))
      }
    },
    "object" => {
      // If we have field definitions, ensure object has required structure
      if let Some(ref type_def) = field_def.type_definition {
        if let Some(fields) = &type_def.fields {
          let mut obj = Map::new();
          match value {
            // Convert string to {name: string} if that's the structure we want
            Value::String(s) if fields.len() == 1 && fields[0].name == "name" => {
              obj.insert("name".to_string(), Value::String(s.clone()));
              Ok(Value::Object(obj))
            },
            Value::Object(m) => {
              // Copy over matching fields with coercion
              for field in fields {
                if let Some(v) = m.get(&field.name) {
                  obj.insert(field.name.clone(), coerce_to_type(v, field)?);
                }
              }
              Ok(Value::Object(obj))
            },
            _ => Ok(value.clone()),
          }
        } else {
          Ok(value.clone())
        }
      } else {
        Ok(value.clone())
      }
    },
    // Add other type coercions as needed
    _ => Ok(value.clone()),
  }
}

/// Get a value from JSON using a path
// Change return type to owned Value
fn get_path_value(json: &Value, path: &[&str]) -> Result<Option<Value>> {
  let mut current = json.clone();

  for &component in path {
    match current {
      Value::Object(map) =>
        if let Some(value) = map.get(component) {
          current = value.clone();
        } else {
          return Ok(None);
        },
      Value::Array(arr) => {
        // If component is numeric, use it as array index
        if let Ok(index) = component.parse::<usize>() {
          if let Some(value) = arr.get(index) {
            current = value.clone();
          } else {
            return Ok(None);
          }
        } else {
          // Otherwise collect matching values from array elements
          let values: Vec<Value> = arr
            .iter()
            .filter_map(|item| match item {
              Value::Object(map) => map.get(component).cloned(),
              _ => None,
            })
            .collect();

          if values.is_empty() {
            return Ok(None);
          } else if values.len() == 1 {
            current = values[0].clone();
          } else {
            return Ok(Some(Value::Array(values)));
          }
        }
      },
      _ => return Ok(None),
    }
  }

  Ok(Some(current))
}

/// Apply a transform to a JSON value
fn apply_transform(value: &Value, transform: &Transform) -> Result<Value> {
  dbg!(&value);
  match transform {
    Transform::Replace { pattern, replacement } => {
      let text = value.as_str().ok_or_else(|| {
        LearnerError::ApiError("Replace transform requires string input".to_string())
      })?;
      let re =
        Regex::new(pattern).map_err(|e| LearnerError::ApiError(format!("Invalid regex: {}", e)))?;
      Ok(Value::String(re.replace_all(text, replacement.as_str()).into_owned()))
    },

    Transform::Date { from_format, to_format } => {
      let text = value.as_str().ok_or_else(|| {
        LearnerError::ApiError("Date transform requires string input".to_string())
      })?;
      let dt = chrono::NaiveDateTime::parse_from_str(text, from_format)
        .map_err(|e| LearnerError::ApiError(format!("Invalid date: {}", e)))?;
      Ok(Value::String(dt.format(to_format).to_string()))
    },

    Transform::Url { base, suffix } => {
      let text = value
        .as_str()
        .ok_or_else(|| LearnerError::ApiError("URL transform requires string input".to_string()))?;
      Ok(Value::String(format!(
        "{}{}",
        base.replace("{value}", text),
        suffix.as_deref().unwrap_or("")
      )))
    },

    Transform::Compose { sources, format } => {
      // Extract values from each source
      let values: Vec<Value> = sources
        .iter()
        .filter_map(|source| match source {
          Source::Path(path) => {
            let components: Vec<&str> = path.split('/').collect();
            get_path_value(value, &components).ok().flatten()
          },
          Source::Literal(text) => Some(Value::String(text.clone())),
          Source::KeyValue { key: _, path } => {
            let components: Vec<&str> = path.split('/').collect();
            get_path_value(value, &components).ok().flatten()
          },
        })
        .collect();

      dbg!(&values);

      // Apply the format to the collected values
      match format {
        ComposeFormat::Join { delimiter } => {
          // Convert values to strings and join
          let strings: Vec<String> = values
            .iter()
            .filter_map(|v| match v {
              Value::String(s) => Some(s.clone()),
              Value::Array(arr) if arr.len() == 1 => arr[0].as_str().map(|s| s.to_string()),
              _ => None,
            })
            .collect();
          Ok(Value::String(strings.join(delimiter)))
        },

        ComposeFormat::Object => {
          dbg!("inside here");
          let mut obj = Map::new();
          dbg!(&sources);
          for (source, value) in sources.iter().zip(values.iter()) {
            dbg!(&source);
            if let Source::KeyValue { key, .. } = source {
              dbg!(key);
              obj.insert(key.clone(), value.clone());
            }
          }
          dbg!(&obj);
          Ok(Value::Object(obj))
        },

        ComposeFormat::ArrayOfObjects { template } => {
          match value {
            // Handle single string -> array of objects
            Value::String(s) => {
              let mut obj = Map::new();
              for (key, template_value) in template {
                let value = template_value.replace("{value}", s);
                obj.insert(key.clone(), Value::String(value));
              }
              Ok(Value::Array(vec![Value::Object(obj)]))
            },

            // Handle array -> array of objects
            Value::Array(arr) => {
              dbg!(&arr);
              let objects: Vec<Value> = arr
                .iter()
                .filter_map(|item| {
                  dbg!(&item);
                  let mut obj = Map::new();
                  for (key, template_value) in template {
                    let value = match item {
                      Value::String(s) => template_value.replace("{value}", s),
                      Value::Object(obj) => {
                        dbg!(obj);
                        let mut keys_and_vals = Vec::new();
                        sources.iter().for_each(|source| {
                          if let Source::KeyValue { key, path } = source {
                            if let Some(val) = obj.get(path) {
                              keys_and_vals.push((key, val))
                            }
                          }
                        });
                        dbg!(&key);
                        keys_and_vals.into_iter().fold(template_value.clone(), |acc, (k, v)| {
                          let replacement = format!("{{{k}}}");
                          acc.replace(&replacement, v.as_str().unwrap_or_default())
                        })
                      },
                      _ => return None,
                    };
                    obj.insert(key.clone(), Value::String(value));
                  }
                  Some(Value::Object(obj))
                })
                .collect();
              Ok(Value::Array(objects))
            },

            _ => Err(LearnerError::ApiError(
              "ArrayOfObjects transform requires string or array input".to_string(),
            )),
          }
        },
      }
    },
  }
}
