//! Database management and operations for academic paper metadata.
//!
//! This module provides a flexible SQLite-based storage system for managing academic paper
//! metadata and references while allowing users to maintain control over how and where their
//! documents are stored. The database tracks:
//!
//! - Paper metadata (title, authors, abstract, publication date)
//! - Source information (arXiv, DOI, IACR)
//! - Document storage locations
//! - Full-text search capabilities
//!
//! The design emphasizes:
//! - User control over data storage locations
//! - Flexible integration with external PDF viewers and tools
//! - Efficient querying and organization of paper metadata
//! - Separation of metadata from document storage
//!
//! # Architecture
//!
//! The database module uses a command pattern through the [`DatabaseInstruction`] trait,
//! allowing for type-safe and composable database operations. Common operations are
//! implemented as distinct instruction types:
//!
//! - [`Query`] - For searching and retrieving papers
//! - [`Add`] - For adding new papers and documents
//! - [`Remove`] - For removing papers from the database
//!
//! # Examples
//!
//! ```no_run
//! use learner::{
//!   database::{Add, Database, Query},
//!   paper::Paper,
//!   prelude::*,
//!   Learner,
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a default learner to open database at default location
//! let mut learner = Learner::builder().build().await?;
//!
//! // Add a paper
//! let paper = learner.retriever.get_paper("2301.07041").await?;
//! Add::paper(&paper).execute(&mut learner.database).await?;
//!
//! // Search for papers about neural networks
//! let papers = Query::text("neural networks").execute(&mut learner.database).await?;
//!
//! // Customize document storage location
//! learner.database.set_storage_path("~/Documents/research/papers").await?;
//! # Ok(())
//! # }
//! ```

use tokio_rusqlite::Connection;

use super::*;

mod instruction;
// pub mod models;
#[cfg(test)] mod tests;

pub use self::instruction::{
  add::Add,
  query::{OrderField, Query, QueryCriteria},
  remove::Remove,
  DatabaseInstruction,
};

/// Main database connection handler for the paper management system.
///
/// The `Database` struct provides the primary interface for interacting with the SQLite
/// database that stores paper metadata and document references. It handles:
///
/// - Database initialization and schema management
/// - Storage path configuration for documents
/// - Connection management for async database operations
///
/// The database is designed to separate metadata storage (managed by this system)
/// from document storage (which can be managed by external tools), allowing users
/// to maintain their preferred document organization while benefiting from the
/// metadata management features.
#[derive(Debug)]
pub struct Database {
  /// Active connection to the SQLite database
  pub conn: Connection,
}

impl Database {
  /// Opens an existing database or creates a new one at the specified path.
  ///
  /// This method performs complete database initialization:
  /// 1. Creates parent directories if they don't exist
  /// 2. Initializes the SQLite database file
  /// 3. Applies schema migrations
  /// 4. Sets up full-text search indexes for paper metadata
  /// 5. Configures default storage paths if not already set
  ///
  /// # Arguments
  ///
  /// * `path` - Path where the database file should be created or opened. This can be:
  ///   - An absolute path to a specific location
  ///   - A relative path from the current directory
  ///   - The result of [`Database::default_path()`] for platform-specific default location
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - A [`Database`] handle ready for operations
  /// - A [`LearnerError`] if initialization fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Database;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Use platform-specific default location
  /// let db = Database::open(Database::default_path()).await?;
  ///
  /// // Or specify a custom location
  /// let db = Database::open("/path/to/papers.db").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
    // Create parent directories if needed
    if let Some(parent) = path.as_ref().parent() {
      std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path.as_ref()).await?;

    // Initialize schema
    conn
      .call(|conn| {
        Ok(conn.execute_batch(include_str!(concat!(
          env!("CARGO_MANIFEST_DIR"),
          "/migrations/init.sql"
        )))?)
      })
      .await?;

    let db = Self { conn };

    // Check if storage path is set, if not, set default
    if db.get_storage_path().await.is_err() {
      db.set_storage_path(Self::default_storage_path()).await?;
    }

    Ok(db)
  }

  /// Gets the configured storage path for document files.
  ///
  /// The storage path determines where document files (like PDFs) will be saved
  /// when downloaded through the system. This path is stored in the database
  /// configuration and can be modified using [`Database::set_storage_path()`].
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing either:
  /// - The configured [`PathBuf`] for document storage
  /// - A [`LearnerError`] if the path cannot be retrieved
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Database;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = Database::open(Database::default_path()).await?;
  /// let storage_path = db.get_storage_path().await?;
  /// println!("Documents are stored in: {}", storage_path.display());
  /// # Ok(())
  /// # }
  /// ```
  pub async fn get_storage_path(&self) -> Result<PathBuf> {
    Ok(
      self
        .conn
        .call(|conn| {
          Ok(
            conn
              .prepare_cached("SELECT value FROM config WHERE key = 'storage_path'")?
              .query_row([], |row| Ok(PathBuf::from(row.get::<_, String>(0)?)))?,
          )
        })
        .await?,
    )
  }

  /// Sets the storage path for document files, validating that the path is usable.
  ///
  /// This method configures where document files (like PDFs) will be stored when
  /// downloaded through the system. It performs extensive validation to ensure the
  /// path is usable and accessible:
  ///
  /// - Verifies the path exists or can be created
  /// - Confirms the filesystem is writable
  /// - Validates sufficient permissions exist
  /// - Ensures the path is absolute for reliability
  ///
  /// When changing the storage path, existing documents are not automatically moved.
  /// Users should manually migrate their documents if needed.
  ///
  /// # Arguments
  ///
  /// * `path` - The path where document files should be stored. Must be an absolute path.
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing:
  /// - `Ok(())` if the path is valid and has been configured
  /// - `Err(LearnerError)` if the path is invalid or cannot be used
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - The path is not absolute
  /// - The path cannot be created
  /// - The filesystem is read-only
  /// - Insufficient permissions exist
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::database::Database;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = Database::open(Database::default_path()).await?;
  ///
  /// // Set custom storage location
  /// db.set_storage_path("/data/papers").await?;
  ///
  /// // Or use home directory
  /// db.set_storage_path("~/Documents/papers").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn set_storage_path(&self, path: impl AsRef<Path>) -> Result<()> {
    let original_path_result = self.get_storage_path().await;
    let path = path.as_ref();

    // Convert relative paths to absolute using current working directory
    let absolute_path =
      if !path.is_absolute() { std::env::current_dir()?.join(path) } else { path.to_path_buf() };

    // Create a test file to verify write permissions
    let test_file = absolute_path.join(".learner_write_test");

    // First try to create the directory structure
    match std::fs::create_dir_all(&absolute_path) {
      Ok(_) => {
        // Rest of the code remains the same, but use absolute_path instead of path
        match std::fs::write(&test_file, b"test") {
          Ok(_) => {
            // Clean up test file
            let _ = std::fs::remove_file(&test_file);
          },
          Err(e) => {
            return Err(match e.kind() {
              std::io::ErrorKind::PermissionDenied => LearnerError::Path(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Insufficient permissions to write to storage directory",
              )),
              std::io::ErrorKind::ReadOnlyFilesystem => LearnerError::Path(std::io::Error::new(
                std::io::ErrorKind::ReadOnlyFilesystem,
                "Storage location is on a read-only filesystem",
              )),
              _ => LearnerError::Path(e),
            });
          },
        }
      },
      Err(e) => {
        return Err(LearnerError::Path(std::io::Error::new(
          e.kind(),
          format!("Failed to create storage directory: {}", e),
        )));
      },
    }

    // If we get here, the path is valid and writable
    let path_str = absolute_path.to_string_lossy().to_string();

    self
      .conn
      .call(move |conn| {
        Ok(
          conn
            .execute("INSERT OR REPLACE INTO config (key, value) VALUES ('storage_path', ?1)", [
              path_str,
            ])?,
        )
      })
      .await?;

    if let Ok(original_path) = original_path_result {
      warn!(
        "Original storage path was {:?}, set a new path to {:?}. Please be careful to check that \
         your documents have been moved or that you intended to do this operation!",
        original_path, absolute_path
      );
    }

    Ok(())
  }

  /// Returns the platform-specific default path for the database file.
  ///
  /// This method provides a sensible default location for the database file
  /// following platform conventions:
  ///
  /// - Unix: `~/.local/share/learner/learner.db`
  /// - macOS: `~/Library/Application Support/learner/learner.db`
  /// - Windows: `%APPDATA%\learner\learner.db`
  /// - Fallback: `./learner.db` in the current directory
  ///
  /// # Returns
  ///
  /// Returns a [`PathBuf`] pointing to the default database location for the
  /// current platform.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// use learner::database::Database;
  ///
  /// let path = Database::default_path();
  /// println!("Default database location: {}", path.display());
  /// ```
  pub fn default_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("learner").join("learner.db")
  }

  /// Returns the platform-specific default path for document storage.
  ///
  /// This method provides a sensible default location for storing document files
  /// following platform conventions. The returned path is always absolute and follows
  /// these patterns:
  ///
  /// - Unix: `~/Documents/learner/papers`
  /// - macOS: `~/Documents/learner/papers`
  /// - Windows: `Documents\learner\papers`
  /// - Fallback: `<current_directory>/papers`
  ///
  /// The method ensures the path is absolute by:
  /// - Using platform-specific document directories when available
  /// - Falling back to the current working directory when needed
  /// - Resolving all relative components
  ///
  /// Users can override this default using [`Database::set_storage_path()`].
  ///
  /// # Returns
  ///
  /// Returns an absolute [`PathBuf`] pointing to the default document storage
  /// location for the current platform.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// use learner::database::Database;
  ///
  /// let path = Database::default_storage_path();
  /// assert!(path.is_absolute());
  /// println!("Default document storage: {}", path.display());
  ///
  /// // On Unix-like systems, might print something like:
  /// // "/home/user/Documents/learner/papers"
  ///
  /// // On Windows, might print something like:
  /// // "C:\Users\user\Documents\learner\papers"
  /// ```
  ///
  /// Note that while the base directory may vary by platform, the returned path
  /// is guaranteed to be absolute and usable for document storage.
  pub fn default_storage_path() -> PathBuf {
    let base_path = dirs::document_dir().unwrap_or_else(|| PathBuf::from("."));
    // Make sure we return an absolute path
    if !base_path.is_absolute() {
      std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(base_path)
    } else {
      base_path
    }
    .join("learner")
    .join("papers")
  }
}
