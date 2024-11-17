use std::collections::HashMap;

use quick_xml::{events::Event, Reader};
use serde::Deserialize;

use super::*;

// TODO (autoparallel): I don't think the `paths` field is ever needed.

// TODO (autoparallel): We should deserialize into a regex for the transforms and likely make it
// static too so it isn't rebuilt every time

/// Configuration for XML response processing
#[derive(Debug, Clone, Deserialize)]
pub struct XmlConfig {
  /// Whether to strip namespaces from response
  #[serde(default)]
  pub strip_namespaces: bool,
  /// Paths to extract text content from
  pub paths:            HashMap<String, String>,
  /// How to construct fields from extracted content
  pub field_maps:       HashMap<String, FieldMap>,
}

/// Mapping configuration for a paper field
#[derive(Debug, Clone, Deserialize)]
pub struct FieldMap {
  /// Path(s) to extract content from
  pub paths:          Vec<String>,
  /// How to join multiple values if found
  #[serde(default)]
  pub join_separator: Option<String>,
  /// Optional transformation to apply
  #[serde(default)]
  pub transform:      Option<Transform>,
}

#[async_trait]
impl ResponseProcessor for XmlConfig {
  async fn process_response(&self, data: &[u8]) -> Result<PaperNew> {
    let mut xml = String::from_utf8_lossy(data).to_string();
    if self.strip_namespaces {
      xml = strip_xml_namespaces(&xml);
    }

    trace!("Processing XML: {}", xml);

    let content = self.extract_content(&xml)?;

    // Helper closure to extract required field
    let get_field = |name: &str| -> Result<String> {
      let map = self
        .field_maps
        .get(name)
        .ok_or_else(|| LearnerError::ApiError(format!("Missing field mapping for {}", name)))?;

      let values: Vec<String> =
        map.paths.iter().filter_map(|path| content.get(path)).cloned().collect();

      if values.is_empty() {
        return Err(LearnerError::ApiError(format!("No content found for {}", name)));
      }

      // Join values if needed
      let value =
        if let Some(sep) = &map.join_separator { values.join(sep) } else { values[0].clone() };

      // Apply transformation if configured
      if let Some(transform) = &map.transform {
        self.apply_transform(&value, transform)
      } else {
        Ok(value)
      }
    };

    // Extract required fields
    let title = get_field("title")?;
    let abstract_text = get_field("abstract")?;
    let publication_date_str = get_field("publication_date")?;

    // Parse the date, supporting multiple formats
    let publication_date =
      if let Ok(date) = chrono::DateTime::parse_from_rfc3339(&publication_date_str) {
        date.with_timezone(&Utc)
      } else {
        // Try parsing with different formats
        chrono::NaiveDateTime::parse_from_str(&publication_date_str, "%Y-%m-%dT%H:%M:%SZ")
          .map(|dt| Utc.from_utc_datetime(&dt))
          .map_err(|e| {
            LearnerError::ApiError(format!(
              "Invalid date format: {} - Error: {}",
              publication_date_str, e
            ))
          })?
      };

    // Extract authors
    let author_map = self
      .field_maps
      .get("authors")
      .ok_or_else(|| LearnerError::ApiError("Missing authors mapping".to_string()))?;
    let authors: Vec<Author> = author_map
      .paths
      .iter()
      .filter_map(|path| content.get(path))
      .map(|name| Author { name: name.clone(), affiliation: None, email: None })
      .collect();

    if authors.is_empty() {
      return Err(LearnerError::ApiError("No authors found".to_string()));
    }

    // Extract optional PDF URL
    let pdf_url = if let Some(map) = self.field_maps.get("pdf_url") {
      map
        .paths
        .first()
        .and_then(|path| content.get(path))
        .map(|url| {
          if let Some(transform) = &map.transform {
            self.apply_transform(url, transform).ok()
          } else {
            Some(url.clone())
          }
        })
        .flatten()
    } else {
      None
    };

    // Extract optional DOI
    let doi = self
      .field_maps
      .get("doi")
      .and_then(|map| map.paths.first())
      .and_then(|path| content.get(path))
      .cloned();

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

impl XmlConfig {
  fn extract_content(&self, xml: &str) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut content = HashMap::new();
    let mut current_path = Vec::new();
    let mut buf = Vec::new();

    loop {
      match reader.read_event_into(&mut buf) {
        Ok(Event::Start(ref e)) => {
          current_path.push(e.name().as_ref().to_vec());
        },
        Ok(Event::Text(e)) => {
          let text = e.unescape().unwrap_or_default().into_owned();
          if !text.trim().is_empty() {
            let path = current_path
              .iter()
              .map(|p| String::from_utf8_lossy(p).into_owned())
              .collect::<Vec<_>>()
              .join("/");
            content.insert(path, text.trim().to_string());
          }
        },
        Ok(Event::End(_)) => {
          current_path.pop();
        },
        Ok(Event::Eof) => break,
        Err(e) => return Err(LearnerError::ApiError(format!("Failed to parse XML: {}", e))),
        _ => (),
      }
      buf.clear();
    }

    trace!("Extracted content: {:?}", content);
    Ok(content)
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

/// Strip namespaces from XML string
fn strip_xml_namespaces(xml: &str) -> String {
  let mut result = xml.to_string();
  for ns in &[
    r#"xmlns="http://www.w3.org/2005/Atom""#,
    r#"xmlns:oai_dc="http://www.openarchives.org/OAI/2.0/oai_dc/""#,
    r#"xmlns:dc="http://purl.org/dc/elements/1.1/""#,
    r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#,
  ] {
    result = result.replace(ns, "");
  }
  result = result.replace("oai_dc:", "").replace("dc:", "");
  result
}
