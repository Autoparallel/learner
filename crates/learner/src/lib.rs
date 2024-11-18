//! Academic paper management and metadata retrieval library.
//!
//! `learner` is a library for managing academic papers, providing:
//!
//! - Paper metadata retrieval from multiple sources
//! - Local document management
//! - Database storage and querying
//! - Full-text search capabilities
//! - Flexible document organization
//!
//! # Features
//!
//! - **Multi-source support**: Fetch papers from:
//!   - arXiv (with support for both new and old-style identifiers)
//!   - IACR (International Association for Cryptologic Research)
//!   - DOI (Digital Object Identifier)
//! - **Flexible storage**: Choose where and how to store documents
//! - **Rich metadata**: Track authors, abstracts, and publication dates
//! - **Database operations**: Type-safe queries and modifications
//! - **Command pattern**: Composable database operations
//!
//! # Getting Started
//!
//! ```no_run
//! use learner::{
//!   database::{Add, Database, Query},
//!   paper::{Paper, Source},
//!   prelude::*,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   // Create or open a database
//!   let mut db = Database::open(Database::default_path()).await?;
//!
//!   // Fetch a paper from arXiv
//!   let paper = Paper::new("2301.07041").await?;
//!   println!("Title: {}", paper.title);
//!
//!   // Add to database with document
//!   Add::complete(&paper).execute(&mut db).await?;
//!
//!   // Search for related papers
//!   let papers = Query::text("quantum computing").execute(&mut db).await?;
//!
//!   Ok(())
//! }
//! ```
//!
//! # Module Organization
//!
//! - [`paper`]: Core paper types and metadata handling
//! - [`database`]: Database operations and storage management
//! - [`clients`]: Source-specific API clients
//! - [`llm`]: Language model integration for paper analysis
//! - [`pdf`]: PDF document handling and text extraction
//! - [`prelude`]: Common traits and types for ergonomic imports
//!
//! # Design Philosophy
//!
//! This library emphasizes:
//! - User control over document storage and organization
//! - Separation of metadata from document management
//! - Type-safe database operations
//! - Extensible command pattern for operations
//! - Clear error handling and propagation

#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![feature(str_from_utf16_endian)]

use std::{
  fmt::Display,
  path::{Path, PathBuf},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use paper::{Author, Paper};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};
#[cfg(test)]
use {tempfile::tempdir, tracing_test::traced_test};

pub mod client;
pub mod database;
pub mod retriever;

pub mod error;
pub mod format;
pub mod llm;
pub mod paper;
pub mod pdf;

use crate::{database::*, error::*, retriever::*};

/// Common traits and types for ergonomic imports.
///
/// This module provides a convenient way to import frequently used traits
/// and types with a single glob import. It includes:
///
/// - Database operation traits
/// - Error types and common `Result` type
/// - Commonly used trait implementations
///
/// # Usage
///
/// ```no_run
/// use learner::{
///   database::{Add, Database},
///   paper::Paper,
///   prelude::*,
/// };
///
/// async fn example() -> Result<(), LearnerError> {
///   // Now you can use both `DatabaseInstruction` and our `LearnerError`` type
///   let paper = Paper::new("2301.07041").await?;
///   let mut db = Database::open(Database::default_path()).await?;
///   Add::paper(&paper).execute(&mut db).await?;
///   Ok(())
/// }
/// ```
///
/// # Contents
///
/// Currently exports:
/// - [`DatabaseInstruction`]: Trait for implementing database operations
/// - [`LearnerError`]: Core error type for the library
///
/// Future additions may include:
/// - Additional trait implementations
/// - Common type aliases
/// - Builder pattern traits
/// - Conversion traits
pub mod prelude {
  pub use crate::{
    database::DatabaseInstruction, error::LearnerError, retriever::ResponseProcessor,
  };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  #[serde(default = "Database::default_path")]
  pub database_path: PathBuf,

  #[serde(default = "Config::default_retrievers_path")]
  pub retrievers_path: PathBuf,
}

impl Config {
  /// Get the config directory, creating it if it doesn't exist
  pub fn default_path() -> Result<PathBuf> {
    let config_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".learner");

    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
  }

  /// Default path for retriever configs
  pub fn default_retrievers_path() -> PathBuf {
    Self::default_path().unwrap_or_else(|_| PathBuf::from(".")).join("retrievers")
  }

  /// Load or create default configuration
  pub fn load() -> Result<Self> {
    let config_file = Self::default_path()?.join("config.toml");

    if config_file.exists() {
      let content = std::fs::read_to_string(&config_file)?;
      toml::from_str(&content).map_err(|e| LearnerError::Config(e.to_string()))
    } else {
      let config = Self::default();
      config.save()?;
      Ok(config)
    }
  }

  /// Save configuration to disk
  pub fn save(&self) -> Result<()> {
    let config_str =
      toml::to_string_pretty(self).map_err(|e| LearnerError::Config(e.to_string()))?;

    let config_file = Self::default_path()?.join("config.toml");
    debug!("Initializing config to: {config_file:?}");
    std::fs::write(config_file, config_str)?;

    // Ensure retriever config directory exists
    std::fs::create_dir_all(&self.retrievers_path)?;

    Ok(())
  }

  /// Initialize a new configuration with example retriever configs
  pub fn init() -> Result<Self> {
    let config = Self::default();
    config.save()?;

    // Write example retriever configs
    let retrievers_dir = &config.retrievers_path;
    std::fs::create_dir_all(retrievers_dir)?;

    // Include default retriever configs as const strings
    const ARXIV_CONFIG: &str = include_str!("../config/retrievers/arxiv.toml");
    const DOI_CONFIG: &str = include_str!("../config/retrievers/doi.toml");

    std::fs::write(retrievers_dir.join("arxiv.toml"), ARXIV_CONFIG)?;
    std::fs::write(retrievers_dir.join("doi.toml"), DOI_CONFIG)?;

    Ok(config)
  }

  pub fn with_database_path(self, database_path: &Path) -> Self {
    let Self { retrievers_path, .. } = self;
    Self { database_path: database_path.to_path_buf(), retrievers_path }
  }

  pub fn with_retrievers_path(self, retrievers_path: &Path) -> Self {
    let Self { database_path, .. } = self;
    Self { database_path, retrievers_path: retrievers_path.to_path_buf() }
  }
}

impl Default for Config {
  fn default() -> Self {
    Self {
      database_path:   Database::default_path(),
      retrievers_path: Self::default_retrievers_path(),
    }
  }
}

// Then we can update our Learner struct to use this config:
pub struct Learner {
  config:    Config,
  database:  Database,
  retriever: Retriever,
}
pub struct LearnerBuilder {
  config:      Option<Config>,
  config_path: Option<PathBuf>,
}

impl Default for LearnerBuilder {
  fn default() -> Self { Self { config: None, config_path: None } }
}

impl LearnerBuilder {
  pub fn new() -> Self { Self::default() }

  pub fn with_config(mut self, config: Config) -> Self {
    self.config = Some(config);
    self
  }

  pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
    self.config_path = Some(path.as_ref().to_path_buf());
    self
  }

  pub async fn build(self) -> Result<Learner> {
    let config = if let Some(config) = self.config {
      config
    } else if let Some(path) = self.config_path {
      let content = std::fs::read_to_string(path)?;
      toml::from_str(&content).map_err(|e| LearnerError::Config(e.to_string()))?
    } else {
      Config::load()?
    };

    let database = Database::open(&config.database_path).await?;
    let retriever = Retriever::new().with_config_dir(&config.retrievers_path)?;

    Ok(Learner { config, database, retriever })
  }
}

impl Learner {
  /// Returns a builder for creating a new Learner instance
  pub fn builder() -> LearnerBuilder { LearnerBuilder::new() }

  /// Creates a new Learner with default configuration
  pub async fn new() -> Result<Self> { Self::builder().build().await }

  /// Creates a new Learner from a specific config file path
  pub async fn from_path(path: impl AsRef<Path>) -> Result<Self> {
    Self::builder().with_path(path).build().await
  }

  /// Creates a new Learner with a specific config
  pub async fn with_config(config: Config) -> Result<Self> {
    Self::builder().with_config(config).build().await
  }

  /// Initialize a new Learner with example configuration
  pub async fn init() -> Result<Self> { Self::with_config(Config::init()?).await }
}

// Usage examples:

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_learner_creation() -> Result<()> {
    // Default configuration
    let learner = Learner::new().await?;

    // From specific path
    let learner = Learner::from_path("~/.learner/config.toml").await?;

    // Using builder pattern
    let learner = Learner::builder().with_path("custom/path/config.toml").build().await?;

    // Custom config
    let config = Config::default()
      .with_database_path(Path::new("/custom/db/path"))
      .with_retrievers_path(Path::new("/custom/retrievers"));
    let learner = Learner::builder().with_config(config).build().await?;

    Ok(())
  }
}
