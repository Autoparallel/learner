use resource::{datetime_to_json, FieldDefinition, TypeDefinition};
use serde_json::{self, Number};

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  pub field_maps: BTreeMap<String, FieldMap>,
}

// TODO: Refactor this
impl ResponseProcessor for JsonConfig {
  fn process_response(&self, data: &[u8], resource_config: &ResourceConfig) -> Result<Resource> {
    todo!()
    // // Parse raw JSON data
    // let json: serde_json::Value = serde_json::from_slice(data)
    //   .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    // trace!("Processing JSON response: {}", serde_json::to_string_pretty(&json).unwrap());

    // let mut resource = BTreeMap::new();

    // // Process each field according to resource configuration
    // for field_def in &resource_config.fields {
    //   if let Some(field_map) = self.field_maps.get(&field_def.name) {
    //     // Extract raw value if present, now passing the full field definition
    //     if let Some(value) = self.extract_value(&json, field_map, field_def)? {
    //       resource.insert(field_def.name.clone(), value);
    //     } else if field_def.required {
    //       return Err(LearnerError::ApiError(format!(
    //         "Required field '{}' not found in response",
    //         field_def.name
    //       )));
    //     } else if let Some(default) = &field_def.default {
    //       resource.insert(field_def.name.clone(), default.clone());
    //     }
    //   }
    // }

    // Ok(resource)
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
