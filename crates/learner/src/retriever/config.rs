use std::os::macos::raw;

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
  pub resource_mappings: BTreeMap<String, FieldMap>,

  pub retrieval_template: Template,
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
      ResponseFormat::Xml { strip_namespaces } => xml_to_json(&data, *strip_namespaces),
      ResponseFormat::Json => serde_json::from_slice(&data)?,
    };

    let (mut resource, retrieval) = self.process_json_value(&json)?;

    // Add source metadata
    resource.insert("source".into(), Value::String(self.source.clone()));
    resource.insert("source_identifier".into(), Value::String(identifier.to_string()));

    // Validate full resource against config
    // TODO: Add in validations here.
    // self.resource_template.validate(dbg!(&resource))?;
    // self.retrieval_template.validate(dbg!(&retrieval))?;

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
  mappings: &BTreeMap<String, FieldMap>,
) -> Result<BTreeMap<String, Value>> {
  let mut result = BTreeMap::new();

  for field_def in &template.fields {
    if let Some(field_map) = mappings.get(dbg!(&field_def.name)) {
      if let Some(value) = extract_mapped_value(json, field_map, field_def)? {
        result.insert(field_def.name.clone(), value);
      } else if field_def.required {
        return Err(LearnerError::ApiError(format!(
          "Required field '{}' not found in response",
          field_def.name
        )));
      } else if let Some(default) = &field_def.default {
        result.insert(field_def.name.clone(), default.clone());
      }
    }
  }

  Ok(result)
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
  let mut value = raw_value;
  for transform in &field_map.transforms {
    value = apply_transform(&value, dbg!(transform))?;
  }
  value = if let Some(structure) = &field_map.structure {
    let mut object = BTreeMap::new();
    for (key, to_replace) in structure {
      // TODO: Remove unwrap
      object.insert(key, to_replace.replace("{value}", value.as_str().unwrap()));
    }
    json!(object)
  } else {
    value
  };

  // Coerce a single value into an array if needed
  if field_def.field_type.as_str() == "array" {
    value = dbg!(into_array(value));
  }

  Ok(Some(value))
}

fn into_array(value: Value) -> Value {
  match value {
    // Single value -> wrap in array
    Value::Array(_) => value,
    _ => json!(vec![value]),
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
      _ => return Some(json.clone()),
    }
  }

  Some(current)
}

/// Apply a transform to a JSON value
fn apply_transform(value: &Value, transform: &Transform) -> Result<Value> {
  match transform {
    Transform::Replace { pattern, replacement } => {
      let text = value.as_str().ok_or_else(|| {
        LearnerError::ApiError("Replace transform requires string input".to_string())
      })?;
      let re =
        Regex::new(pattern).map_err(|e| LearnerError::ApiError(format!("Invalid regex: {e}")))?;
      Ok(Value::String(re.replace_all(text, replacement.as_str()).into_owned()))
    },
    Transform::Combine { subpaths, delimiter } => {
      // TODO: fix unwrap
      println!("INSIDE OF COMBINE WITH SUBPATHS: {:?}", subpaths);
      match value.as_array() {
        Some(arr) =>
          return Ok(Value::Array(
            arr.iter().map(|v| combine_path_values(v, subpaths, delimiter)).collect(),
          )),
        None => return Ok(combine_path_values(value, subpaths, delimiter)),
      }
    },
  }
}

fn combine_path_values(value: &Value, subpaths: &Vec<String>, delimiter: &str) -> Value {
  Value::String(
    subpaths
      .iter()
      .fold(String::new(), |mut acc, subpath| {
        if !acc.is_empty() {
          acc.push_str(delimiter);
        }
        let subpath: Vec<&str> = subpath.split("/").collect();
        acc.push_str(dbg!(get_path_value(value, &subpath).unwrap().as_str().unwrap()));
        acc
      })
      .to_string(),
  )
}
