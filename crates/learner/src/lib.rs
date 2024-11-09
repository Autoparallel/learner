//! A library for fetching academic papers and their metadata from various sources
//! including arXiv, IACR, and DOI-based repositories.
//!
//! # Example
//! ```no_run
//! use learner::paper::{Paper, Source};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   // Fetch from arXiv
//!   let paper = Paper::new("2301.07041").await?;
//!   println!("Title: {}", paper.title);
//!
//!   Ok(())
//! }
//! ```

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use std::{path::PathBuf, str::FromStr};

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};
#[cfg(test)]
use {tempfile::tempdir, tracing_test::traced_test};

pub mod clients;
pub mod database;
pub mod errors;
pub mod format;
pub mod paper;
pub mod pdf;

use clients::{ArxivClient, DOIClient, IACRClient};
use database::Database;
use errors::LearnerError;
use paper::{Author, Paper, Source};
