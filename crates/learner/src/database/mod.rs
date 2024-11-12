#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use tokio_rusqlite::Connection;

use super::*;

pub mod models;
pub mod query;
#[cfg(test)] mod tests;

pub use self::{models::*, query::*};

/// Main database connection handler
pub struct Database {
  conn: Connection,
}

impl Database {
  /// Opens an existing database or creates a new one at the specified path.
  ///
  /// This method will:
  /// 1. Create the database file if it doesn't exist
  /// 2. Initialize the schema using migrations
  /// 3. Set up full-text search indexes
  ///
  /// # Arguments
  ///
  /// * `path` - Path where the database file should be created or opened
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - A [`Database`] handle for database operations
  /// - A [`LearnerError`] if database creation or initialization fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Database;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Open in a specific location
  /// let db = Database::open("papers.db").await?;
  ///
  /// // Or use the default location
  /// let db = Database::open(Database::default_path()).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
    let conn = Connection::open(path.as_ref()).await?;

    // Initialize schema
    conn
      .call(|conn| {
        conn.execute_batch(include_str!(concat!(
          env!("CARGO_MANIFEST_DIR"),
          "/migrations/init.sql"
        )))?;
        Ok(())
      })
      .await?;

    Ok(Self { conn })
  }

  /// Returns the default path for the database file.
  ///
  /// The path is constructed as follows:
  /// - On Unix: `~/.local/share/learner/learner.db`
  /// - On macOS: `~/Library/Application Support/learner/learner.db`
  /// - On Windows: `%APPDATA%\learner\learner.db`
  /// - Fallback: `./learner.db` in the current directory
  ///
  /// # Examples
  ///
  /// ```no_run
  /// let path = learner::database::Database::default_path();
  /// println!("Database will be stored at: {}", path.display());
  /// ```
  pub fn default_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("learner").join("learner.db")
  }

  /// Returns the default path for PDF storage.
  ///
  /// The path is constructed as follows:
  /// - On Unix: `~/Documents/learner/papers`
  /// - On macOS: `~/Documents/learner/papers`
  /// - On Windows: `Documents\learner\papers`
  /// - Fallback: `./papers` in the current directory
  ///
  /// # Examples
  ///
  /// ```no_run
  /// let path = learner::database::Database::default_pdf_path();
  /// println!("PDFs will be stored at: {}", path.display());
  /// ```
  pub fn default_pdf_path() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| PathBuf::from(".")).join("learner").join("papers")
  }

  /// Execute a query function
  pub async fn execute<T>(&self, statement: StatementFunction<T>) -> Result<T>
  where T: Send + 'static {
    self.conn.call(|conn| Ok(statement(conn))).await?
  }

  //   /// Start building a save query
  //   pub fn save(&self) -> QueryBuilder<'_, Add> { QueryBuilder::new(self) }

  //   /// Start building a remove query
  //   pub fn remove(&self) -> QueryBuilder<'_, Remove> { QueryBuilder::new(self) }

  //   /// Start building a query to get a paper
  //   pub fn get(&self) -> QueryBuilder<'_, Get> { QueryBuilder::new(self) }

  //   /// Start building a search query
  //   pub fn search(&self) -> QueryBuilder<'_, Search> { QueryBuilder::new(self) }

  //   /// Start building a list query
  //   pub fn list(&self) -> QueryBuilder<'_, List> { QueryBuilder::new(self) }
}
