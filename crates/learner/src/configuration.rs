use std::ffi::OsStr;

use template::{Template, TemplateType, TemplatedItem};

use super::*;

/// ArXiv default configuration
pub const ARXIV_CONFIG: &str = include_str!("../config/retrievers/arxiv.toml");
/// DOI default configuration
pub const DOI_CONFIG: &str = include_str!("../config/retrievers/doi.toml");
/// IACR default configuration
pub const IACR_CONFIG: &str = include_str!("../config/retrievers/iacr.toml");

/// Paper default configuration
pub const PAPER_CONFIG: &str = include_str!("../config/resources/paper.toml");

// TODO: Making defaults here would probably just be smart
/// Main configuration manager that handles loading and access to all configs
#[derive(Debug)]
pub struct ConfigurationManager {
  config_root: PathBuf,
  // Cache the actual parsed configurations
  resources:   BTreeMap<String, Template>,
  retrievers:  BTreeMap<String, Retriever>,
  // Core templates that apply to all resources
  state:       Option<Template>,
  storage:     Option<Template>,
  retrieval:   Option<Template>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
  /// The resource type this record manages
  pub resource: TemplatedItem,

  /// State tracking configuration
  pub state: TemplatedItem,

  /// Storage configuration
  pub storage: TemplatedItem,

  /// Retrieval configuration
  pub retrieval: TemplatedItem,
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
      state: None,
      storage: None,
      retrieval: None,
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

    // Collect all potential TOML files recursively
    let mut toml_files = Vec::new();
    for entry in walkdir::WalkDir::new(&self.config_root).into_iter().filter_map(|e| e.ok()) {
      let path = entry.path();

      if path.extension() == Some(OsStr::new("toml")) {
        if path.file_name() == Some(OsStr::new("config.toml")) {
          continue;
        }
        toml_files.push(path.to_path_buf());
      }
    }

    // First pass - load all templates
    for path in &toml_files {
      if let Ok(template) = self.load_toml::<Template>(path) {
        match template.template_type {
          TemplateType::Resource => {
            debug!(name = %template.name, "Found resource template");
            self.resources.insert(template.name.clone(), template);
          },
          TemplateType::State => {
            debug!(name = %template.name, "Found state template");
            self.state = Some(template);
          },
          TemplateType::Storage => {
            debug!(name = %template.name, "Found storage template");
            self.storage = Some(template);
          },
          TemplateType::Retrieval => {
            debug!(name = %template.name, "Found retrieval template");
            self.retrieval = Some(template);
          },
        }
      }
    }

    // Second pass - try to load retrievers (which need templates to be loaded first)
    for path in &toml_files {
      // Try to load as raw TOML first
      if let Ok(raw_config) = self.load_toml::<toml::Value>(path) {
        // Check if this looks like a retriever config
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

    // First get the referenced template names
    let resource_template_name = raw_config
      .get("resource_template")
      .and_then(|v| v.as_str())
      .ok_or_else(|| LearnerError::Config("Retriever missing resource_template".into()))?;

    // Get the resource template from resources
    let resource_template = self.get_resource_template(resource_template_name)?;

    // Get the retrieval template
    let retrieval_template = self
      .retrieval
      .as_ref()
      .ok_or_else(|| LearnerError::Config("Retrieval template not loaded".into()))?;

    let partial: RetrieverPartial = raw_config.try_into()?;

    // Now construct the full Retriever
    Ok(Retriever {
      name:               partial.name,
      description:        partial.description,
      base_url:           partial.base_url,
      pattern:            partial.pattern,
      source:             partial.source,
      endpoint_template:  partial.endpoint_template,
      response_format:    partial.response_format,
      headers:            partial.headers,
      resource_template:  resource_template.clone(),
      resource_mappings:  partial.resource_mappings,
      retrieval_template: retrieval_template.clone(),
      retrieval_mappings: partial.retrieval_mappings,
    })
  }

  #[instrument(skip(self))]
  pub fn reload_config(&mut self) -> Result<()> {
    info!("Reloading configuration");

    // Load and process all configurations
    self.scan_configurations()?;

    // Validate we have all required templates
    if self.state.is_none() {
      error!("Missing state template");
      return Err(LearnerError::Config("Missing state template".into()));
    }
    if self.storage.is_none() {
      error!("Missing storage template");
      return Err(LearnerError::Config("Missing storage template".into()));
    }
    if self.retrieval.is_none() {
      error!("Missing retrieval template");
      return Err(LearnerError::Config("Missing retrieval template".into()));
    }
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

  // Public access methods
  pub fn get_resource_types(&self) -> Vec<String> { self.resources.keys().cloned().collect() }

  pub fn get_retrievers(&self) -> Vec<String> { self.retrievers.keys().cloned().collect() }

  pub fn get_resource_template(&self, name: &str) -> Result<&Template> {
    self
      .resources
      .get(name)
      .ok_or_else(|| LearnerError::Config(format!("Resource template {name} not found")))
  }

  pub fn get_retriever_template(&self, name: &str) -> Result<&Retriever> {
    self
      .retrievers
      .get(name)
      .ok_or_else(|| LearnerError::Config(format!("Retriever {name} not found")))
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  #[traced_test]
  fn test_config_loading() {
    let mut manager = ConfigurationManager::new(PathBuf::from("config")).unwrap();

    // Explicit loading
    manager.reload_config().unwrap();

    // Access configuration
    let resource_types = manager.get_resource_types();
    assert!(resource_types.contains(&"paper".to_string()));

    // Test reload
    manager.reload_config().unwrap();
    let retrievers = manager.get_retrievers();
    dbg!(&retrievers);
    assert!(retrievers.contains(&"arxiv".to_string()));

    // Get specific templates
    let paper_template = manager.get_resource_template("paper").unwrap();
    assert!(paper_template.fields.iter().any(|f| f.name == "title"));
  }
}
