use std::fs;

use learner::resource::ResourceConfig;

use super::*;

#[tokio::test]
async fn test_arxiv_retriever_integration() -> TestResult<()> {
  let ret_config_str = fs::read_to_string("config/retrievers/arxiv.toml").expect(
    "Failed to read config
    file",
  );
  let res_config_str = fs::read_to_string("config/resources/paper.toml").expect(
    "Failed to read config
    file",
  );

  let retriever: RetrieverConfig = toml::from_str(&ret_config_str).expect("Failed to parse config");
  let resource: ResourceConfig = toml::from_str(&res_config_str).expect("Failed to parse config");

  // Test with a real arXiv paper
  let paper = retriever.retrieve_resource("2301.07041", resource).await?;

  dbg!(&paper);

  assert_eq!(
    paper.get("title").unwrap().as_str().unwrap(),
    "Verifiable Fully Homomorphic Encryption"
  );
  // assert!(!paper.title.is_empty());
  // assert!(!paper.authors.is_empty());
  // assert!(!paper.abstract_text.is_empty());
  // assert!(paper.pdf_url.is_some());
  // assert_eq!(paper.source, "arxiv");
  // assert_eq!(paper.source_identifier, "2301.07041");
  Ok(())
}

#[traced_test]
#[tokio::test]
async fn test_arxiv_pdf_from_paper() -> TestResult<()> {
  let config_str = fs::read_to_string("config/retrievers/arxiv.toml").expect(
    "Failed to read config
        file",
  );

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  todo!()
  // // Test with a real arXiv paper
  // let paper = retriever.retrieve_paper("2301.07041").await?;
  // let dir = tempdir()?;
  // paper.download_pdf(dir.path()).await?;
  // let path = dir.into_path().join(paper.filename());
  // assert!(path.exists());
  // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
  // assert!(pdf_content.pages[0].text.contains("arXiv:2301.07041v2"));

  // Ok(())
}

#[tokio::test]
async fn test_iacr_retriever_integration() {
  let config_str =
    fs::read_to_string("config/retrievers/iacr.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  // // Test with a real IACR paper
  // let paper = retriever.retrieve_paper("2016/260").await.unwrap();

  // assert!(!paper.title.is_empty());
  // assert!(!paper.authors.is_empty());
  // assert!(!paper.abstract_text.is_empty());
  // assert!(paper.pdf_url.is_some());
  // assert_eq!(paper.source, "iacr");
  // assert_eq!(paper.source_identifier, "2016/260");
}

#[traced_test]
#[tokio::test]
async fn test_iacr_pdf_from_paper() -> TestResult<()> {
  let config_str =
    fs::read_to_string("config/retrievers/iacr.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");

  todo!()
  // // Test with a real arXiv paper
  // let paper = retriever.retrieve_paper("2016/260").await?;
  // let dir = tempdir()?;
  // paper.download_pdf(dir.path()).await?;
  // let path = dir.into_path().join(paper.filename());
  // assert!(path.exists());
  // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
  // assert!(pdf_content.pages[0].text.contains("On the Size"));

  // Ok(())
}

#[tokio::test]
#[traced_test]
async fn test_doi_retriever_integration() -> TestResult<()> {
  let ret_config_str = fs::read_to_string("config/retrievers/doi.toml").expect(
    "Failed to read config
    file",
  );
  let res_config_str = fs::read_to_string("config/resources/paper.toml").expect(
    "Failed to read config
    file",
  );

  let retriever: RetrieverConfig = toml::from_str(&ret_config_str).expect("Failed to parse config");
  let resource: ResourceConfig = toml::from_str(&res_config_str).expect("Failed to parse config");

  // Test with a real DOI paper
  let paper = retriever.retrieve_resource("10.1145/1327452.1327492", resource).await?;

  dbg!(&paper);
  // assert!(!paper.title.is_empty());
  // assert!(!paper.authors.is_empty());
  // assert!(!paper.abstract_text.is_empty());
  // assert!(paper.pdf_url.is_some());
  // assert_eq!(paper.source, "doi");
  // assert_eq!(paper.source_identifier, "10.1145/1327452.1327492");
  // assert!(paper.doi.is_some());
  Ok(())
}

#[ignore = "This PDF downloads properly but it does not parse correctly with Lopdf due to: `Error: \
            Lopdf(Type)`"]
#[traced_test]
#[tokio::test]
async fn test_doi_pdf_from_paper() -> TestResult<()> {
  let config_str =
    fs::read_to_string("config/retrievers/doi.toml").expect("Failed to read config file");

  let retriever: RetrieverConfig = toml::from_str(&config_str).expect("Failed to parse config");
  todo!()
  // Test with a real arXiv paper
  // let paper = retriever.retrieve_paper("10.1145/1327452.1327492").await?;
  // let dir = tempdir()?;
  // paper.download_pdf(dir.path()).await?;
  // let path = dir.into_path().join(paper.filename());
  // assert!(path.exists());
  // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
  // assert!(pdf_content.pages[0].text.contains("arXiv:2301.07041v2"));

  // Ok(())
}
