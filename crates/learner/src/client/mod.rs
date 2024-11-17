//! Client implementations for fetching papers from various academic sources.
//!
//! This module provides specialized clients for different paper repositories and
//! citation systems. Each submodule implements source-specific logic for:
//! - Parsing identifiers
//! - Making API requests
//! - Converting responses to the common [`Paper`] format
//!
//! # Supported Sources
//!
//! - [`arxiv`] - Client for the arXiv.org preprint server
//! - [`iacr`] - Client for the International Association for Cryptologic Research
//! - [`doi`] - Client for resolving Digital Object Identifiers (DOIs)
//!
//! # Examples
//!
//! ```no_run
//! use learner::clients::{arxiv::ArxivClient, doi::DOIClient, iacr::IACRClient};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Fetch from arXiv
//! let arxiv_paper = ArxivClient::new().fetch_paper("2301.07041").await?;
//!
//! // Fetch from IACR
//! let iacr_paper = IACRClient::new().fetch_paper("2023/123").await?;
//!
//! // Fetch using DOI
//! let doi_paper = DOIClient::new().fetch_paper("10.1145/1327452.1327492").await?;
//! # Ok(())
//! # }
//! ```

use quick_xml::de::from_str;

pub mod arxiv;
pub mod doi;
pub mod iacr;

pub use arxiv::ArxivClient;
pub use doi::DOIClient;
pub use iacr::IACRClient;

use super::*;

pub trait PaperRetriever {
  const HOST: &str;
  fn retrieve_paper(&self, identifier: &str) -> Result<Paper>;
  fn extract_identifier(&self, input: &str) -> Result<&str>;
}
