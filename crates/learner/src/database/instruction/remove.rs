// database/instruction/remove.rs

use rusqlite::OptionalExtension;

use super::*;

/// Options for configuring the remove operation
#[derive(Default)]
pub struct RemoveOptions {
  /// If true, only simulate the removal and return what would be removed
  pub dry_run: bool,
}

pub struct Remove {
  source:     Source,
  identifier: String,
  options:    RemoveOptions,
}

impl Remove {
  pub fn new(source: Source, identifier: impl Into<String>) -> Self {
    Self { source, identifier: identifier.into(), options: RemoveOptions::default() }
  }

  /// Enable dry run mode - no papers will actually be removed
  pub fn dry_run(mut self) -> Self {
    self.options.dry_run = true;
    self
  }

  /// Helper function to fetch paper data before removal
  fn fetch_papers(tx: &rusqlite::Transaction, id: i64) -> Result<Paper> {
    // Get paper details
    let mut paper_stmt = tx.prepare_cached(
      "SELECT title, abstract_text, publication_date,
                    source, source_identifier, pdf_url, doi
             FROM papers 
             WHERE id = ?",
    )?;

    let paper = paper_stmt.query_row([id], |row| {
      Ok(Paper {
        title:             row.get(0)?,
        abstract_text:     row.get(1)?,
        publication_date:  DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
          .map(|dt| dt.with_timezone(&Utc))
          .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
          })?,
        source:            Source::from_str(&row.get::<_, String>(3)?).map_err(|e| {
          rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
        })?,
        source_identifier: row.get(4)?,
        pdf_url:           row.get(5)?,
        doi:               row.get(6)?,
        authors:           Vec::new(),
      })
    })?;

    // Get authors
    let mut author_stmt = tx.prepare_cached(
      "SELECT name, affiliation, email
             FROM authors
             WHERE paper_id = ?",
    )?;

    let authors = author_stmt
      .query_map([id], |row| {
        Ok(Author { name: row.get(0)?, affiliation: row.get(1)?, email: row.get(2)? })
      })?
      .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Paper { authors, ..paper })
  }
}

impl DatabaseInstruction for Remove {
  type Output = Vec<Paper>;

  fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let tx = db.conn.transaction()?;

    // First get the paper IDs to remove
    let paper_id: Option<i64> = {
      let mut stmt = tx.prepare_cached(
        "SELECT id FROM papers 
                 WHERE source = ?1 AND source_identifier = ?2",
      )?;

      stmt
        .query_row(params![self.source.to_string(), self.identifier], |row| row.get(0))
        .optional()?
    };

    let Some(id) = paper_id else {
      return Ok(Vec::new());
    };

    // Fetch the paper data before removal
    let paper = Self::fetch_papers(&tx, id)?;
    let removed_papers = vec![paper];

    if !self.options.dry_run {
      // Remove authors first (though this should cascade automatically)
      tx.execute("DELETE FROM authors WHERE paper_id = ?1", params![id])?;

      // Remove the paper
      tx.execute("DELETE FROM papers WHERE id = ?1", params![id])?;

      tx.commit()?;
    }

    Ok(removed_papers)
  }
}
