use serde_json::{Map, Value};

use super::*;

mod paper;
mod shared;

pub use paper::*;
pub use shared::*;

pub trait Resource: Serialize + for<'de> Deserialize<'de> {
  fn resource_type(&self) -> String;

  fn fields(&self) -> Result<Map<String, Value>> {
    Ok(
      serde_json::to_value(self)?
        .as_object()
        .cloned()
        .ok_or_else(|| LearnerError::InvalidResource)?,
    )
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
  type_name: String,
  fields:    Map<String, Value>,
}

impl Resource for ResourceConfig {
  fn resource_type(&self) -> String { self.type_name.clone() }

  fn fields(&self) -> Result<Map<String, Value>> { Ok(self.fields.clone()) }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_thesis_resource() -> Result<()> {
    // Create a thesis resource
    let mut fields = Map::new();
    fields.insert("title".into(), json!("Understanding Quantum Computing Effects"));
    fields.insert("author".into(), json!(["Alice Researcher", "Bob Scientist"]));
    fields.insert("university".into(), json!("Tech University"));
    fields.insert("department".into(), json!("Computer Science"));
    fields.insert("defense_date".into(), json!("2024-06-15T14:00:00Z"));
    fields.insert(
      "committee".into(),
      json!(["Prof. Carol Chair", "Dr. David Member", "Dr. Eve External"]),
    );
    fields
      .insert("keywords".into(), json!(["quantum computing", "decoherence", "error correction"]));

    let thesis = ResourceConfig { type_name: "thesis".to_string(), fields };

    // Test resource_type
    assert_eq!(thesis.resource_type(), "thesis");

    // Test fields method
    let fields = thesis.fields()?;

    // Verify we can access specific fields with proper types
    assert!(fields.get("title").unwrap().is_string());
    assert!(fields.get("author").unwrap().as_array().unwrap().len() == 2);

    // Test JSON serialization/deserialization roundtrip
    let serialized = serde_json::to_string(&thesis)?;
    let deserialized: ResourceConfig = serde_json::from_str(&serialized)?;
    assert_eq!(thesis.fields.get("title"), deserialized.fields.get("title"));

    Ok(())
  }

  #[test]
  fn test_thesis_from_toml() -> Result<()> {
    let toml_str = include_str!("../../config/resource/thesis.toml");
    let config: ResourceConfig = toml::from_str(toml_str)?;
    dbg!(&config);

    assert_eq!(config.resource_type(), "thesis");

    // Test that we can access the field definitions
    let fields = config.fields()?;
    dbg!(&fields);
    assert!(fields.contains_key("title"));
    assert!(fields.contains_key("author"));
    assert!(fields.contains_key("university"));

    Ok(())
  }
}
