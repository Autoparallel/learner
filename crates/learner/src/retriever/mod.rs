//! Paper retrieval and metadata extraction framework.
//!
//! This module provides a flexible, configuration-driven system for retrieving academic papers
//! and their metadata from various sources. It supports both XML and JSON-based APIs through
//! a common interface, with configurable field mapping and transformation capabilities.
//!
//! # Architecture
//!
//! The retriever system consists of several key components:
//!
//! - [`Retriever`]: Main entry point for paper retrieval operations
//! - [`RetrieverConfig`]: Configuration for specific paper sources
//! - [`ResponseFormat`]: Format-specific parsing logic (XML/JSON)
//! - [`ResponseProcessor`]: Trait for processing API responses
//!
//! # Features
//!
//! - Configuration-driven paper retrieval
//! - Support for multiple paper sources
//! - Flexible field mapping
//! - Custom field transformations
//! - Automatic source detection
//!
//! # Configuration
//!
//! Retrievers are configured using TOML files that specify:
//!
//! - API endpoints and authentication
//! - Field mapping rules
//! - Response format handling
//! - Identifier patterns
//!
//! # Examples
//!
//! Configure and use a retriever:
//!
//! ```no_run
//! use learner::retriever::{Retriever, RetrieverConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new retriever
//! let retriever =
//!   Retriever::new().with_config_file("config/arxiv.toml")?.with_config_file("config/doi.toml")?;
//!
//! // Retrieve a paper (automatically detects source)
//! let paper = retriever.get_paper("10.1145/1327452.1327492").await?;
//! println!("Retrieved paper: {}", paper.title);
//! # Ok(())
//! # }
//! ```
//!
//! Load multiple configurations:
//!
//! ```no_run
//! # use learner::retriever::Retriever;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load all TOML configs from a directory
//! let retriever = Retriever::new().with_config_dir("config/")?;
//!
//! // Retriever will automatically match source based on input format
//! let arxiv_paper = retriever.get_paper("2301.07041").await?;
//! let doi_paper = retriever.get_paper("10.1145/1327452.1327492").await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use super::*;

mod json;
mod xml;

/// Main entry point for paper retrieval operations.
///
/// The `Retriever` struct manages a collection of paper source configurations and
/// provides a unified interface for retrieving papers from any configured source.
/// It automatically detects the appropriate source based on the input identifier
/// format.
///
/// # Examples
///
/// ```no_run
/// # use learner::retriever::Retriever;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let retriever = Retriever::new().with_config_dir("config/")?;
///
/// // Retrieve papers from different sources
/// let paper1 = retriever.get_paper("2301.07041").await?; // arXiv
/// let paper2 = retriever.get_paper("2023/123").await?; // IACR
/// let paper3 = retriever.get_paper("10.1145/1327452.1327492").await?; // DOI
/// # Ok(())
/// # }
/// ```
#[derive(Default, Debug)]
pub struct Retriever {
  /// The collection of configurations used for this [`Retriever`].
  configs: HashMap<String, RetrieverConfig>,
}

/// Configuration for a specific paper source retriever.
///
/// This struct defines how to interact with a particular paper source's API,
/// including URL patterns, authentication, and response parsing rules.
///
/// # Examples
///
/// Example TOML configuration:
///
/// ```toml
/// name = "arxiv"
/// base_url = "http://export.arxiv.org/api/query"
/// pattern = "^\\d{4}\\.\\d{4,5}$"
/// source = "arxiv"
/// endpoint_template = "http://export.arxiv.org/api/query?id_list={identifier}"
///
/// [response_format]
/// type = "xml"
/// strip_namespaces = true
///
/// [response_format.field_maps]
/// title = { path = "entry/title" }
/// abstract = { path = "entry/summary" }
/// publication_date = { path = "entry/published" }
/// authors = { path = "entry/author/name" }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct RetrieverConfig {
  /// Name of this retriever configuration
  pub name:              String,
  /// Base URL for API requests
  pub base_url:          String,
  /// Regex pattern for matching and extracting paper identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:           Regex,
  /// Source identifier for papers from this retriever
  pub source:            String,
  /// Template for constructing API endpoint URLs
  pub endpoint_template: String,
  /// Format and parsing configuration for API responses
  pub response_format:   ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:           HashMap<String, String>,
}

/// Available response format handlers.
///
/// Specifies how to parse and extract paper metadata from API responses
/// in different formats.
///
/// # Examples
///
/// XML configuration:
/// ```toml
/// [response_format]
/// type = "xml"
/// strip_namespaces = true
///
/// [response_format.field_maps]
/// title = { path = "entry/title" }
/// ```
///
/// JSON configuration:
/// ```toml
/// [response_format]
/// type = "json"
///
/// [response_format.field_maps]
/// title = { path = "message/title/0" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  /// XML response parser configuration
  #[serde(rename = "xml")]
  Xml(xml::XmlConfig),
  /// JSON response parser configuration
  #[serde(rename = "json")]
  Json(json::JsonConfig),
}

/// Field mapping configuration.
///
/// Defines how to extract and transform specific fields from API responses.
///
/// # Examples
///
/// ```toml
/// [field_maps.title]
/// path = "entry/title"
/// transform = { type = "replace", pattern = "\\s+", replacement = " " }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct FieldMap {
  /// Path to field in response (e.g., JSON path or XPath)
  pub path:      String,
  /// Optional transformation to apply to extracted value
  #[serde(default)]
  pub transform: Option<Transform>,
}

/// Available field value transformations.
///
/// Transformations that can be applied to extracted field values
/// before constructing the final Paper object.
///
/// # Examples
///
/// ```toml
/// # Clean up whitespace
/// transform = { type = "replace", pattern = "\\s+", replacement = " " }
///
/// # Convert date format
/// transform = { type = "date", from_format = "%Y-%m-%d", to_format = "%Y-%m-%dT00:00:00Z" }
///
/// # Construct full URL
/// transform = { type = "url", base = "https://example.com/", suffix = ".pdf" }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Transform {
  /// Replace text using regex pattern
  Replace {
    /// Regular expression pattern to match
    pattern:     String,
    /// Text to replace matched patterns with
    replacement: String,
  },
  /// Convert between date formats
  Date {
    /// Source date format string using chrono syntax (e.g., "%Y-%m-%d")
    from_format: String,
    /// Target date format string using chrono syntax (e.g., "%Y-%m-%dT%H:%M:%SZ")
    to_format:   String,
  },
  /// Construct URL from parts
  Url {
    /// Base URL template, may contain {value} placeholder
    base:   String,
    /// Optional suffix to append to the URL (e.g., ".pdf")
    suffix: Option<String>,
  },
}

/// Trait for processing API responses into Paper objects.
///
/// Implementors of this trait handle the conversion of raw API response data
/// into structured Paper metadata. The trait is implemented separately for
/// different response formats (XML, JSON) to provide a unified interface for
/// paper retrieval.
///
/// # Examples
///
/// ```no_run
/// # use learner::retriever::{ResponseProcessor, Paper};
/// # use learner::Result;
/// struct CustomProcessor;
///
/// #[async_trait]
/// impl ResponseProcessor for CustomProcessor {
///   async fn process_response(&self, data: &[u8]) -> Result<Paper> {
///     // Parse response data and construct Paper
///     todo!()
///   }
/// }
/// ```
#[async_trait]
pub trait ResponseProcessor: Send + Sync {
  /// Process raw response data into a Paper object.
  ///
  /// # Arguments
  ///
  /// * `data` - Raw bytes from the API response
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - A fully populated Paper object
  /// - A LearnerError if parsing fails
  async fn process_response(&self, data: &[u8]) -> Result<Paper>;
}

impl Retriever {
  /// Creates a new empty retriever with no configurations.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// use learner::retriever::Retriever;
  ///
  /// let retriever = Retriever::new();
  /// ```
  pub fn new() -> Self { Self::default() }

  /// Adds a retriever configuration to this instance.
  ///
  /// This method configures support for a new paper source using the provided
  /// configuration. Multiple configurations can be added to support different sources.
  ///
  /// # Arguments
  ///
  /// * `config` - Configuration for the paper source
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::{Retriever, RetrieverConfig};
  /// # fn example(config: RetrieverConfig) {
  /// let retriever = Retriever::new().with_config(config);
  /// # }
  /// ```
  pub fn with_config(mut self, config: RetrieverConfig) {
    self.configs.insert(config.name.clone(), config);
  }

  /// Adds a retriever configuration from a TOML string.
  ///
  /// Parses the provided TOML string into a RetrieverConfig and adds it
  /// to this instance.
  ///
  /// # Arguments
  ///
  /// * `toml_str` - TOML configuration string
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The updated Retriever instance
  /// - A LearnerError if parsing fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retriever;
  /// let toml = r#"
  ///     name = "arxiv"
  ///     base_url = "http://export.arxiv.org/api/query"
  ///     pattern = "^\\d{4}\\.\\d{4,5}$"
  ///     source = "arxiv"
  ///     endpoint_template = "http://export.arxiv.org/api/query?id_list={identifier}"
  /// "#;
  ///
  /// let retriever = Retriever::new().with_config_str(toml)?;
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn with_config_str(mut self, toml_str: &str) -> Result<Self> {
    let config: RetrieverConfig = toml::from_str(toml_str)?;
    self.configs.insert(config.name.clone(), config);
    Ok(self)
  }

  /// Adds a retriever configuration from a TOML file.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to TOML configuration file
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The updated Retriever instance
  /// - A LearnerError if reading or parsing fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retriever;
  /// let retriever = Retriever::new().with_config_file("config/arxiv.toml")?;
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn with_config_file(self, path: impl AsRef<Path>) -> Result<Self> {
    let content = std::fs::read_to_string(path)?;
    self.with_config_str(&content)
  }

  /// Adds multiple configurations from a directory of TOML files.
  ///
  /// This method loads all .toml files from the specified directory and
  /// adds them as configurations.
  ///
  /// # Arguments
  ///
  /// * `dir` - Path to directory containing TOML configuration files
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The updated Retriever instance
  /// - A LearnerError if directory access or parsing fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retriever;
  /// let retriever = Retriever::new().with_config_dir("config/")?;
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
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

  /// Attempts to retrieve a paper using any matching configuration.
  ///
  /// This method tries to match the input against all configured retrievers
  /// and uses the first matching configuration to fetch the paper.
  ///
  /// # Arguments
  ///
  /// * `input` - Paper identifier or URL
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The retrieved Paper object
  /// - A LearnerError if no matching configuration is found or retrieval fails
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - No configuration matches the input format
  /// - Multiple configurations match ambiguously
  /// - Paper retrieval fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retriever;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let retriever = Retriever::new().with_config_dir("config/")?;
  ///
  /// // Retrieve from different sources
  /// let paper1 = retriever.get_paper("2301.07041").await?;
  /// let paper2 = retriever.get_paper("10.1145/1327452.1327492").await?;
  /// # Ok(())
  /// # }
  /// ```
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

impl RetrieverConfig {
  /// Extracts the canonical identifier from an input string.
  ///
  /// Uses the configured regex pattern to extract the standardized
  /// identifier from various input formats (URLs, DOIs, etc.).
  ///
  /// # Arguments
  ///
  /// * `input` - Input string containing a paper identifier
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The extracted identifier as a string slice
  /// - A LearnerError if the input doesn't match the pattern
  pub fn extract_identifier<'a>(&self, input: &'a str) -> Result<&'a str> {
    self
      .pattern
      .captures(input)
      .and_then(|cap| cap.get(1))
      .map(|m| m.as_str())
      .ok_or(LearnerError::InvalidIdentifier)
  }

  /// Retrieves a paper using this configuration.
  ///
  /// This method:
  /// 1. Extracts the canonical identifier
  /// 2. Constructs the API URL
  /// 3. Makes the HTTP request
  /// 4. Processes the response
  ///
  /// # Arguments
  ///
  /// * `input` - Paper identifier or URL
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The retrieved Paper object
  /// - A LearnerError if any step fails
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - The identifier cannot be extracted
  /// - The HTTP request fails
  /// - The response cannot be parsed
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

    let response_processor = match &self.response_format {
      ResponseFormat::Xml(config) => config as &dyn ResponseProcessor,
      ResponseFormat::Json(config) => config as &dyn ResponseProcessor,
    };
    let mut paper = response_processor.process_response(&data).await?;
    paper.source = self.source.clone();
    paper.source_identifier = identifier.to_string();
    Ok(paper)
  }
}

/// Custom deserializer for converting string patterns into Regex objects.
///
/// Used with serde's derive functionality to automatically deserialize
/// regex patterns from configuration files.
///
/// # Errors
///
/// Returns a deserialization error if the pattern is not a valid regular expression.
fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
where D: serde::Deserializer<'de> {
  let s: String = String::deserialize(deserializer)?;
  Regex::new(&s).map_err(serde::de::Error::custom)
}

/// Applies a transformation to a string value based on the transform type.
///
/// Handles three types of transformations:
/// - Regular expression replacements
/// - Date format conversions
/// - URL construction
///
/// # Errors
///
/// Returns a LearnerError if:
/// - Regex pattern is invalid
/// - Date parsing fails
/// - Date format is invalid
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
