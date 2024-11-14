use rusqlite::params_from_iter;

use super::*;

/// Represents different ways to query papers
#[derive(Debug)]
pub enum QueryCriteria {
  /// Full-text search across titles
  Text(String),
  /// Search by source and identifier
  SourceId { source: Source, identifier: String },
  /// Search by author name
  Author(String),
  /// List all papers
  All,
}

/// Valid fields for ordering results
#[derive(Debug, Clone, Copy)]
pub enum OrderField {
  Title,
  PublicationDate,
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

pub struct Query {
  criteria:   QueryCriteria,
  order_by:   Option<OrderField>,
  descending: bool,
}

impl Query {
  pub fn new(criteria: QueryCriteria) -> Self {
    Self { criteria, order_by: None, descending: false }
  }

  pub fn text(query: impl Into<String>) -> Self {
    Self::new(QueryCriteria::Text(query.into().to_lowercase()))
  }

  pub fn by_source(source: Source, identifier: impl Into<String>) -> Self {
    Self::new(QueryCriteria::SourceId { source, identifier: identifier.into() })
  }

  pub fn by_author(name: impl Into<String>) -> Self {
    Self::new(QueryCriteria::Author(name.into()))
  }

  pub fn list_all() -> Self { Self::new(QueryCriteria::All) }

  pub fn order_by(mut self, field: OrderField) -> Self {
    self.order_by = Some(field);
    self
  }

  pub fn descending(mut self) -> Self {
    self.descending = true;
    self
  }
}

impl DatabaseStatement for Query {
  type Output = Vec<Paper>;

  fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let mut papers = Vec::new();
    let tx = db.conn.transaction()?;

    // Get paper IDs based on search criteria
    let paper_ids = {
      // Get the appropriate SQL and parameters for each criteria
      let (sql, params) = match &self.criteria {
        QueryCriteria::Text(query) => (
          "SELECT p.id
           FROM papers p
           JOIN papers_fts f ON p.id = f.rowid
           WHERE papers_fts MATCH ?1 || '*' 
           ORDER BY rank",
          vec![query.to_string()],
        ),
        QueryCriteria::SourceId { source, identifier } => (
          "SELECT id FROM papers 
                     WHERE source = ?1 AND source_identifier = ?2",
          vec![source.to_string(), identifier.to_string()],
        ),
        QueryCriteria::Author(name) => (
          "SELECT DISTINCT p.id
                     FROM papers p
                     JOIN authors a ON p.id = a.paper_id
                     WHERE a.name LIKE ?1",
          vec![format!("%{}%", name)],
        ),
        QueryCriteria::All => ("SELECT id FROM papers", vec![]),
      };

      // Prepare and execute statement
      let mut stmt = tx.prepare_cached(sql)?;
      let mut rows = stmt.query(params_from_iter(params))?;
      let mut ids = Vec::new();
      while let Some(row) = rows.next()? {
        ids.push(row.get::<_, i64>(0)?);
      }
      ids
    };

    // Build the full paper query with ordering
    let paper_query = if let Some(order_field) = &self.order_by {
      let direction = if self.descending { "DESC" } else { "ASC" };
      format!(
        "SELECT title, abstract_text, publication_date,
                        source, source_identifier, pdf_url, doi
                 FROM papers 
                 WHERE id = ?
                 ORDER BY {} {}",
        order_field.as_sql_str(),
        direction
      )
    } else {
      "SELECT title, abstract_text, publication_date,
                    source, source_identifier, pdf_url, doi
             FROM papers 
             WHERE id = ?"
        .to_string()
    };

    // Fetch complete paper data for each ID
    for paper_id in paper_ids {
      let mut paper_stmt = tx.prepare_cached(&paper_query)?;

      let paper = paper_stmt.query_row([paper_id], |row| {
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

    // Sort the papers according to the ordering criteria
    if let Some(order_field) = &self.order_by {
      papers.sort_by(|a, b| {
        let cmp = match order_field {
          OrderField::Title => a.title.cmp(&b.title),
          OrderField::PublicationDate => a.publication_date.cmp(&b.publication_date),
          OrderField::Source => (a.source.to_string(), &a.source_identifier)
            .cmp(&(b.source.to_string(), &b.source_identifier)),
        };
        if self.descending {
          cmp.reverse()
        } else {
          cmp
        }
      });
    }

    Ok(papers)
  }
}
