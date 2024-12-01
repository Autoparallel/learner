use super::*;

pub mod json;
pub mod xml;

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
  // New transform for combining fields
  CombineFields {
    fields:      Vec<String>,            // Fields to combine for name
    inner_paths: Option<Vec<InnerPath>>, // Additional paths to collect
  },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InnerPath {
  pub new_key_name: String,
  pub path:         String,
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
/// # use learner::{retriever::ResponseProcessor, resource::Paper};
/// # use learner::error::LearnerError;
/// struct CustomProcessor;
///
/// #[async_trait::async_trait]
/// impl ResponseProcessor for CustomProcessor {
///   async fn process_response(&self, data: &[u8]) -> Result<Paper, LearnerError> {
///     // Parse response data and construct Paper
///     todo!()
///   }
/// }
/// ```
// #[async_trait]
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
  fn process_response(
    &self,
    data: &[u8],
    // retriever_config: RetrieverConfig,
    resource_config: &ResourceConfig,
  ) -> Result<Resource>;
}
