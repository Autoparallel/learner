use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
  /// JSON path mappings for fields
  pub field_maps: HashMap<String, FieldMap>,
}

#[async_trait]
impl ResponseProcessor for JsonConfig {
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

  fn get_by_path<'a>(&self, json: &'a Value, path: &str) -> Option<String> {
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

fn get_path_value<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
  let mut current = json;
  for part in path.split('/') {
    current = current.get(part)?;
  }
  Some(current)
}
