use quick_xml::{events::Event, Reader};

use super::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMap {
  /// Path to field in response (e.g., JSON path or XPath)
  pub path:      String,
  /// Optional transformation to apply to extracted value
  #[serde(default)]
  pub transform: Option<Transform>,
}

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
  Object { template: BTreeMap<String, String> },
  /// Create an array of objects with specified structure
  ArrayOfObjects {
    /// How to structure each object
    template: BTreeMap<String, String>,
  },
}

pub fn xml_to_json(data: &[u8], strip_namespaces: bool) -> Value {
  // Handle namespace stripping
  let xml = if strip_namespaces {
    strip_xml_namespaces(&String::from_utf8_lossy(data))
  } else {
    String::from_utf8_lossy(data).to_string()
  };

  trace!("Processing XML response: {:#?}", &xml);
  let mut reader = Reader::from_str(&xml);
  let mut stack = Vec::new();
  let mut current = Map::new();

  while let Ok(event) = reader.read_event() {
    match event {
      Event::Start(ref e) => {
        let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

        // Create new object for this element
        let mut new_obj = Map::new();

        // Handle attributes
        for attr in e.attributes().flatten() {
          if let Ok(key) = String::from_utf8(attr.key.as_ref().to_vec()) {
            if let Ok(value) = attr.unescape_value() {
              new_obj.insert(format!("@{}", key), Value::String(value.into_owned()));
            }
          }
        }

        // Add this element to its parent
        match current.get_mut(&tag) {
          Some(Value::Array(_)) => {
            // Element already exists as array, push onto it later
            stack.push((tag, current, true));
          },
          Some(_) => {
            // Element exists but not as array, convert to array
            let existing = current.remove(&tag).unwrap();
            current.insert(tag.clone(), Value::Array(vec![existing]));
            stack.push((tag, current, true));
          },
          None => {
            // First occurrence of this element
            stack.push((tag, current, false));
          },
        }

        current = new_obj;
      },
      Event::Text(e) => {
        if let Ok(txt) = e.unescape() {
          let text = txt.trim();
          if !text.is_empty() {
            if current.is_empty() {
              // No attributes, just text content
              current.insert("$text".to_string(), Value::String(text.to_string()));
            } else {
              // Has attributes, add text alongside them
              current.insert("$text".to_string(), Value::String(text.to_string()));
            }
          }
        }
      },
      Event::End(_) => {
        if let Some((tag, mut parent, is_array)) = stack.pop() {
          // Simplify if only text content
          let value = if current.len() == 1 && current.contains_key("$text") {
            current.remove("$text").unwrap()
          } else {
            Value::Object(current)
          };

          // Add to parent according to array status
          if is_array {
            if let Some(Value::Array(arr)) = parent.get_mut(&tag) {
              arr.push(value);
            }
          } else {
            parent.insert(tag, value);
          }

          current = parent;
        }
      },
      Event::Eof => break,
      _ => (),
    }
  }

  dbg!(Value::Object(current))
}

/// Removes XML namespace declarations and prefixes from content.
///
/// Strips both namespace declarations (xmlns attributes) and namespace
/// prefixes from element names for simpler path-based access.
///
/// # Arguments
///
/// * `xml` - Raw XML content
///
/// # Returns
///
/// XML content with namespaces removed
fn strip_xml_namespaces(xml: &str) -> String {
  let re = regex::Regex::new(r#"xmlns(?::\w+)?="[^"]*""#).unwrap();
  let mut result = re.replace_all(xml, "").to_string();
  result = result.replace("oai_dc:", "").replace("dc:", "");
  result
}
