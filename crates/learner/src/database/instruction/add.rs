use query::Query;

use super::*;

/// What we're trying to add
pub enum Addition {
  /// Add just the paper metadata
  Paper(Paper),
  /// Add both paper and its document
  Complete(Paper),
  /// Add documents for papers matching a query
  Documents(Query),
}

pub struct Add {
  addition: Addition,
}

impl Add {
  /// Add a new paper to the database
  pub fn paper(paper: Paper) -> Self { Self { addition: Addition::Paper(paper) } }

  /// Add a new paper along with its document
  pub fn complete(paper: Paper) -> Self { Self { addition: Addition::Complete(paper) } }

  /// Add documents for papers matching the query
  pub fn documents(query: Query) -> Self { Self { addition: Addition::Documents(query) } }

  /// Chain a document addition to a paper addition
  pub fn with_document(self) -> Self {
    match self.addition {
      Addition::Paper(paper) => Self { addition: Addition::Complete(paper) },
      _ => self,
    }
  }

  /// Helper to check for existing paper
  async fn check_existing_paper(db: &mut Database, paper: &Paper) -> Result<Paper> {
    Query::by_source(paper.source.clone(), &paper.source_identifier)
      .execute(db)
      .await?
      .into_iter()
      .next()
      .ok_or(LearnerError::DatabasePaperNotFound)
  }

  /// Helper to check for existing document
  fn check_existing_document(db: &mut Database, paper: &Paper) -> Result<bool> {
    let tx = db.conn.transaction()?;
    let res = tx
      .prepare_cached(
        "SELECT EXISTS(
              SELECT 1 FROM files f
              JOIN papers p ON p.id = f.paper_id
              WHERE p.source = ? 
              AND p.source_identifier = ? 
              AND f.download_status = 'Success'
          )",
      )?
      .query_row(params![paper.source.to_string(), paper.source_identifier], |row| row.get(0))?;
    Ok(res)
  }

  /// Helper to store document for a paper
  async fn store_document(db: &mut Database, paper: &Paper) -> Result<()> {
    let storage_path = db.get_storage_path()?;
    let filename = paper.download_pdf(&storage_path).await?;

    let tx = db.conn.transaction()?;
    tx.execute(
      "INSERT INTO files (paper_id, path, filename, download_status)
             SELECT p.id, ?, ?, 'Success'
             FROM papers p
             WHERE p.source = ? AND p.source_identifier = ?",
      params![
        storage_path.to_string_lossy(),
        filename.to_string_lossy(),
        paper.source.to_string(),
        paper.source_identifier,
      ],
    )?;
    tx.commit()?;

    Ok(())
  }
}

#[async_trait::async_trait]
impl DatabaseInstruction for Add {
  type Output = Vec<Paper>;

  // Return affected papers

  async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let storage_path = db.get_storage_path()?;

    match &self.addition {
      Addition::Paper(paper) => {
        // Check for existing paper
        if let Err(LearnerError::DatabasePaperNotFound) =
          Self::check_existing_paper(db, paper).await
        {
        } else {
          return Err(LearnerError::DatabaseDuplicatePaper(paper.title.clone()));
        }

        // Add the paper
        let tx = db.conn.transaction()?;
        tx.execute(
          "INSERT INTO papers (
                        title, abstract_text, publication_date,
                        source, source_identifier, pdf_url, doi
                    ) VALUES (?, ?, ?, ?, ?, ?, ?)",
          params![
            paper.title,
            paper.abstract_text,
            paper.publication_date.to_rfc3339(),
            paper.source.to_string(),
            paper.source_identifier,
            paper.pdf_url,
            paper.doi,
          ],
        )?;

        // Add authors
        for author in &paper.authors {
          tx.execute(
            "INSERT INTO authors (paper_id, name, affiliation, email)
                         SELECT id, ?, ?, ?
                         FROM papers
                         WHERE source = ? AND source_identifier = ?",
            params![
              author.name,
              author.affiliation,
              author.email,
              paper.source.to_string(),
              paper.source_identifier,
            ],
          )?;
        }
        tx.commit()?;

        Ok(vec![paper.clone()])
      },

      Addition::Complete(paper) => {
        // Add paper first
        Add::paper(paper.clone()).execute(db).await?;

        // Then add document
        Self::store_document(db, paper).await?;
        Ok(vec![paper.clone()])
      },

      Addition::Documents(query) => {
        let mut added = Vec::new();
        let papers = query.execute(db).await?;

        for paper in papers {
          if !Self::check_existing_document(db, &paper)? {
            match Self::store_document(db, &paper).await {
              Ok(()) => added.push(paper),
              Err(e) => eprintln!("Failed to store document for {}: {}", paper.title, e),
            }
          }
        }

        Ok(added)
      },
    }
  }
}
