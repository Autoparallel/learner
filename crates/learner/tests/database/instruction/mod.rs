use learner::database::{Database, *};
use tempfile::{tempdir, TempDir};

mod add;
mod query;
mod remove;

async fn setup_test_db() -> (Database, TempDir) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  let db = Database::open(&db_path).await.unwrap();
  (db, dir)
}
