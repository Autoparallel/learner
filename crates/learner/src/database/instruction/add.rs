use super::*;

pub struct Add {
  paper: Paper,
}

impl Add {
  pub fn new(paper: Paper) -> Self { Self { paper } }
}

impl DatabaseInstruction for Add {
  type Output = i64;

  fn execute(&self, db: &mut Database) -> Result<Self::Output> {
    let tx = db.conn.transaction()?;

    let paper_id = {
      let mut stmt = tx.prepare_cached(
        "INSERT INTO papers (
                    title, abstract_text, publication_date,
                    source, source_identifier, pdf_url, doi
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                RETURNING id",
      )?;

      stmt.query_row(
        params![
          self.paper.title,
          self.paper.abstract_text,
          self.paper.publication_date.to_rfc3339(),
          self.paper.source.to_string(),
          self.paper.source_identifier,
          self.paper.pdf_url,
          self.paper.doi,
        ],
        |row| row.get(0),
      )?
    };

    {
      let mut stmt = tx.prepare_cached(
        "INSERT INTO authors (paper_id, name, affiliation, email)
                 VALUES (?1, ?2, ?3, ?4)",
      )?;

      for author in &self.paper.authors {
        stmt.execute(params![paper_id, author.name, author.affiliation, author.email,])?;
      }
    }

    tx.commit()?;
    Ok(paper_id)
  }
}
