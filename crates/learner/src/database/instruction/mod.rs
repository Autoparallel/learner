//! Database instruction implementations for structured database operations.
//!
//! This module provides a trait-based abstraction for database operations using the
//! Command pattern. This design allows for:
//!
//! - Type-safe database operations
//! - Composable and reusable commands
//! - Clear separation of operation logic
//! - Consistent error handling
//!
//! # Architecture
//!
//! The module is organized around three main operation types:
//!
//! - [`query`] - Read operations for searching and retrieving papers
//! - [`add`] - Write operations for adding papers and documents
//! - [`remove`] - Delete operations for removing papers from the database
//!
//! Each operation type implements the [`DatabaseInstruction`] trait, providing
//! a consistent interface while allowing for operation-specific behavior.
//!
//! # Usage
//!
//! Operations are constructed as instructions and then executed against a database:
//!
//! ```no_run
//! use learner::{
//!   database::{
//!     instruction::{Add, Query, Remove},
//!     Database,
//!   },
//!   paper::Paper,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut db = Database::open("papers.db").await?;
//!
//! // Query papers
//! let papers = Query::text("quantum computing").execute(&mut db).await?;
//!
//! // Add a new paper
//! let paper = Paper::new("2301.07041").await?;
//! Add::paper(&paper).execute(&mut db).await?;
//!
//! // Remove papers by author
//! Remove::by_author("Alice Researcher").execute(&mut db).await?;
//! # Ok(())
//! # }
//! ```

use super::*;

pub mod add;
pub mod query;
pub mod remove;

use async_trait::async_trait;
use rusqlite::{params_from_iter, ToSql};

use self::query::Query;

/// Trait for implementing type-safe database operations.
///
/// This trait defines the core interface for the Command pattern used in database
/// operations. Each implementation represents a specific operation (like querying,
/// adding, or removing papers) and encapsulates its own:
///
/// - SQL generation and execution
/// - Parameter handling
/// - Result type specification
/// - Error handling
///
/// The trait is async to support non-blocking database operations while maintaining
/// proper connection management.
///
/// # Type Parameters
///
/// * `Output` - The type returned by executing this instruction. Common types include:
///   - `Vec<Paper>` for query operations
///   - `()` for operations that don't return data
///   - Custom types for specialized operations
///
/// # Implementation Notes
///
/// When implementing this trait:
/// - Keep SQL generation and execution within the implementation
/// - Use proper parameter binding for SQL injection prevention
/// - Handle errors appropriately and convert to [`LearnerError`]
/// - Consider optimizing repeated operations with prepared statements
///
/// # Examples
///
/// Querying papers with different criteria:
///
/// ```no_run
/// # use learner::database::{Database, instruction::{DatabaseInstruction, Query}};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut db = Database::open("papers.db").await?;
///
/// // Full-text search
/// let papers = Query::text("neural networks").execute(&mut db).await?;
///
/// // Search by author
/// let papers = Query::by_author("Alice Researcher").execute(&mut db).await?;
///
/// // Search by publication date
/// use chrono::{DateTime, Utc};
/// let papers =
///   Query::before_date(DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")?.with_timezone(&Utc))
///     .execute(&mut db)
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// Implementing a custom instruction:
///
/// ```no_run
/// # use learner::database::{Database, instruction::DatabaseInstruction};
/// # use async_trait::async_trait;
/// struct CountPapers;
///
/// #[async_trait]
/// impl DatabaseInstruction for CountPapers {
///   type Output = i64;
///
///   async fn execute(&self, db: &mut Database) -> Result<Self::Output> {
///     db.conn
///       .call(|conn| {
///         conn.query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0)).map_err(Into::into)
///       })
///       .await
///   }
/// }
/// # type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// ```
#[async_trait]
pub trait DatabaseInstruction {
  /// The type returned by executing this instruction.
  type Output;

  // TODO (autoparallel): It may honestly be worth having two traits -- one that takes &mut db and
  // another that takes &db so you don't need to have shared mutability access
  /// Executes the instruction against a database connection.
  ///
  /// This method performs the actual database operation, managing:
  /// - SQL execution
  /// - Parameter binding
  /// - Result processing
  /// - Error handling
  ///
  /// # Arguments
  ///
  /// * `db` - Mutable reference to the database connection
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing either:
  /// - The operation's output of type `Self::Output`
  /// - A [`LearnerError`] if the operation fails
  ///
  /// # Notes
  ///
  /// The mutable database reference is required for operations that modify
  /// the database. A future enhancement might split this into separate traits
  /// for read-only and read-write operations.
  async fn execute(&self, db: &mut Database) -> Result<Self::Output>;
}
