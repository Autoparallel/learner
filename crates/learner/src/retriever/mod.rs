use std::collections::HashMap;

use super::*;

mod xml;
pub use xml::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperNew {
  /// The paper's full title
  pub title:             String,
  /// Complete list of paper authors with affiliations
  pub authors:           Vec<Author>,
  /// Full abstract or summary text
  pub abstract_text:     String,
  /// Publication or last update timestamp
  pub publication_date:  DateTime<Utc>,
  /// Source repository or system (arXiv, IACR, DOI)
  pub source:            String,
  /// Source-specific paper identifier
  pub source_identifier: String,
  /// Optional URL to PDF document
  pub pdf_url:           Option<String>,
  /// Optional DOI reference
  pub doi:               Option<String>,
}

#[async_trait]
pub trait ResponseProcessor: Send + Sync {
  async fn process_response(&self, data: &[u8]) -> Result<PaperNew>;
}

/// Supported response formats
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  /// XML responses
  #[serde(rename = "xml")]
  Xml(XmlConfig),
}

/// A paper retriever configuration and implementation
#[derive(Debug, Clone, Deserialize)]
pub struct Retriever {
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

/// Custom deserializer for Regex
fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
where D: serde::Deserializer<'de> {
  let s: String = String::deserialize(deserializer)?;
  Regex::new(&s).map_err(serde::de::Error::custom)
}

impl Retriever {
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
  pub async fn retrieve_paper(&self, input: &str) -> Result<PaperNew> {
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

    match &self.response_format {
      ResponseFormat::Xml(config) => {
        let mut paper = config.process_response(&data).await?;
        paper.source = self.source.clone();
        paper.source_identifier = identifier.to_string();
        Ok(paper)
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use std::fs;

  use super::*;

  const RETRIEVER_ARXIV_JSON: &str = "tests/.config/retriever_arxiv.json";

  #[test]
  fn test_arxiv_config_deserialization() {
    let config_str = fs::read_to_string(RETRIEVER_ARXIV_JSON).expect("Failed to read config file");

    let retriever: Retriever = serde_json::from_str(&config_str).expect("Failed to parse config");

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
    match &retriever.response_format {
      ResponseFormat::Xml(config) => {
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
          assert!(map.paths.contains(&"entry/id".to_string()));
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
      },
    }

    // Verify headers
    assert_eq!(
      retriever.headers.get("User-Agent").unwrap(),
      "learner/0.7.0 (https://github.com/yourusername/learner)"
    );
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }

  #[tokio::test]
  #[traced_test]
  async fn test_arxiv_retriever_integration() {
    let config_str = fs::read_to_string(RETRIEVER_ARXIV_JSON).expect(
      "Failed to read config
  file",
    );

    let retriever: Retriever = serde_json::from_str(&config_str).expect("Failed to parse config");

    // Test with a real arXiv paper
    let paper = retriever.retrieve_paper("2301.07041").await.unwrap();

    dbg!(&paper);
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
      fs::read_to_string("tests/.config/retriever_iacr.json").expect("Failed to read config file");

    let retriever: Retriever = serde_json::from_str(&config_str).expect("Failed to parse config");

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
    match &retriever.response_format {
      ResponseFormat::Xml(config) => {
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
          assert!(map.paths.contains(&"OAI-PMH/GetRecord/record/metadata/dc/title".to_string()));
        } else {
          panic!("Missing title field map");
        }
      },
    }

    // Verify headers
    assert_eq!(
      retriever.headers.get("User-Agent").unwrap(),
      "learner/0.7.0 (https://github.com/yourusername/learner)"
    );
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }

  #[tokio::test]
  async fn test_iacr_retriever_integration() {
    let config_str =
      fs::read_to_string("tests/.config/retriever_iacr.json").expect("Failed to read config file");

    let retriever: Retriever = serde_json::from_str(&config_str).expect("Failed to parse config");

    // Test with a real IACR paper
    let paper = retriever.retrieve_paper("2016/260").await.unwrap();
    dbg!(&paper);

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert!(!paper.abstract_text.is_empty());
    assert!(paper.pdf_url.is_some());
    assert_eq!(paper.source, "iacr");
    assert_eq!(paper.source_identifier, "2016/260");
  }
}
