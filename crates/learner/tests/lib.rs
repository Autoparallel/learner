use std::{
  error::Error,
  path::{Path, PathBuf},
  str::FromStr,
};

use learner::{
  error::LearnerError,
  llm::{LlamaRequest, Model},
  pdf::PDFContentBuilder,
  prelude::*,
  resource::{Author, Paper},
  Config, Learner,
};
use tempfile::{tempdir, TempDir};
use tracing_test::traced_test;

mod llm;
mod workflows;

pub type TestResult<T> = Result<T, Box<dyn Error>>;

// #[tokio::test]
pub async fn create_test_learner() -> (Learner, TempDir, TempDir, TempDir) {
  let config_dir = tempdir().unwrap();
  let database_dir = tempdir().unwrap();
  let storage_dir = tempdir().unwrap();
  let config = Config::default()
    .with_database_path(&database_dir.path().join("learner.db"))
    .with_retrievers_path(Path::new("config/retrievers/"))
    .with_resources_path(Path::new("config/resources/"))
    .with_storage_path(storage_dir.path());
  let learner =
    Learner::builder().with_path(config_dir.path()).with_config(config).build().await.unwrap();
  (learner, config_dir, database_dir, storage_dir)
}
