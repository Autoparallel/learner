#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use rusqlite::{Connection, OptionalExtension};

use super::*;

pub mod instruction;
// pub mod models;
#[cfg(test)] mod tests;

pub use self::instruction::*;

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
  pub fn open(path: impl AsRef<Path>) -> Result<Self> {
    // Create parent directories if needed
    if let Some(parent) = path.as_ref().parent() {
      std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path.as_ref())?;

    // Initialize schema
    conn
      .execute_batch(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/init.sql")))?;

    let db = Self { conn };

    // Check if storage path is set, if not, set default
    if db.get_storage_path()?.is_none() {
      db.set_storage_path(Self::default_storage_path())?;
    }

    Ok(db)
  }

  /// Get the current storage path for document files
  pub fn get_storage_path(&self) -> Result<Option<PathBuf>> {
    self
      .conn
      .prepare_cached("SELECT value FROM config WHERE key = 'storage_path'")?
      .query_row([], |row| Ok(PathBuf::from(row.get::<_, String>(0)?)))
      .optional()
      .map_err(|e| e.into())
  }

  /// Set the storage path for document files
  pub fn set_storage_path(&self, path: impl AsRef<Path>) -> Result<()> {
    let path_str = path.as_ref().to_string_lossy();

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(path.as_ref())?;

    self
      .conn
      .execute("INSERT OR REPLACE INTO config (key, value) VALUES ('storage_path', ?1)", [
        path_str.as_ref(),
      ])?;

    Ok(())
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
  /// let path = learner::database::Database::default_storage_path();
  /// println!("PDFs will be stored at: {}", path.display());
  /// ```
  pub fn default_storage_path() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| PathBuf::from(".")).join("learner").join("papers")
  }
}
