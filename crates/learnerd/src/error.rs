//! Error types for the learnerd CLI application.
//!
//! This module provides a comprehensive error type that encompasses all possible
//! failure modes when running the CLI, including:
//! - User interaction errors
//! - Database and paper management errors
//! - File system operations
//! - Pattern matching errors
//!
//! The errors are designed to be transparent, allowing the underlying error
//! details to be displayed to the user while maintaining proper error
//! handling and propagation.

use thiserror::Error;

/// Error type alias used for the [`learnerd`] crate.
pub type Result<T> = core::result::Result<T, LearnerdError>;

/// Errors that can occur during CLI operations.
///
/// This enum wraps various error types from dependencies and the underlying
/// library into a single error type for the CLI application. It uses the
/// `transparent` error handling pattern to preserve the original error
/// messages and context.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
///
/// use learnerd::LearnerdErrors;
///
/// # fn example() -> Result<(), LearnerdErrors> {
/// // File operations may result in IO errors
/// std::fs::create_dir_all(PathBuf::from("some/path"))?;
///
/// // User interactions may result in Dialoguer errors
/// let input = dialoguer::Input::<String>::new().with_prompt("Enter something").interact()?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Error, Debug)]
pub enum LearnerdError {
  /// Errors from user interaction dialogs
  #[error(transparent)]
  Dialoguer(#[from] dialoguer::Error),

  /// Errors from the underlying learner library
  #[error(transparent)]
  Learner(#[from] learner::error::LearnerError),

  /// File system and IO operation errors
  #[error(transparent)]
  IO(#[from] std::io::Error),

  /// Glob pattern matching errors
  #[error(transparent)]
  Glob(#[from] glob::PatternError),

  /// Tracing appender initialization error.
  #[error(transparent)]
  TracingInit(#[from] tracing_appender::rolling::InitError),

  /// Daemon-specific errors.
  #[error("Daemon error: {0}")]
  Daemon(String),

  /// Error parsing a datetime format.
  #[error(transparent)]
  ChronoParse(#[from] chrono::format::ParseError),

  /// Error serializing toml
  #[error(transparent)]
  Toml(#[from] toml::ser::Error),
}
