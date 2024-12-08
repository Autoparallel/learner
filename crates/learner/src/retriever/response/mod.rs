use super::*;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  /// XML response parser configuration
  #[serde(rename = "xml")]
  Xml {
    #[serde(default)]
    strip_namespaces: bool,
  },
  /// JSON response parser configuration
  #[serde(rename = "json")]
  Json,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
  Compose {
    /// List of field paths or direct values to combine
    sources: Vec<Source>,
    /// How to format the combined result
    format:  ComposeFormat,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Source {
  /// Path to a field to extract
  #[serde(rename = "path")]
  Path(String),
  /// A literal string value
  #[serde(rename = "literal")]
  Literal(String),
  /// A field mapping with a new key name
  #[serde(rename = "key_value")]
  KeyValue { key: String, path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComposeFormat {
  /// Join fields with a delimiter
  Join { delimiter: String },
  /// Create an object with key-value pairs
  Object,
  /// Create an array of objects with specified structure
  ArrayOfObjects {
    /// How to structure each object
    template: BTreeMap<String, String>,
  },
}
