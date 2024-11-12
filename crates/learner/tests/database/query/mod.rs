use learner::database::{Database, *};
use tempfile::{tempdir, TempDir};

mod add;
mod search;

fn setup_test_db() -> (Database, TempDir) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  let db = Database::open(&db_path).unwrap();
  (db, dir)
}
