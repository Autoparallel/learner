use super::*;

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
