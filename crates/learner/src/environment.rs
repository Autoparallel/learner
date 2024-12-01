use std::sync::OnceLock;

use super::*;

// Global singleton instance
static INSTANCE: OnceLock<Environment> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Environment {
  config_dir:     PathBuf,
  resources_dir:  PathBuf,
  retrievers_dir: PathBuf,
}

/// Builder for constructing Environment instances with custom paths.
/// This allows flexible configuration while maintaining the standard structure.
#[derive(Default)]
pub struct EnvironmentBuilder {
  // Base configuration directory is required
  config_dir:     Option<PathBuf>,
  // Optional custom paths for subdirectories
  resources_dir:  Option<PathBuf>,
  retrievers_dir: Option<PathBuf>,
}

impl Environment {
  /// Starts building a new Environment instance.
  /// This is the entry point for custom environment configuration.
  pub fn builder() -> EnvironmentBuilder { EnvironmentBuilder::default() }

  /// Creates a new Environment directly from paths.
  /// Used internally after validation by the builder.
  fn new(
    config_dir: PathBuf,
    resources_dir: Option<PathBuf>,
    retrievers_dir: Option<PathBuf>,
  ) -> Self {
    Self {
      // Use provided subdirectory paths or default to standard locations
      resources_dir: resources_dir.unwrap_or_else(|| config_dir.join("resources")),
      retrievers_dir: retrievers_dir.unwrap_or_else(|| config_dir.join("retrievers")),
      config_dir,
    }
  }

  pub fn global() -> &'static Environment {
    INSTANCE.get_or_init(|| {
      Self::new(Config::default_path().unwrap_or_else(|_| PathBuf::from(".")), None, None)
    })
  }

  pub fn set_global(env: Environment) -> Result<()> {
    INSTANCE
      .set(env)
      .map_err(|_| LearnerError::Config("Global environment already initialized".into()))
  }

  pub fn resolve_resource_path(name: &str) -> PathBuf {
    let filename =
      if !name.ends_with(".toml") { format!("{}.toml", name) } else { name.to_string() };
    Self::global().resources_dir.join(filename)
  }

  pub fn resolve_retriever_path(name: &str) -> PathBuf {
    let filename =
      if !name.ends_with(".toml") { format!("{}.toml", name) } else { name.to_string() };
    Self::global().retrievers_dir.join(filename)
  }

  pub fn config_dir() -> PathBuf { Self::global().config_dir.clone() }

  pub fn resources_dir() -> PathBuf { Self::global().resources_dir.clone() }

  pub fn retrievers_dir() -> PathBuf { Self::global().retrievers_dir.clone() }
}

impl EnvironmentBuilder {
  pub fn config_dir(mut self, path: impl Into<PathBuf>) -> Self {
    self.config_dir = Some(path.into());
    self
  }

  pub fn resources_dir(mut self, path: impl Into<PathBuf>) -> Self {
    self.resources_dir = Some(path.into());
    self
  }

  pub fn retrievers_dir(mut self, path: impl Into<PathBuf>) -> Self {
    self.retrievers_dir = Some(path.into());
    self
  }

  pub fn build(self) -> Result<Environment> {
    let config_dir = self
      .config_dir
      .ok_or_else(|| LearnerError::Config("Configuration directory must be specified".into()))?;

    Ok(Environment::new(config_dir, self.resources_dir, self.retrievers_dir))
  }
}
