use std::fs;

use super::*;

#[tokio::test]
async fn test_arxiv_retriever_integration() {
  let config_str = fs::read_to_string("config/retrievers/arxiv.toml").expect(
    "Failed to read config
    file",
  );

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Test with a real arXiv paper
  let paper = retriever.retrieve_paper("2301.07041").await.unwrap();

  assert!(!paper.title.is_empty());
  assert!(!paper.authors.is_empty());
  assert!(!paper.abstract_text.is_empty());
  assert!(paper.pdf_url.is_some());
  assert_eq!(paper.source, "arxiv");
  assert_eq!(paper.source_identifier, "2301.07041");
}

#[tokio::test]
async fn test_iacr_retriever_integration() {
  let config_str =
    fs::read_to_string("config/retrievers/iacr.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Test with a real IACR paper
  let paper = retriever.retrieve_paper("2016/260").await.unwrap();

  assert!(!paper.title.is_empty());
  assert!(!paper.authors.is_empty());
  assert!(!paper.abstract_text.is_empty());
  assert!(paper.pdf_url.is_some());
  assert_eq!(paper.source, "iacr");
  assert_eq!(paper.source_identifier, "2016/260");
}

#[tokio::test]
async fn test_doi_retriever_integration() {
  let config_str =
    fs::read_to_string("config/retrievers/doi.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // Test with a real DOI paper
  let paper = retriever.retrieve_paper("10.1145/1327452.1327492").await.unwrap();

  assert!(!paper.title.is_empty());
  assert!(!paper.authors.is_empty());
  assert!(!paper.abstract_text.is_empty());
  assert!(paper.pdf_url.is_some());
  assert_eq!(paper.source, "doi");
  assert_eq!(paper.source_identifier, "10.1145/1327452.1327492");
  assert!(paper.doi.is_some());
}
