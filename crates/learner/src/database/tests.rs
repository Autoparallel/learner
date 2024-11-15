use super::*;

/// Helper function to set up a test database
fn setup_test_db() -> (Database, PathBuf, tempfile::TempDir) {
  let dir = tempdir().unwrap();
  let path = dir.path().join("test.db");
  let db = Database::open(&path).unwrap();
  (db, path, dir)
}

#[traced_test]
#[test]
fn test_database_creation() {
  let (_db, path, _dir) = setup_test_db();

  // Check that file exists
  assert!(path.exists());
}

#[traced_test]
#[test]
fn test_default_path() {
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
#[test]
fn test_default_storage_path() {
  let path = Database::default_storage_path();

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
#[test]
fn test_new_db_uses_default_storage() {
  let (db, _path, _dir) = setup_test_db();

  let storage_path = db.get_storage_path().expect("Storage path should be set");
  assert_eq!(storage_path, Database::default_storage_path());
}

#[test]
fn test_storage_path_persistence() -> Result<()> {
  let (db, db_path, _dir) = setup_test_db();

  // Set custom storage path
  let custom_path = PathBuf::from("/tmp/custom/storage");
  db.set_storage_path(&custom_path)?;

  // Reopen database and check path
  drop(db);
  let db = Database::open(db_path)?;
  let storage_path = db.get_storage_path().expect("Storage path should be set");
  assert_eq!(storage_path, custom_path);

  Ok(())
}

#[test]
fn test_storage_path_creates_directory() -> Result<()> {
  let (db, _path, dir) = setup_test_db();

  let custom_path = dir.path().join("custom_storage");
  db.set_storage_path(&custom_path)?;

  assert!(custom_path.exists());
  assert!(custom_path.is_dir());

  Ok(())
}
