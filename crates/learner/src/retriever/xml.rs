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
/// # use learner::retriever::{XmlConfig, FieldMap};
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
  pub field_maps:       HashMap<String, FieldMap>,
}

#[async_trait]
impl ResponseProcessor for XmlConfig {
  /// Processes an XML API response into a Paper object.
  ///
  /// Extracts paper metadata from the XML response using configured field mappings.
  /// Handles namespace stripping if enabled and validates required fields.
  ///
  /// # Arguments
  ///
  /// * `data` - Raw XML response bytes
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - A populated Paper object
  /// - A LearnerError if parsing fails or required fields are missing
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - XML parsing fails
  /// - Required fields are missing
  /// - Field values are invalid or cannot be transformed
  async fn process_response(&self, data: &[u8]) -> Result<Paper> {
    let xml = if self.strip_namespaces {
      strip_xml_namespaces(&String::from_utf8_lossy(data))
    } else {
      String::from_utf8_lossy(data).to_string()
    };

    let content = self.extract_content(&xml)?;

    // Helper function to extract and transform field
    let get_field = |name: &str| -> Result<String> {
      let map = self
        .field_maps
        .get(name)
        .ok_or_else(|| LearnerError::ApiError(format!("Missing field mapping for {}", name)))?;

      let value = content
        .get(&map.path)
        .ok_or_else(|| LearnerError::ApiError(format!("No content found for {}", name)))?;

      if let Some(transform) = &map.transform {
        apply_transform(value, transform)
      } else {
        Ok(value.clone())
      }
    };

    let title = get_field("title")?;
    let abstract_text = get_field("abstract")?;
    let publication_date = chrono::DateTime::parse_from_rfc3339(&get_field("publication_date")?)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| LearnerError::ApiError(format!("Invalid date format: {}", e)))?;

    // Extract authors
    let authors = if let Some(map) = self.field_maps.get("authors") {
      let names: Vec<Author> = content
        .get(&map.path)
        .map(|s| {
          s.split(';')
            .map(|name| Author {
              name:        name.trim().to_string(),
              affiliation: None,
              email:       None,
            })
            .collect()
        })
        .unwrap_or_default();
      if names.is_empty() {
        return Err(LearnerError::ApiError("No authors found".to_string()));
      }
      names
    } else {
      return Err(LearnerError::ApiError("Missing authors mapping".to_string()));
    };

    // Optional fields
    let pdf_url = self.field_maps.get("pdf_url").and_then(|map| {
      content.get(&map.path).map(|url| {
        if let Some(transform) = &map.transform {
          apply_transform(url, transform).ok().unwrap_or_else(|| url.clone())
        } else {
          url.clone()
        }
      })
    });

    let doi = self.field_maps.get("doi").and_then(|map| content.get(&map.path)).map(String::from);

    Ok(Paper {
      title,
      authors,
      abstract_text,
      publication_date,
      source: String::new(),
      source_identifier: String::new(),
      pdf_url,
      doi,
    })
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
  fn extract_content(&self, xml: &str) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut content = HashMap::new();
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
