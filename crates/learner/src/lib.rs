//! Academic paper management and metadata retrieval library.
//!
//! `learner` is a flexible library for managing academic papers that emphasizes user choice
//! and interoperability with existing tools. Unlike monolithic paper management solutions,
//! it focuses on providing robust metadata handling and storage while allowing users to
//! choose their own tools for viewing, annotating, and organizing papers.
//!
//! # Core Features
//!
//! - **Multi-source Retrieval**:
//!   - arXiv (supporting both new-style "2301.07041" and old-style "math.AG/0601001" identifiers)
//!   - IACR (International Association for Cryptologic Research)
//!   - DOI (Digital Object Identifier)
//!   - Extensible retriever system for adding new sources
//!
//! - **Flexible Storage and Organization**:
//!   - Configurable document storage locations
//!   - User-controlled directory structure
//!   - Separation of metadata and document storage
//!   - Integration with existing file organization
//!
//! - **Rich Metadata Management**:
//!   - Comprehensive paper metadata
//!   - Author information with affiliations
//!   - Publication dates and version tracking
//!   - Abstract text and citations
//!   - Custom metadata fields
//!
//! - **Database Operations**:
//!   - Type-safe query building
//!   - Full-text search capabilities
//!   - Composable operations using command pattern
//!   - Robust error handling
//!
//! # Getting Started
//!
//! ```no_run
//! use learner::{
//!   database::{Add, OrderField, Query},
//!   prelude::*,
//!   resource::Paper,
//!   Learner,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   // Initialize with default configuration
//!   let mut learner = Learner::builder().build().await?;
//!
//!   // Fetch papers from any supported source
//!   let arxiv_paper = learner.retriever.get_paper("2301.07041").await?;
//!   let doi_paper = learner.retriever.get_paper("10.1145/1327452.1327492").await?;
//!
//!   println!("Retrieved: {}", arxiv_paper.title);
//!
//!   // Store paper with its PDF
//!   Add::complete(&arxiv_paper).execute(&mut learner.database).await?;
//!
//!   // Search the database
//!   let papers = Query::text("quantum computing")
//!     .order_by(OrderField::PublicationDate)
//!     .descending()
//!     .execute(&mut learner.database)
//!     .await?;
//!
//!   // Find papers by author
//!   let author_papers = Query::by_author("Alice Researcher").execute(&mut learner.database).await?;
//!
//!   Ok(())
//! }
//! ```
//!
//! # Module Organization
//!
//! The library is organized into focused, composable modules:
//!
//! - [`paper`]: Core paper types and metadata management
//!   - Paper struct with comprehensive metadata
//!   - Multi-source identifier handling
//!   - Author information management
//!
//! - [`database`]: Storage and querying functionality
//!   - Type-safe query building
//!   - Full-text search implementation
//!   - Document storage management
//!   - Command pattern operations
//!
//! - [`clients`]: API clients for paper sources
//!   - Source-specific implementations
//!   - Response parsing and validation
//!   - Error handling and retry logic
//!
//! - [`retriever`]: Configurable paper retrieval system
//!   - Automatic source detection
//!   - XML and JSON response handling
//!   - Custom field mapping
//!
//! - [`prelude`]: Common imports for ergonomic use
//!   - Essential traits
//!   - Common type definitions
//!   - Error types
//!
//! # Design Philosophy
//!
//! `learner` is built on several key principles:
//!
//! - **User Control**: Users should have full control over document storage and organization,
//!   allowing integration with their existing workflows and tools.
//!
//! - **Separation of Concerns**: Clear separation between metadata management and document storage,
//!   enabling flexible integration with external tools.
//!
//! - **Type Safety**: Database operations and API interactions are designed to be type-safe and
//!   verified at compile time when possible.
//!
//! - **Extensibility**: The command pattern for database operations and configurable retrievers
//!   make the system easy to extend.
//!
//! - **Error Handling**: Clear error types and propagation make it easy to handle and debug issues
//!   at every level.
//!
//! # Configuration
//!
//! The library can be configured through TOML files or programmatically:
//!
//! ```toml
//! # ~/.learner/config.toml
//! database_path = "~/.local/share/learner/papers.db"
//! storage_path = "~/Documents/papers"
//! retrievers_path = "~/.learner/retrievers"
//! ```
//!
//! ```no_run
//! # use learner::{Config, Learner};
//! # use std::path::PathBuf;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Programmatic configuration
//! let config = Config::default()
//!   .with_storage_path(&PathBuf::from("~/papers"))
//!   .with_database_path(&PathBuf::from("~/.papers.db"));
//!
//! let learner = Learner::with_config(config).await?;
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![feature(str_from_utf16_endian)]

use std::{
  collections::BTreeMap,
  fmt::Display,
  path::{Path, PathBuf},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Url;
use resource::{ResourceConfig, Resources};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};
#[cfg(test)]
use {tempfile::tempdir, tracing_test::traced_test};

pub mod database;
pub mod retriever;

pub mod configuration;
pub mod error;
pub mod format;
pub mod llm;
pub mod pdf;
pub mod resource;

use crate::{
  database::*,
  error::*,
  prelude::*,
  resource::{Author, Paper},
  retriever::*,
};

/// ArXiv default configuration
pub const ARXIV_CONFIG: &str = include_str!("../config/retrievers/arxiv.toml");
/// DOI default configuration
pub const DOI_CONFIG: &str = include_str!("../config/retrievers/doi.toml");
/// IACR default configuration
pub const IACR_CONFIG: &str = include_str!("../config/retrievers/iacr.toml");

/// Paper default configuration
pub const PAPER_CONFIG: &str = include_str!("../config/resources/paper.toml");
/// Book default configuration
pub const BOOK_CONFIG: &str = include_str!("../config/resources/book.toml");
/// Thesis default configuration
pub const THESIS_CONFIG: &str = include_str!("../config/resources/thesis.toml");

/// Common traits and types for ergonomic imports.
///
/// This module provides a convenient way to import frequently used traits
/// and types with a single glob import. It includes:
///
/// - Database operation traits
/// - Error types and common `Result` type
/// - Response processing traits
///
/// # Usage
///
/// ```no_run
/// use learner::{
///   database::{Add, Database},
///   prelude::*,
///   resource::Paper,
///   Learner,
/// };
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///   let mut learner = Learner::builder().build().await?;
///
///   // Now you can use core traits and types
///   let paper = learner.retriever.get_paper("2301.07041").await?;
///   Add::paper(&paper).execute(&mut learner.database).await?;
///   Ok(())
/// }
/// ```
pub mod prelude {
  pub use crate::{
    configuration::{Configurable, Identifiable},
    database::DatabaseInstruction,
    error::LearnerError,
    retriever::ResponseProcessor,
  };
}

/// Core configuration for the library.
///
/// Manages paths for database, document storage, and retriever configurations.
/// The configuration can be loaded from disk or created programmatically.
///
/// # Examples
///
/// ```no_run
/// # use learner::Config;
/// # use std::path::PathBuf;
/// // Load existing config
/// let config = Config::load()?;
///
/// // Or create custom config
/// let config = Config::default()
///   .with_database_path(&PathBuf::from("papers.db"))
///   .with_storage_path(&PathBuf::from("papers"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  /// The path to store the database.
  #[serde(default = "Database::default_path")]
  pub database_path: PathBuf,

  /// The path to store associated documents and files.
  #[serde(default = "Database::default_storage_path")]
  pub storage_path: PathBuf,

  /// The path to load retriever configs from.
  #[serde(default = "Config::default_retrievers_path")]
  pub retrievers_path: PathBuf,

  /// The path to load retriever configs from.
  #[serde(default = "Config::default_resources_path")]
  pub resources_path: PathBuf,
}

// TODO: We should really let the database storage path be set prior to opening. We need a slightly
// better database builder pattern.

/// Main entry point for the library.
///
/// Coordinates database access, paper retrieval, and configuration management.
/// Use the builder pattern to create configured instances.
///
/// # Examples
///
/// ```no_run
/// # use learner::Learner;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create with defaults
/// let learner = Learner::new().await?;
///
/// // Or use the builder for more control
/// let learner = Learner::builder().with_path("config/").build().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Learner {
  /// Active configuration
  pub config:    Config,
  /// Database connection and operations
  pub database:  Database,
  /// Paper retrieval system
  pub retriever: Retriever,
  /// Resources to use
  pub resources: Resources,
}

/// Builder for creating configured Learner instances.
///
/// Provides a flexible way to construct Learner instances with
/// custom configurations and paths.
///
/// # Examples
///
/// ```no_run
/// # use learner::{Config, Learner};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Build with explicit config
/// let config = Config::default();
/// let learner = Learner::builder().with_config(config).build().await?;
///
/// // Or from a config path
/// let learner = Learner::builder().with_path("~/.learner").build().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct LearnerBuilder {
  /// Explicit configuration if provided
  config:      Option<Config>,
  /// Path to load configuration from
  config_path: Option<PathBuf>,
}

impl Config {
  /// Returns the default configuration directory path, creating it if needed.
  ///
  /// The default location is:
  /// - Unix: `~/.learner`
  /// - Windows: `%USERPROFILE%\.learner`
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Home directory cannot be determined
  /// - Directory creation fails
  /// - Insufficient permissions
  pub fn default_path() -> Result<PathBuf> {
    let config_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".learner");

    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
  }

  /// Returns the default path for retriever configuration files.
  ///
  /// The path is constructed as `{config_dir}/retrievers` where
  /// config_dir is determined by [`default_path()`](Config::default_path).
  pub fn default_retrievers_path() -> PathBuf {
    Self::default_path().unwrap_or_else(|_| PathBuf::from(".")).join("retrievers")
  }

  /// Returns the default path for resource configuration files.
  ///
  /// The path is constructed as `{config_dir}/retrievers` where
  /// config_dir is determined by [`default_path()`](Config::default_path).
  pub fn default_resources_path() -> PathBuf {
    Self::default_path().unwrap_or_else(|_| PathBuf::from(".")).join("resources")
  }

  /// Loads existing configuration or creates new with defaults.
  ///
  /// Looks for configuration file at the default path. If not found,
  /// creates new configuration file with default settings.
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration file exists but cannot be read
  /// - TOML parsing fails
  /// - File creation fails when saving defaults
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

  /// Saves current configuration to disk.
  ///
  /// Writes configuration to the default path in TOML format and ensures
  /// all required directories exist.
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - TOML serialization fails
  /// - File write fails
  /// - Directory creation fails
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

  /// Creates new configuration with example retriever configurations.
  ///
  /// Initializes configuration with defaults and creates example
  /// configurations for arXiv and DOI retrievers.
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration save fails
  /// - Retriever directory creation fails
  /// - Example config writes fail
  pub fn init() -> Result<Self> {
    let config = Self::default();
    config.save()?;

    // Write example resource configs
    let resources_dir = &config.resources_path;
    std::fs::create_dir_all(resources_dir)?;

    std::fs::write(resources_dir.join("paper.toml"), PAPER_CONFIG)?;
    std::fs::write(resources_dir.join("book.toml"), BOOK_CONFIG)?;
    std::fs::write(resources_dir.join("thesis.toml"), THESIS_CONFIG)?;

    // Write example retriever configs
    let retrievers_dir = &config.retrievers_path;
    std::fs::create_dir_all(retrievers_dir)?;

    std::fs::write(retrievers_dir.join("arxiv.toml"), ARXIV_CONFIG)?;
    std::fs::write(retrievers_dir.join("doi.toml"), DOI_CONFIG)?;
    std::fs::write(retrievers_dir.join("iacr.toml"), IACR_CONFIG)?;

    Ok(config)
  }

  /// Sets the database file path.
  ///
  /// # Arguments
  ///
  /// * `database_path` - Path where the SQLite database file should be stored
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Config;
  /// # use std::path::PathBuf;
  /// let config = Config::default().with_database_path(&PathBuf::from("~/papers/db.sqlite"));
  /// ```
  pub fn with_database_path(mut self, database_path: &Path) -> Self {
    self.database_path = database_path.to_path_buf();
    self
  }

  /// Sets the path for retriever configuration files.
  ///
  /// # Arguments
  ///
  /// * `retrievers_path` - Directory where retriever TOML configs are stored
  pub fn with_retrievers_path(mut self, retrievers_path: &Path) -> Self {
    self.retrievers_path = retrievers_path.to_path_buf();
    self
  }

  /// Sets the path for retriever configuration files.
  ///
  /// # Arguments
  ///
  /// * `retrievers_path` - Directory where retriever TOML configs are stored
  pub fn with_resources_path(mut self, resources_path: &Path) -> Self {
    self.resources_path = resources_path.to_path_buf();
    self
  }

  /// Sets the path for paper document storage.
  ///
  /// # Arguments
  ///
  /// * `storage_path` - Directory where paper PDFs will be stored
  pub fn with_storage_path(mut self, storage_path: &Path) -> Self {
    self.storage_path = storage_path.to_path_buf();
    self
  }
}

impl Default for Config {
  fn default() -> Self {
    Self {
      database_path:   Database::default_path(),
      storage_path:    Database::default_storage_path(),
      retrievers_path: Self::default_retrievers_path(),
      resources_path:  Self::default_resources_path(),
    }
  }
}

impl LearnerBuilder {
  /// Creates a new builder instance with no configuration.
  pub fn new() -> Self { Self::default() }

  /// Sets explicit configuration for the builder.
  ///
  /// # Arguments
  ///
  /// * `config` - Configuration to use
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{Config, Learner};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let config = Config::default();
  /// let learner = Learner::builder().with_config(config).build().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_config(mut self, config: Config) -> Self {
    self.config = Some(config);
    self
  }

  /// Sets path to load configuration from.
  ///
  /// # Arguments
  ///
  /// * `path` - Directory containing config.toml
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let learner = Learner::builder().with_path("~/.learner").build().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
    self.config_path = Some(path.as_ref().to_path_buf());
    self
  }

  /// Builds a new [`Learner`] instance with the configured options.
  ///
  /// This method:
  /// 1. Resolves configuration from provided sources
  /// 2. Ensures required directories exist
  /// 3. Opens database connection
  /// 4. Initializes paper retriever
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration loading fails
  /// - Directory creation fails
  /// - Database initialization fails
  /// - Retriever configuration fails
  pub async fn build(self) -> Result<Learner> {
    let config = if let Some(config) = self.config {
      config
    } else if let Some(path) = self.config_path {
      let config_file = path.join("config.toml");
      let content = std::fs::read_to_string(config_file)?;
      toml::from_str(&content).map_err(|e| LearnerError::Config(e.to_string()))?
    } else {
      Config::load()?
    };

    // Ensure paths exist
    std::fs::create_dir_all(&config.resources_path)?;
    std::fs::create_dir_all(&config.retrievers_path)?;
    if let Some(parent) = config.database_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&config.storage_path)?;

    let database = Database::open(&config.database_path).await?;
    database.set_storage_path(&config.storage_path).await?;

    let retriever = Retriever::new().with_config_dir(&config.retrievers_path)?;
    let resources = Resources::new().with_config_dir(&config.resources_path)?;

    Ok(Learner { config, database, retriever, resources })
  }
}

impl Learner {
  /// Returns a builder for creating a new configured Learner instance.
  ///
  /// This is the recommended way to construct a Learner as it provides
  /// fine-grained control over initialization options.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let learner = Learner::builder().with_path("~/.learner").build().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn builder() -> LearnerBuilder { LearnerBuilder::new() }

  /// Creates a new Learner instance with default configuration.
  ///
  /// This will:
  /// 1. Load or create configuration at default location
  /// 2. Initialize database in default location
  /// 3. Set up default paper storage
  /// 4. Configure default retrievers
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration loading fails
  /// - Directory creation fails
  /// - Database initialization fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut learner = Learner::new().await?;
  ///
  /// // Ready to use with default configuration
  /// let paper = learner.retriever.get_paper("2301.07041").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn new() -> Result<Self> { Self::builder().build().await }

  /// Creates a new Learner instance from a configuration file path.
  ///
  /// Loads configuration from the specified directory, which should
  /// contain a config.toml file.
  ///
  /// # Arguments
  ///
  /// * `path` - Directory containing config.toml
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration file does not exist
  /// - TOML parsing fails
  /// - Directory creation fails
  /// - Initialization fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Load from custom location
  /// let learner = Learner::from_path("~/research/papers/config").await?;
  ///
  /// // Or use environment-specific config
  /// let learner = Learner::from_path("/etc/learner").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn from_path(path: impl AsRef<Path>) -> Result<Self> {
    Self::builder().with_path(path).build().await
  }

  /// Creates a new Learner instance with explicit configuration.
  ///
  /// Use this when you need complete control over the configuration
  /// or are generating configuration programmatically.
  ///
  /// # Arguments
  ///
  /// * `config` - Complete configuration to use
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Directory creation fails
  /// - Database initialization fails
  /// - Retriever configuration fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{Config, Learner};
  /// # use std::path::PathBuf;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Create custom configuration
  /// let config = Config::default()
  ///   .with_database_path(&PathBuf::from("papers.db"))
  ///   .with_storage_path(&PathBuf::from("papers"));
  ///
  /// // Initialize with custom config
  /// let learner = Learner::with_config(config).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn with_config(config: Config) -> Result<Self> {
    Self::builder().with_config(config).build().await
  }

  /// Initializes a new Learner instance with example configuration.
  ///
  /// This method:
  /// 1. Creates example configuration
  /// 2. Writes example retriever configs (arXiv, DOI)
  /// 3. Sets up directory structure
  /// 4. Initializes empty database
  ///
  /// Ideal for first-time setup or testing.
  ///
  /// # Errors
  ///
  /// Returns error if:
  /// - Configuration initialization fails
  /// - Directory creation fails
  /// - Example config writing fails
  /// - Database initialization fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::Learner;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // Initialize new installation with examples
  /// let mut learner = Learner::init().await?;
  ///
  /// // Ready to use with example retrievers
  /// let paper = learner.retriever.get_paper("2301.07041").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn init() -> Result<Self> { Self::with_config(Config::init()?).await }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_learner_creation() {
    let config_dir = tempdir().unwrap();
    let database_dir = tempdir().unwrap();
    let storage_dir = tempdir().unwrap();
    let config = Config::default()
      .with_database_path(&database_dir.path().join("learner.db"))
      .with_resources_path(&config_dir.path().join("config/resources/"))
      .with_retrievers_path(&config_dir.path().join("config/retrievers/"))
      .with_storage_path(storage_dir.path());
    let learner =
      Learner::builder().with_path(config_dir.path()).with_config(config).build().await.unwrap();

    assert_eq!(learner.config.resources_path, config_dir.path().join("config/resources/"));
    assert_eq!(learner.config.retrievers_path, config_dir.path().join("config/retrievers/"));
    assert_eq!(learner.config.database_path, database_dir.path().join("learner.db"));
    assert_eq!(learner.database.get_storage_path().await.unwrap(), storage_dir.path());
  }
}
