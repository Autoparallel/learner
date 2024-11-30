use super::*;

pub trait Identifiable {
  fn name(&self) -> String;
}

pub trait Configurable: Sized {
  type Config: Identifiable + for<'de> Deserialize<'de>;
  fn insert(&mut self, config_name: String, config: Self::Config);

  fn with_config(mut self, config: Self::Config) { self.insert(config.name(), config); }

  fn with_config_str(mut self, toml_str: &str) -> Result<Self> {
    let config: Self::Config = toml::from_str(toml_str)?;
    self.insert(config.name(), config);
    Ok(self)
  }

  fn with_config_file(self, path: impl AsRef<Path>) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    self.with_config_str(&content)
  }

  fn with_config_dir(self, dir: impl AsRef<Path>) -> Result<Self> {
    let dir = dir.as_ref();
    dbg!(&dir);
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
