use std::fs::read_to_string;

use learner::retriever::{ResponseFormat, RetrieverConfig, Transform};

use super::*;

mod paper_retrieval;

#[test]
fn test_arxiv_config_deserialization() {
  let config_str =
    read_to_string("tests/.config/retriever_arxiv.toml").expect("Failed to read config file");

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

#[test]
fn test_doi_config_deserialization() {
  let config_str =
    read_to_string("tests/.config/retriever_doi.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Verify basic fields
  assert_eq!(retriever.name, "doi");
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
    ResponseFormat::Json(config) => {
      // Verify field mappings
      let field_maps = &config.field_maps;
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
  let config_str =
    read_to_string("tests/.config/retriever_iacr.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Verify basic fields
  assert_eq!(retriever.name, "iacr");
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
  assert_eq!(retriever.extract_identifier("https://eprint.iacr.org/2016/260").unwrap(), "2016/260");
  assert_eq!(
    retriever.extract_identifier("https://eprint.iacr.org/2016/260.pdf").unwrap(),
    "2016/260"
  );

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
