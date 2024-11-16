use super::*;

/// Options for configuring the remove operation
#[derive(Default)]
pub struct RemoveOptions {
  /// If true, only simulate the removal and return what would be removed
  pub dry_run: bool,
}

/// Remove instruction for papers in the database
pub struct Remove<'a> {
  query:   Query<'a>,
  options: RemoveOptions,
}

impl<'a> Remove<'a> {
  /// Create a remove instruction from any query
  pub fn from_query(query: Query<'a>) -> Self { Self { query, options: RemoveOptions::default() } }

  /// Convenience method for removing by source and id
  pub fn by_source(source: Source, identifier: &'a str) -> Self {
    Self::from_query(Query::by_source(source, identifier))
  }

  /// Convenience method for removing by author
  pub fn by_author(name: &'a str) -> Self { Self::from_query(Query::by_author(name)) }

  /// Enable dry run mode - no papers will actually be removed
  pub fn dry_run(mut self) -> Self {
    self.options.dry_run = true;
    self
  }

  /// Build SQL to get paper IDs
  fn build_paper_ids_sql(paper: &Paper) -> (String, Vec<Option<String>>) {
    ("SELECT id FROM papers WHERE source = ? AND source_identifier = ?".to_string(), vec![
      Some(paper.source.to_string()),
      Some(paper.source_identifier.clone()),
    ])
  }

  /// Build SQL to remove papers and related data
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
