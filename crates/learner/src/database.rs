//! Local SQLite database management for storing and retrieving papers.
//!
//! This module provides functionality to persist paper metadata in a local SQLite database.
//! It supports:
//! - Paper metadata storage and retrieval
//! - Author information management
//! - Full-text search across papers
//! - Source-specific identifier lookups
//!
//! The database schema is automatically initialized when opening a database, and includes
//! tables for papers, authors, and full-text search indexes.
//!
//! # Examples
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Open or create a database
//! let db = learner::database::Database::open("papers.db").await?;
//!
//! // Fetch and save a paper
//! let paper = learner::paper::Paper::new("2301.07041").await?;
//! let id = db.save_paper(&paper).await?;
//!
//! // Search for papers
//! let results = db.search_papers("neural networks").await?;
//! for paper in results {
//!   println!("Found: {}", paper.title);
//! }
//! # Ok(())
//! # }
//! ```

use rusqlite::params;
use tokio_rusqlite::Connection;

use super::*;

/// Handle for interacting with the paper database.
///
/// This struct manages an async connection to a SQLite database and provides
/// methods for storing and retrieving paper metadata. It uses SQLite's full-text
/// search capabilities for efficient paper discovery.
///
/// The database is automatically initialized with the required schema when opened.
/// If the database file doesn't exist, it will be created.
pub struct Database {
  /// Async SQLite connection handle
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
  pub async fn open(path: impl AsRef<Path>) -> Result<Self, LearnerError> {
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

  /// Saves a paper and its authors to the database.
  ///
  /// This method will:
  /// 1. Insert the paper's metadata into the papers table
  /// 2. Insert all authors into the authors table
  /// 3. Update the full-text search index
  ///
  /// The operation is performed in a transaction to ensure data consistency.
  ///
  /// # Arguments
  ///
  /// * `paper` - The paper to save
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - The database ID of the saved paper
  /// - A [`LearnerError`] if the save operation fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{database::Database, paper::Paper};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = Database::open("papers.db").await?;
  /// let paper = Paper::new("2301.07041").await?;
  /// let id = db.save_paper(&paper).await?;
  /// println!("Saved paper with ID: {}", id);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn save_paper(&self, paper: &Paper) -> Result<i64, LearnerError> {
    let paper = paper.clone();
    self
      .conn
      .call(move |conn| {
        let tx = conn.transaction()?;

        // Insert paper
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
              &paper.title,
              &paper.abstract_text,
              &paper.publication_date,
              paper.source.to_string(),
              &paper.source_identifier,
              &paper.pdf_url,
              &paper.doi,
            ],
            |row| row.get::<_, i64>(0),
          )?
        };

        // Insert authors
        {
          let mut stmt = tx.prepare_cached(
            "INSERT INTO authors (paper_id, name, affiliation, email)
                         VALUES (?1, ?2, ?3, ?4)",
          )?;

          for author in &paper.authors {
            stmt.execute(params![paper_id, &author.name, &author.affiliation, &author.email,])?;
          }
        }

        tx.commit()?;
        Ok(paper_id)
      })
      .await
      .map_err(LearnerError::from)
  }

  /// Retrieves a paper using its source and identifier.
  ///
  /// This method looks up a paper based on its origin (e.g., arXiv, DOI)
  /// and its source-specific identifier. It also fetches all associated
  /// author information.
  ///
  /// # Arguments
  ///
  /// * `source` - The paper's source system (arXiv, IACR, DOI)
  /// * `source_id` - The source-specific identifier
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - `Some(Paper)` if found
  /// - `None` if no matching paper exists
  /// - A [`LearnerError`] if the query fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{database::Database, paper::Source};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = Database::open("papers.db").await?;
  /// if let Some(paper) = db.get_paper_by_source_id(&Source::Arxiv, "2301.07041").await? {
  ///   println!("Found paper: {}", paper.title);
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub async fn get_paper_by_source_id(
    &self,
    source: &Source,
    source_id: &str,
  ) -> Result<Option<Paper>, LearnerError> {
    // Clone the values before moving into the async closure
    let source = source.to_string();
    let source_id = source_id.to_string();

    self
      .conn
      .call(move |conn| {
        let mut paper_stmt = conn.prepare_cached(
          "SELECT id, title, abstract_text, publication_date, source,
                            source_identifier, pdf_url, doi
                     FROM papers 
                     WHERE source = ?1 AND source_identifier = ?2",
        )?;

        let mut author_stmt = conn.prepare_cached(
          "SELECT name, affiliation, email
                     FROM authors
                     WHERE paper_id = ?",
        )?;

        let paper_result = paper_stmt.query_row(params![source, source_id], |row| {
          Ok(Paper {
            title:             row.get(1)?,
            abstract_text:     row.get(2)?,
            publication_date:  row.get(3)?,
            source:            Source::from_str(&row.get::<_, String>(4)?).map_err(|e| {
              rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
            })?,
            source_identifier: row.get(5)?,
            pdf_url:           row.get(6)?,
            doi:               row.get(7)?,
            authors:           Vec::new(), // Filled in below
          })
        });

        match paper_result {
          Ok(mut paper) => {
            let paper_id: i64 =
              paper_stmt.query_row(params![source, source_id], |row| row.get(0))?;

            let authors = author_stmt.query_map([paper_id], |row| {
              Ok(Author {
                name:        row.get(0)?,
                affiliation: row.get(1)?,
                email:       row.get(2)?,
              })
            })?;

            paper.authors = authors.collect::<Result<Vec<_>, _>>()?;
            Ok(Some(paper))
          },
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(e.into()),
        }
      })
      .await
      .map_err(LearnerError::from)
  }

  /// Searches for papers using full-text search.
  ///
  /// This method uses SQLite's FTS5 module to perform full-text search across:
  /// - Paper titles
  /// - Paper abstracts
  ///
  /// Results are ordered by relevance using FTS5's built-in ranking algorithm.
  ///
  /// # Arguments
  ///
  /// * `query` - The search query using FTS5 syntax
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - A vector of matching papers
  /// - A [`LearnerError`] if the search fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = learner::database::Database::open("papers.db").await?;
  ///
  /// // Simple word search
  /// let papers = db.search_papers("quantum").await?;
  ///
  /// // Phrase search
  /// let papers = db.search_papers("\"neural networks\"").await?;
  ///
  /// // Complex query
  /// let papers = db.search_papers("machine learning NOT regression").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn search_papers(&self, query: &str) -> Result<Vec<Paper>, LearnerError> {
    let query = query.to_lowercase(); // Make search case-insensitive

    self
      .conn
      .call(move |conn| {
        // First get all paper IDs matching the search
        let mut id_stmt = conn.prepare_cached(
          "SELECT p.id
                 FROM papers p
                 JOIN papers_fts f ON p.id = f.rowid
                 WHERE papers_fts MATCH ?1 
                 ORDER BY rank",
        )?;

        // Collect matching IDs first
        let paper_ids: Vec<i64> =
          id_stmt.query_map([&query], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;

        let mut papers = Vec::new();

        // Now fetch complete paper data for each ID
        for paper_id in paper_ids {
          // Get paper details
          let mut paper_stmt = conn.prepare_cached(
            "SELECT title, abstract_text, publication_date,
                            source, source_identifier, pdf_url, doi
                     FROM papers 
                     WHERE id = ?",
          )?;

          let paper = paper_stmt.query_row([paper_id], |row| {
            Ok(Paper {
              title:             row.get(0)?,
              abstract_text:     row.get(1)?,
              publication_date:  row.get(2)?,
              source:            Source::from_str(&row.get::<_, String>(3)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                  3,
                  rusqlite::types::Type::Text,
                  Box::new(e),
                )
              })?,
              source_identifier: row.get(4)?,
              pdf_url:           row.get(5)?,
              doi:               row.get(6)?,
              authors:           Vec::new(),
            })
          })?;

          // Get authors for this paper
          let mut author_stmt = conn.prepare_cached(
            "SELECT name, affiliation, email
                     FROM authors
                     WHERE paper_id = ?",
          )?;

          let authors = author_stmt
            .query_map([paper_id], |row| {
              Ok(Author {
                name:        row.get(0)?,
                affiliation: row.get(1)?,
                email:       row.get(2)?,
              })
            })?
            .collect::<Result<Vec<_>, _>>()?;

          // Create the complete paper with authors
          let mut paper = paper;
          paper.authors = authors;
          papers.push(paper);
        }

        Ok(papers)
      })
      .await
      .map_err(LearnerError::from)
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

  /// Sets a configuration value in the database.
  ///
  /// # Arguments
  ///
  /// * `key` - The configuration key
  /// * `value` - The value to store
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] indicating success or failure
  pub async fn set_config(&self, key: &str, value: &str) -> Result<(), LearnerError> {
    let key = key.to_string();
    let value = value.to_string();
    self
      .conn
      .call(move |conn| {
        Ok(
          conn
            .execute("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)", params![
              key, value
            ])
            .map(|_| ()),
        )
      })
      .await?
      .map_err(LearnerError::from)
  }

  /// Gets a configuration value from the database.
  ///
  /// # Arguments
  ///
  /// * `key` - The configuration key to retrieve
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - Some(String) with the configuration value
  /// - None if the key doesn't exist
  pub async fn get_config(&self, key: &str) -> Result<Option<String>, LearnerError> {
    let key = key.to_string();
    self
      .conn
      .call(move |conn| {
        let mut stmt = conn.prepare_cached("SELECT value FROM config WHERE key = ?1")?;

        let result = stmt.query_row([key], |row| row.get::<_, String>(0));

        match result {
          Ok(value) => Ok(Some(value)),
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(e.into()),
        }
      })
      .await
      .map_err(LearnerError::from)
  }

  /// Records a PDF file location and status for a paper.
  ///
  /// # Arguments
  ///
  /// * `paper_id` - The database ID of the paper
  /// * `path` - Full path to the file
  /// * `filename` - The filename
  /// * `status` - Download status ('success', 'failed', 'pending')
  /// * `error` - Optional error message if download failed
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing the file ID on success
  pub async fn record_pdf(
    &self,
    paper_id: i64,
    path: PathBuf,
    filename: String,
    status: &str,
    error: Option<String>,
  ) -> Result<i64, LearnerError> {
    let path_str = path.to_string_lossy().to_string();
    let status = status.to_string();

    self
      .conn
      .call(move |conn| {
        let tx = conn.transaction()?;

        let id = tx.query_row(
          "INSERT OR REPLACE INTO files (
                      paper_id, path, filename, download_status, error_message
                  ) VALUES (?1, ?2, ?3, ?4, ?5)
                  RETURNING id",
          params![paper_id, path_str, filename, status, error],
          |row| row.get(0),
        )?;

        tx.commit()?;
        Ok(id)
      })
      .await
      .map_err(LearnerError::from)
  }

  /// Gets the PDF status for a paper.
  ///
  /// # Arguments
  ///
  /// * `paper_id` - The database ID of the paper
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - Some((PathBuf, String, String, Option<String>)) with the path, filename, status, and error
  /// - None if no PDF entry exists
  pub async fn get_pdf_status(
    &self,
    paper_id: i64,
  ) -> Result<Option<(PathBuf, String, String, Option<String>)>, LearnerError> {
    self
      .conn
      .call(move |conn| {
        let mut stmt = conn.prepare_cached(
          "SELECT path, filename, download_status, error_message FROM files 
                   WHERE paper_id = ?1",
        )?;

        let result = stmt.query_row([paper_id], |row| {
          Ok((
            PathBuf::from(row.get::<_, String>(0)?),
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
          ))
        });

        match result {
          Ok(info) => Ok(Some(info)),
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(e.into()),
        }
      })
      .await
      .map_err(LearnerError::from)
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  /// Helper function to set up a test database
  async fn setup_test_db() -> (Database, PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = Database::open(&path).await.unwrap();
    (db, path, dir)
  }

  #[traced_test]
  #[tokio::test]
  async fn test_database_creation() {
    let (_db, path, _dir) = setup_test_db().await;

    // Check that file exists
    assert!(path.exists());
  }

  #[traced_test]
  #[tokio::test]
  async fn test_get_nonexistent_paper() {
    let (db, _path, _dir) = setup_test_db().await;

    let result = db.get_paper_by_source_id(&Source::Arxiv, "nonexistent").await.unwrap();

    assert!(result.is_none());
  }

  #[traced_test]
  #[tokio::test]
  async fn test_default_pdf_path() {
    let path = Database::default_pdf_path();

    // Should end with learner/papers
    assert!(path.ends_with("learner/papers") || path.ends_with("learner\\papers"));

    // Should be rooted in a valid directory
    assert!(path
      .parent()
      .unwrap()
      .starts_with(dirs::document_dir().unwrap_or_else(|| PathBuf::from("."))));
  }

  #[traced_test]
  #[tokio::test]
  async fn test_config_operations() {
    let (db, _path, _dir) = setup_test_db().await;

    // Test setting and getting a config value
    db.set_config("test_key", "test_value").await.unwrap();
    let value = db.get_config("test_key").await.unwrap();
    assert_eq!(value, Some("test_value".to_string()));

    // Test getting non-existent config
    let missing = db.get_config("nonexistent").await.unwrap();
    assert_eq!(missing, None);

    // Test updating existing config
    db.set_config("test_key", "new_value").await.unwrap();
    let updated = db.get_config("test_key").await.unwrap();
    assert_eq!(updated, Some("new_value".to_string()));
  }

  #[traced_test]
  #[tokio::test]
  async fn test_config_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create database and set config
    {
      let db = Database::open(&db_path).await.unwrap();
      db.set_config("pdf_dir", "/test/path").await.unwrap();
    }

    // Reopen database and verify config persists
    {
      let db = Database::open(&db_path).await.unwrap();
      let value = db.get_config("pdf_dir").await.unwrap();
      assert_eq!(value, Some("/test/path".to_string()));
    }
  }
}
