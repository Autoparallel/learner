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

#[cfg(test)]
mod tests {
  use std::fs;

  use super::*;

  #[test]
  fn test_arxiv_config_deserialization() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_arxiv.toml").expect("Failed to read config file");

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Verify basic fields
    assert_eq!(retriever.name, "arxiv");
    assert_eq!(retriever.base_url, "http://export.arxiv.org");
    assert_eq!(retriever.source, "arxiv");

    // Test pattern matching
    assert!(retriever.pattern.is_match("2301.07041"));
    assert!(retriever.pattern.is_match("math.AG/0601001"));
    assert!(retriever.pattern.is_match("https://arxiv.org/abs/2301.07041"));
    assert!(retriever.pattern.is_match("https://arxiv.org/pdf/2301.07041"));
    assert!(retriever.pattern.is_match("https://arxiv.org/abs/math.AG/0601001"));

    // Test identifier extraction
    assert_eq!(retriever.extract_identifier("2301.07041").unwrap(), "2301.07041");
    assert_eq!(
      retriever.extract_identifier("https://arxiv.org/abs/2301.07041").unwrap(),
      "2301.07041"
    );
    assert_eq!(retriever.extract_identifier("math.AG/0601001").unwrap(), "math.AG/0601001");

    // Verify response format

    if let ResponseFormat::Xml(config) = &retriever.response_format {
      assert!(config.strip_namespaces);

      // Verify field mappings
      let field_maps = &config.field_maps;
      assert!(field_maps.contains_key("title"));
      assert!(field_maps.contains_key("abstract"));
      assert!(field_maps.contains_key("authors"));
      assert!(field_maps.contains_key("publication_date"));
      assert!(field_maps.contains_key("pdf_url"));

      // Verify PDF transform
      if let Some(map) = field_maps.get("pdf_url") {
        match &map.transform {
          Some(Transform::Replace { pattern, replacement }) => {
            assert_eq!(pattern, "/abs/");
            assert_eq!(replacement, "/pdf/");
          },
          _ => panic!("Expected Replace transform for pdf_url"),
        }
      } else {
        panic!("Missing pdf_url field map");
      }
    } else {
      panic!("Expected an XML configuration, but did not get one.")
    }

    // Verify headers
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }

  #[tokio::test]
  async fn test_arxiv_retriever_integration() {
    let config_str = fs::read_to_string("tests/.config/retriever_arxiv.toml").expect(
      "Failed to read config
  file",
    );

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Test with a real arXiv paper
    let paper = retriever.retrieve_paper("2301.07041").await.unwrap();

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert!(!paper.abstract_text.is_empty());
    assert!(paper.pdf_url.is_some());
    assert_eq!(paper.source, "arxiv");
    assert_eq!(paper.source_identifier, "2301.07041");
  }

  #[test]
  fn test_iacr_config_deserialization() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_iacr.toml").expect("Failed to read config file");

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Verify basic fields
    assert_eq!(retriever.name, "iacr");
    assert_eq!(retriever.base_url, "https://eprint.iacr.org");
    assert_eq!(retriever.source, "iacr");

    // Test pattern matching
    let test_cases = [
      ("2016/260", true),
      ("2023/123", true),
      ("https://eprint.iacr.org/2016/260", true),
      ("https://eprint.iacr.org/2016/260.pdf", true),
      ("invalid/format", false),
      ("https://wrong.url/2016/260", false),
    ];

    for (input, expected) in test_cases {
      assert_eq!(
        retriever.pattern.is_match(input),
        expected,
        "Pattern match failed for input: {}",
        input
      );
    }

    // Test identifier extraction
    assert_eq!(retriever.extract_identifier("2016/260").unwrap(), "2016/260");
    assert_eq!(
      retriever.extract_identifier("https://eprint.iacr.org/2016/260").unwrap(),
      "2016/260"
    );
    assert_eq!(
      retriever.extract_identifier("https://eprint.iacr.org/2016/260.pdf").unwrap(),
      "2016/260"
    );

    // Verify response format
    if let ResponseFormat::Xml(config) = &retriever.response_format {
      assert!(config.strip_namespaces);

      // Verify field mappings
      let field_maps = &config.field_maps;
      assert!(field_maps.contains_key("title"));
      assert!(field_maps.contains_key("abstract"));
      assert!(field_maps.contains_key("authors"));
      assert!(field_maps.contains_key("publication_date"));
      assert!(field_maps.contains_key("pdf_url"));

      // Verify OAI-PMH paths
      if let Some(map) = field_maps.get("title") {
        assert!(map.path.contains(&"OAI-PMH/GetRecord/record/metadata/dc/title".to_string()));
      } else {
        panic!("Missing title field map");
      }
    } else {
      panic!("Expected an XML configuration, but did not get one.")
    }

    // Verify headers
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }

  #[tokio::test]
  async fn test_iacr_retriever_integration() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_iacr.toml").expect("Failed to read config file");

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Test with a real IACR paper
    let paper = retriever.retrieve_paper("2016/260").await.unwrap();

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert!(!paper.abstract_text.is_empty());
    assert!(paper.pdf_url.is_some());
    assert_eq!(paper.source, "iacr");
    assert_eq!(paper.source_identifier, "2016/260");
  }

  #[test]
  fn test_doi_config_deserialization() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_doi.toml").expect("Failed to read config file");

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Verify basic fields
    assert_eq!(retriever.name, "doi");
    assert_eq!(retriever.base_url, "https://api.crossref.org/works");
    assert_eq!(retriever.source, "doi");

    // Test pattern matching
    let test_cases = [
      ("10.1145/1327452.1327492", true),
      ("https://doi.org/10.1145/1327452.1327492", true),
      ("invalid-doi", false),
      ("https://wrong.url/10.1145/1327452.1327492", false),
    ];

    for (input, expected) in test_cases {
      assert_eq!(
        retriever.pattern.is_match(input),
        expected,
        "Pattern match failed for input: {}",
        input
      );
    }

    // Test identifier extraction
    assert_eq!(
      retriever.extract_identifier("10.1145/1327452.1327492").unwrap(),
      "10.1145/1327452.1327492"
    );
    assert_eq!(
      retriever.extract_identifier("https://doi.org/10.1145/1327452.1327492").unwrap(),
      "10.1145/1327452.1327492"
    );

    // Verify response format
    match &retriever.response_format {
      ResponseFormat::Json(config) => {
        // Verify field mappings
        let field_maps = &config.field_maps;
        assert!(field_maps.contains_key("title"));
        assert!(field_maps.contains_key("abstract"));
        assert!(field_maps.contains_key("authors"));
        assert!(field_maps.contains_key("publication_date"));
        assert!(field_maps.contains_key("pdf_url"));
        assert!(field_maps.contains_key("doi"));
      },
      _ => panic!("Expected JSON response format"),
    }
  }

  #[tokio::test]
  async fn test_doi_retriever_integration() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_doi.toml").expect("Failed to read config file");

    let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

    // Test with a real DOI paper
    let paper = retriever.retrieve_paper("10.1145/1327452.1327492").await.unwrap();

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert!(!paper.abstract_text.is_empty());
    assert!(paper.pdf_url.is_some());
    assert_eq!(paper.source, "doi");
    assert_eq!(paper.source_identifier, "10.1145/1327452.1327492");
    assert!(paper.doi.is_some());
  }
}
