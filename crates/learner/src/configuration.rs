use template::Template;

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

  pub fn load_config(&mut self) -> Result<Template> {
    let config_path = self.config_paths.join("config.toml");
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: toml::Value = toml::from_str(&content)?;

    // Load core templates
    let core_templates = ["state_template", "storage_template", "retrieval_template"];
    let mut composed_config = toml::value::Table::new();

    for template_field in &core_templates {
      if let Some(toml::Value::String(template_name)) = config.get(template_field) {
        let template_path = self.config_paths.join(format!("{template_name}.toml"));
        let template_content = std::fs::read_to_string(template_path)?;
        let template_config: toml::Value = toml::from_str(&template_content)?;

        if let toml::Value::Table(template_table) = template_config {
          composed_config.extend(template_table);
        }
      }
    }

    // Load resource templates
    if let Some(toml::Value::Array(resources)) = config.get("resources") {
      let mut resource_values = Vec::new();

      for resource in resources {
        if let Some(toml::Value::String(template_name)) = resource.get("template") {
          let template_path = self.config_paths.join(format!("{template_name}.toml"));
          let template_content = std::fs::read_to_string(template_path)?;
          // Keep as toml::Value instead of converting to Template
          let template_config: toml::Value = toml::from_str(&template_content)?;
          resource_values.push(template_config);
        }
      }

      composed_config.insert("resources".into(), toml::Value::Array(resource_values));
    }

    Ok(toml::Value::Table(composed_config).try_into()?)
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test_config_extension() {
    let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));

    let template = dbg!(manager.load_config().unwrap());
    // Load configurations in order
    // let paper: Template = dbg!(manager.load_config("config_new/paper.toml").unwrap());

    // let retreival: Template = dbg!(manager.load_config("config_new/retrieval.toml")).unwrap();

    // let arxiv_retriever: Retriever = dbg!(manager.load_config("config_new/arxiv.toml").unwrap());
    // let doi_retriever: Retriever = dbg!(manager.load_config("config_new/doi.toml").unwrap());
    // let iacr_retriever: Retriever = dbg!(manager.load_config("config_new/iacr.toml").unwrap());

    todo!("Clean this up")
    // The paper_record now has all fields from base_resource and paper,
    // plus its own record-specific configuration

    // assert_eq!(paper_record.item.resource.resource_type, "paper");
    // assert!(paper_record.item.resource.required_fields.contains(&"abstract_text".to_string()));
    // assert!(paper_record.item.state_tracking.progress_tracking);
  }
}
