use serde_json::Map;
use template::{FieldDefinition, Resource, Template};

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Retriever {
  pub name:              String,
  pub description:       Option<String>,
  /// Base URL for API requests
  pub base_url:          String,
  /// Regex pattern for matching and extracting paper identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:           Regex,
  /// Source identifier for papers from this retriever
  pub source:            String,
  /// Template for constructing API endpoint URLs
  pub endpoint_template: String,
  // TODO: This is now more like "how to get the thing to map into the resource"
  // #[serde(flatten)]
  pub response_format:   ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:           BTreeMap<String, String>,

  #[serde(rename = "resource")]
  pub resource_template: Template,
  #[serde(default)]
  pub resource_mappings: BTreeMap<String, FieldMap>,

  #[serde(default)]
  pub retrieval_mappings: BTreeMap<String, FieldMap>,
}

impl Retriever {
  /// Extracts the canonical identifier from an input string.
  ///
  /// Uses the configured regex pattern to extract the standardized
  /// identifier from various input formats (URLs, DOIs, etc.).
  ///
  /// # Arguments
  ///
  /// * `input` - Input string containing a paper identifier
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The extracted identifier as a string slice
  /// - A LearnerError if the input doesn't match the pattern
  pub fn extract_identifier<'a>(&self, input: &'a str) -> Result<&'a str> {
    self
      .pattern
      .captures(input)
      .and_then(|cap| cap.get(1))
      .map(|m| m.as_str())
      .ok_or(LearnerError::InvalidIdentifier)
  }

  pub async fn retrieve_resource(&self, input: &str) -> Result<Resource> {
    let identifier = self.extract_identifier(input)?;

    // Send request and get response
    let url = self.endpoint_template.replace("{identifier}", identifier);
    debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;

    trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    // Process the response using configured processor
    let json = match &self.response_format {
      ResponseFormat::Xml { strip_namespaces } => xml::convert_to_json(&data, *strip_namespaces),
      ResponseFormat::Json => serde_json::from_slice(&data)?,
    };

    // Process response and get resource
    // TODO: this should probably be a method
    let mut resource = self.process_json_value(&json)?;

    // Add source metadata
    resource.insert("source".into(), Value::String(self.source.clone()));
    resource.insert("source_identifier".into(), Value::String(identifier.to_string()));

    // Validate full resource against config
    self.resource_template.validate(&resource)?;
    Ok(resource)
  }

  pub fn process_json_value(&self, json: &Value) -> Result<Resource> {
    let mut resource = Resource::new();

    for field_def in &self.resource_template.fields {
      if let Some(field_map) = self.resource_mappings.get(&field_def.name) {
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
}

/// Extract and transform a value from JSON using a field mapping
fn extract_mapped_value(
  json: &Value,
  field_map: &FieldMap,
  field_def: &FieldDefinition,
) -> Result<Option<Value>> {
  let path_components: Vec<&str> = field_map.path.split('/').collect();

  // Extract raw value using path
  let raw_value = get_path_value(json, &path_components);

  // If no value found, return None
  let Some(raw_value) = raw_value else {
    return Ok(None);
  };

  // First apply any explicit transforms
  let value = if let Some(transform) = &field_map.transform {
    apply_transform(&raw_value, transform)?
  } else {
    raw_value
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
fn get_path_value(json: &Value, path: &[&str]) -> Option<Value> {
  let mut current = json.clone();

  for &component in path {
    match current {
      Value::Object(map) =>
        if let Some(value) = map.get(component) {
          current = value.clone();
        } else {
          return None;
        },
      Value::Array(arr) => {
        // If component is numeric, use it as array index
        if let Ok(index) = component.parse::<usize>() {
          if let Some(value) = arr.get(index) {
            current = value.clone();
          } else {
            return None;
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
            return None;
          } else if values.len() == 1 {
            current = values[0].clone();
          } else {
            return Some(Value::Array(values));
          }
        }
      },
      _ => return None,
    }
  }

  Some(current)
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
            get_path_value(value, &components)
          },
          Source::Literal(text) => Some(Value::String(text.clone())),
          Source::KeyValue { key: _, path } => {
            let components: Vec<&str> = path.split('/').collect();
            get_path_value(value, &components)
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
