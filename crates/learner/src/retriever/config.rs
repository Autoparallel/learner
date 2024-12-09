use record::{Record, State, StorageData};
use serde_json::json;

use super::*;
use crate::template::{FieldDefinition, Template, TemplatedItem};

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

  pub response_format: ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:         BTreeMap<String, String>,

  pub resource_template: Template,
  #[serde(default)]
  pub resource_mappings: BTreeMap<String, Mapping>,

  pub retrieval_template: Template,
  #[serde(default)]
  pub retrieval_mappings: BTreeMap<String, Mapping>,
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

  pub async fn retrieve_resource(&self, input: &str) -> Result<Record> {
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
      ResponseFormat::Xml { strip_namespaces, clean_content } => {
        let xml = if *strip_namespaces {
          response::strip_xml_namespaces(&String::from_utf8_lossy(&data))
        } else {
          String::from_utf8_lossy(&data).to_string()
        };

        // Convert to JSON value
        let mut value = xml_to_json(&xml);

        // Clean content if requested
        if *clean_content {
          clean_value(&mut value);
        }

        value
      },

      ResponseFormat::Json { clean_content } => {
        let mut value = serde_json::from_slice(&data)?;

        if *clean_content {
          clean_value(&mut value);
        }

        value
      },
    };

    let (mut resource, retrieval) = self.process_json_value(&json)?;

    // Add source metadata
    resource.insert("source".into(), Value::String(self.source.clone()));
    resource.insert("source_identifier".into(), Value::String(identifier.to_string()));

    // Validate full resource against config
    // TODO: Add in validations here.
    self.resource_template.validate(dbg!(&resource))?;
    self.retrieval_template.validate(dbg!(&retrieval))?;

    Ok(Record { resource, state: State::default(), storage: StorageData::default(), retrieval })
  }

  pub fn process_json_value(&self, json: &Value) -> Result<(TemplatedItem, TemplatedItem)> {
    let resource = process_template_fields(json, &self.resource_template, &self.resource_mappings)?;
    let retrieval =
      process_template_fields(json, &self.retrieval_template, &self.retrieval_mappings)?;

    Ok((resource, retrieval))
  }
}

fn process_template_fields(
  json: &Value,
  template: &Template,
  mappings: &BTreeMap<String, Mapping>,
) -> Result<BTreeMap<String, Value>> {
  let mut result = BTreeMap::new();

  for field_def in &template.fields {
    if let Some(mapping) = mappings.get(&field_def.name) {
      match extract_mapped_value(json, mapping, field_def) {
        Ok(Some(value)) => {
          result.insert(field_def.name.clone(), value);
        },
        Ok(None) if field_def.required => {
          return Err(LearnerError::ApiError(format!(
            "Required field '{}' not found in response",
            field_def.name
          )));
        },
        Err(e) => return Err(e),
        _ => continue,
      }
    }
  }

  Ok(result)
}

// TODO: Fix unwraps in here
fn extract_mapped_value(
  json: &Value,
  mapping: &Mapping,
  field_def: &FieldDefinition,
) -> Result<Option<Value>> {
  let value = match mapping {
    // Simple path extraction - most common case
    Mapping::Path(path) => {
      let components: Vec<&str> = path.split('/').collect();
      get_path_value(json, &components)
        .ok_or_else(|| LearnerError::ApiError(format!("Path '{}' not found", path)))?
    },

    // Join multiple string values with a delimiter
    Mapping::Join { paths, with } => {
      let parts: Result<Vec<String>> = paths
        .iter()
        .map(|path| {
          let components: Vec<&str> = path.split('/').collect();
          get_path_value(json, &components)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| LearnerError::ApiError(format!("Path '{}' is not a string", path)))
        })
        .collect();
      Value::String(parts?.join(with))
    },

    // Map values into new structures - handles both arrays and objects
    Mapping::Map { from, map } => {
      // Get the source to map from, if specified
      let source = if let Some(path) = from {
        let components: Vec<&str> = path.split('/').collect();
        get_path_value(json, &components)
          .ok_or_else(|| LearnerError::ApiError(format!("Path '{}' not found", path)))?
      } else {
        json.clone()
      };

      // Process based on whether the source is an array or not
      match source {
        Value::Array(items) => {
          // Map each array item
          let mapped: Result<Vec<Value>> = items
            .iter()
            .map(|item| {
              let mut obj = Map::new();
              for (key, mapping) in map {
                if let Ok(Some(value)) = extract_mapped_value(item, mapping, field_def) {
                  obj.insert(key.clone(), value);
                }
              }
              Ok(Value::Object(obj))
            })
            .collect();
          Value::Array(mapped?)
        },
        // Process as a single object
        _ => {
          let mut obj = Map::new();
          for (key, mapping) in map {
            if let Ok(Some(value)) = extract_mapped_value(&source, mapping, field_def) {
              obj.insert(key.clone(), value);
            }
          }
          Value::Object(obj)
        },
      }
    },
  };

  Ok(Some(value))
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
      _ => return Some(json.clone()),
    }
  }

  Some(current)
}
