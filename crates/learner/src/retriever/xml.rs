use std::collections::HashMap;

use quick_xml::{events::Event, Reader};
use serde::Deserialize;

use super::*;

/// Configuration for XML response processing
#[derive(Debug, Clone, Deserialize)]
pub struct XmlConfig {
  /// Whether to strip namespaces from response
  #[serde(default)]
  pub strip_namespaces: bool,
  /// How to construct fields from extracted content
  pub field_maps:       HashMap<String, FieldMap>,
}

#[async_trait]
impl ResponseProcessor for XmlConfig {
  async fn process_response(&self, data: &[u8]) -> Result<PaperNew> {
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

    Ok(PaperNew {
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

fn strip_xml_namespaces(xml: &str) -> String {
  let re = regex::Regex::new(r#"xmlns(?::\w+)?="[^"]*""#).unwrap();
  let mut result = re.replace_all(xml, "").to_string();
  result = result.replace("oai_dc:", "").replace("dc:", "");
  result
}
