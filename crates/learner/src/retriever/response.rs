use quick_xml::{events::Event, Reader};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
  #[serde(rename = "xml")]
  Xml {
    /// Whether to strip XML namespace declarations and prefixes
    #[serde(default)]
    strip_namespaces: bool,
    /// Whether to clean content by removing markup tags and normalizing whitespace
    #[serde(default)]
    clean_content:    bool,
  },

  #[serde(rename = "json")]
  Json {
    /// Whether to clean string values by removing markup and normalizing content
    #[serde(default)]
    clean_content: bool,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mapping {
  // A single path string - most common case
  Path(String),

  // Multiple paths to join with optional delimiter
  Join {
    paths: Vec<String>,
    #[serde(default = "default_delimiter")]
    with:  String,
  },

  // Map values into new structures
  Map {
    from: Option<String>,
    map:  BTreeMap<String, Mapping>,
  },
}

fn default_delimiter() -> String { "".to_string() }

pub fn xml_to_json(data: &str) -> Value {
  trace!("Processing XML response: {:#?}", data);
  let mut reader = Reader::from_str(data);
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
pub fn strip_xml_namespaces(xml: &str) -> String {
  let re = regex::Regex::new(r#"xmlns(?::\w+)?="[^"]*""#).unwrap();
  let mut result = re.replace_all(xml, "").to_string();
  result = result.replace("oai_dc:", "").replace("dc:", "");

  result
}

pub fn clean_value(value: &mut Value) {
  match value {
    // Clean string content
    Value::String(s) =>
      if s.contains('<') || s.contains('\n') {
        *s = clean_content(s);
      },
    // Recursively clean arrays and objects
    Value::Array(arr) =>
      for item in arr {
        clean_value(item);
      },
    Value::Object(obj) =>
      for (_, val) in obj {
        clean_value(val);
      },
    _ => (), // Other value types don't need cleaning
  }
}

pub fn clean_content(s: &str) -> String {
  let mut cleaned = s.to_string();

  // Remove various markup tags
  let tag_patterns = [
    // JATS tags
    r"<jats:[^>]+>",
    r"</jats:[^>]+>",
    // Generic XML tags
    r"<[^>]+>",
    // Any remaining XML-like tags
    r"</?[a-zA-Z][^>]*>",
  ];

  for pattern in &tag_patterns {
    if let Ok(re) = Regex::new(pattern) {
      cleaned = re.replace_all(&cleaned, "").to_string();
    }
  }

  // Normalize whitespace
  if let Ok(re) = Regex::new(r"\s+") {
    cleaned = re.replace_all(cleaned.trim(), " ").to_string();
  }

  cleaned
}
