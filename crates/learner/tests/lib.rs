use std::{error::Error, path::PathBuf, str::FromStr};

use learner::{
  error::LearnerError,
  format,
  llm::{LlamaRequest, Model},
  paper::{Author, Paper},
  pdf::PDFContentBuilder,
  prelude::*,
  Config, Learner,
};
use tempfile::{tempdir, TempDir};
use tracing_test::traced_test;

mod database;
mod llm;
mod workflows;

pub type TestResult<T> = Result<T, Box<dyn Error>>;

pub async fn create_test_learner() -> Learner {
  let config_dir = tempdir().unwrap();
  let database_dir = tempdir().unwrap();
  let retrievers_dir = tempdir().unwrap();
  let config = Config::default()
    .with_database_path(database_dir.path())
    .with_retrievers_path(retrievers_dir.path());
  Learner::builder().with_path(config_dir.path()).with_config(config).build().await.unwrap()
}
