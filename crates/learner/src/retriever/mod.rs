use super::*;

mod config;
mod response;

pub use config::*;
pub use response::*;
use template::Resource;

// TODO: This should be `BTreeMap<String, Config<Retriever>>`
#[derive(Default, Debug, Clone)]
pub struct Retrievers {
  /// The collection of configurations used for this [`Retrievers`].
  configs: BTreeMap<String, Retriever>,
}

// impl Configurable for Retrievers {
//   type Config = Retriever;

//   fn as_map(&mut self) -> &mut BTreeMap<String, Self::Config> { &mut self.configs }
// }

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

  pub async fn get_resource_file(&self, input: &str) -> Result<Resource> {
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
fn deserialize_regex<'de, D>(deserializer: D) -> std::result::Result<Regex, D::Error>
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

    if let ResponseFormat::Xml { strip_namespaces } = &retriever.response_format {
      assert!(strip_namespaces);

      // Verify field mappings
      let field_maps = &retriever.resource_mappings;
      assert!(field_maps.contains_key("title"));
      assert!(field_maps.contains_key("abstract"));
      assert!(field_maps.contains_key("authors"));
      assert!(field_maps.contains_key("publication_date"));
      assert!(field_maps.contains_key("pdf_url"));

      // Verify PDF transform
      if let Some(map) = field_maps.get("pdf_url") {
        match &map.transform {
          Some(Transform::Replace { pattern, replacement }) => {
            assert_eq!(pattern, "/abs/");
            assert_eq!(replacement, "/pdf/");
          },
          _ => panic!("Expected Replace transform for pdf_url"),
        }
      } else {
        panic!("Missing pdf_url field map");
      }
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
      ResponseFormat::Json => {
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
    let config_str = include_str!("../../config/retrievers/iacr.toml");

    let retriever: Retriever = toml::from_str(config_str).expect("Failed to parse config");

    // Verify basic fields
    // assert_eq!(retriever.name, "iacr");
    assert_eq!(retriever.base_url, "https://eprint.iacr.org");
    assert_eq!(retriever.source, "iacr");

    // Test pattern matching
    let test_cases = [
      ("2016/260", true),
      ("2023/123", true),
      ("https://eprint.iacr.org/2016/260", true),
      ("https://eprint.iacr.org/2016/260.pdf", true),
      ("invalid/format", false),
      ("https://wrong.url/2016/260", false),
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
    assert_eq!(retriever.extract_identifier("2016/260").unwrap(), "2016/260");
    assert_eq!(
      retriever.extract_identifier("https://eprint.iacr.org/2016/260").unwrap(),
      "2016/260"
    );
    assert_eq!(
      retriever.extract_identifier("https://eprint.iacr.org/2016/260.pdf").unwrap(),
      "2016/260"
    );

    // Verify response format
    if let ResponseFormat::Xml { strip_namespaces } = &retriever.response_format {
      assert!(strip_namespaces);

      // Verify field mappings
      let field_maps = &retriever.resource_mappings;
      assert!(field_maps.contains_key("title"));
      assert!(field_maps.contains_key("abstract"));
      assert!(field_maps.contains_key("authors"));
      assert!(field_maps.contains_key("publication_date"));
      assert!(field_maps.contains_key("pdf_url"));

      // Verify OAI-PMH paths
      if let Some(map) = field_maps.get("title") {
        assert!(map.path.contains(&"OAI-PMH/GetRecord/record/metadata/dc/title".to_string()));
      } else {
        panic!("Missing title field map");
      }
    } else {
      panic!("Expected an XML configuration, but did not get one.")
    }

    // Verify headers
    assert_eq!(retriever.headers.get("Accept").unwrap(), "application/xml");
  }
}
