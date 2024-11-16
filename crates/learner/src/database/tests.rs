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

#[tokio::test]
async fn test_new_db_uses_default_storage() {
  let (db, _path, _dir) = setup_test_db().await;

  let storage_path = db.get_storage_path().await.expect("Storage path should be set");
  assert_eq!(storage_path, Database::default_storage_path());
}

#[tokio::test]
async fn test_storage_path_persistence() {
  let (db, db_path, _dir) = setup_test_db().await;

  // Set custom storage path
  let custom_path = PathBuf::from("/tmp/custom/storage");
  db.set_storage_path(&custom_path).await.unwrap();

  // Reopen database and check path
  drop(db);
  let db = Database::open(db_path).await.unwrap();
  let storage_path = db.get_storage_path().await.expect("Storage path should be set");
  assert_eq!(storage_path, custom_path);
}

#[tokio::test]
async fn test_storage_path_creates_directory() {
  let (db, _path, dir) = setup_test_db().await;

  let custom_path = dir.path().join("custom_storage");
  db.set_storage_path(&custom_path).await.unwrap();

  assert!(custom_path.exists());
  assert!(custom_path.is_dir());
}
