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

mod config;
mod response;

pub use config::*;
pub use response::*;

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
#[derive(Default, Debug, Clone)]
pub struct Retriever {
  /// The collection of configurations used for this [`Retriever`].
  configs: HashMap<String, RetrieverConfig>,
}

impl Configurable for Retriever {
  type Config = RetrieverConfig;

  fn insert(&mut self, config_name: String, config: Self::Config) {
    self.configs.insert(config_name, config);
  }
}

impl Retriever {
  /// Checks whether the retreivers map is empty.
  ///
  /// This is useful for handling the case where no retreivers are specified and
  /// we wish to inform the user.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retriever;
  /// # use learner::error::LearnerError;
  ///
  /// # fn check_is_empty() -> Result<(), LearnerError> {
  /// let retriever = Retriever::new();
  ///
  /// if retriever.is_empty() {
  ///   return Err(LearnerError::Config("No retriever configured.".to_string()));
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub fn is_empty(&self) -> bool { self.configs.is_empty() }
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

  /// Sanitizes and normalizes a paper identifier using configured retrieval patterns.
  ///
  /// This function processes an input string (which could be a URL, DOI, arXiv ID, etc.)
  /// and attempts to match it against configured paper source patterns to extract a
  /// standardized source and identifier pair.
  ///
  /// # Arguments
  ///
  /// * `input` - The input string to sanitize. Can be:
  ///   - A full URL (e.g., "https://arxiv.org/abs/2301.07041")
  ///   - A DOI (e.g., "10.1145/1327452.1327492")
  ///   - An arXiv ID (e.g., "2301.07041" or "math.AG/0601001")
  ///   - An IACR ID (e.g., "2023/123")
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing:
  /// - `Ok((String, String))` - A tuple of (source, identifier) where:
  ///   - source: The normalized source name (e.g., "arxiv", "doi", "iacr")
  ///   - identifier: The extracted canonical identifier
  /// - `Err(LearnerError)` with either:
  ///   - `InvalidIdentifier` if no configured pattern matches the input
  ///   - `AmbiguousIdentifier` if multiple patterns match the input
  ///
  /// # Examples
  ///
  /// ```
  /// # use learner::retriever::Retriever;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let retriever = Retriever::new().with_config_dir("config/")?;
  ///
  /// // Sanitize an arXiv URL
  /// let (source, id) = retriever.sanitize_identifier("https://arxiv.org/abs/2301.07041")?;
  /// assert_eq!(source, "arxiv");
  /// assert_eq!(id, "2301.07041");
  ///
  /// // Sanitize a bare DOI
  /// let (source, id) = retriever.sanitize_identifier("10.1145/1327452.1327492")?;
  /// assert_eq!(source, "doi");
  /// assert_eq!(id, "10.1145/1327452.1327492");
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// Will return `LearnerError::InvalidIdentifier` if:
  /// - The input string doesn't match any configured source patterns
  /// - The input matches a pattern but the identifier extraction fails
  ///
  /// Will return `LearnerError::AmbiguousIdentifier` if:
  /// - The input matches multiple source patterns
  /// - Includes the list of matching sources in the error
  ///
  /// # Implementation Notes
  ///
  /// The function:
  /// 1. Checks the input against all configured source patterns
  /// 2. Attempts to extract identifiers from all matching patterns
  /// 3. Validates that exactly one pattern matched
  /// 4. Returns the normalized source and identifier
  ///
  /// The matching process uses regex patterns defined in the retriever configuration
  /// files, allowing for flexible addition of new paper sources.
  pub fn sanitize_identifier(&self, input: &str) -> Result<(String, String)> {
    let mut matches = Vec::new();

    for config in self.configs.values() {
      if config.pattern.is_match(input) {
        matches.push((config.source.clone(), config.extract_identifier(input)?.to_string()));
      }
    }

    match matches.len() {
      0 => Err(LearnerError::InvalidIdentifier),
      1 => Ok(matches.remove(0)),
      _ => Err(LearnerError::AmbiguousIdentifier(
        matches.into_iter().map(|(source, _)| source).collect(),
      )),
    }
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
