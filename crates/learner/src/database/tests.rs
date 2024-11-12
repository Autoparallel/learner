use super::*;

/// Helper function to set up a test database
async fn setup_test_db() -> (Database, PathBuf, tempfile::TempDir) {
  let dir = tempdir().unwrap();
  let path = dir.path().join("test.db");
  let db = Database::open(&path).await.unwrap();
  (db, path, dir)
}

#[traced_test]
#[tokio::test]
async fn test_database_creation() {
  let (_db, path, _dir) = setup_test_db().await;

  // Check that file exists
  assert!(path.exists());
}

#[traced_test]
#[tokio::test]
async fn test_default_path() {
  let path = Database::default_path();

  // Should end with learner/papers
  assert!(path.ends_with("learner/learner.db") || path.ends_with("learner\\learner.db"));

  // Should be rooted in a valid directory
  assert!(path
    .parent()
    .unwrap()
    .starts_with(dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))));
}

#[traced_test]
#[tokio::test]
async fn test_default_pdf_path() {
  let path = Database::default_pdf_path();

  // Should end with learner/papers
  assert!(path.ends_with("learner/papers") || path.ends_with("learner\\papers"));

  // Should be rooted in a valid directory
  assert!(path
    .parent()
    .unwrap()
    .starts_with(dirs::document_dir().unwrap_or_else(|| PathBuf::from("."))));
}

#[traced_test]
#[tokio::test]
async fn test_get_nonexistent_paper() {
  todo!("Perhaps move this to integration tests with query/get.rs")
  //   let (db, _path, _dir) = setup_test_db().await;

  //   let result = db.get_paper_by_source_id(&Source::Arxiv, "nonexistent").await.unwrap();

  //   assert!(result.is_none());
}

#[traced_test]
#[tokio::test]
async fn test_config_operations() {
  todo!();
  //   let (db, _path, _dir) = setup_test_db().await;

  //   // Test setting and getting a config value
  //   db.set_config("test_key", "test_value").await.unwrap();
  //   let value = db.get_config("test_key").await.unwrap();
  //   assert_eq!(value, Some("test_value".to_string()));

  //   // Test getting non-existent config
  //   let missing = db.get_config("nonexistent").await.unwrap();
  //   assert_eq!(missing, None);

  //   // Test updating existing config
  //   db.set_config("test_key", "new_value").await.unwrap();
  //   let updated = db.get_config("test_key").await.unwrap();
  //   assert_eq!(updated, Some("new_value".to_string()));
}

#[traced_test]
#[tokio::test]
async fn test_config_persistence() {
  todo!();
  //   let dir = tempdir().unwrap();
  //   let db_path = dir.path().join("test.db");

  //   // Create database and set config
  //   {
  //     let db = Database::open(&db_path).await.unwrap();
  //     db.set_config("pdf_dir", "/test/path").await.unwrap();
  //   }

  //   // Reopen database and verify config persists
  //   {
  //     let db = Database::open(&db_path).await.unwrap();
  //     let value = db.get_config("pdf_dir").await.unwrap();
  //     assert_eq!(value, Some("/test/path".to_string()));
}
