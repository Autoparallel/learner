// NOTES:
// Instead of calling this `pdf`, I think `document` may be better. Now, implementing this properly
// is going to take modifying the `Paper` struct to have it hold on to its own id created by a hash,
// it should also hold onto a filename it is associated with, this would make life easier down the
// road too.

// database/instruction/pdf.rs
use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

pub enum PdfAction {
  Store(QueryCriteria), // Use existing query criteria
  GetStatus(i64),       // Get status by paper ID
}

pub enum PdfStatus {
  Success,
  Failed,
  Pending,
}

pub struct Pdf {
  action: PdfAction,
}

impl Paper {
  // Add method to generate paper ID
  pub fn id(&self) -> i64 {
    let mut hasher = DefaultHasher::new();
    self.hash(&mut hasher);
    hasher.finish() as i64
  }
}

// Make Paper hashable for ID generation
impl Hash for Paper {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.title.hash(state);
    self.abstract_text.hash(state);
    self.publication_date.to_rfc3339().hash(state);
    self.source.to_string().hash(state);
    self.source_identifier.hash(state);
    // Note: We don't hash PDF URL or DOI as they might not be available
    // but are still the same paper
  }
}

impl Pdf {
  /// Store PDFs for papers matching the given criteria
  pub fn store(criteria: impl Into<QueryCriteria>) -> Self {
    Self { action: PdfAction::Store(criteria.into()) }
  }

  /// Store PDF for a specific paper ID
  pub fn store_one(paper_id: i64) -> Self {
    Self { action: PdfAction::Store(QueryCriteria::PaperId(paper_id)) }
  }

  /// Get status of a specific paper's PDF
  pub fn status(paper_id: i64) -> Self { Self { action: PdfAction::GetStatus(paper_id) } }

  // Helper to generate standardized filename
  fn generate_filename(paper: &Paper) -> String { format!("{}.pdf", paper.id()) }
}

impl DatabaseInstruction for Pdf {
  type Output = Vec<(i64, PdfStatus)>;

  // Returns paper IDs and their PDF statuses

  fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let tx = db.conn.transaction()?;

    match &self.action {
      PdfAction::Store(criteria) => {
        // Use Query to find matching papers
        let papers = Query::new(criteria.clone()).execute_in_tx(&tx)?;
        let mut results = Vec::new();

        for paper in papers {
          let paper_id = paper.id();
          let filename = Self::generate_filename(&paper);
          let pdf_dir = db.get_pdf_dir(&tx)?;
          let path = pdf_dir.join(&filename);

          // Initial status is pending
          tx.execute(
            "INSERT OR REPLACE INTO files (
                            paper_id, path, filename, download_status, error_message
                        ) VALUES (?, ?, ?, ?, ?)",
            params![
              paper_id,
              path.to_string_lossy().to_string(),
              filename,
              "Pending",
              Option::<String>::None,
            ],
          )?;

          results.push((paper_id, PdfStatus::Pending));
        }

        tx.commit()?;
        Ok(results)
      },
      PdfAction::GetStatus(paper_id) => {
        let result =
          tx.query_row("SELECT download_status FROM files WHERE paper_id = ?", [paper_id], |row| {
            let status: String = row.get(0)?;
            Ok(match status.as_str() {
              "Success" => PdfStatus::Success,
              "Failed" => PdfStatus::Failed,
              _ => PdfStatus::Pending,
            })
          });

        match result {
          Ok(status) => Ok(vec![(*paper_id, status)]),
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(vec![]),
          Err(e) => Err(e.into()),
        }
      },
    }
  }
}

// Usage examples:
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_store_pdfs_by_query() -> Result<()> {
    let (mut db, _dir) = setup_test_db();

    // Add some test papers
    Add::new(create_test_paper()).execute(&mut db)?;
    Add::new(create_second_test_paper()).execute(&mut db)?;

    // Store PDFs for all papers with "quantum" in title
    let results = Pdf::store(Query::text("quantum")).execute(&mut db)?;

    assert!(!results.is_empty());
    for (_, status) in results {
      assert!(matches!(status, PdfStatus::Pending));
    }

    Ok(())
  }

  #[test]
  fn test_store_single_pdf() -> Result<()> {
    let (mut db, _dir) = setup_test_db();

    let paper = create_test_paper();
    let paper_id = paper.id();
    Add::new(paper).execute(&mut db)?;

    let results = Pdf::store_one(paper_id).execute(&mut db)?;

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].1, PdfStatus::Pending));

    Ok(())
  }
}
