use tokio_rusqlite::params;

use super::*;

pub struct Add {
  paper: Paper,
}

impl Add {
  pub fn paper(paper: Paper) -> Self { Self { paper } }
}

impl Statement<i64> for Add {
  fn build(self) -> Result<StatementFunction<i64>> {
    let title = self.paper.title;
    let abstract_text = self.paper.abstract_text;
    let publication_date = self.paper.publication_date.to_rfc3339();
    let source = self.paper.source.to_string();
    let source_identifier = self.paper.source_identifier;
    let pdf_url = self.paper.pdf_url;
    let doi = self.paper.doi;
    let authors = self.paper.authors;

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

        for author in authors {
          stmt.execute(params![paper_id, author.name, author.affiliation, author.email,])?;
        }
      }

      tx.commit()?;
      Ok(paper_id)
    }))
  }
}
