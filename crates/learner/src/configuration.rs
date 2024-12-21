use std::{
  collections::BTreeMap,
  ffi::OsStr,
  path::{Path, PathBuf},
};

use record::{Resource, Retrieval, State, Storage};
use template::{Template, TemplateType, TemplatedItem};
use tracing::{debug, error, info, instrument, warn};

use super::*;

/// Default configurations provided with the library
mod defaults {
  pub const ARXIV_CONFIG: &str = include_str!("../config/retrievers/arxiv.toml");
  pub const DOI_CONFIG: &str = include_str!("../config/retrievers/doi.toml");
  pub const IACR_CONFIG: &str = include_str!("../config/retrievers/iacr.toml");
  pub const PAPER_CONFIG: &str = include_str!("../config/resources/paper.toml");
}

pub struct ConfigurationManager {
  config_root: PathBuf,
  // Cache the actual constructed types
  resources:   BTreeMap<String, Resource>,
  retrievers:  BTreeMap<String, Retriever>,
  // Core constructed types
  state:       State,
  storage:     Storage,
  retrieval:   Retrieval,
}

impl ConfigurationManager {
  #[instrument(skip_all, fields(path = %config_root.as_ref().display()))]
  pub fn new(config_root: impl AsRef<Path>) -> Result<Self> {
    let config_root = config_root.as_ref().to_path_buf();
    info!("Initializing configuration manager");

    let manager = Self {
      config_root,
      resources: BTreeMap::new(),
      retrievers: BTreeMap::new(),
      state: State::default(),
      storage: Storage::default(),
      retrieval: Retrieval::default(),
    };

    Ok(manager)
  }

  #[instrument(skip(self))]
  fn load_toml<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T> {
    debug!(path = %path.display(), "Loading TOML file");
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
  }

  #[instrument(skip(self))]
  fn scan_configurations(&mut self) -> Result<()> {
    debug!("Starting configuration scan");

    // Clear existing caches
    self.resources.clear();
    self.retrievers.clear();

    // First pass - collect all TOML files
    let mut toml_files = Vec::new();
    for entry in std::fs::read_dir(&self.config_root)? {
      let entry = entry?;
      let path = entry.path();
      if path.extension() == Some(OsStr::new("toml"))
        && path.file_name() != Some(OsStr::new("config.toml"))
      {
        toml_files.push(path);
      }
    }

    // First pass - load and process templates
    for path in &toml_files {
      if let Ok(template) = self.load_toml::<Template>(path) {
        match template.template_type {
          TemplateType::Resource => {
            debug!(name = %template.name, "Found resource template");
            // Create Resource with extended fields from template
            let mut extended = TemplatedItem::new();
            for field in &template.fields {
              if field.name != "title" {
                // Skip base fields
                extended.insert(field.name.clone(), serde_json::Value::Object(Default::default()));
              }
            }
            let resource = Resource {
              title: String::new(), // Will be filled when used
              extended,
            };
            self.resources.insert(template.name.clone(), resource);
          },
          TemplateType::State => {
            debug!(name = %template.name, "Found state template");
            // Add template fields to base State
            let mut extended = TemplatedItem::new();
            for field in &template.fields {
              extended.insert(field.name.clone(), serde_json::Value::Object(Default::default()));
            }
            self.state.extended = extended;
          },
          TemplateType::Storage => {
            debug!(name = %template.name, "Found storage template");
            // Add template fields to base Storage
            let mut extended = TemplatedItem::new();
            for field in &template.fields {
              extended.insert(field.name.clone(), serde_json::Value::Object(Default::default()));
            }
            self.storage.extended = extended;
          },
          TemplateType::Retrieval => {
            debug!(name = %template.name, "Found retrieval template");
            // Add template fields to base Retrieval
            let mut extended = TemplatedItem::new();
            for field in &template.fields {
              extended.insert(field.name.clone(), serde_json::Value::Object(Default::default()));
            }
            self.retrieval.extended = extended;
          },
        }
      }
    }

    // Second pass - process retrievers
    for path in &toml_files {
      if let Ok(raw_config) = self.load_toml::<toml::Value>(path) {
        if raw_config.get("resource_template").is_some()
          && raw_config.get("retrieval_template").is_some()
        {
          debug!(path = %path.display(), "Found potential retriever config");
          match self.process_retriever(raw_config) {
            Ok(retriever) => {
              debug!(name = %retriever.name, "Loaded retriever");
              self.retrievers.insert(retriever.name.clone(), retriever);
            },
            Err(e) => {
              warn!(error = %e, "Failed to process retriever config");
            },
          }
        }
      }
    }

    info!(
        resource_count = %self.resources.len(),
        retriever_count = %self.retrievers.len(),
        "Configuration scan complete"
    );
    Ok(())
  }

  #[instrument(skip(self))]
  fn process_retriever(&self, raw_config: toml::Value) -> Result<Retriever> {
    // First deserialize the raw config without the templates
    #[derive(Deserialize)]
    struct RetrieverPartial {
      name:               String,
      description:        Option<String>,
      base_url:           String,
      #[serde(deserialize_with = "deserialize_regex")]
      pattern:            Regex,
      source:             String,
      endpoint_template:  String,
      response_format:    ResponseFormat,
      #[serde(default)]
      headers:            BTreeMap<String, String>,
      #[serde(default)]
      resource_mappings:  BTreeMap<String, Mapping>,
      #[serde(default)]
      retrieval_mappings: BTreeMap<String, Mapping>,
    }
    debug!("Processing retriever configuration");

    // First parse the basic retriever fields
    let partial: RetrieverPartial = raw_config.clone().try_into()?;

    // Get the resource template name and construct Resource
    let resource_template_name = raw_config
      .get("resource_template")
      .and_then(|v| v.as_str())
      .ok_or_else(|| LearnerError::Config("Retriever missing resource_template".into()))?;

    let resource = self.resources.get(resource_template_name).ok_or_else(|| {
      LearnerError::Config(format!("Resource template {resource_template_name} not found"))
    })?;

    // Construct the final Retriever
    Ok(Retriever {
      name:               partial.name,
      description:        partial.description,
      base_url:           partial.base_url,
      pattern:            partial.pattern,
      source:             partial.source,
      endpoint_template:  partial.endpoint_template,
      response_format:    partial.response_format,
      headers:            partial.headers,
      resource:           resource.clone(),
      retrieval:          self.retrieval.clone(),
      resource_mappings:  partial.resource_mappings,
      retrieval_mappings: partial.retrieval_mappings,
    })
  }

  #[instrument(skip(self))]
  pub fn reload_config(&mut self) -> Result<()> {
    info!("Reloading configuration");

    self.scan_configurations()?;

    // Validate required templates
    if self.resources.is_empty() {
      error!("No resource templates loaded");
      return Err(LearnerError::Config("No resource templates loaded".into()));
    }
    if self.retrievers.is_empty() {
      error!("No retrievers loaded");
      return Err(LearnerError::Config("No retrievers loaded".into()));
    }

    Ok(())
  }

  // Accessors
  pub fn get_resource_types(&self) -> Vec<String> { self.resources.keys().cloned().collect() }

  pub fn get_retrievers(&self) -> Vec<String> { self.retrievers.keys().cloned().collect() }

  pub fn get_resource(&self, name: &str) -> Result<&Resource> {
    self
      .resources
      .get(name)
      .ok_or_else(|| LearnerError::Config(format!("Resource template {name} not found")))
  }

  pub fn get_retriever(&self, name: &str) -> Result<&Retriever> {
    self
      .retrievers
      .get(name)
      .ok_or_else(|| LearnerError::Config(format!("Retriever {name} not found")))
  }
}

#[cfg(test)]
mod tests {
  use record::{Progress, State};
  use tempfile::tempdir;

  use super::*;

  fn setup_test_configs() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().to_path_buf();

    // Create test resource
    std::fs::write(
      config_dir.join("resource.toml"),
      r#"
          name = "resource"
          type = "resource"
          
          [abstract]
          base_type = "string"
          required  = false
          "#,
    )
    .unwrap();

    // Create test state extensions
    std::fs::write(
      config_dir.join("state.toml"),
      r#"
          name = "state"
          type = "state"

          [importance]
          base_type = "number"
          required = false
          validation = { minimum = 1, maximum = 5 }

          [due_date]
          base_type = "string"
          required = false
          validation = { datetime = true }
          "#,
    )
    .unwrap();

    // Create test storage extensions
    std::fs::write(
      config_dir.join("storage.toml"),
      r#"
          name = "storage"
          type = "storage"

          [backup_location]
          base_type = "string"
          required = false

          [file_format]
          base_type = "string"
          required = false
          validation = { enum_values = ["pdf", "epub", "mobi"] }
          "#,
    )
    .unwrap();

    // Create test retrieval extensions
    std::fs::write(
      config_dir.join("retrieval.toml"),
      r#"
          name = "retrieval"
          type = "retrieval"

          [access_type]
          base_type = "string"
          required = false
          validation = { enum_values = ["open", "subscription", "institutional"] }

          [citation_key]
          base_type = "string"
          required = false
          "#,
    )
    .unwrap();

    // Create test retriever
    std::fs::write(
      config_dir.join("retriever.toml"),
      r#"
        name = "retriever"
        type = "retriever"

        resource_template  = "resource"
        retrieval_template = "retrieval"
  
        base_url          = "http://example.com"
        endpoint_template = "http://example.com"
        pattern           = ""
        source            = "test_source"
        
        [response_format]
        clean_content    = true
        strip_namespaces = true
        type             = "xml"
  
        [resource_mappings]
        abstract = "response/abstract"
        title    = "response/title"
  
        [headers]
        Accept = "application/xml"
        "#,
    )
    .unwrap();

    (dir, config_dir)
  }

  #[test]
  #[traced_test]
  fn test_template_loading_and_extension() {
    let (_dir, config_dir) = setup_test_configs();
    let mut manager = ConfigurationManager::new(config_dir).unwrap();

    // Load configurations
    manager.reload_config().unwrap();

    // Verify state template was applied
    assert!(manager.state.extended.contains_key("importance"));
    assert!(manager.state.extended.contains_key("due_date"));

    // Create a state with actual values
    let state = State {
      progress:      Progress::Opened(Some(0.5)),
      starred:       true,
      tags:          vec!["important".to_string()],
      last_accessed: Some(Utc::now()),
      extended:      toml::toml! {
          importance = 4
          due_date = "2024-12-31T00:00:00Z"
      }
      .try_into()
      .unwrap(),
    };

    // Verify storage template was applied
    assert!(manager.storage.extended.contains_key("backup_location"));
    assert!(manager.storage.extended.contains_key("file_format"));

    // Verify retrieval template was applied
    assert!(manager.retrieval.extended.contains_key("access_type"));
    assert!(manager.retrieval.extended.contains_key("citation_key"));

    // Serialize and verify structure
    let json = serde_json::to_string_pretty(&state).unwrap();
    let value: serde_json::Value = dbg!(serde_json::from_str(&json).unwrap());

    // Base fields should be present
    assert!(value["progress"].is_object());
    assert!(value["starred"].is_boolean());

    // Extended fields should be at the same level
    assert_eq!(value["importance"], 4);
    assert_eq!(value["due_date"], "2024-12-31T00:00:00Z");
  }
}
