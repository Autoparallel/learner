//! JSON response parser implementation.
//!
//! This module handles parsing of JSON API responses into Paper objects using
//! configurable field mappings. It supports flexible path-based field extraction
//! with optional transformations.
//!
//! # Example Configuration
//!
//! ```toml
//! [response_format]
//! type = "json"
//!
//! [response_format.field_maps]
//! title = { path = "message/title/0" }
//! abstract = { path = "message/abstract" }
//! publication_date = { path = "message/published-print/date-parts/0" }
//! authors = { path = "message/author" }
//! ```

use serde_json::Value;

use super::*;

/// Configuration for processing JSON API responses.
///
/// Provides field mapping rules to extract paper metadata from JSON responses
/// using path-based access patterns.
///
/// # Examples
///
/// ```no_run
/// # use std::collections::HashMap;
/// # use learner::retriever::{json::JsonConfig, FieldMap};
/// let config = JsonConfig {
///   field_maps: HashMap::from([("title".to_string(), FieldMap {
///     path:      "message/title/0".to_string(),
///     transform: None,
///   })]),
/// };
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  /// JSON path mappings for paper metadata fields
  pub field_maps: HashMap<String, FieldMap>,
}

#[async_trait]
impl ResponseProcessor for JsonConfig {
  /// Processes a JSON API response into a Paper object.
  ///
  /// Extracts paper metadata from the JSON response using configured field mappings.
  /// Required fields (title, abstract, publication date, authors) must be present
  /// and valid.
  ///
  /// # Arguments
  ///
  /// * `data` - Raw JSON response bytes
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
  /// - JSON parsing fails
  /// - Required fields are missing
  /// - Field values are invalid or cannot be transformed
  async fn process_response(&self, data: &[u8]) -> Result<Paper> {
    let json: Value = serde_json::from_slice(data)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    trace!("Processing JSON response: {}", serde_json::to_string_pretty(&json).unwrap());

    let title = self.extract_field(&json, "title")?;
    let abstract_text = self.extract_field(&json, "abstract")?;
    let publication_date =
      chrono::DateTime::parse_from_rfc3339(&self.extract_field(&json, "publication_date")?)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| LearnerError::ApiError(format!("Invalid date format: {}", e)))?;

    let authors = if let Some(map) = self.field_maps.get("authors") {
      self.extract_authors(&json, map)?
    } else {
      return Err(LearnerError::ApiError("Missing authors mapping".to_string()));
    };

    let pdf_url = self.field_maps.get("pdf_url").and_then(|map| {
      self.get_by_path(&json, &map.path).map(|url| {
        if let Some(transform) = &map.transform {
          apply_transform(&url, transform).ok().unwrap_or_else(|| url.clone())
        } else {
          url.clone()
        }
      })
    });

    let doi = self
      .field_maps
      .get("doi")
      .and_then(|map| self.get_by_path(&json, &map.path))
      .map(String::from);

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

impl JsonConfig {
  /// Extracts a single field value using configured mapping.
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Field mapping is missing
  /// - Field value cannot be found
  /// - Value transformation fails
  fn extract_field(&self, json: &Value, field: &str) -> Result<String> {
    let map = self
      .field_maps
      .get(field)
      .ok_or_else(|| LearnerError::ApiError(format!("Missing field mapping for {}", field)))?;

    let value = self
      .get_by_path(json, &map.path)
      .ok_or_else(|| LearnerError::ApiError(format!("No content found for {}", field)))?;

    if let Some(transform) = &map.transform {
      apply_transform(&value, transform)
    } else {
      Ok(value)
    }
  }

  /// Retrieves a value from JSON using slash-separated path.
  ///
  /// Supports both object key and array index access:
  /// - "message/title" -> object access
  /// - "authors/0/name" -> array access
  ///
  /// Handles string, array, and number values with appropriate conversion.
  fn get_by_path(&self, json: &Value, path: &str) -> Option<String> {
    let mut current = json;

    for part in path.split('/') {
      current = if let Ok(index) = part.parse::<usize>() {
        // Handle numeric indices for arrays
        current.as_array()?.get(index)?
      } else {
        // Handle regular object keys
        current.get(part)?
      };
    }

    match current {
      Value::String(s) => Some(s.clone()),
      Value::Array(arr) if !arr.is_empty() => arr[0].as_str().map(String::from),
      Value::Number(n) => Some(n.to_string()),
      _ => current.as_str().map(String::from),
    }
  }

  /// Extracts and processes author information from JSON.
  ///
  /// Handles author objects with given/family name fields and optional
  /// affiliation information. Expects authors as an array matching the
  /// configured path.
  ///
  /// # Errors
  ///
  /// Returns error if no valid authors are found in the response.
  fn extract_authors(&self, json: &Value, map: &FieldMap) -> Result<Vec<Author>> {
    let authors = if let Some(Value::Array(arr)) = get_path_value(json, &map.path) {
      arr
        .iter()
        .filter_map(|author| {
          let name = match (author.get("given"), author.get("family")) {
            (Some(given), Some(family)) => {
              format!("{} {}", given.as_str().unwrap_or(""), family.as_str().unwrap_or(""))
            },
            (Some(given), None) => given.as_str()?.to_string(),
            (None, Some(family)) => family.as_str()?.to_string(),
            (None, None) => return None,
          };

          let affiliation = author
            .get("affiliation")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|aff| aff.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

          Some(Author { name, affiliation, email: None })
        })
        .collect()
    } else {
      Vec::new()
    };

    if authors.is_empty() {
      Err(LearnerError::ApiError("No authors found".to_string()))
    } else {
      Ok(authors)
    }
  }
}

/// Helper function to navigate JSON structure using path.
///
/// Similar to get_by_path but returns raw JSON Value instead of
/// converted string.
fn get_path_value<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
  let mut current = json;
  for part in path.split('/') {
    current = current.get(part)?;
  }
  Some(current)
}
