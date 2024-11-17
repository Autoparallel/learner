use super::*;

/// Helper function to set up a test database
async fn setup_test_db() -> (Database, PathBuf, tempfile::TempDir) {
  let dir = tempdir().unwrap();
  // Make both paths absolute
  let abs_dir = dir.path().canonicalize().unwrap();
  let db_path = abs_dir.join("test.db");
  let storage_path = abs_dir.join("storage"); // Add a storage directory

  // Create the database and explicitly set the storage path
  let db = Database::open(&db_path).await.unwrap();
  db.set_storage_path(&storage_path).await.unwrap();

  (db, db_path, dir)
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

#[traced_test]
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

#[traced_test]
#[tokio::test]
async fn test_storage_path_creates_directory() {
  let (db, _path, dir) = setup_test_db().await;

  let custom_path = dir.path().join("custom_storage");
  db.set_storage_path(&custom_path).await.unwrap();

  assert!(custom_path.exists());
  assert!(custom_path.is_dir());
}

#[traced_test]
#[tokio::test]
async fn test_storage_path_valid() -> Result<()> {
  let (db, _path, dir) = setup_test_db().await;

  // Create an absolute path without requiring existence
  let test_path = dir.path().join("storage");

  // Set the storage path (this will create the directory)
  db.set_storage_path(&test_path).await?;

  // Get and verify the stored path
  let stored_path = db.get_storage_path().await?;
  assert_eq!(stored_path, test_path);
  assert!(test_path.exists());

  // Verify we can write to the directory
  let test_file = test_path.join("test.txt");
  std::fs::write(&test_file, b"test")?;
  assert!(test_file.exists());

  Ok(())
}

#[traced_test]
#[tokio::test]
async fn test_storage_path_relative() {
  let (db, _path, _dir) = setup_test_db().await;
  let result = db.set_storage_path("relative/path").await;

  assert!(matches!(
      result,
      Err(LearnerError::Path(e)) if e.kind() == std::io::ErrorKind::InvalidInput
  ));
}

#[cfg(unix)]
#[traced_test]
#[tokio::test]
async fn test_storage_path_readonly() -> Result<()> {
  use std::os::unix::fs::PermissionsExt;

  let (db, _path, dir) = setup_test_db().await;
  let test_path = dir.path().join("readonly");
  std::fs::create_dir(&test_path)?;

  // Make directory read-only
  std::fs::set_permissions(&test_path, std::fs::Permissions::from_mode(0o444))?;

  let result = db.set_storage_path(&test_path).await;
  assert!(matches!(
      result,
      Err(LearnerError::Path(e)) if e.kind() == std::io::ErrorKind::PermissionDenied
  ));
  Ok(())
}
