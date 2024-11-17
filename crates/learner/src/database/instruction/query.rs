//! Query instruction implementation for retrieving papers from the database.
//!
//! This module provides a flexible query system for searching and retrieving papers
//! using various criteria. It supports:
//!
//! - Full-text search across titles and abstracts
//! - Source-specific identifier lookups
//! - Author name searches
//! - Publication date filtering
//! - Custom result ordering
//!
//! The implementation prioritizes:
//! - Efficient query execution using prepared statements
//! - SQLite full-text search integration
//! - Type-safe query construction
//! - Flexible result ordering
//!
//! # Examples
//!
//! ```no_run
//! use learner::{
//!   database::{Database, OrderField, Query},
//!   paper::Source,
//!   prelude::*,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut db = Database::open("papers.db").await?;
//!
//! // Full-text search
//! let papers = Query::text("quantum computing")
//!   .order_by(OrderField::PublicationDate)
//!   .descending()
//!   .execute(&mut db)
//!   .await?;
//!
//! // Search by author
//! let papers = Query::by_author("Alice Researcher").execute(&mut db).await?;
//!
//! // Lookup by source identifier
//! let papers = Query::by_source(Source::Arxiv, "2301.07041").execute(&mut db).await?;
//! # Ok(())
//! # }
//! ```

use super::*;

/// Represents different ways to query papers in the database.
///
/// This enum defines the supported search criteria for paper queries,
/// each providing different ways to locate papers in the database:
///
/// - Text-based searching using SQLite FTS
/// - Direct lookups by source identifiers
/// - Author-based searches
/// - Publication date filtering
/// - Complete collection retrieval
#[derive(Debug)]
pub enum QueryCriteria<'a> {
  /// Full-text search across titles and abstracts using SQLite FTS
  Text(&'a str),
  /// Direct lookup by source system and identifier
  SourceId {
    /// The source system (e.g., arXiv, DOI)
    source:     Source,
    /// The source-specific identifier
    identifier: &'a str,
  },
  /// Search by author name with partial matching
  Author(&'a str),
  /// Retrieve the complete paper collection
  All,
  /// Filter papers by publication date
  BeforeDate(DateTime<Utc>),
}

/// Available fields for ordering query results.
///
/// This enum defines the paper attributes that can be used for
/// sorting query results. Each field maps to specific database
/// columns and handles appropriate comparison logic.
#[derive(Debug, Clone, Copy)]
pub enum OrderField {
  /// Order alphabetically by paper title
  Title,
  /// Order chronologically by publication date
  PublicationDate,
  /// Order by source system and identifier
  Source,
}

impl OrderField {
  /// Converts the ordering field to its SQL representation.
  ///
  /// Returns the appropriate SQL column names for ORDER BY clauses,
  /// handling both single-column and multi-column ordering.
  fn as_sql_str(&self) -> &'static str {
    match self {
      OrderField::Title => "title",
      OrderField::PublicationDate => "publication_date",
      OrderField::Source => "source, source_identifier",
    }
  }
}

/// A query builder for retrieving papers from the database.
///
/// This struct provides a fluent interface for constructing paper queries,
/// supporting various search criteria and result ordering options. It handles:
///
/// - Query criteria specification
/// - Result ordering configuration
/// - SQL generation and execution
/// - Paper reconstruction from rows
#[derive(Debug)]
pub struct Query<'a> {
  /// The search criteria to apply
  criteria:   QueryCriteria<'a>,
  /// Optional field to sort results by
  order_by:   Option<OrderField>,
  /// Whether to sort in descending order
  descending: bool,
}

impl<'a> Query<'a> {
  /// Creates a new query with the given criteria.
  ///
  /// # Arguments
  ///
  /// * `criteria` - The search criteria to use
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::{Query, QueryCriteria};
  /// let query = Query::new(QueryCriteria::All);
  /// ```
  pub fn new(criteria: QueryCriteria<'a>) -> Self {
    Self { criteria, order_by: None, descending: false }
  }

  /// Creates a full-text search query.
  ///
  /// Searches through paper titles and abstracts using SQLite's FTS5
  /// full-text search engine with wildcard matching.
  ///
  /// # Arguments
  ///
  /// * `query` - The text to search for
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// let query = Query::text("quantum computing");
  /// ```
  pub fn text(query: &'a str) -> Self { Self::new(QueryCriteria::Text(query)) }

  /// Creates a query to find a specific paper.
  ///
  /// # Arguments
  ///
  /// * `paper` - The paper whose source and identifier should be matched
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// # use learner::paper::Paper;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let paper = Paper::new("2301.07041").await?;
  /// let query = Query::by_paper(&paper);
  /// # Ok(())
  /// # }
  /// ```
  pub fn by_paper(paper: &'a Paper) -> Self {
    Self::new(QueryCriteria::SourceId {
      source:     paper.source,
      identifier: &paper.source_identifier,
    })
  }

  /// Creates a query to find a paper by its source and identifier.
  ///
  /// # Arguments
  ///
  /// * `source` - The paper source (arXiv, DOI, etc.)
  /// * `identifier` - The source-specific identifier
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// # use learner::paper::Source;
  /// let query = Query::by_source(Source::Arxiv, "2301.07041");
  /// ```
  pub fn by_source(source: Source, identifier: &'a str) -> Self {
    Self::new(QueryCriteria::SourceId { source, identifier })
  }

  /// Creates a query to find papers by author name.
  ///
  /// Performs a partial match on author names, allowing for flexible
  /// name searches.
  ///
  /// # Arguments
  ///
  /// * `name` - The author name to search for
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// let query = Query::by_author("Alice Researcher");
  /// ```
  pub fn by_author(name: &'a str) -> Self { Self::new(QueryCriteria::Author(name)) }

  /// Creates a query that returns all papers.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// let query = Query::list_all();
  /// ```
  pub fn list_all() -> Self { Self::new(QueryCriteria::All) }

  /// Creates a query for papers published before a specific date.
  ///
  /// # Arguments
  ///
  /// * `date` - The cutoff date for publication
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Query;
  /// # use chrono::{DateTime, Utc};
  /// let date = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
  /// let query = Query::before_date(date);
  /// ```
  pub fn before_date(date: DateTime<Utc>) -> Self { Self::new(QueryCriteria::BeforeDate(date)) }

  /// Sets the field to order results by.
  ///
  /// # Arguments
  ///
  /// * `field` - The field to sort by
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::{Query, OrderField};
  /// let query = Query::list_all().order_by(OrderField::PublicationDate);
  /// ```
  pub fn order_by(mut self, field: OrderField) -> Self {
    self.order_by = Some(field);
    self
  }

  /// Sets the order to descending (default is ascending).
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::{Query, OrderField};
  /// let query = Query::list_all().order_by(OrderField::PublicationDate).descending();
  /// ```
  pub fn descending(mut self) -> Self {
    self.descending = true;
    self
  }

  /// Builds the SQL for retrieving paper IDs based on search criteria.
  fn build_criteria_sql(&self) -> (String, Vec<impl ToSql>) {
    match &self.criteria {
      QueryCriteria::Text(query) => (
        "SELECT p.id
                 FROM papers p
                 JOIN papers_fts f ON p.id = f.rowid
                 WHERE papers_fts MATCH ?1 || '*'
                 ORDER BY rank"
          .into(),
        vec![(*query).to_string()],
      ),
      QueryCriteria::SourceId { source, identifier } => (
        "SELECT id FROM papers 
                 WHERE source = ?1 AND source_identifier = ?2"
          .into(),
        vec![source.to_string(), (*identifier).to_string()],
      ),
      QueryCriteria::Author(name) => (
        "SELECT DISTINCT p.id
                 FROM papers p
                 JOIN authors a ON p.id = a.paper_id
                 WHERE a.name LIKE ?1"
          .into(),
        vec![format!("%{}%", name)],
      ),
      QueryCriteria::All => ("SELECT id FROM papers".into(), Vec::new()),
      QueryCriteria::BeforeDate(date) => (
        "SELECT id FROM papers 
                 WHERE publication_date < ?1"
          .into(),
        vec![date.to_rfc3339()],
      ),
    }
  }

  /// Builds the SQL for retrieving complete paper data.
  fn build_paper_sql(&self) -> String {
    let base = "SELECT title, abstract_text, publication_date,
                           source, source_identifier, pdf_url, doi
                    FROM papers 
                    WHERE id = ?1";

    if let Some(order_field) = &self.order_by {
      let direction = if self.descending { "DESC" } else { "ASC" };
      format!("{} ORDER BY {} {}", base, order_field.as_sql_str(), direction)
    } else {
      base.to_string()
    }
  }
}

#[async_trait]
impl DatabaseInstruction for Query<'_> {
  type Output = Vec<Paper>;

  async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let (criteria_sql, params) = self.build_criteria_sql();
    let paper_sql = self.build_paper_sql();
    let order_by = self.order_by;
    let descending = self.descending;

    let papers = db
      .conn
      .call(move |conn| {
        let mut papers = Vec::new();
        let tx = conn.transaction()?;

        // Get paper IDs based on search criteria
        let paper_ids = {
          let mut stmt = tx.prepare_cached(&criteria_sql)?;
          let mut rows = stmt.query(params_from_iter(params))?;
          let mut ids = Vec::new();
          while let Some(row) = rows.next()? {
            ids.push(row.get::<_, i64>(0)?);
          }
          ids
        };

        // Fetch complete paper data for each ID
        for paper_id in paper_ids {
          let mut paper_stmt = tx.prepare_cached(&paper_sql)?;
          let paper = paper_stmt.query_row([paper_id], |row| {
            Ok(Paper {
              title:             row.get(0)?,
              abstract_text:     row.get(1)?,
              publication_date:  DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                  rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                  )
                })?,
              source:            Source::from_str(&row.get::<_, String>(3)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                  3,
                  rusqlite::types::Type::Text,
                  Box::new(e),
                )
              })?,
              source_identifier: row.get(4)?,
              pdf_url:           row.get(5)?,
              doi:               row.get(6)?,
              authors:           Vec::new(),
            })
          })?;

          // Get authors for this paper
          let mut author_stmt = tx.prepare_cached(
            "SELECT name, affiliation, email
                     FROM authors
                     WHERE paper_id = ?",
          )?;

          let authors = author_stmt
            .query_map([paper_id], |row| {
              Ok(Author {
                name:        row.get(0)?,
                affiliation: row.get(1)?,
                email:       row.get(2)?,
              })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

          let mut paper = paper;
          paper.authors = authors;
          papers.push(paper);
        }

        // Sort if needed
        if let Some(order_field) = order_by {
          papers.sort_by(|a, b| {
            let cmp = match order_field {
              OrderField::Title => a.title.cmp(&b.title),
              OrderField::PublicationDate => a.publication_date.cmp(&b.publication_date),
              OrderField::Source => (a.source.to_string(), &a.source_identifier)
                .cmp(&(b.source.to_string(), &b.source_identifier)),
            };
            if descending {
              cmp.reverse()
            } else {
              cmp
            }
          });
        }

        Ok(papers)
      })
      .await?;

    Ok(papers)
  }
}
