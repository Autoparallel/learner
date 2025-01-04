// use std::fs;

// use learner::configuration::ConfigurationManager;

// use super::*;

// // #[traced_test]
// #[tokio::test]
// async fn test_arxiv_retriever_integration() -> TestResult<()> {
//   let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));
//   let retriever: Retriever = manager.load_config("config_new/arxiv.toml")?;

//   let paper = retriever.retrieve_resource("2301.07041").await?;

//   dbg!(&paper);
//   // assert!(resource.validate(&paper)?);

//   // assert_eq!(
//   //   paper.get("title").unwrap().as_str().unwrap(),
//   //   "Verifiable Fully Homomorphic Encryption"
//   // );
//   todo!("This needs cleaned up.");
//   // assert!(!paper.title.is_empty());
//   // assert!(!paper.authors.is_empty());
//   // assert!(!paper.abstract_text.is_empty());
//   // assert!(paper.pdf_url.is_some());
//   // assert_eq!(paper.source, "arxiv");
//   // assert_eq!(paper.source_identifier, "2301.07041");
//   Ok(())
// }

// #[traced_test]
// #[tokio::test]
// async fn test_arxiv_pdf_from_paper() -> TestResult<()> {
//   let config_str = fs::read_to_string("config/retrievers/arxiv.toml").expect(
//     "Failed to read config
//         file",
//   );

//   let retriever: Retriever = toml::from_str(&config_str).expect("Failed to parse config");

//   todo!()
//   // // Test with a real arXiv paper
//   // let paper = retriever.retrieve_paper("2301.07041").await?;
//   // let dir = tempdir()?;
//   // paper.download_pdf(dir.path()).await?;
//   // let path = dir.into_path().join(paper.filename());
//   // assert!(path.exists());
//   // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
//   // assert!(pdf_content.pages[0].text.contains("arXiv:2301.07041v2"));

//   // Ok(())
// }

// // #[traced_test]
// #[tokio::test]
// async fn test_iacr_retriever_integration() -> TestResult<()> {
//   let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));
//   let retriever: Retriever = manager.load_config("config_new/iacr.toml")?;

//   let paper = retriever.retrieve_resource("2019/953").await.unwrap(); // plonk
//                                                                       // let paper =
// retriever.retrieve_resource("2016/260").await.unwrap(); // groth 16                              
// // assert!(resource.validate(&paper)?); // TODO: validation already happens internally, to be
// fair                                                                       // that validation may
// not be working totally right   dbg!(&paper);

//   todo!("This needs cleaned up.");
//   // assert!(!paper.title.is_empty());
//   // assert!(!paper.authors.is_empty());
//   // assert!(!paper.abstract_text.is_empty());
//   // assert!(paper.pdf_url.is_some());
//   // assert_eq!(paper.source, "iacr");
//   // assert_eq!(paper.source_identifier, "2016/260");

//   Ok(())
// }

// #[traced_test]
// #[tokio::test]
// async fn test_iacr_pdf_from_paper() -> TestResult<()> {
//   let config_str =
//     fs::read_to_string("config/retrievers/iacr.toml").expect("Failed to read config file");

//   let retriever: Retriever = toml::from_str(&config_str).expect("Failed to parse config");

//   todo!()
//   // // Test with a real arXiv paper
//   // let paper = retriever.retrieve_paper("2016/260").await?;
//   // let dir = tempdir()?;
//   // paper.download_pdf(dir.path()).await?;
//   // let path = dir.into_path().join(paper.filename());
//   // assert!(path.exists());
//   // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
//   // assert!(pdf_content.pages[0].text.contains("On the Size"));

//   // Ok(())
// }

// #[tokio::test]
// // #[traced_test]
// async fn test_doi_retriever_integration() -> TestResult<()> {
//   let mut manager = ConfigurationManager::new(PathBuf::from("config_new"));
//   let retriever: Retriever = dbg!(manager.load_config("config_new/doi.toml")?);

//   // Test with a real DOI paper
//   let paper = retriever.retrieve_resource("10.1145/1327452.1327492").await?;
//   // assert!(resource.validate(&paper)?);
//   dbg!(&paper);
//   todo!("Clean this up");
//   // assert!(!paper.title.is_empty());
//   // assert!(!paper.authors.is_empty());
//   // assert!(!paper.abstract_text.is_empty());
//   // assert!(paper.pdf_url.is_some());
//   // assert_eq!(paper.source, "doi");
//   // assert_eq!(paper.source_identifier, "10.1145/1327452.1327492");
//   // assert!(paper.doi.is_some());
//   Ok(())
// }

// #[ignore = "This PDF downloads properly but it does not parse correctly with Lopdf due to:
// `Error: \             Lopdf(Type)`"]
// #[traced_test]
// #[tokio::test]
// async fn test_doi_pdf_from_paper() -> TestResult<()> {
//   let config_str =
//     fs::read_to_string("config/retrievers/doi.toml").expect("Failed to read config file");

//   let retriever: Retriever = toml::from_str(&config_str).expect("Failed to parse config");
//   todo!()
//   // Test with a real arXiv paper
//   // let paper = retriever.retrieve_paper("10.1145/1327452.1327492").await?;
//   // let dir = tempdir()?;
//   // paper.download_pdf(dir.path()).await?;
//   // let path = dir.into_path().join(paper.filename());
//   // assert!(path.exists());
//   // let pdf_content = PDFContentBuilder::new().path(path).analyze()?;
//   // assert!(pdf_content.pages[0].text.contains("arXiv:2301.07041v2"));

//   // Ok(())
// }
