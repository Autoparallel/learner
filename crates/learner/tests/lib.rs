use std::{error::Error, path::PathBuf};

use learner::{
  database_old::Database,
  format,
  llm::{LlamaRequest, Model},
  paper::{Author, Paper, Source},
  pdf::PDFContentBuilder,
};
use tempfile::{tempdir, TempDir};
use tracing_test::traced_test;

mod database;
mod llm;

/// Helper function to set up a test database
async fn setup_test_db() -> (Database, TempDir) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  let db = Database::open(&db_path).await.unwrap();
  (db, dir)
}

/// Helper function to create a test paper
fn create_test_paper() -> Paper {
  Paper {
    title:             "Test Paper".to_string(),
    abstract_text:     "This is a test abstract".to_string(),
    publication_date:  chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2024, 1, 1, 0, 0, 0)
      .unwrap(),
    source:            Source::Arxiv,
    source_identifier: "2401.00000".to_string(),
    pdf_url:           Some("https://arxiv.org/pdf/2401.00000".to_string()),
    doi:               Some("10.1000/test.123".to_string()),
    authors:           vec![
      Author {
        name:        "John Doe".to_string(),
        affiliation: Some("Test University".to_string()),
        email:       Some("john@test.edu".to_string()),
      },
      Author { name: "Jane Smith".to_string(), affiliation: None, email: None },
    ],
  }
}
