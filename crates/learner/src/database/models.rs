//! Database models and type conversions.
//!
//! This module provides the intermediate representations for database rows
//! and their conversions to and from domain types.

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;

use super::*;

/// Represents a paper row from the database.
#[derive(Debug)]
pub struct PaperRow {
  pub id:                i64,
  pub title:             String,
  pub abstract_text:     String,
  pub publication_date:  String,
  pub source:            String,
  pub source_identifier: String,
  pub pdf_url:           Option<String>,
  pub doi:               Option<String>,
}

impl PaperRow {
  /// Creates a new PaperRow from a database row.
  pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
    Ok(Self {
      id:                row.get(0)?,
      title:             row.get(1)?,
      abstract_text:     row.get(2)?,
      publication_date:  row.get(3)?,
      source:            row.get(4)?,
      source_identifier: row.get(5)?,
      pdf_url:           row.get(6)?,
      doi:               row.get(7)?,
    })
  }

  /// Converts this row into a Paper domain object.
  pub fn into_paper(self, authors: Vec<AuthorRow>) -> Result<Paper> {
    Ok(Paper {
      title:             self.title,
      abstract_text:     self.abstract_text,
      publication_date:  DateTime::parse_from_rfc3339(&self.publication_date)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| LearnerError::Database(e.to_string()))?,
      source:            Source::from_str(&self.source)?,
      source_identifier: self.source_identifier,
      pdf_url:           self.pdf_url,
      doi:               self.doi,
      authors:           authors.into_iter().map(|a| a.into_author()).collect(),
    })
  }
}

/// Represents an author row from the database.
#[derive(Debug)]
pub struct AuthorRow {
  pub id:          i64,
  pub paper_id:    i64,
  pub name:        String,
  pub affiliation: Option<String>,
  pub email:       Option<String>,
}

impl AuthorRow {
  /// Creates a new AuthorRow from a database row.
  pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
    Ok(Self {
      id:          row.get(0)?,
      paper_id:    row.get(1)?,
      name:        row.get(2)?,
      affiliation: row.get(3)?,
      email:       row.get(4)?,
    })
  }

  /// Converts this row into an Author domain object.
  pub fn into_author(self) -> Author {
    Author { name: self.name, affiliation: self.affiliation, email: self.email }
  }
}

/// Represents a configuration row from the database.
#[derive(Debug)]
pub struct ConfigRow {
  pub key:   String,
  pub value: String,
}

impl ConfigRow {
  /// Creates a new ConfigRow from a database row.
  pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
    Ok(Self { key: row.get(0)?, value: row.get(1)? })
  }
}

/// Represents a file row from the database.
#[derive(Debug)]
pub struct FileRow {
  pub id:              i64,
  pub paper_id:        i64,
  pub path:            String,
  pub filename:        String,
  pub download_status: String,
  pub error_message:   Option<String>,
}

impl FileRow {
  /// Creates a new FileRow from a database row.
  pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
    Ok(Self {
      id:              row.get(0)?,
      paper_id:        row.get(1)?,
      path:            row.get(2)?,
      filename:        row.get(3)?,
      download_status: row.get(4)?,
      error_message:   row.get(5)?,
    })
  }
}

/// Helper trait for converting domain objects into database parameters.
pub(crate) trait ToParams {
  /// Converts the object into database parameters.
  fn to_params(&self) -> Vec<Box<dyn rusqlite::ToSql>>;
}

impl ToParams for Paper {
  fn to_params(&self) -> Vec<Box<dyn rusqlite::ToSql>> {
    vec![
      Box::new(self.title.clone()),
      Box::new(self.abstract_text.clone()),
      Box::new(self.publication_date.to_rfc3339()),
      Box::new(self.source.to_string()),
      Box::new(self.source_identifier.clone()),
      Box::new(self.pdf_url.clone()),
      Box::new(self.doi.clone()),
    ]
  }
}

#[cfg(test)]
mod tests {
  use rusqlite::{params, Connection};

  use super::*;

  #[test]
  fn test_paper_row_from_row() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
      "CREATE TABLE papers (
                id INTEGER PRIMARY KEY,
                title TEXT,
                abstract_text TEXT,
                publication_date TEXT,
                source TEXT,
                source_identifier TEXT,
                pdf_url TEXT,
                doi TEXT
            )",
    )?;

    conn.execute("INSERT INTO papers VALUES (?, ?, ?, ?, ?, ?, ?, ?)", params![
      1,
      "Test Paper",
      "Abstract",
      "2024-01-01T00:00:00Z",
      "Arxiv",
      "2301.07041",
      "http://example.com/paper.pdf",
      "10.1000/test"
    ])?;

    let mut stmt = conn.prepare("SELECT * FROM papers")?;
    let row = stmt.query_row([], |row| PaperRow::from_row(row))?;

    assert_eq!(row.id, 1);
    assert_eq!(row.title, "Test Paper");
    assert_eq!(row.source_identifier, "2301.07041");

    // Test conversion to Paper
    let authors = vec![AuthorRow {
      id:          1,
      paper_id:    1,
      name:        "Test Author".to_string(),
      affiliation: Some("Test University".to_string()),
      email:       Some("test@example.com".to_string()),
    }];

    let paper = row.into_paper(authors)?;
    assert_eq!(paper.title, "Test Paper");
    assert_eq!(paper.authors.len(), 1);
    assert_eq!(paper.authors[0].name, "Test Author");

    Ok(())
  }
}
