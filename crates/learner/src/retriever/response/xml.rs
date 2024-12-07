//! XML response parser implementation.
//!
//! This module handles parsing of XML API responses into Paper objects using
//! configurable field mappings. It provides namespace handling and path-based
//! field extraction with optional transformations.
//!
//! # Example Configuration
//!
//! ```toml
//! [response_format]
//! type = "xml"
//! strip_namespaces = true
//!
//! [response_format.field_maps]
//! title = { path = "entry/title" }
//! abstract = { path = "entry/summary" }
//! publication_date = { path = "entry/published" }
//! authors = { path = "entry/author/name" }
//! ```

use quick_xml::{events::Event, Reader};

use super::*;

/// Configuration for processing XML API responses.
///
/// Provides field mapping rules and namespace handling options to extract
/// paper metadata from XML responses using path-based access patterns.
///
/// # Examples
///
/// ```no_run
/// # use std::collections::HashMap;
/// # use learner::retriever::{xml::XmlConfig, FieldMap};
/// let config = XmlConfig {
///   strip_namespaces: true,
///   field_maps:       HashMap::from([("title".to_string(), FieldMap {
///     path:      "entry/title".to_string(),
///     transform: None,
///   })]),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlConfig {
  /// Whether to remove XML namespace declarations and prefixes
  #[serde(default)]
  pub strip_namespaces: bool,
  // / XML path mappings for paper metadata fields
  // pub field_maps:       BTreeMap<String, FieldMap>,
}

impl ResponseProcessor for XmlConfig {
  fn process_response(&self, data: &[u8], resource_config: &ResourceConfig) -> Result<Resource> {
    // Handle namespace stripping
    let xml = if self.strip_namespaces {
      strip_xml_namespaces(&String::from_utf8_lossy(data))
    } else {
      String::from_utf8_lossy(data).to_string()
    };

    trace!("Processing XML response: {:#?}", &xml);

    // Extract raw XML content into JSON equivalent
    let json = convert_to_json(&xml);
    dbg!(process_json_value(&json, &self.field_maps, resource_config))
  }
}

use serde_json::{Map, Value};

pub fn convert_to_json(xml: &str) -> Value {
  let mut reader = Reader::from_str(xml);
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
