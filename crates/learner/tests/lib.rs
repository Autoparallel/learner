use std::{error::Error, path::PathBuf, str::FromStr};

use learner::{
  error::LearnerError,
  format,
  llm::{LlamaRequest, Model},
  paper::{Author, Paper},
  pdf::PDFContentBuilder,
  prelude::*,
};
use tempfile::{tempdir, TempDir};
use tracing_test::traced_test;

mod database;
mod llm;

pub type TestResult<T> = Result<T, Box<dyn Error>>;
