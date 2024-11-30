use std::sync::OnceLock;

use super::*;

// In environment.rs
#[derive(Debug, Clone)]
pub struct Environment {
  config_dir:     PathBuf,
  resources_dir:  PathBuf,
  retrievers_dir: PathBuf,
}

impl Environment {
  pub fn global() -> &'static Environment {
    static INSTANCE: OnceLock<Environment> = OnceLock::new();
    INSTANCE.get_or_init(|| Environment {
      config_dir:     Config::default_path().unwrap_or_else(|_| PathBuf::from(".")),
      resources_dir:  Config::default_resources_path(),
      retrievers_dir: Config::default_retrievers_path(),
    })
  }

  pub fn set_global(config_dir: PathBuf) -> Result<()> {
    static INSTANCE: OnceLock<Environment> = OnceLock::new();

    let env = Environment {
      config_dir:     config_dir.clone(),
      resources_dir:  config_dir.join("resources"),
      retrievers_dir: config_dir.join("retrievers"),
    };

    INSTANCE
      .set(env)
      .map_err(|_| LearnerError::Config("Global environment already initialized".into()))
  }

  // Add getters since we want to access these paths
  pub fn config_dir() -> PathBuf { Self::global().config_dir.clone() }

  pub fn resources_dir() -> PathBuf { Self::global().resources_dir.clone() }

  pub fn retrievers_dir() -> PathBuf { Self::global().retrievers_dir.clone() }

  pub fn resolve_resource_path(resource: &str) -> PathBuf {
    // Add .toml if needed
    let resource_file = if !resource.ends_with(".toml") {
      format!("{}.toml", resource)
    } else {
      resource.to_string()
    };
    Self::global().resources_dir.join(resource_file)
  }

  pub fn resolve_retriever_path(retriever: &str) -> PathBuf {
    let retriever_file = if !retriever.ends_with(".toml") {
      format!("{}.toml", retriever)
    } else {
      retriever.to_string()
    };
    Self::global().retrievers_dir.join(retriever_file)
  }
}
