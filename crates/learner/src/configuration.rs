use serde::de::DeserializeOwned;

use super::*;

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

  pub fn load_config<T>(&mut self, path: impl AsRef<Path>) -> Result<T>
  where T: DeserializeOwned + std::fmt::Debug {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let mut raw_config: toml::Value = toml::from_str(&content)?;

    // If this is a Retriever config, handle resource reference
    if std::any::type_name::<T>() == std::any::type_name::<Retriever>() {
      if let Some(toml::Value::String(resource_name)) = raw_config.get("resource") {
        // Load the referenced resource
        let resource_path = self.config_paths.join(format!("{resource_name}.toml"));
        let resource_content = std::fs::read_to_string(resource_path)?;
        let resource_config: toml::Value = toml::from_str(&resource_content)?;

        // Replace the string reference with the resource config
        if let Some(table) = raw_config.as_table_mut() {
          table.insert("resource".into(), resource_config);
        }
      }
    }

    // Convert directly to final type
    let typed_config: T = raw_config.try_into()?;
    Ok(typed_config)
  }
}

#[cfg(test)]
mod tests {
  use template::Template;

  use super::*;

  #[test]
  fn test_config_extension() {
    let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));

    // Load configurations in order
    let paper: Template = dbg!(manager.load_config("config_new/paper.toml").unwrap());

    let arxiv_retriever: Retriever = dbg!(manager.load_config("config_new/arxiv.toml").unwrap());

    todo!("Clean this up")
    // The paper_record now has all fields from base_resource and paper,
    // plus its own record-specific configuration

    // assert_eq!(paper_record.item.resource.resource_type, "paper");
    // assert!(paper_record.item.resource.required_fields.contains(&"abstract_text".to_string()));
    // assert!(paper_record.item.state_tracking.progress_tracking);
  }
}
