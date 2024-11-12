//! Error types for the learner library.
//!
//! This module provides a comprehensive error type that encompasses all possible
//! failure modes when working with papers, including:
//! - Network and API errors
//! - Database operations
//! - Input validation
//! - Resource access
//!
//! # Examples
//!
//! ```
//! use learner::{error::LearnerError, paper::Paper};
//!
//! # async fn example() -> Result<(), LearnerError> {
//! // Network errors are automatically converted
//! let result = Paper::new("invalid-id").await;
//! match result {
//!   Err(LearnerError::InvalidIdentifier) => println!("Invalid paper ID format"),
//!   Err(LearnerError::Network(e)) => println!("Network error: {}", e),
//!   Err(e) => println!("Other error: {}", e),
//!   Ok(_) => println!("Success!"),
//! }
//! # Ok(())
//! # }
//! ```

use thiserror::Error;

/// Error type alias used for the [`learnerd`] crate.
pub type Result<T> = core::result::Result<T, LearnerError>;

/// Errors that can occur when working with the learner library.
///
/// This enum provides a comprehensive set of error cases that can occur when:
/// - Fetching papers from external sources
/// - Working with the local database
/// - Parsing identifiers and URLs
/// - Handling file system operations
///
/// Most error variants provide additional context through either custom messages
/// or wrapped underlying errors.
#[derive(Error, Debug)]
pub enum LearnerError {
  /// The provided paper identifier doesn't match the expected format.
  ///
  /// This can occur when:
  /// - arXiv ID format is invalid (e.g., wrong number of digits)
  /// - IACR ID doesn't match YYYY/NNN format
  /// - DOI format is malformed
  #[error("Invalid identifier format")]
  InvalidIdentifier,

  /// The provided source type string couldn't be parsed.
  ///
  /// This typically occurs when retrieving data from the database
  /// and the stored source type doesn't match any known variant.
  ///
  /// The string parameter contains the invalid source value for debugging.
  #[error("Invalid source type, see `learner::paper::Source`")]
  InvalidSource(String),

  /// A network request failed.
  ///
  /// This can occur when:
  /// - The network is unavailable
  /// - The server is unreachable
  /// - The request times out
  /// - TLS/SSL errors occur
  #[error(transparent)]
  Network(#[from] reqwest::Error),

  /// The requested paper couldn't be found.
  ///
  /// This occurs when the paper identifier is valid but:
  /// - The paper doesn't exist in the source repository
  /// - The paper has been removed or retracted
  /// - The paper is not publicly accessible
  #[error("Paper not found")]
  NotFound,

  /// An API returned an error response.
  ///
  /// This occurs when the external API (arXiv, IACR, DOI) returns
  /// an error response. The string parameter contains the error
  /// message from the API for debugging.
  #[error("API error: {0}")]
  ApiError(String),

  /// A SQLite operation failed.
  ///
  /// This wraps errors from the `rusqlite` crate, covering:
  /// - SQL syntax errors
  /// - Constraint violations
  /// - Schema errors
  /// - Type conversion errors
  #[error(transparent)]
  Sqlite(#[from] rusqlite::Error),

  /// An async SQLite operation failed.
  ///
  /// This wraps errors from the `tokio-rusqlite` crate, covering
  /// async-specific failures in database operations.
  #[error(transparent)]
  AsyncSqlite(#[from] tokio_rusqlite::Error),

  /// A file system operation failed.
  ///
  /// This occurs when:
  /// - Creating the database file fails
  /// - Reading/writing to the filesystem fails
  /// - Permission errors occur
  #[error(transparent)]
  Path(#[from] std::io::Error),

  /// A numeric conversion failed, typically in database operations.
  ///
  /// This occurs when converting between different numeric types,
  /// usually when dealing with database column indices or sizes.
  #[error(transparent)]
  ColumnOverflow(#[from] std::num::TryFromIntError),

  /// PDF parsing and processing errors from the lopdf library.
  ///
  /// This variant wraps errors from the lopdf library, which can occur during:
  /// - Initial PDF file parsing
  /// - Object access within the PDF structure
  /// - Stream decompression and content extraction
  /// - Dictionary access and type conversion
  ///
  /// Common error cases include:
  /// - Malformed or corrupted PDF files
  /// - Missing required PDF objects or references
  /// - Invalid stream encoding
  /// - Type mismatches when accessing PDF objects
  /// - Encrypted PDF files that require passwords
  #[error(transparent)]
  Lopdf(#[from] lopdf::Error),

  /// A model was not specified for the LLM request.
  ///
  /// This occurs when attempting to send a request to the LLM without
  /// first specifying which model to use. This can happen when:
  /// - The request is built without calling `with_model()`
  /// - The model field is explicitly set to None
  ///
  /// The error helps ensure that requests are properly configured
  /// before being sent to avoid API errors.
  #[error("No model was chosen for the LLM.")]
  LLMMissingModel,

  /// No messages were provided in the LLM request.
  ///
  /// This occurs when attempting to send a request to the LLM with
  /// an empty message queue. This can happen when:
  /// - The request is built without calling `with_message()`
  /// - All messages are removed before sending
  ///
  /// The error prevents sending empty requests to the LLM which
  /// would result in API errors or meaningless responses.
  #[error("No messages were supplied to send to the LLM.")]
  LLMMissingMessage,

  // TODO (autoparallel): This can be gotten rid of if we use an enum to handle the ways to sort
  // data in the database instead of a string.
  /// An error working with the database.
  #[error("{0}")]
  Database(String),
}

impl LearnerError {
  /// Checks if this error represents a duplicate entry in the database.
  ///
  /// This helper method checks for SQLite's unique constraint violation, which
  /// occurs when trying to insert a paper that already exists in the database
  /// (matching source and source_identifier).
  ///
  /// # Examples
  ///
  /// ```
  /// use learner::error::LearnerError;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let db = learner::database::Database::open("papers.db").await?;
  /// let paper = learner::paper::Paper::new("2301.07041").await?;
  ///
  /// match paper.save(&db).await {
  ///   Ok(id) => println!("Saved paper with ID: {}", id),
  ///   Err(e) if e.is_duplicate_error() => println!("Paper already exists!"),
  ///   Err(e) => return Err(e.into()),
  /// }
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// This is particularly useful for providing friendly error messages when
  /// attempting to add papers that are already in the database.
  pub fn is_duplicate_error(&self) -> bool {
    matches!(
        self,
        LearnerError::AsyncSqlite(tokio_rusqlite::Error::Rusqlite(
            rusqlite::Error::SqliteFailure(error, _)
        )) if error.code == rusqlite::ErrorCode::ConstraintViolation
    )
  }
}