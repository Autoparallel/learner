use super::*;

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
