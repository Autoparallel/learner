//! Remove instruction implementation for paper deletion from the database.
//!
//! This module provides functionality for safely removing papers and their associated
//! data from the database. It supports:
//!
//! - Query-based paper removal
//! - Dry run simulation
//! - Cascade deletion of related data
//! - Atomic transactions
//!
//! The implementation emphasizes:
//! - Safe deletion with transaction support
//! - Cascading removals across related tables
//! - Validation before deletion
//! - Preview capabilities through dry runs
//!
//! # Examples
//!
//! ```no_run
//! use learner::{
//!   database::{Database, Query, Remove},
//!   paper::Source,
//!   prelude::*,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut db = Database::open("papers.db").await?;
//!
//! // Remove a specific paper
//! Remove::by_source(Source::Arxiv, "2301.07041").execute(&mut db).await?;
//!
//! // Preview deletion with dry run
//! let papers = Remove::by_author("Alice Researcher").dry_run().execute(&mut db).await?;
//!
//! println!("Would remove {} papers", papers.len());
//! # Ok(())
//! # }
//! ```

use super::*;

/// Configuration options for paper removal operations.
///
/// This struct allows customization of how the remove operation
/// behaves, particularly useful for validation and testing.
#[derive(Default)]
pub struct RemoveOptions {
  /// When true, simulates the removal operation without modifying the database.
  ///
  /// This is useful for:
  /// - Previewing which papers would be removed
  /// - Validating removal queries
  /// - Testing removal logic safely
  pub dry_run: bool,
}

/// Instruction for removing papers from the database.
///
/// This struct implements the [`DatabaseInstruction`] trait to provide
/// paper removal functionality. It handles:
///
/// - Paper identification through queries
/// - Related data cleanup (authors, files)
/// - Transaction management
/// - Dry run simulation
pub struct Remove<'a> {
  /// The query identifying papers to remove
  query:   Query<'a>,
  /// Configuration options for the removal
  options: RemoveOptions,
}

impl<'a> Remove<'a> {
  /// Creates a remove instruction from an existing query.
  ///
  /// This method allows any query to be converted into a remove operation,
  /// providing maximum flexibility in identifying papers to remove.
  ///
  /// # Arguments
  ///
  /// * `query` - The query that identifies papers to remove
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::{Remove, Query};
  /// // Remove papers matching a text search
  /// let query = Query::text("quantum computing");
  /// let remove = Remove::from_query(query);
  ///
  /// // Remove papers before a date
  /// use chrono::{DateTime, Utc};
  /// let date = DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
  /// let query = Query::before_date(date);
  /// let remove = Remove::from_query(query);
  /// ```
  pub fn from_query(query: Query<'a>) -> Self { Self { query, options: RemoveOptions::default() } }

  /// Creates a remove instruction for a specific paper by its source and identifier.
  ///
  /// This is a convenience method for the common case of removing
  /// a single paper identified by its source system and ID.
  ///
  /// # Arguments
  ///
  /// * `source` - The paper's source system (arXiv, DOI, etc.)
  /// * `identifier` - The source-specific identifier
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Remove;
  /// # use learner::paper::Source;
  /// // Remove an arXiv paper
  /// let remove = Remove::by_source(Source::Arxiv, "2301.07041");
  ///
  /// // Remove a DOI paper
  /// let remove = Remove::by_source(Source::DOI, "10.1145/1327452.1327492");
  /// ```
  pub fn by_source(source: Source, identifier: &'a str) -> Self {
    Self::from_query(Query::by_source(source, identifier))
  }

  /// Creates a remove instruction for all papers by a specific author.
  ///
  /// This method provides a way to remove all papers associated with
  /// a particular author name. It performs partial matching on the name.
  ///
  /// # Arguments
  ///
  /// * `name` - The author name to match
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Remove;
  /// // Remove all papers by an author
  /// let remove = Remove::by_author("Alice Researcher");
  /// ```
  pub fn by_author(name: &'a str) -> Self { Self::from_query(Query::by_author(name)) }

  /// Enables dry run mode for the remove operation.
  ///
  /// In dry run mode, the operation will:
  /// - Query papers that would be removed
  /// - Return the list of papers
  /// - Not modify the database
  ///
  /// This is useful for:
  /// - Previewing removal operations
  /// - Validating queries
  /// - Testing removal logic
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{database::Remove, prelude::*};
  /// # use learner::paper::Source;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// # let mut db = learner::database::Database::open("papers.db").await?;
  /// // Preview papers that would be removed
  /// let papers = Remove::by_author("Alice Researcher").dry_run().execute(&mut db).await?;
  ///
  /// println!("Would remove {} papers", papers.len());
  /// # Ok(())
  /// # }
  /// ```
  pub fn dry_run(mut self) -> Self {
    self.options.dry_run = true;
    self
  }

  /// Builds SQL to retrieve paper IDs for removal.
  ///
  /// Generates the SQL and parameters needed to find database IDs
  /// for papers matching the removal criteria.
  fn build_paper_ids_sql(paper: &Paper) -> (String, Vec<Option<String>>) {
    ("SELECT id FROM papers WHERE source = ? AND source_identifier = ?".to_string(), vec![
      Some(paper.source.to_string()),
      Some(paper.source_identifier.clone()),
    ])
  }

  /// Builds SQL to remove papers and all related data.
  ///
  /// Generates cascading DELETE statements to remove papers and their
  /// associated data (authors, files) in the correct order to maintain
  /// referential integrity.
  fn build_remove_sql(ids: &[i64]) -> (String, Vec<Option<String>>) {
    let ids_str = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    (
      format!(
        "DELETE FROM authors WHERE paper_id IN ({0});
                 DELETE FROM files WHERE paper_id IN ({0});
                 DELETE FROM papers WHERE id IN ({0});",
        ids_str
      ),
      Vec::new(), // No params needed since IDs are embedded in SQL
    )
  }
}

#[async_trait]
impl DatabaseInstruction for Remove<'_> {
  type Output = Vec<Paper>;

  async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    // Use Query to find the papers to remove
    let papers = self.query.execute(db).await?;

    if !self.options.dry_run && !papers.is_empty() {
      // Collect all paper IDs
      let papers_clone = papers.clone();
      let ids: Vec<i64> = db
        .conn
        .call(move |conn| {
          let mut ids = Vec::new();
          let tx = conn.transaction()?;

          for paper in &papers_clone {
            let (sql, params) = Self::build_paper_ids_sql(paper);
            if let Ok(id) = tx.query_row(&sql, params_from_iter(params), |row| row.get(0)) {
              ids.push(id);
            }
          }

          tx.commit()?;
          Ok(ids)
        })
        .await?;

      if !ids.is_empty() {
        // Remove the papers and their related data
        let (remove_sql, _) = Self::build_remove_sql(&ids);

        db.conn
          .call(move |conn| {
            let tx = conn.transaction()?;
            tx.execute_batch(&remove_sql)?;
            tx.commit()?;
            Ok(())
          })
          .await?;
      }
    }

    Ok(papers)
  }
}
