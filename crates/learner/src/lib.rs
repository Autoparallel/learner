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
#![feature(str_from_utf16_endian)]

use std::{
  fmt::Display,
  path::{Path, PathBuf},
  str::FromStr,
};

use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use paper::{Author, Paper, Source};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};
#[cfg(test)]
use {tempfile::tempdir, tracing_test::traced_test};

pub mod clients;
pub mod database;
pub mod errors;
pub mod format;
pub mod llm;
pub mod paper;
pub mod pdf;

use crate::{
  clients::{ArxivClient, DOIClient, IACRClient},
  database::Database,
  errors::LearnerError,
};
