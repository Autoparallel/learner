use resource::{TypeDefinition, ValidationRules};
// use resource::FieldDefinition;
use serde::de::DeserializeOwned;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
  //   /// Name of the field
  //   pub name:        String,
  /// Type of the field (should be a JSON Value type)
  pub field_type:  String,
  /// Whether this field must be present
  #[serde(default)]
  pub required:    bool,
  /// Human-readable description
  #[serde(default)]
  pub description: Option<String>,
  /// Default value if field is absent
  #[serde(default)]
  pub default:     Option<Value>,
  /// Optional validation rules
  #[serde(default)]
  pub validation:  Option<ValidationRules>,

  pub type_definition: Option<TypeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config<T> {
  /// Name of this configuration
  pub name:        String,
  /// Optional description
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub extends:     Option<Vec<String>>,

  #[serde(default)]
  pub additional_fields: BTreeMap<String, Value>,
  /// The specific configuration type
  pub item:              T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
  /// Required fields for any academic resource
  pub title:            FieldDefinition,
  pub authors:          FieldDefinition,
  pub publication_date: FieldDefinition,
  pub abstract_text:    Option<FieldDefinition>,

  /// Resource-type specific requirements
  pub resource_type:   String, // paper, book, thesis, etc.
  pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
  /// The resource type this record manages
  pub resource: Resource,

  /// State tracking configuration
  pub state_tracking: State,

  /// Storage configuration
  pub storage: Storage,

  /// Retrieval configuration
  pub retrieval: Retrieval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retriever {
  /// The record type this retriever populates
  pub record_type: String,

  /// API configuration
  pub base_url:          String,
  pub endpoint_template: String,
  pub pattern:           String,
  #[serde(default)]
  pub headers:           BTreeMap<String, String>,

  /// How to process responses
  pub response_format: ResponseFormat,

  /// Field mappings
  pub resource_mappings: BTreeMap<String, FieldMap>,
  pub record_mappings:   BTreeMap<String, FieldMap>,
}

/// Configuration for state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
  pub progress_tracking: bool,
  pub rating_system:     Option<u8>,
  pub allow_notes:       bool,
  pub track_access_time: bool,
}

/// Configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
  pub required_files:     Vec<String>,
  pub track_checksums:    bool,
  pub track_file_history: bool,
}

/// Configuration for retrieval metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retrieval {
  pub track_urls:    bool,
  pub verify_access: bool,
  pub url_types:     Vec<String>,
  pub access_types:  Vec<String>,
}

pub trait Identifiable {
  fn name(&self) -> String;
}

pub trait Configurable: Sized {
  type Config: Identifiable + for<'de> Deserialize<'de>;
  fn as_map(&mut self) -> &mut BTreeMap<String, Self::Config>;

  fn with_config(mut self, config: Self::Config) { self.as_map().insert(config.name(), config); }

  fn with_config_str(mut self, toml_str: &str) -> Result<Self> {
    let config: Self::Config = toml::from_str(toml_str)?;
    self.as_map().insert(config.name(), config);
    Ok(self)
  }

  fn with_config_file(self, path: impl AsRef<Path>) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    self.with_config_str(&content)
  }

  fn with_config_dir(self, dir: impl AsRef<Path>) -> Result<Self> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
      return Err(LearnerError::Path(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Config directory not found",
      )));
    }

    let mut configurable = self;
    for entry in std::fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.extension().is_some_and(|ext| ext == "toml") {
        configurable = configurable.with_config_file(path)?;
      }
    }
    Ok(configurable)
  }
}

/// Helper for managing configurations with inheritance
pub struct ConfigurationManager {
  builder:        config::ConfigBuilder<config::builder::DefaultState>,
  loaded_configs: BTreeMap<String, Value>,
}

impl ConfigurationManager {
  pub fn new() -> Self {
    Self { builder: config::Config::builder(), loaded_configs: BTreeMap::new() }
  }

  // TODO: Remove unwraps
  pub fn load_config<T>(&mut self, path: impl AsRef<Path>) -> Result<Config<T>>
  where T: Serialize + DeserializeOwned + std::fmt::Debug {
    let path = path.as_ref();
    let content =
      std::fs::read_to_string(path).map_err(|e| config::ConfigError::Foreign(Box::new(e))).unwrap();

    // Try to parse and provide detailed error information
    match toml::from_str::<Config<T>>(&content) {
      Ok(config) => {
        let value = serde_json::to_value(&config)
          .map_err(|e| config::ConfigError::Foreign(Box::new(e)))
          .unwrap();
        self.loaded_configs.insert(config.name.clone(), value);
        Ok(config)
      },
      Err(e) => {
        println!("Failed to parse configuration file: {}", path.display());
        println!("Error: {}", e);
        println!("\nExpected structure for {} configuration:", std::any::type_name::<T>());
        panic!()
        // Print example structure if we're parsing a Resource
        // if std::any::type_name::<T>() == std::any::type_name::<Resource>() {
        //   Resource::print_example_structure();
        // }
        // Err(config::ConfigError::Foreign(Box::new(e)))
      },
    }
  }

  fn merge_configs(&self, base: Value, override_with: Value) -> Result<Value> {
    use serde_json::Value::*;

    match (base, override_with) {
      (Object(mut base_map), Object(override_map)) => {
        for (k, v) in override_map {
          match base_map.get(&k) {
            Some(base_value) => {
              let merged = self.merge_configs(base_value.clone(), v)?;
              base_map.insert(k, merged);
            },
            None => {
              base_map.insert(k, v);
            },
          }
        }
        Ok(Object(base_map))
      },
      // Arrays could be merged if needed
      (Array(mut base_arr), Array(override_arr)) => {
        // For now, just append new items
        base_arr.extend(override_arr);
        Ok(Array(base_arr))
      },
      // For all other cases, override takes precedence
      (_, override_with) => Ok(override_with),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_extension() {
    let mut manager = ConfigurationManager::new();

    // Load configurations in order
    let base_resource: Config<Resource> =
      dbg!(manager.load_config("config_new/base_resource.toml").unwrap());

    // let paper: Config<Resource> = dbg!(manager.load_config("config_new/paper.toml").unwrap());

    // let paper_record: Config<Record> =
    //   dbg!(manager.load_config("config_new/paper_record.toml").unwrap());

    // The paper_record now has all fields from base_resource and paper,
    // plus its own record-specific configuration

    // assert_eq!(paper_record.item.resource.resource_type, "paper");
    // assert!(paper_record.item.resource.required_fields.contains(&"abstract_text".to_string()));
    // assert!(paper_record.item.state_tracking.progress_tracking);
  }
}
