//! Database instruction implementations for paper management.
//!
//! This module provides a trait-based abstraction for database operations,
//! allowing for type-safe and composable database queries and modifications.
//! Each instruction type implements specific database operations while
//! maintaining proper borrowing semantics and async safety.

use super::*;

/// Represents different ways to query papers in the database.
#[derive(Debug)]
pub enum QueryCriteria<'a> {
  /// Full-text search across titles and abstracts
  Text(&'a str),
  /// Search by source system and identifier
  SourceId { source: Source, identifier: &'a str },
  /// Search by author name (partial matches supported)
  Author(&'a str),
  /// Retrieve all papers
  All,
  /// Papers published before a specific date
  BeforeDate(DateTime<Utc>),
}

/// Available fields for ordering query results
#[derive(Debug, Clone, Copy)]
pub enum OrderField {
  /// Order by paper title
  Title,
  /// Order by publication date
  PublicationDate,
  /// Order by source and identifier
  Source,
}

impl OrderField {
  fn as_sql_str(&self) -> &'static str {
    match self {
      OrderField::Title => "title",
      OrderField::PublicationDate => "publication_date",
      OrderField::Source => "source, source_identifier",
    }
  }
}

/// A query for retrieving papers from the database
#[derive(Debug)]
pub struct Query<'a> {
  criteria:   QueryCriteria<'a>,
  order_by:   Option<OrderField>,
  descending: bool,
}

impl<'a> Query<'a> {
  /// Creates a new query with the given criteria
  pub fn new(criteria: QueryCriteria<'a>) -> Self {
    Self { criteria, order_by: None, descending: false }
  }

  /// Creates a full-text search query
  pub fn text(query: &'a str) -> Self { Self::new(QueryCriteria::Text(query)) }

  /// Creates a query to find a paper by its source and identifier
  pub fn by_source(source: Source, identifier: &'a str) -> Self {
    Self::new(QueryCriteria::SourceId { source, identifier })
  }

  /// Creates a query to find papers by author name
  pub fn by_author(name: &'a str) -> Self { Self::new(QueryCriteria::Author(name)) }

  /// Creates a query that returns all papers
  pub fn list_all() -> Self { Self::new(QueryCriteria::All) }

  /// Creates a query for papers published before a specific date
  pub fn before_date(date: DateTime<Utc>) -> Self { Self::new(QueryCriteria::BeforeDate(date)) }

  /// Sets the field to order results by
  pub fn order_by(mut self, field: OrderField) -> Self {
    self.order_by = Some(field);
    self
  }

  /// Sets the order to descending (default is ascending)
  pub fn descending(mut self) -> Self {
    self.descending = true;
    self
  }

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
