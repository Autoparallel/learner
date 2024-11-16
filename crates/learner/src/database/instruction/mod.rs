use super::*;

pub mod add;
pub mod query;
pub mod remove;

use std::borrow::Cow;

use async_trait::async_trait;
use rusqlite::{params, params_from_iter, ToSql};

/// Trait for database operations that can be executed against the paper database.
///
/// This trait provides a unified interface for all database operations, whether
/// they are queries, insertions, updates, or deletions. Each implementation
/// specifies its own output type and handles its own SQL generation.
///
/// # Type Parameters
///
/// * `Output` - The type of data returned by executing this instruction
///
/// # Examples
///
/// ```no_run
/// # use learner::database::{Database, instruction::{DatabaseInstruction, Query}};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut db = Database::open("papers.db").await?;
///
/// // Search for papers about neural networks
/// let query = Query::text("neural networks");
/// let papers = query.execute(&mut db).await?;
///
/// // Or search by author
/// let query = Query::by_author("Alice Researcher");
/// let papers = query.execute(&mut db).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait DatabaseInstruction {
  type Output;

  // Take &mut reference to avoid taking ownership and allow multiple operations
  async fn execute(&self, db: &mut Database) -> Result<Self::Output>;
}
