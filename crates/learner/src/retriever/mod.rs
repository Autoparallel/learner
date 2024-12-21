use super::*;

mod response;

use record::{Resource, Retrieval, State, Storage};
pub use response::*;
use serde_json::Map;

use crate::{
  record::Record,
  template::{FieldDefinition, Template, TemplatedItem},
};

#[derive(Debug, Clone, Deserialize)]
pub struct Retriever {
  pub name:              String,
  pub description:       Option<String>,
  /// Base URL for API requests
  pub base_url:          String,
  /// Regex pattern for matching and extracting paper identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:           Regex,
  /// Source identifier for papers from this retriever
  pub source:            String,
  /// Template for constructing API endpoint URLs
  pub endpoint_template: String,

  pub response_format: ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:         BTreeMap<String, String>,

  pub resource:          Resource,
  #[serde(default)]
  pub resource_mappings: BTreeMap<String, Mapping>,

  pub retrieval:          Retrieval,
  #[serde(default)]
  pub retrieval_mappings: BTreeMap<String, Mapping>,
}

impl Retriever {
  /// Extracts the canonical identifier from an input string.
  ///
  /// Uses the configured regex pattern to extract the standardized
  /// identifier from various input formats (URLs, DOIs, etc.).
  ///
  /// # Arguments
  ///
  /// * `input` - Input string containing a paper identifier
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The extracted identifier as a string slice
  /// - A LearnerError if the input doesn't match the pattern
  pub fn extract_identifier<'a>(&self, input: &'a str) -> Result<&'a str> {
    self
      .pattern
      .captures(input)
      .and_then(|cap| cap.get(1))
      .map(|m| m.as_str())
      .ok_or(LearnerError::InvalidIdentifier)
  }

  pub async fn retrieve_resource(&self, input: &str) -> Result<Record> {
    let identifier = self.extract_identifier(input)?;

    // Send request and get response
    let url = self.endpoint_template.replace("{identifier}", identifier);
    debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;

    trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    // Process the response using configured processor
    let json = match &self.response_format {
      ResponseFormat::Xml { strip_namespaces, clean_content } => {
        let xml = if *strip_namespaces {
          response::strip_xml_namespaces(&String::from_utf8_lossy(&data))
        } else {
          String::from_utf8_lossy(&data).to_string()
        };

        // Convert to JSON value
        let mut value = xml_to_json(&xml);

        // Clean content if requested
        if *clean_content {
          clean_value(&mut value);
        }

        value
      },

      ResponseFormat::Json { clean_content } => {
        let mut value = serde_json::from_slice(&data)?;

        if *clean_content {
          clean_value(&mut value);
        }

        value
      },
    };

    let (mut resource, retrieval) = self.process_json_value(&json)?;

    // Add source metadata
    resource.insert("source".into(), Value::String(self.source.clone()));
    resource.insert("source_identifier".into(), Value::String(identifier.to_string()));

    // Validate full resource against config
    // TODO: Need to validate but also place into the respective struct. So we should validate the
    // struct itself.
    // self.resource_template.validate(dbg!(&resource))?;
    // self.retrieval_template.validate(dbg!(&retrieval))?;

    // TODO: Fix this
    // Ok(Record { resource, state: State::default(), storage: Storage::default(), retrieval })
    todo!("Needs fixed")
  }

  pub fn process_json_value(&self, json: &Value) -> Result<(TemplatedItem, TemplatedItem)> {
    todo!("Needs fixed")
    // let resource = process_template_fields(json, &self.resource_template,
    // &self.resource_mappings)?; let retrieval =
    //   process_template_fields(json, &self.retrieval_template, &self.retrieval_mappings)?;

    // Ok((resource, retrieval))
  }
}

fn process_template_fields(
  json: &Value,
  template: &Template,
  mappings: &BTreeMap<String, Mapping>,
) -> Result<BTreeMap<String, Value>> {
  let mut result = BTreeMap::new();

  for field_def in &template.fields {
    if let Some(mapping) = mappings.get(&field_def.name) {
      match extract_mapped_value(json, mapping, field_def) {
        Ok(Some(value)) => {
          result.insert(field_def.name.clone(), value);
        },
        Ok(None) if field_def.required => {
          return Err(LearnerError::ApiError(format!(
            "Required field '{}' not found in response",
            field_def.name
          )));
        },
        Err(e) => return Err(e),
        _ => continue,
      }
    }
  }

  Ok(result)
}

// TODO: Fix unwraps in here
fn extract_mapped_value(
  json: &Value,
  mapping: &Mapping,
  field_def: &FieldDefinition,
) -> Result<Option<Value>> {
  // First get the raw value through mapping
  let raw_value = match mapping {
    Mapping::Path(path) => {
      let components: Vec<&str> = path.split('/').collect();
      get_path_value(json, &components)
        .ok_or_else(|| LearnerError::ApiError(format!("Path '{path}' not found")))?
    },
    Mapping::Join { paths, with } => {
      let parts: Result<Vec<String>> = paths
        .iter()
        .map(|path| {
          let components: Vec<&str> = path.split('/').collect();
          get_path_value(json, &components)
            .and_then(|v| v.as_str().map(std::string::ToString::to_string))
            .ok_or_else(|| LearnerError::ApiError(format!("Path '{path}' is not a string")))
        })
        .collect();
      Value::String(parts?.join(with))
    },
    Mapping::Map { from, map } => {
      // Get the source to map from, if specified
      let source = if let Some(path) = from {
        let components: Vec<&str> = path.split('/').collect();
        get_path_value(json, &components)
          .ok_or_else(|| LearnerError::ApiError(format!("Path '{path}' not found")))?
      } else {
        json.clone()
      };

      if let Value::Array(items) = source {
        let mapped: Result<Vec<Value>> = items
          .iter()
          .map(|item| {
            let mut obj = Map::new();
            for (key, mapping) in map {
              if let Ok(Some(value)) =
                extract_mapped_value(item, mapping, &get_field_def(field_def, key))
              {
                obj.insert(key.clone(), value);
              }
            }
            Ok(Value::Object(obj))
          })
          .collect();
        Value::Array(mapped?)
      } else {
        let mut obj = Map::new();
        for (key, mapping) in map {
          if let Ok(Some(value)) =
            extract_mapped_value(&source, mapping, &get_field_def(field_def, key))
          {
            obj.insert(key.clone(), value);
          }
        }
        Value::Object(obj)
      }
    },
  };

  // Then coerce the value based on the expected type
  let coerced = coerce_value(&raw_value, field_def)?;

  Ok(Some(coerced))
}

// Helper function to get field definition for nested fields
fn get_field_def<'a>(parent: &'a FieldDefinition, field_name: &str) -> FieldDefinition {
  // Check for object fields first
  if let Some(fields) = &parent.fields {
    if let Some(field) = fields.iter().find(|f| f.name == field_name) {
      return field.clone();
    }
  }

  // Then check array items if they exist
  if let Some(items) = &parent.items {
    if let Some(fields) = &items.fields {
      if let Some(field) = fields.iter().find(|f| f.name == field_name) {
        return field.clone();
      }
    }
  }

  // Return a default field definition if not found
  FieldDefinition {
    name:        field_name.to_string(),
    base_type:   "string".to_string(),
    required:    false,
    description: None,
    validation:  None,
    items:       None,
    fields:      None,
  }
}

// Helper function to coerce values based on expected type
fn coerce_value(value: &Value, field_def: &FieldDefinition) -> Result<Value> {
  let result = match field_def.base_type.as_str() {
    "array" => match value {
      Value::Array(_) => value.clone(),
      // If not an array but should be, wrap it
      _ => Value::Array(vec![value.clone()]),
    },
    "string" => match value {
      // If we have a single-element array and need a string
      Value::Array(arr) if arr.len() == 1 =>
        arr[0].as_str().map_or_else(|| arr[0].clone(), |s| Value::String(s.to_string())),
      _ => value.clone(),
    },
    "object" => match value {
      Value::Object(obj) => {
        let mut new_obj = Map::new();
        // If we have fields defined, try to coerce each field
        if let Some(fields) = &field_def.fields {
          for field in fields {
            if let Some(val) = obj.get(&field.name) {
              new_obj.insert(field.name.clone(), coerce_value(val, field)?);
            }
          }
          Value::Object(new_obj)
        } else {
          value.clone()
        }
      },
      _ => value.clone(),
    },
    _ => value.clone(),
  };
  Ok(result)
}

/// Get a value from JSON using a path
// Change return type to owned Value
fn get_path_value(json: &Value, path: &[&str]) -> Option<Value> {
  let mut current = json.clone();

  for &component in path {
    match current {
      Value::Object(map) =>
        if let Some(value) = map.get(component) {
          current = value.clone();
        } else {
          return None;
        },
      Value::Array(arr) => {
        // If component is numeric, use it as array index
        if let Ok(index) = component.parse::<usize>() {
          if let Some(value) = arr.get(index) {
            current = value.clone();
          } else {
            return None;
          }
        } else {
          // Otherwise collect matching values from array elements
          let values: Vec<Value> = arr
            .iter()
            .filter_map(|item| match item {
              Value::Object(map) => map.get(component).cloned(),
              _ => None,
            })
            .collect();

          if values.is_empty() {
            return None;
          } else if values.len() == 1 {
            current = values[0].clone();
          } else {
            return Some(Value::Array(values));
          }
        }
      },
      _ => return Some(json.clone()),
    }
  }

  Some(current)
}

// TODO: We don't really need `Retrievers` if we handle the configuration stuff properly

#[derive(Default, Debug, Clone)]
pub struct Retrievers {
  /// The collection of configurations used for this [`Retrievers`].
  configs: BTreeMap<String, Retriever>,
}

impl Retrievers {
  /// Checks whether the retreivers map is empty.
  ///
  /// This is useful for handling the case where no retreivers are specified and
  /// we wish to inform the user.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::retriever::Retrievers;
  /// # use learner::error::LearnerError;
  ///
  /// # fn check_is_empty() -> Result<(), LearnerError> {
  /// let retriever = Retrievers::new();
  ///
  /// if retriever.is_empty() {
  ///   return Err(LearnerError::Config("No retriever configured.".to_string()));
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub fn is_empty(&self) -> bool { self.configs.is_empty() }
}

impl Retrievers {
  /// Creates a new empty retriever with no configurations.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// use learner::retriever::Retrievers;
  ///
  /// let retriever = Retrievers::new();
  /// ```
  pub fn new() -> Self { Self::default() }

  pub async fn get_resource_file(&self, input: &str) -> Result<TemplatedItem> {
    todo!(
      "Arguably, we don't even need this. We could instead just have this handled by `Learner` so \
       the API is simpler"
    )
  }

  /// Sanitizes and normalizes a paper identifier using configured retrieval patterns.
  ///
  /// This function processes an input string (which could be a URL, DOI, arXiv ID, etc.)
  /// and attempts to match it against configured paper source patterns to extract a
  /// standardized source and identifier pair.
  ///
  /// # Arguments
  ///
  /// * `input` - The input string to sanitize. Can be:
  ///   - A full URL (e.g., "https://arxiv.org/abs/2301.07041")
  ///   - A DOI (e.g., "10.1145/1327452.1327492")
  ///   - An arXiv ID (e.g., "2301.07041" or "math.AG/0601001")
  ///   - An IACR ID (e.g., "2023/123")
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing:
  /// - `Ok((String, String))` - A tuple of (source, identifier) where:
  ///   - source: The normalized source name (e.g., "arxiv", "doi", "iacr")
  ///   - identifier: The extracted canonical identifier
  /// - `Err(LearnerError)` with either:
  ///   - `InvalidIdentifier` if no configured pattern matches the input
  ///   - `AmbiguousIdentifier` if multiple patterns match the input
  ///
  /// # Examples
  ///
  /// ```
  /// # use learner::retriever::Retrievers;
  /// # use learner::prelude::*;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let retriever = Retrievers::new().with_config_dir("config/")?;
  ///
  /// // Sanitize an arXiv URL
  /// let (source, id) = retriever.sanitize_identifier("https://arxiv.org/abs/2301.07041")?;
  /// assert_eq!(source, "arxiv");
  /// assert_eq!(id, "2301.07041");
  ///
  /// // Sanitize a bare DOI
  /// let (source, id) = retriever.sanitize_identifier("10.1145/1327452.1327492")?;
  /// assert_eq!(source, "doi");
  /// assert_eq!(id, "10.1145/1327452.1327492");
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// Will return `LearnerError::InvalidIdentifier` if:
  /// - The input string doesn't match any configured source patterns
  /// - The input matches a pattern but the identifier extraction fails
  ///
  /// Will return `LearnerError::AmbiguousIdentifier` if:
  /// - The input matches multiple source patterns
  /// - Includes the list of matching sources in the error
  ///
  /// # Implementation Notes
  ///
  /// The function:
  /// 1. Checks the input against all configured source patterns
  /// 2. Attempts to extract identifiers from all matching patterns
  /// 3. Validates that exactly one pattern matched
  /// 4. Returns the normalized source and identifier
  ///
  /// The matching process uses regex patterns defined in the retriever configuration
  /// files, allowing for flexible addition of new paper sources.
  pub fn sanitize_identifier(&self, input: &str) -> Result<(String, String)> {
    let mut matches = Vec::new();

    for config in self.configs.values() {
      if config.pattern.is_match(input) {
        matches.push((config.source.clone(), config.extract_identifier(input)?.to_string()));
      }
    }

    match matches.len() {
      0 => Err(LearnerError::InvalidIdentifier),
      1 => Ok(matches.remove(0)),
      _ => Err(LearnerError::AmbiguousIdentifier(
        matches.into_iter().map(|(source, _)| source).collect(),
      )),
    }
  }
}

/// Custom deserializer for converting string patterns into Regex objects.
///
/// Used with serde's derive functionality to automatically deserialize
/// regex patterns from configuration files.
///
/// # Errors
///
/// Returns a deserialization error if the pattern is not a valid regular expression.
pub fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
where D: serde::Deserializer<'de> {
  let s: String = String::deserialize(deserializer)?;
  Regex::new(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_arxiv_config() {
    let config_str = include_str!("../../config/retrievers/arxiv.toml");

    let retriever: Retriever = toml::from_str(config_str).expect("Failed to parse config");

    // Verify basic fields
    // assert_eq!(retriever.name, "arxiv");
    assert_eq!(retriever.base_url, "http://export.arxiv.org");
    assert_eq!(retriever.source, "arxiv");

    // Test pattern matching
    assert!(retriever.pattern.is_match("2301.07041"));
    assert!(retriever.pattern.is_match("math.AG/0601001"));
    assert!(retriever.pattern.is_match("https://arxiv.org/abs/2301.07041"));
    assert!(retriever.pattern.is_match("https://arxiv.org/pdf/2301.07041"));
    assert!(retriever.pattern.is_match("https://arxiv.org/abs/math.AG/0601001"));
    assert!(retriever.pattern.is_match("https://arxiv.org/abs/math/0404443"));

    // Test identifier extraction
    assert_eq!(retriever.extract_identifier("2301.07041").unwrap(), "2301.07041");
    assert_eq!(
      retriever.extract_identifier("https://arxiv.org/abs/2301.07041").unwrap(),
      "2301.07041"
    );
    assert_eq!(retriever.extract_identifier("math.AG/0601001").unwrap(), "math.AG/0601001");

    // Verify response format

    if let ResponseFormat::Xml { strip_namespaces, clean_content } = &retriever.response_format {
      assert!(strip_namespaces);
      assert!(clean_content);

      // Verify field mappings
      let field_maps = &retriever.resource_mappings;
      assert!(field_maps.contains_key("title"));
      assert!(field_maps.contains_key("abstract"));
      assert!(field_maps.contains_key("authors"));
      assert!(field_maps.contains_key("publication_date"));
      assert!(field_maps.contains_key("pdf_url"));

      // Verify PDF transform
      todo!("Fix this");
    //   if let Some(map) = field_maps.get("pdf_url") {
    //     match &map.transform {
    //       Some(Transform::Replace { pattern, replacement }) => {
    //         assert_eq!(pattern, "/abs/");
    //         assert_eq!(replacement, "/pdf/");
    //       },
    //       _ => panic!("Expected Replace transform for pdf_url"),
    //     }
    //   } else {
    //     panic!("Missing pdf_url field map");
    //   }
    } else {
      panic!("Expected an XML configuration, but did not get one.")
    }

    // Verify headers
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }

  #[test]
  fn test_doi_config_deserialization() {
    let config_str = include_str!("../../config/retrievers/doi.toml");

    let retriever: Retriever = toml::from_str(config_str).expect("Failed to parse config");

    dbg!(&retriever);

    // Verify basic fields
    // assert_eq!(retriever.name, "doi");
    assert_eq!(retriever.base_url, "https://api.crossref.org/works");
    assert_eq!(retriever.source, "doi");

    // Test pattern matching
    let test_cases = [
      ("10.1145/1327452.1327492", true),
      ("https://doi.org/10.1145/1327452.1327492", true),
      ("invalid-doi", false),
      ("https://wrong.url/10.1145/1327452.1327492", false),
    ];

    for (input, expected) in test_cases {
      assert_eq!(
        retriever.pattern.is_match(input),
        expected,
        "Pattern match failed for input: {}",
        input
      );
    }

    // Test identifier extraction
    assert_eq!(
      retriever.extract_identifier("10.1145/1327452.1327492").unwrap(),
      "10.1145/1327452.1327492"
    );
    assert_eq!(
      retriever.extract_identifier("https://doi.org/10.1145/1327452.1327492").unwrap(),
      "10.1145/1327452.1327492"
    );

    // Verify response format
    match &retriever.response_format {
      ResponseFormat::Json { clean_content } => {
        assert!(clean_content);
        // Verify field mappings
        let field_maps = &retriever.resource_mappings;
        assert!(field_maps.contains_key("title"));
        assert!(field_maps.contains_key("abstract"));
        assert!(field_maps.contains_key("authors"));
        assert!(field_maps.contains_key("publication_date"));
        assert!(field_maps.contains_key("pdf_url"));
        assert!(field_maps.contains_key("doi"));
      },
      _ => panic!("Expected JSON response format"),
    }
  }

  #[test]
  fn test_iacr_config_deserialization() {
    todo!("Fix this")
    // let config_str = include_str!("../../config/retrievers/iacr.toml");

    // let retriever: Retriever = toml::from_str(config_str).expect("Failed to parse config");

    // // Verify basic fields
    // // assert_eq!(retriever.name, "iacr");
    // assert_eq!(retriever.base_url, "https://eprint.iacr.org");
    // assert_eq!(retriever.source, "iacr");

    // // Test pattern matching
    // let test_cases = [
    //   ("2016/260", true),
    //   ("2023/123", true),
    //   ("https://eprint.iacr.org/2016/260", true),
    //   ("https://eprint.iacr.org/2016/260.pdf", true),
    //   ("invalid/format", false),
    //   ("https://wrong.url/2016/260", false),
    // ];

    // for (input, expected) in test_cases {
    //   assert_eq!(
    //     retriever.pattern.is_match(input),
    //     expected,
    //     "Pattern match failed for input: {}",
    //     input
    //   );
    // }

    // // Test identifier extraction
    // assert_eq!(retriever.extract_identifier("2016/260").unwrap(), "2016/260");
    // assert_eq!(
    //   retriever.extract_identifier("https://eprint.iacr.org/2016/260").unwrap(),
    //   "2016/260"
    // );
    // assert_eq!(
    //   retriever.extract_identifier("https://eprint.iacr.org/2016/260.pdf").unwrap(),
    //   "2016/260"
    // );

    // // Verify response format
    // if let ResponseFormat::Xml { strip_namespaces } = &retriever.response_format {
    //   assert!(strip_namespaces);

    //   // Verify field mappings
    //   let field_maps = &retriever.resource_mappings;
    //   assert!(field_maps.contains_key("title"));
    //   assert!(field_maps.contains_key("abstract"));
    //   assert!(field_maps.contains_key("authors"));
    //   assert!(field_maps.contains_key("publication_date"));
    //   assert!(field_maps.contains_key("pdf_url"));

    //   // Verify OAI-PMH paths
    //   if let Some(map) = field_maps.get("title") {
    //     assert!(map.path.contains(&"OAI-PMH/GetRecord/record/metadata/dc/title".to_string()));
    //   } else {
    //     panic!("Missing title field map");
    //   }
    // } else {
    //   panic!("Expected an XML configuration, but did not get one.")
    // }

    // // Verify headers
    // assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }
}
