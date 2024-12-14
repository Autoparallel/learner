use std::collections::BTreeMap;

use template::Template;

use super::*;

/// Represents the complete application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Configuration {
  /// Common state tracking template
  pub state:      Template,
  /// Common storage configuration template
  pub storage:    Template,
  /// Common retrieval configuration template
  pub retrieval:  Template,
  /// Available resource types (paper, book, etc.)
  pub resources:  BTreeMap<String, Template>,
  /// Available retrievers (arxiv, doi, etc.)
  pub retrievers: BTreeMap<String, Retriever>,
}

pub struct ConfigurationManager {
  config_paths:  PathBuf,
  /// Cached configuration components
  configuration: Option<Configuration>,
}

impl ConfigurationManager {
  #[instrument(skip_all, fields(path = %config_path.as_ref().display()))]
  pub fn new(config_path: impl AsRef<Path>) -> Self {
    debug!("Creating new configuration manager");
    Self { config_paths: config_path.as_ref().to_path_buf(), configuration: None }
  }

  #[instrument(skip_all, fields(template = %name))]
  fn load_template(&self, name: &str) -> Result<Template> {
    let path = self.config_paths.join(format!("{name}.toml"));
    debug!(path = %path.display(), "Loading template");

    match std::fs::read_to_string(&path) {
      Ok(content) => match toml::from_str(&content) {
        Ok(template) => {
          debug!("Successfully loaded template");
          Ok(template)
        },
        Err(e) => {
          error!(error = %e, "Failed to parse template TOML");
          Err(e.into())
        },
      },
      Err(e) => {
        error!(error = %e, "Failed to read template file");
        Err(e.into())
      },
    }
  }

  #[instrument(skip_all, fields(retriever = %name))]
  fn load_retriever(&self, name: &str) -> Result<Retriever> {
    let path = self.config_paths.join(format!("{name}.toml"));
    debug!(path = %path.display(), "Loading retriever");

    // First load as generic TOML value
    let content = std::fs::read_to_string(&path)?;
    let mut raw_config: toml::Value = toml::from_str(&content)?;

    // Handle template references
    let template_fields = ["resource_template", "retrieval_template"];

    for field in &template_fields {
      if let Some(toml::Value::String(template_name)) = raw_config.get(field) {
        debug!(field = %field, template = %template_name, "Loading template reference");
        // Load the referenced template
        let template_path = self.config_paths.join(format!("{template_name}.toml"));
        let template_content = std::fs::read_to_string(template_path)?;
        let template_config: toml::Value = toml::from_str(&template_content)?;

        // Replace the string reference with the template config
        if let Some(table) = raw_config.as_table_mut() {
          table.insert((*field).to_string(), template_config);
        }
      }
    }

    // Now convert to final Retriever type
    let retriever: Retriever = raw_config.try_into()?;
    Ok(retriever)
  }

  #[instrument(skip(self))]
  pub fn reload_config(&mut self) -> Result<()> {
    info!("Reloading configuration");
    let config_path = self.config_paths.join("config.toml");

    debug!(path = %config_path.display(), "Reading config file");
    let content = std::fs::read_to_string(&config_path)?;
    let config: toml::Value = toml::from_str(&content)?;

    // We build up all parts of the configuration before setting it
    let mut composed_config = BTreeMap::new();

    // 1. Load core templates first
    debug!("Loading core templates");
    let state = self.load_template(
      config.get("state_template").and_then(|v| v.as_str()).ok_or_else(|| {
        error!("Missing state_template in config");
        LearnerError::Config("Missing state_template".into())
      })?,
    )?;
    composed_config.insert("state".to_string(), state.clone());

    let storage = self.load_template(
      config.get("storage_template").and_then(|v| v.as_str()).ok_or_else(|| {
        error!("Missing storage_template in config");
        LearnerError::Config("Missing storage_template".into())
      })?,
    )?;
    composed_config.insert("storage".to_string(), storage.clone());

    let retrieval = self.load_template(
      config.get("retrieval_template").and_then(|v| v.as_str()).ok_or_else(|| {
        error!("Missing retrieval_template in config");
        LearnerError::Config("Missing retrieval_template".into())
      })?,
    )?;
    composed_config.insert("retrieval".to_string(), retrieval.clone());

    // 2. Load resource templates next
    debug!("Loading resource templates");
    let mut resources = BTreeMap::new();
    if let Some(resource_list) = config.get("resources").and_then(|v| v.as_array()) {
      for resource in resource_list {
        if let Some(template_name) = resource.get("template").and_then(|v| v.as_str()) {
          debug!(template = %template_name, "Loading resource template");
          match self.load_template(template_name) {
            Ok(template) => {
              resources.insert(template_name.to_string(), template);
            },
            Err(e) => {
              error!(error = %e, template = %template_name, "Failed to load resource template");
              return Err(e);
            },
          }
        }
      }
    }
    if resources.is_empty() {
      error!("No resource templates loaded successfully");
      return Err(LearnerError::Config("No resource templates loaded".into()));
    }
    composed_config.extend(resources.clone());

    // 3. Finally load retrievers, which can now reference the loaded resources
    debug!("Loading retriever templates");
    let mut retrievers = BTreeMap::new();
    if let Some(retriever_list) = config.get("retrievers").and_then(|v| v.as_array()) {
      for retriever in retriever_list {
        if let Some(template_name) = retriever.get("template").and_then(|v| v.as_str()) {
          debug!(template = %template_name, "Loading retriever template");
          match self.load_retriever(template_name) {
            Ok(retriever_config) => {
              retrievers.insert(template_name.to_string(), retriever_config);
            },
            Err(e) => {
              error!(error = %e, template = %template_name, "Failed to load retriever");
              return Err(e);
            },
          }
        }
      }
    }
    if retrievers.is_empty() {
      error!("No retriever templates loaded successfully");
      return Err(LearnerError::Config("No retriever templates loaded".into()));
    }

    info!(
        resource_count = %resources.len(),
        retriever_count = %retrievers.len(),
        "Configuration loaded successfully"
    );

    // Set the complete configuration
    self.configuration = Some(Configuration { state, storage, retrieval, resources, retrievers });

    Ok(())
  }

  /// Get a reference to the current configuration
  fn config(&self) -> Result<&Configuration> {
    self
      .configuration
      .as_ref()
      .ok_or_else(|| LearnerError::Config("Configuration not loaded".into()))
  }

  // Public interface methods that use the loaded configuration

  /// Get all available resource types
  pub fn get_resource_types(&self) -> Result<Vec<String>> {
    Ok(self.config()?.resources.keys().cloned().collect())
  }

  /// Get all available retrievers
  pub fn get_retrievers(&self) -> Result<Vec<String>> {
    Ok(self.config()?.retrievers.keys().cloned().collect())
  }

  /// Get a specific resource template
  pub fn get_resource_template(&self, name: &str) -> Result<&Template> {
    Ok(
      self
        .config()?
        .resources
        .get(name)
        .ok_or_else(|| LearnerError::Config(format!("Resource template {name} not found")))?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  #[traced_test]
  fn test_config_loading() {
    let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));

    // Explicit loading
    manager.reload_config().unwrap();

    // Access configuration
    let resource_types = manager.get_resource_types().unwrap();
    assert!(resource_types.contains(&"paper".to_string()));

    // Test reload
    manager.reload_config().unwrap();
    let retrievers = manager.get_retrievers().unwrap();
    assert!(retrievers.contains(&"arxiv".to_string()));

    // Get specific templates
    let paper_template = manager.get_resource_template("paper").unwrap();
    assert!(paper_template.fields.iter().any(|f| f.name == "title"));
  }
}
