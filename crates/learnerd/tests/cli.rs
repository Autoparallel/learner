//! Integration tests for the learnerd CLI commands.
//!
//! Basic functionality tests running in serial to avoid database conflicts.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use tempfile::tempdir;

// Helper function to create a clean command instance
fn learner() -> Command { Command::cargo_bin("learner").unwrap() }

// Helper to get a temporary database path
fn temp_db() -> (tempfile::TempDir, PathBuf) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  (dir, db_path)
}

#[test]
#[serial]
fn test_init_and_clean() {
  let (dir, db_path) = temp_db();

  // Initialize database
  learner()
    .arg("init")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("initialized successfully"));

  assert!(db_path.exists());

  // Clean with force flag
  learner()
    .arg("clean")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("Database files cleaned"));

  assert!(!db_path.exists());
  dir.close().unwrap();
}

#[tokio::test]
#[serial]
async fn test_basic_paper_workflow() {
  let (dir, db_path) = temp_db();

  // Initialize database
  learner().arg("init").arg("--path").arg(&db_path).arg("--accept-defaults").assert().success();

  // Add a paper
  learner()
    .arg("add")
    .arg("2301.07041")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("Found paper"))
    .stdout(predicate::str::contains("Verifiable Fully Homomorphic"));

  // Try adding same paper again to test duplicate handling
  learner()
    .arg("add")
    .arg("2301.07041")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("already in your database"));

  // Get the paper
  learner()
    .arg("get")
    .arg("arxiv")
    .arg("2301.07041")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("Paper details"))
    .stdout(predicate::str::contains("Verifiable Fully Homomorphic"));

  // Search for the paper
  learner()
    .arg("search")
    .arg("Homomorphic")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("Found"))
    .stdout(predicate::str::contains("Verifiable Fully Homomorphic"));

  // Search for nonexistent paper
  learner()
    .arg("search")
    .arg("ThisPaperDoesNotExist123")
    .arg("--path")
    .arg(&db_path)
    .arg("--accept-defaults")
    .assert()
    .success()
    .stdout(predicate::str::contains("No papers found"));

  dir.close().unwrap();
}
