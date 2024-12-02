use serde_json::{self};

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  pub field_maps: BTreeMap<String, FieldMap>,
}

// TODO: Refactor this
impl ResponseProcessor for JsonConfig {
  fn process_response(&self, data: &[u8], resource_config: &ResourceConfig) -> Result<Resource> {
    // Parse raw JSON data
    let json: serde_json::Value = serde_json::from_slice(data)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    dbg!(process_json_value(dbg!(&json), &self.field_maps, resource_config))
  }
}
