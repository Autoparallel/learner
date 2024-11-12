use tokio_rusqlite::params;

use super::*;

impl QueryBuilder<Add> {
  pub fn paper(mut self, paper: Paper) -> Self {
    self.paper = Some(paper);
    self
  }

  pub fn build(self) -> Result<QueryFn<i64>> {
    let paper = self.paper.ok_or_else(|| LearnerError::Database("paper required".into()))?;

    // Clone values needed in closure
    let title = paper.title;
    let abstract_text = paper.abstract_text;
    let publication_date = paper.publication_date.to_rfc3339();
    let source = paper.source.to_string();
    let source_identifier = paper.source_identifier;
    let pdf_url = paper.pdf_url;
    let doi = paper.doi;
    let authors = paper.authors;

    Ok(Box::new(move |conn: &mut rusqlite::Connection| {
      // Now we're using the rusqlite::Connection directly
      let tx = conn.transaction()?;

      // Insert paper
      let paper_id = {
        let mut stmt = tx.prepare_cached(
          "INSERT INTO papers (
                        title, abstract_text, publication_date,
                        source, source_identifier, pdf_url, doi
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    RETURNING id",
        )?;

        stmt.query_row(
          params![title, abstract_text, publication_date, source, source_identifier, pdf_url, doi,],
          |row| row.get(0),
        )?
      };

      // Insert authors
      {
        let mut stmt = tx.prepare_cached(
          "INSERT INTO authors (paper_id, name, affiliation, email)
                     VALUES (?1, ?2, ?3, ?4)",
        )?;

        for author in &authors {
          stmt.execute(params![paper_id, &author.name, &author.affiliation, &author.email,])?;
        }
      }

      tx.commit()?;
      Ok(paper_id)
    }))
  }
}
