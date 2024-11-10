use std::{error::Error, path::PathBuf};

use learner::{
  database::Database,
  format,
  llm::{LlamaRequest, Model},
  paper::{Author, Paper, Source},
  pdf::PDFContentBuilder,
};
use tempfile::{tempdir, TempDir};
use tracing_test::traced_test;

/// Helper function to set up a test database
async fn setup_test_db() -> (Database, TempDir) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  let db = Database::open(&db_path).await.unwrap();
  (db, dir)
}

/// Helper function to create a test paper
fn create_test_paper() -> Paper {
  Paper {
    title:             "Test Paper".to_string(),
    abstract_text:     "This is a test abstract".to_string(),
    publication_date:  chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2024, 1, 1, 0, 0, 0)
      .unwrap(),
    source:            Source::Arxiv,
    source_identifier: "2401.00000".to_string(),
    pdf_url:           Some("https://arxiv.org/pdf/2401.00000".to_string()),
    doi:               Some("10.1000/test.123".to_string()),
    authors:           vec![
      Author {
        name:        "John Doe".to_string(),
        affiliation: Some("Test University".to_string()),
        email:       Some("john@test.edu".to_string()),
      },
      Author { name: "Jane Smith".to_string(), affiliation: None, email: None },
    ],
  }
}

#[traced_test]
#[tokio::test]
async fn test_save_and_retrieve_paper() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper
  let paper_id = db.save_paper(&paper).await.unwrap();
  assert!(paper_id > 0);

  // Retrieve paper
  let retrieved = db
    .get_paper_by_source_id(&paper.source, &paper.source_identifier)
    .await
    .unwrap()
    .expect("Paper should exist");

  // Verify paper data
  assert_eq!(retrieved.title, paper.title);
  assert_eq!(retrieved.abstract_text, paper.abstract_text);
  assert_eq!(retrieved.publication_date, paper.publication_date);
  assert_eq!(retrieved.source, paper.source);
  assert_eq!(retrieved.source_identifier, paper.source_identifier);
  assert_eq!(retrieved.pdf_url, paper.pdf_url);
  assert_eq!(retrieved.doi, paper.doi);

  // Verify authors
  assert_eq!(retrieved.authors.len(), paper.authors.len());
  assert_eq!(retrieved.authors[0].name, paper.authors[0].name);
  assert_eq!(retrieved.authors[0].affiliation, paper.authors[0].affiliation);
  assert_eq!(retrieved.authors[0].email, paper.authors[0].email);
  assert_eq!(retrieved.authors[1].name, paper.authors[1].name);
  assert_eq!(retrieved.authors[1].affiliation, None);
  assert_eq!(retrieved.authors[1].email, None);
}

#[traced_test]
#[tokio::test]
async fn test_full_text_search() {
  let (db, _dir) = setup_test_db().await;

  // Save a few papers
  let mut paper1 = create_test_paper();
  paper1.title = "Neural Networks in Machine Learning".to_string();
  paper1.abstract_text = "This paper discusses deep learning".to_string();
  paper1.source_identifier = "2401.00001".to_string();

  let mut paper2 = create_test_paper();
  paper2.title = "Advanced Algorithms".to_string();
  paper2.abstract_text = "Classical computer science topics".to_string();
  paper2.source_identifier = "2401.00002".to_string();

  db.save_paper(&paper1).await.unwrap();
  db.save_paper(&paper2).await.unwrap();

  // Search for papers
  let results = db.search_papers("neural").await.unwrap();
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, paper1.title);

  let results = db.search_papers("learning").await.unwrap();
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].source_identifier, paper1.source_identifier);

  let results = db.search_papers("algorithms").await.unwrap();
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, paper2.title);
}

#[traced_test]
#[tokio::test]
async fn test_duplicate_paper_handling() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper first time
  let result1 = db.save_paper(&paper).await;
  assert!(result1.is_ok());

  // Try to save the same paper again
  let result2 = db.save_paper(&paper).await;
  assert!(result2.is_err()); // Should fail due to UNIQUE constraint
}

#[traced_test]
#[tokio::test]
async fn test_pdf_recording() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper first to get an ID
  let paper_id = db.save_paper(&paper).await.unwrap();

  // Test recording successful PDF download
  let path = PathBuf::from("/test/path/paper.pdf");
  let filename = "paper.pdf".to_string();

  let file_id =
    db.record_pdf(paper_id, path.clone(), filename.clone(), "success", None).await.unwrap();

  assert!(file_id > 0);

  // Test retrieving PDF status
  let status = db.get_pdf_status(paper_id).await.unwrap();
  assert!(status.is_some());

  let (stored_path, stored_filename, stored_status, error) = status.unwrap();
  assert_eq!(stored_path, path);
  assert_eq!(stored_filename, filename);
  assert_eq!(stored_status, "success");
  assert_eq!(error, None);
}

#[traced_test]
#[tokio::test]
async fn test_pdf_failure_recording() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper first to get an ID
  let paper_id = db.save_paper(&paper).await.unwrap();

  // Test recording failed PDF download
  let path = PathBuf::from("/test/path/paper.pdf");
  let filename = "paper.pdf".to_string();
  let error_msg = "HTTP 403: Access Denied".to_string();

  db.record_pdf(paper_id, path.clone(), filename.clone(), "failed", Some(error_msg.clone()))
    .await
    .unwrap();

  // Test retrieving failed status
  let status = db.get_pdf_status(paper_id).await.unwrap();
  assert!(status.is_some());

  let (stored_path, stored_filename, stored_status, error) = status.unwrap();
  assert_eq!(stored_path, path);
  assert_eq!(stored_filename, filename);
  assert_eq!(stored_status, "failed");
  assert_eq!(error, Some(error_msg));
}

#[traced_test]
#[tokio::test]
async fn test_pdf_status_nonexistent() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper first to get an ID
  let paper_id = db.save_paper(&paper).await.unwrap();

  // Test getting status for paper with no PDF record
  let status = db.get_pdf_status(paper_id).await.unwrap();
  assert_eq!(status, None);
}

#[traced_test]
#[tokio::test]
async fn test_pdf_status_update() {
  let (db, _dir) = setup_test_db().await;
  let paper = create_test_paper();

  // Save paper first to get an ID
  let paper_id = db.save_paper(&paper).await.unwrap();

  let path = PathBuf::from("/test/path/paper.pdf");
  let filename = "paper.pdf".to_string();

  // First record as pending
  db.record_pdf(paper_id, path.clone(), filename.clone(), "pending", None).await.unwrap();

  // Then update to success
  db.record_pdf(paper_id, path.clone(), filename.clone(), "success", None).await.unwrap();

  // Verify final status
  let status = db.get_pdf_status(paper_id).await.unwrap();
  let (_, _, stored_status, _) = status.unwrap();
  assert_eq!(stored_status, "success");
}

#[ignore = "Can't run this in general -- relies on local LLM endpoint."]
#[tokio::test]
#[traced_test]
async fn test_download_then_send_pdf() -> Result<(), Box<dyn Error>> {
  // Download a PDF
  let dir = tempdir().unwrap();
  let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
  paper.download_pdf(dir.path().to_path_buf()).await.unwrap();

  // Get the content of the PDF
  let formatted_title = format::format_title(&paper.title, None); // use default 50
  let path = dir.into_path().join(format!("{}.pdf", formatted_title));
  let pdf_content = PDFContentBuilder::new().path(path).analyze().unwrap();

  let mut message = "Please act like a researcher and digest this text from a PDF for me and give \
                     me an excellent summary. The summary can be long and descriptive. \n"
    .to_owned();

  message.push_str(&serde_json::to_string(&pdf_content.metadata).unwrap());
  message.push_str(&serde_json::to_string(&pdf_content.pages[0..5]).unwrap());

  let response =
    LlamaRequest::new().with_model(Model::Llama3p2c3b).with_message(&message).send().await?;
  dbg!(response.message);
  Ok(())
}
