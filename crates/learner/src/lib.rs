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
  str::FromStr,
};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
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

use crate::{client::*, error::*};

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
