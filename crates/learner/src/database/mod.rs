use tokio_rusqlite::Connection;

use super::*;

mod instruction;
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

  /// Get the current storage path for document files
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
  /// This function performs several checks to ensure the path is valid for storing files:
  /// - Verifies the path exists or can be created
  /// - Checks if the filesystem is writable
  /// - Validates sufficient permissions
  /// - Ensures the path is absolute
  ///
  /// # Arguments
  ///
  /// * `path` - The path where document files should be stored
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing:
  /// - `Ok(())` if the path is valid and has been set
  /// - `Err(LearnerError)` if the path is invalid or cannot be used
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - The path cannot be created
  /// - The filesystem is read-only
  /// - Insufficient permissions exist
  /// - The path is not absolute
  pub async fn set_storage_path(&self, path: impl AsRef<Path>) -> Result<()> {
    let original_path_result = self.get_storage_path().await;
    let path = path.as_ref();

    // Ensure path is absolute
    if !path.is_absolute() {
      return Err(LearnerError::Path(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Storage path must be absolute",
      )));
    }

    // Create a test file to verify write permissions
    let test_file = path.join(".learner_write_test");

    // First try to create the directory structure
    match std::fs::create_dir_all(path) {
      Ok(_) => {
        // Test write permissions by creating and removing a file
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
    let path_str = path.to_string_lossy().to_string();

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
        original_path, path
      );
    }

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
