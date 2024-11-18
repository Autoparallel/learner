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
//! use learner::paper::Paper;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // From arXiv URL
//! let paper = Paper::new("https://arxiv.org/abs/2301.07041").await?;
//! println!("Title: {}", paper.title);
//!
//! // From DOI
//! let paper = Paper::new("10.1145/1327452.1327492").await?;
//!
//! // From IACR
//! let paper = Paper::new("2023/123").await?;
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
/// # use learner::paper::Paper;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create from identifier
/// let paper = Paper::new("2301.07041").await?;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Author information for academic papers.
///
/// Represents a single author of a paper, including their name and optional
/// institutional details. This struct supports varying levels of author
/// information availability across different sources.
///
/// # Examples
///
/// ```
/// use learner::paper::Author;
///
/// let author = Author {
///   name:        "Alice Researcher".to_string(),
///   affiliation: Some("Example University".to_string()),
///   email:       Some("alice@example.edu".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
  /// Author's full name
  pub name:        String,
  /// Optional institutional affiliation
  pub affiliation: Option<String>,
  /// Optional contact email
  pub email:       Option<String>,
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
  /// # use learner::paper::Paper;
  /// # use std::path::PathBuf;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let paper = Paper::new("2301.07041").await?;
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
  /// # use learner::paper::Paper;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let paper = Paper::new("2301.07041").await?;
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

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test_test_test() { todo!("Need to rehash all these tests now.") }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_arxiv_paper_from_id() {
  //   todo!();
  //   let paper = Paper::new("2301.07041").await.unwrap();
  //   assert!(!paper.title.is_empty());
  //   assert!(!paper.authors.is_empty());
  //   assert_eq!(paper.source, "arxiv");
  //   dbg!(paper);
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_arxiv_paper_from_url() {
  //   todo!();
  //   let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
  //   assert_eq!(paper.source, "arxiv");
  //   assert_eq!(paper.source_identifier, "2301.07041");
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_iacr_paper_from_id() {
  //   todo!();
  //   let paper = Paper::new("2016/260").await.unwrap();
  //   assert!(!paper.title.is_empty());
  //   assert!(!paper.authors.is_empty());
  //   assert_eq!(paper.source, "iacr");
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_iacr_paper_from_url() {
  //   todo!();
  //   let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
  //   assert!(!paper.title.is_empty());
  //   assert!(!paper.authors.is_empty());
  //   assert_eq!(paper.source, "iacr");
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_doi_paper_from_id() {
  //   todo!();
  //   let paper = Paper::new("10.1145/1327452.1327492").await.unwrap();
  //   assert!(!paper.title.is_empty());
  //   assert!(!paper.authors.is_empty());
  //   assert_eq!(paper.source, "doi");
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_doi_paper_from_url() {
  //   todo!();
  //   let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
  //   assert!(!paper.title.is_empty());
  //   assert!(!paper.authors.is_empty());
  //   assert_eq!(paper.source, "doi");
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_arxiv_pdf_from_paper() {
  //   todo!();
  //   let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
  //   let dir = tempdir().unwrap();
  //   paper.download_pdf(dir.path()).await.unwrap();
  //   let formatted_title = format::format_title("Verifiable Fully Homomorphic Encryption",
  // Some(50));   let path = dir.into_path().join(format!("{}.pdf", formatted_title));
  //   assert!(path.exists());
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_iacr_pdf_from_paper() {
  //   todo!();
  //   let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
  //   let dir = tempdir().unwrap();
  //   paper.download_pdf(dir.path()).await.unwrap();
  //   let formatted_title =
  //     format::format_title("On the Size of Pairing-based Non-interactive Arguments", Some(50));
  //   let path = dir.into_path().join(format!("{}.pdf", formatted_title));
  //   assert!(path.exists());
  // }

  // // TODO (autoparallel): This technically passes, but it is not actually getting a PDF from this
  // // site.
  // #[traced_test]
  // #[tokio::test]
  // async fn test_doi_pdf_from_paper() {
  //   todo!();
  //   let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
  //   dbg!(&paper);
  //   let dir = tempdir().unwrap();
  //   paper.download_pdf(dir.path()).await.unwrap();
  //   let path = dir.into_path().join(paper.filename());
  //   assert!(path.exists());
  // }

  // #[traced_test]
  // #[tokio::test]
  // async fn test_broken_api_link() {
  //   todo!();
  //   assert!(Paper::new("https://arxiv.org/abs/2401.00000").await.is_err());
  // }

  //  TODO (autoparallel): Convenient entrypoint to try seeing if the PDF comes out correct. What I
  // have tried now is using a `reqwest` client with ```
  // let _ = client.get("https://dl.acm.org/").send().await.unwrap();
  //
  // let response = client
  //   .get(pdf_url)
  //   .header(header::REFERER, "https://dl.acm.org/")
  //   .header(header::ACCEPT, "application/pdf")
  //   .header(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
  //   .header(header::ACCEPT_ENCODING, "gzip, deflate, br")
  //   .send()
  //   .await?;
  // ```
  // This required having the "cookies" feature for reqwest.

  // #[traced_test]
  // #[tokio::test]
  // async fn test_iacr_pdf_from_paper_test() -> anyhow::Result<()> {
  //   let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
  //   paper.download_pdf(PathBuf::new().join(".")).await;
  //   Ok(())
  // }
}

// https://dl.acm.org/doi/pdf/10.1145/1327452.1327492
