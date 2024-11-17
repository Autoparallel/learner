use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::*;

// TODO (autoparallel): We should deserialize into a regex for the transforms and likely make it
// static too so it isn't rebuilt every time

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  /// JSON path mappings for fields
  pub field_maps: HashMap<String, FieldMap>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldMap {
  /// JSON paths to extract values from
  pub paths:          Vec<String>,
  /// How to join multiple values if found
  #[serde(default)]
  pub join_separator: Option<String>,
  /// Optional transformation to apply
  #[serde(default)]
  pub transform:      Option<Transform>,
}

#[async_trait]
impl ResponseProcessor for JsonConfig {
  async fn process_response(&self, data: &[u8]) -> Result<PaperNew> {
    let json: Value = serde_json::from_slice(data)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    trace!("Processing JSON response: {}", serde_json::to_string_pretty(&json).unwrap());

    // Extract required fields
    let title = self.extract_field(&json, "title")?;
    let abstract_text = self.extract_field(&json, "abstract")?;
    let publication_date_str = self.extract_field(&json, "publication_date")?;

    let publication_date = chrono::DateTime::parse_from_rfc3339(&publication_date_str)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| LearnerError::ApiError(format!("Invalid date format: {}", e)))?;

    // Extract authors
    let authors = if let Some(map) = self.field_maps.get("authors") {
      self.extract_authors(&json, map)?
    } else {
      return Err(LearnerError::ApiError("Missing authors mapping".to_string()));
    };

    // Extract optional fields
    let pdf_url = if let Some(map) = self.field_maps.get("pdf_url") {
      self.extract_optional_field(&json, map)?
    } else {
      None
    };

    let doi = if let Some(map) = self.field_maps.get("doi") {
      self.extract_optional_field(&json, map)?
    } else {
      None
    };

    Ok(PaperNew {
      title,
      authors,
      abstract_text,
      publication_date,
      source: String::new(),            // Will be filled by retriever
      source_identifier: String::new(), // Will be filled by retriever
      pdf_url,
      doi,
    })
  }
}

impl JsonConfig {
  fn extract_field(&self, json: &Value, field: &str) -> Result<String> {
    let map = self
      .field_maps
      .get(field)
      .ok_or_else(|| LearnerError::ApiError(format!("Missing field mapping for {}", field)))?;

    let mut values = Vec::new();
    for path in &map.paths {
      let mut current = json;
      for part in path.split('/') {
        current = match current.get(part) {
          Some(value) => value,
          None => continue,
        };

        // Handle arrays specially
        if current.is_array() {
          if let Some(first) = current.as_array().unwrap().first() {
            // For arrays of strings/numbers, use directly
            if first.is_string() || first.is_number() {
              values.push(first.as_str().unwrap_or_default().to_string());
            } else {
              // For arrays of objects, continue traversing
              current = first;
            }
          }
          continue;
        }
      }

      // At the end of path traversal
      match current {
        Value::String(s) => values.push(s.clone()),
        Value::Array(arr) if !arr.is_empty() =>
          if let Some(s) = arr[0].as_str() {
            values.push(s.to_string());
          },
        Value::Number(n) => values.push(n.to_string()),
        _ =>
          if let Some(s) = current.as_str() {
            values.push(s.to_string());
          },
      }
    }

    if values.is_empty() {
      return Err(LearnerError::ApiError(format!("No content found for {}", field)));
    }

    let value =
      if let Some(sep) = &map.join_separator { values.join(sep) } else { values[0].clone() };

    if let Some(transform) = &map.transform {
      self.apply_transform(&value, transform)
    } else {
      Ok(value)
    }
  }

  fn extract_optional_field(&self, json: &Value, map: &FieldMap) -> Result<Option<String>> {
    let mut values = Vec::new();
    for path in &map.paths {
      let mut current = json;
      for part in path.split('/') {
        current = match current.get(part) {
          Some(value) => value,
          None => continue,
        };

        // Handle arrays specially
        if current.is_array() {
          if let Some(first) = current.as_array().unwrap().first() {
            // For arrays of strings/numbers, use directly
            if first.is_string() || first.is_number() {
              values.push(first.as_str().unwrap_or_default().to_string());
            } else {
              // For arrays of objects, continue traversing
              current = first;
            }
          }
          continue;
        }
      }

      // At the end of path traversal
      match current {
        Value::String(s) => values.push(s.clone()),
        Value::Array(arr) if !arr.is_empty() =>
          if let Some(s) = arr[0].as_str() {
            values.push(s.to_string());
          },
        Value::Number(n) => values.push(n.to_string()),
        _ =>
          if let Some(s) = current.as_str() {
            values.push(s.to_string());
          },
      }
    }

    if values.is_empty() {
      Ok(None)
    } else {
      let value =
        if let Some(sep) = &map.join_separator { values.join(sep) } else { values[0].clone() };

      if let Some(transform) = &map.transform {
        self.apply_transform(&value, transform).map(Some)
      } else {
        Ok(Some(value))
      }
    }
  }

  fn extract_authors(&self, json: &Value, map: &FieldMap) -> Result<Vec<Author>> {
    let mut authors = Vec::new();

    for path in &map.paths {
      let parts: Vec<&str> = path.split('/').collect();
      let mut current = json;
      for part in parts {
        current = match current.get(part) {
          Some(value) => value,
          None => continue,
        };
      }

      if let Some(arr) = current.as_array() {
        for author in arr {
          let name = match (author.get("given"), author.get("family")) {
            (Some(given), Some(family)) => {
              format!("{} {}", given.as_str().unwrap_or(""), family.as_str().unwrap_or(""))
            },
            (Some(given), None) => given.as_str().unwrap_or("").to_string(),
            (None, Some(family)) => family.as_str().unwrap_or("").to_string(),
            (None, None) => continue,
          };

          let affiliation = author
            .get("affiliation")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|aff| aff.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

          authors.push(Author { name, affiliation, email: None });
        }
      }
    }

    if authors.is_empty() {
      return Err(LearnerError::ApiError("No authors found".to_string()));
    }

    Ok(authors)
  }

  fn apply_transform(&self, value: &str, transform: &Transform) -> Result<String> {
    match transform {
      Transform::Replace { pattern, replacement } => {
        let re = Regex::new(pattern)
          .map_err(|e| LearnerError::ApiError(format!("Invalid regex: {}", e)))?;
        Ok(re.replace_all(value, replacement.as_str()).into_owned())
      },
      Transform::Date { from_format, to_format } => {
        let dt = chrono::NaiveDateTime::parse_from_str(value, from_format)
          .map_err(|e| LearnerError::ApiError(format!("Invalid date: {}", e)))?;
        Ok(dt.format(to_format).to_string())
      },
      Transform::Url { base, suffix } => {
        let mut url = base.replace("{value}", value);
        if let Some(suffix) = suffix {
          url.push_str(suffix);
        }
        Ok(url)
      },
    }
  }
}
