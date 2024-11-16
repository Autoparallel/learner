// // database/instruction/remove.rs
// use super::{query::Query, *};

// /// Options for configuring the remove operation
// #[derive(Default)]
// pub struct RemoveOptions {
//   /// If true, only simulate the removal and return what would be removed
//   pub dry_run: bool,
// }

// pub struct Remove {
//   query:   Query,
//   options: RemoveOptions,
// }

// impl Remove {
//   /// Create a remove instruction from any query
//   pub fn from_query(query: Query) -> Self { Self { query, options: RemoveOptions::default() } }

//   /// Convenience method for removing by source and id
//   pub fn by_source(source: Source, identifier: impl Into<String>) -> Self {
//     Self::from_query(Query::by_source(source, identifier))
//   }

//   /// Convenience method for removing by author
//   pub fn by_author(name: impl Into<String>) -> Self { Self::from_query(Query::by_author(name)) }

//   /// Enable dry run mode - no papers will actually be removed
//   pub fn dry_run(mut self) -> Self {
//     self.options.dry_run = true;
//     self
//   }
// }

// #[async_trait::async_trait]
// impl DatabaseInstruction for Remove {
//   type Output = Vec<Paper>;

//   async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
//     // Use Query to find the papers to remove
//     let papers = self.query.execute(db).await?;

//     let tx = db.conn.transaction()?;

//     if !self.options.dry_run && !papers.is_empty() {
//       // Get all paper IDs
//       let ids: Vec<_> = papers
//         .iter()
//         .filter_map(|p| {
//           // We need to look up IDs since Query doesn't return them
//           let mut stmt = tx
//             .prepare_cached(
//               "SELECT id FROM papers
//                          WHERE source = ? AND source_identifier = ?",
//             )
//             .ok()?;

//           stmt
//             .query_row(params![p.source.to_string(), p.source_identifier], |row| {
//               row.get::<_, i64>(0)
//             })
//             .ok()
//         })
//         .collect();

//       if !ids.is_empty() {
//         let ids_str = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

//         // Remove the papers and their authors
//         tx.execute(&format!("DELETE FROM authors WHERE paper_id IN ({})", ids_str), [])?;

//         tx.execute(&format!("DELETE FROM papers WHERE id IN ({})", ids_str), [])?;
//       }

//       tx.commit()?;
//     }

//     Ok(papers)
//   }
// }
