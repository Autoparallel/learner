//! Core paper management and metadata types for academic paper handling.
//!
//! This module provides the fundamental types and functionality for working with
//! academic papers from various sources. It handles:
//!
//! - Paper metadata management
//! - Multi-source identifier parsing
//! - Author information
//! - Document downloading
//! - Source-specific identifier formats
//!
//! The implementation supports papers from:
//! - arXiv (both new-style and old-style identifiers)
//! - IACR (International Association for Cryptologic Research)
//! - DOI (Digital Object Identifier)
//!
//! # Examples
//!
//! Creating papers from different sources:
//!
//! ```no_run
//! use learner::{resource::Paper, Learner};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let learner = Learner::builder().build().await?;
//!
//! // From arXiv URL
//! let paper = learner.retriever.get_paper("https://arxiv.org/abs/2301.07041").await?;
//! println!("Title: {}", paper.title);
//!
//! // From DOI
//! let paper = learner.retriever.get_paper("10.1145/1327452.1327492").await?;
//!
//! // From IACR
//! let paper = learner.retriever.get_paper("2023/123").await?;
//!
//! // Download associated PDF
//! use std::path::PathBuf;
//! let storage = PathBuf::from("papers");
//! paper.download_pdf(&storage).await?;
//! # Ok(())
//! # }
//! ```

use super::*;

/// Complete representation of an academic paper with metadata.
///
/// This struct serves as the core data type for paper management, containing
/// all relevant metadata and document references. It supports papers from
/// multiple sources while maintaining a consistent interface for:
///
/// - Basic metadata (title, abstract, dates)
/// - Author information
/// - Source-specific identifiers
/// - Document access
///
/// Papers can be created from various identifier formats and URLs, with the
/// appropriate source being automatically detected.
///
/// # Examples
///
/// Creating and using papers:
///
/// ```no_run
/// # use learner::{Learner, resource::Paper};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a Learner instance to connect to a database
/// let mut learner = Learner::builder().build().await?;
/// // Create from identifier
/// let paper = learner.retriever.get_paper("2301.07041").await?;
///
/// // Access metadata
/// println!("Title: {}", paper.title);
/// println!("Authors: {}", paper.authors.len());
/// println!("Abstract: {}", paper.abstract_text);
///
/// // Handle documents
/// if let Some(url) = &paper.pdf_url {
///   println!("PDF available at: {}", url);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Paper {
  /// The paper's full title
  pub title:             String,
  /// Complete list of paper authors with affiliations
  pub authors:           Vec<Author>,
  /// Full abstract or summary text
  pub abstract_text:     String,
  /// Publication or last update timestamp
  pub publication_date:  DateTime<Utc>,
  /// Source repository or system (arXiv, DOI, IACR, etc.)
  pub source:            String,
  /// Source-specific paper identifier
  pub source_identifier: String,
  /// Optional URL to PDF document
  pub pdf_url:           Option<String>,
  /// Optional DOI reference
  pub doi:               Option<String>,
}

impl Paper {
  /// Downloads the paper's PDF to the specified directory.
  ///
  /// This method handles the retrieval and storage of the paper's PDF
  /// document, if available. It will:
  ///
  /// 1. Check for PDF availability
  /// 2. Download the document
  /// 3. Store it with a formatted filename
  /// 4. Handle network and storage errors
  ///
  /// # Arguments
  ///
  /// * `dir` - Target directory for PDF storage
  ///
  /// # Returns
  ///
  /// Returns a `Result` containing:
  /// - `Ok(PathBuf)` - Path to the stored PDF file
  /// - `Err(LearnerError)` - If download or storage fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{Learner, resource::Paper};
  /// # use std::path::PathBuf;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let learner = Learner::builder().build().await?;
  /// let paper = learner.retriever.get_paper("2301.07041").await?;
  /// let dir = PathBuf::from("papers");
  /// let pdf_path = paper.download_pdf(&dir).await?;
  /// println!("PDF stored at: {}", pdf_path.display());
  /// # Ok(())
  /// # }
  /// ```
  pub async fn download_pdf(&self, dir: &Path) -> Result<PathBuf> {
    let Some(pdf_url) = &self.pdf_url else {
      return Err(LearnerError::ApiError("No PDF URL available".into()));
    };

    let response = reqwest::get(pdf_url).await?;

    // Check the status code of the response
    if response.status().is_success() {
      let bytes = response.bytes().await?;
      let path = dir.join(self.filename());
      debug!("Writing PDF to path: {path:?}");
      std::fs::write(path, bytes)?;
      Ok(self.filename())
    } else {
      // Handle non-successful status codes
      trace!("{} pdf_url response: {response:?}", self.source);
      Err(LearnerError::ApiError(format!("Failed to download PDF: {}", response.status())))
    }
  }

  /// Generates a standardized filename for the paper's PDF.
  ///
  /// Creates a filesystem-safe filename based on the paper's title,
  /// suitable for PDF storage. The filename is:
  /// - Truncated to a reasonable length
  /// - Cleaned of problematic characters
  /// - Suffixed with ".pdf"
  ///
  /// # Returns
  ///
  /// Returns a [`PathBuf`] containing the formatted filename.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::{Learner, resource::Paper};
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  ///
  /// let learner = Learner::builder().build().await?;
  /// let paper = learner.retriever.get_paper("2301.07041").await?;
  /// let filename = paper.filename();
  /// println!("Suggested filename: {}", filename.display());
  /// # Ok(())
  /// # }
  /// ```
  pub fn filename(&self) -> PathBuf {
    let formatted_title = format::format_title(&self.title, Some(50));
    PathBuf::from(format!("{}.pdf", formatted_title))
  }
}
