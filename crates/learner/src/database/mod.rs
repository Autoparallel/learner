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
//!   prelude::*,
//!   resource::Paper,
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

pub use self::instruction::{
  add::Add,
  // query::{OrderField, Query, QueryCriteria},
  // remove::Remove,
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
#[derive(Debug, Clone)]
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

    Ok(db)
  }
}
