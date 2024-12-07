use resource::{FieldDefinition, TypeDefinition, ValidationRules};
use serde::de::DeserializeOwned;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config<T> {
  /// Name of this configuration
  pub name:              String,
  /// Optional description
  #[serde(default)]
  pub description:       Option<String>,
  #[serde(default)]
  pub additional_fields: BTreeMap<String, Value>,
  /// The specific configuration type
  #[serde(flatten)]
  pub item:              T,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct FieldDefinition {
//   /// Type of the field (should be a JSON Value type)
//   pub field_type:  String,
//   /// Whether this field must be present
//   #[serde(default)]
//   pub required:    bool,
//   /// Human-readable description
//   #[serde(default)]
//   pub description: Option<String>,
//   /// Default value if field is absent
//   #[serde(default)]
//   pub default:     Option<Value>,
//   /// Optional validation rules
//   #[serde(default)]
//   pub validation:  Option<ValidationRules>,

//   pub type_definition: Option<TypeDefinition>,
// }

#[derive(Debug, Clone, Serialize)]
pub struct ResourceTemplate {
  /// Field definitions with optional metadata
  #[serde(default)]
  //   #[serde(flatten)]
  pub fields: Vec<FieldDefinition>,
}

impl<'de> Deserialize<'de> for ResourceTemplate {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where D: serde::Deserializer<'de> {
    // First deserialize into a map
    let map: BTreeMap<String, FieldDefinition> = BTreeMap::deserialize(deserializer)?;

    // Convert the map into a Vec, setting the name from the key
    let fields = map
      .into_iter()
      .map(|(key, mut field_def)| {
        field_def.name = key;
        field_def
      })
      .collect();

    Ok(ResourceTemplate { fields })
  }
}

// TODO: These two traits can probably be removed
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

pub struct ConfigurationManager {
  builder:        config::ConfigBuilder<config::builder::DefaultState>,
  loaded_configs: BTreeMap<String, Value>,
  // Track config paths for loading extends
  config_paths:   PathBuf,
}

impl ConfigurationManager {
  pub fn new(config_path: impl AsRef<Path>) -> Self {
    Self {
      builder:        config::Config::builder(),
      loaded_configs: BTreeMap::new(),
      config_paths:   config_path.as_ref().to_path_buf(),
    }
  }

  pub fn load_config<T>(&mut self, path: impl AsRef<Path>) -> Result<Config<T>>
  where T: DeserializeOwned + std::fmt::Debug {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let mut raw_config: toml::Value = toml::from_str(&content)?;

    // If this is a Retriever config, handle resource reference
    if std::any::type_name::<T>() == std::any::type_name::<Retriever>() {
      if let Some(toml::Value::String(resource_name)) = raw_config.get("resource") {
        // Load the referenced resource
        let resource_path = self.config_paths.join(format!("{}.toml", resource_name));
        let resource_content = std::fs::read_to_string(resource_path)?;
        let resource_config: toml::Value = toml::from_str(&resource_content)?;

        // Get just the fields we need (ignore name, description etc)
        let resource_fields = resource_config
          .as_table()
          .and_then(|t| {
            Some(
              t.iter()
                .filter(|(k, _)| !["name", "description"].contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<toml::map::Map<String, toml::Value>>(),
            )
          })
          .ok_or_else(|| config::ConfigError::Message("Invalid resource config structure".into()))
          .unwrap();

        // Replace the string reference with the resource fields
        if let Some(table) = raw_config.as_table_mut() {
          table.insert("resource".into(), toml::Value::Table(resource_fields));
        }
      }
    }

    // Convert directly to final type
    let typed_config: Config<T> = raw_config.try_into()?;
    Ok(typed_config)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_extension() {
    let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));

    // Load configurations in order
    let paper: Config<ResourceTemplate> =
      dbg!(manager.load_config("config_new/paper.toml").unwrap());

    let arxiv_retriever: Config<Retriever> =
      dbg!(manager.load_config("config_new/arxiv.toml").unwrap());

    // The paper_record now has all fields from base_resource and paper,
    // plus its own record-specific configuration

    // assert_eq!(paper_record.item.resource.resource_type, "paper");
    // assert!(paper_record.item.resource.required_fields.contains(&"abstract_text".to_string()));
    // assert!(paper_record.item.state_tracking.progress_tracking);
  }
}
