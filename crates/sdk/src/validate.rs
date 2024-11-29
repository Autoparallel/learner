use learner::resource::ResourceConfig;

use super::*;

pub fn validate_resource(path: &PathBuf) {
  todo!()
  // Check all required fields are present
  // for field in &self.type_config.required_fields {
  //   if !self.values.contains_key(field) {
  //     return Err(LearnerError::InvalidResource(format!("Missing required field: {}", field)));
  //   }
  // }

  // // Validate all present fields match their declared types
  // for (field, value) in &self.values {
  //   if let Some(field_type) = self.type_config.field_types.get(field) {
  //     if !self.type_config.validate_value_type(value, field_type) {
  //       return Err(LearnerError::InvalidResource(format!("Field {} has invalid type", field)));
  //     }
  //   } else {
  //     return Err(LearnerError::InvalidResource(format!("Unknown field: {}", field)));
  //   }
  // }

  // Ok(())
}

pub fn validate_retriever(path: &PathBuf) {
  let config_str =
    read_to_string("config/retrievers/arxiv.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Verify basic fields
  assert_eq!(retriever.name, "arxiv");
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

  if let ResponseFormat::Xml(config) = &retriever.response_format {
    assert!(config.strip_namespaces);

    // Verify field mappings
    let field_maps = &config.field_maps;
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
