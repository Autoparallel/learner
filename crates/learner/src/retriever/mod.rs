use std::collections::HashMap;

use super::*;

mod json;
mod xml;

// TODO: not sure of public re-export here.
pub use json::*;
pub use xml::*;

#[derive(Default)]
pub struct Retriever {
  configs: HashMap<String, RetrieverConfig>,
}

/// A paper retriever configuration and implementation
#[derive(Debug, Clone, Deserialize)]
pub struct RetrieverConfig {
  /// Name of this retriever
  pub name:              String,
  /// Base URL for API requests
  pub base_url:          String,
  /// Pattern to match and extract identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:           Regex,
  /// Source type for papers from this retriever
  pub source:            String,
  /// API endpoint template (with {identifier} placeholder)
  pub endpoint_template: String,
  /// How to parse the response
  pub response_format:   ResponseFormat,
  /// HTTP headers to send with request
  #[serde(default)]
  pub headers:           HashMap<String, String>,
}

impl Retriever {
  /// Create a new empty retriever
  pub fn new() -> Self { Self::default() }

  pub fn with_config(mut self, config: RetrieverConfig) {
    self.configs.insert(config.name.clone(), config);
  }

  /// Add a retriever configuration from TOML string
  pub fn with_config_str(mut self, toml_str: &str) -> Result<Self> {
    let config: RetrieverConfig = toml::from_str(toml_str)?;
    self.configs.insert(config.name.clone(), config);
    Ok(self)
  }

  /// Add a retriever configuration from a TOML file
  pub fn with_config_file(self, path: impl AsRef<Path>) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    self.with_config_str(&content)
  }

  /// Add multiple configurations from a directory of TOML files
  pub fn with_config_dir(self, dir: impl AsRef<Path>) -> Result<Self> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
      return Err(LearnerError::Path(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Config directory not found",
      )));
    }

    let mut retriever = self;
    for entry in std::fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.extension().map_or(false, |ext| ext == "toml") {
        retriever = retriever.with_config_file(path)?;
      }
    }
    Ok(retriever)
  }

  /// Try to retrieve a paper using any matching configuration
  pub async fn get_paper(&self, input: &str) -> Result<Paper> {
    let mut matches = Vec::new();

    // Find all configs that match the input
    for config in self.configs.values() {
      if config.pattern.is_match(input) {
        matches.push(config);
      }
    }

    match matches.len() {
      0 => Err(LearnerError::InvalidIdentifier),
      1 => matches[0].retrieve_paper(input).await,
      _ => Err(LearnerError::AmbiguousIdentifier(
        matches.into_iter().map(|c| c.name.clone()).collect(),
      )),
    }
  }
}

/// Supported response formats
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  /// XML responses
  #[serde(rename = "xml")]
  Xml(XmlConfig),
  /// JSON responses
  #[serde(rename = "json")]
  Json(JsonConfig),
}
#[derive(Debug, Clone, Deserialize)]
pub struct FieldMap {
  /// JSON path to extract value from
  pub path:      String,
  /// Optional transformation to apply
  #[serde(default)]
  pub transform: Option<Transform>,
}

/// Available transformations for field values
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Transform {
  /// Replace text using regex
  Replace { pattern: String, replacement: String },
  /// Format a date string
  Date { from_format: String, to_format: String },
  /// Convert a URL
  Url { base: String, suffix: Option<String> },
}

#[async_trait]
pub trait ResponseProcessor: Send + Sync {
  async fn process_response(&self, data: &[u8]) -> Result<Paper>;
}

impl RetrieverConfig {
  /// Extract canonical identifier from input
  pub fn extract_identifier<'a>(&self, input: &'a str) -> Result<&'a str> {
    self
      .pattern
      .captures(input)
      .and_then(|cap| cap.get(1))
      .map(|m| m.as_str())
      .ok_or(LearnerError::InvalidIdentifier)
  }

  /// Retrieve a paper using this retriever's configuration
  pub async fn retrieve_paper(&self, input: &str) -> Result<Paper> {
    let identifier = self.extract_identifier(input)?;
    let url = self.endpoint_template.replace("{identifier}", identifier);

    debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;

    trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    // TODO (autoparallel): This could be simplified just slightly
    match &self.response_format {
      ResponseFormat::Xml(config) => {
        let mut paper = config.process_response(&data).await?;
        paper.source = self.source.clone();
        paper.source_identifier = identifier.to_string();
        Ok(paper)
      },
      ResponseFormat::Json(config) => {
        let mut paper = config.process_response(&data).await?;
        paper.source = self.source.clone();
        paper.source_identifier = identifier.to_string();
        Ok(paper)
      },
    }
  }
}

/// Custom deserializer for Regex
fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
where D: serde::Deserializer<'de> {
  let s: String = String::deserialize(deserializer)?;
  Regex::new(&s).map_err(serde::de::Error::custom)
}

fn apply_transform(value: &str, transform: &Transform) -> Result<String> {
  match transform {
    Transform::Replace { pattern, replacement } => Regex::new(pattern)
      .map_err(|e| LearnerError::ApiError(format!("Invalid regex: {}", e)))
      .map(|re| re.replace_all(value, replacement.as_str()).into_owned()),
    Transform::Date { from_format, to_format } =>
      chrono::NaiveDateTime::parse_from_str(value, from_format)
        .map_err(|e| LearnerError::ApiError(format!("Invalid date: {}", e)))
        .map(|dt| dt.format(to_format).to_string()),
    Transform::Url { base, suffix } =>
      Ok(format!("{}{}", base.replace("{value}", value), suffix.as_deref().unwrap_or(""))),
  }
}
