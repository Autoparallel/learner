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
#[derive(Debug, Clone, Deserialize)]
pub struct XmlConfig {
  /// Whether to remove XML namespace declarations and prefixes
  #[serde(default)]
  pub strip_namespaces: bool,
  /// XML path mappings for paper metadata fields
  pub field_maps:       BTreeMap<String, FieldMap>,
}

impl ResponseProcessor for XmlConfig {
  fn process_response(
    &self,
    data: &[u8],
    // retriever_config: &RetrieverConfig,
    resource_config: &ResourceConfig,
  ) -> Result<Resource> {
    todo!()
    // // Handle namespace stripping
    // let xml = if self.strip_namespaces {
    //   strip_xml_namespaces(&String::from_utf8_lossy(data))
    // } else {
    //   String::from_utf8_lossy(data).to_string()
    // };

    // trace!("Processing XML response: {:#?}", &xml);

    // // Extract raw XML content into path -> string mapping
    // let content = self.extract_content(&xml)?;
    // let mut resource = BTreeMap::new();

    // // Process each field according to the resource configuration
    // for field_def in &resource_config.fields {
    //   // Look up the field mapping from retriever config
    //   if let Some(field_map) = self.field_maps.get(&field_def.name) {
    //     // Try to get the raw value using configured path
    //     if let Some(raw_value) = content.get(&field_map.path) {
    //       // Apply any configured transformations
    //       let transformed_value = if let Some(transform) = &field_map.transform {
    //         apply_transform(raw_value, transform)?
    //       } else {
    //         raw_value.clone()
    //       };

    //       // Convert string to appropriate TOML type based on field definition
    //       let value = match field_def.field_type.as_str() {
    //         "string" => Value::String(transformed_value),
    //         "datetime" => {
    //           let dt = DateTime::parse_from_rfc3339(&transformed_value).map_err(|e| {
    //             LearnerError::ApiError(format!(
    //               "Invalid date format for field '{}': {}",
    //               field_def.name, e
    //             ))
    //           })?;
    //           Value::String(chrono_to_toml_datetime(dt.with_timezone(&Utc)))
    //         },
    //         "array" => {
    //           // For arrays, split on semicolon and create string array
    //           let values =
    //             transformed_value.split(';').map(|s|
    // Value::String(s.trim().to_string())).collect();           Value::Array(values)
    //         },
    //         // Add other type conversions as needed
    //         unsupported =>
    //           return Err(LearnerError::ApiError(format!(
    //             "Unsupported field type '{}' for field '{}'",
    //             unsupported, field_def.name
    //           ))),
    //       };
    //       resource.insert(field_def.name.clone(), value);
    //     } else if field_def.required {
    //       // Field was required but not found in response
    //       return Err(LearnerError::ApiError(format!(
    //         "Required field '{}' not found in response",
    //         field_def.name
    //       )));
    //     } else if let Some(default) = &field_def.default {
    //       // Use default value if available
    //       resource.insert(field_def.name.clone(), default.clone());
    //     }
    //   }
    // }

    // Ok(resource)
  }
}

impl XmlConfig {
  /// Extracts field values from XML content using path-based navigation.
  ///
  /// Builds a map of path -> value pairs by walking the XML tree and
  /// tracking element paths. Handles nested elements and text content.
  ///
  /// # Arguments
  ///
  /// * `xml` - XML content as string
  ///
  /// # Returns
  ///
  /// Returns a HashMap mapping XML paths to their text content.
  fn extract_content(&self, xml: &str) -> Result<BTreeMap<String, String>> {
    // let ser_xml: Vec<(String, Value)> = quick_xml::de::from_str(xml).unwrap();
    // quick_xml::de::
    // dbg!(ser_xml);

    let mut reader = Reader::from_str(xml);

    let mut map = BTreeMap::new();

    let mut current_key = Vec::new();

    while let Ok(event) = reader.read_event() {
      match event {
        Event::Start(ref e) => {
          let tag = String::from_utf8_lossy(e.trim_ascii()).to_string();
          current_key.push(tag);
        },
        Event::Text(e) => {
          let value = e.unescape().unwrap_or_default().trim().to_string();
          if !value.is_empty() {
            let key = current_key.join(".");
            map
              .entry(key)
              .and_modify(|existing| {
                if let Value::Array(arr) = existing {
                  arr.push(Value::String(value.clone()));
                } else {
                  *existing = Value::Array(vec![existing.clone(), Value::String(value.clone())]);
                }
              })
              .or_insert(Value::String(value));
          }
        },
        Event::End(_) => {
          current_key.pop();
        },
        Event::Eof => break,
        _ => (),
      }
    }

    dbg!(map);

    ////////////////////////////////////////////////////

    let mut reader = Reader::from_str(xml);
    let mut content = BTreeMap::new();
    let mut path_stack = Vec::new();
    let mut buf = Vec::new();

    while let Ok(event) = reader.read_event_into(&mut buf) {
      match event {
        Event::Start(e) => {
          path_stack.push(String::from_utf8_lossy(e.name().as_ref()).into_owned());
        },
        Event::Text(e) =>
          if let Ok(text) = e.unescape() {
            let text = text.trim();
            if !text.is_empty() {
              content.insert(path_stack.join("/"), text.to_string());
            }
          },
        Event::End(_) => {
          path_stack.pop();
        },
        Event::Eof => break,
        _ => (),
      }
      buf.clear();
    }

    Ok(content)
  }
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
