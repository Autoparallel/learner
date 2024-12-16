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

// #![warn(missing_docs, clippy::missing_docs_in_private_items)]
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, instrument, trace, warn};
#[cfg(test)]
use {tempfile::tempdir, tracing_test::traced_test};

pub mod database;
pub mod retriever;

pub mod configuration;

pub mod error;
pub mod format;
pub mod llm;
pub mod pdf;
pub mod record;
pub mod resource;
pub mod template;

use crate::{error::*, retriever::*};

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
  pub use crate::{database::DatabaseInstruction, error::LearnerError};
}
