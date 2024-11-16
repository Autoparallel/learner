use chrono::{TimeZone, Utc};
use learner::{
  database::{
    add::Add,
    query::{OrderField, Query},
    remove::Remove,
    Database,
  },
  paper::Source,
};

use super::*;

mod add;
mod query;
mod remove;

async fn setup_test_db() -> (Database, TempDir) {
  let dir = tempdir().unwrap();
  let db_path = dir.path().join("test.db");
  let db = Database::open(&db_path).await.unwrap();
  db.set_storage_path(dir.path()).await.unwrap();
  (db, dir)
}

/// Helper function to create a test paper
fn create_test_paper() -> Paper {
  Paper {
    title:             "Test Paper".to_string(),
    abstract_text:     "This is a test abstract".to_string(),
    publication_date:  chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2023, 1, 1, 0, 0, 0)
      .unwrap(),
    source:            Source::Arxiv,
    source_identifier: "2301.00000".to_string(),
    pdf_url:           Some("https://arxiv.org/pdf/2301.00000".to_string()),
    doi:               Some("10.0000/test.123".to_string()),
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

fn create_second_test_paper() -> Paper {
  Paper {
    title:             "Test Paper: Two".to_string(),
    abstract_text:     "This is a test abstract, but again!".to_string(),
    publication_date:  chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2024, 1, 1, 0, 0, 0)
      .unwrap(),
    source:            Source::Arxiv,
    source_identifier: "2401.00000".to_string(),
    pdf_url:           Some("https://arxiv.org/pdf/2401.00000".to_string()),
    doi:               Some("10.1000/test.1234".to_string()),
    authors:           vec![
      Author {
        name:        "Alice Scientist".to_string(),
        affiliation: Some("Test State University".to_string()),
        email:       Some("john@test.edu".to_string()),
      },
      Author { name: "Bob Researcher".to_string(), affiliation: None, email: None },
    ],
  }
}

#[tokio::test]
#[traced_test]
async fn test_download_test_paper_is_404() {
  let paper = create_test_paper();
  assert!(paper.download_pdf(&PathBuf::from_str(".").unwrap()).await.is_err());
}
