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
  /// Source repository or system (arXiv, IACR, DOI)
  pub source:            Source,
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

/// Paper source system or repository.
///
/// Represents the different systems from which papers can be retrieved,
/// each with its own identifier format and access patterns. The enum
/// supports:
///
/// - arXiv: Both new (2301.07041) and old (math.AG/0601001) formats
/// - IACR: Cryptology ePrint Archive format (2023/123)
/// - DOI: Standard DOI format (10.1145/1327452.1327492)
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
///
/// use learner::paper::Source;
///
/// let arxiv = Source::from_str("arxiv").unwrap();
/// let doi = Source::from_str("doi").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Source {
  /// arXiv.org papers (e.g., "2301.07041" or "math.AG/0601001")
  Arxiv,
  /// IACR Cryptology ePrint Archive papers (e.g., "2023/123")
  IACR,
  /// Papers with Digital Object Identifiers
  DOI,
}

impl Paper {
  // TODO (autoparallel): This should probably be a `new_from_url` or just `from_url` or something.
  /// Creates a new paper from various identifier formats.
  ///
  /// This method serves as the primary entry point for paper creation,
  /// supporting multiple input formats and automatically determining the
  /// appropriate source handler. It accepts:
  ///
  /// - Full URLs from supported repositories
  /// - Direct identifiers (arXiv ID, DOI, IACR ID)
  /// - Both new and legacy identifier formats
  ///
  /// The method will fetch metadata from the appropriate source and
  /// construct a complete Paper instance.
  ///
  /// # Arguments
  ///
  /// * `input` - Paper identifier in any supported format:
  ///   - arXiv URLs: "https://arxiv.org/abs/2301.07041"
  ///   - arXiv IDs: "2301.07041" or "math.AG/0601001"
  ///   - IACR URLs: "https://eprint.iacr.org/2016/260"
  ///   - IACR IDs: "2023/123"
  ///   - DOI URLs: "https://doi.org/10.1145/1327452.1327492"
  ///   - DOIs: "10.1145/1327452.1327492"
  ///
  /// # Returns
  ///
  /// Returns a `Result<Paper>` which is:
  /// - `Ok(Paper)` - Successfully created paper with metadata
  /// - `Err(LearnerError)` - Failed to parse input or fetch metadata
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::paper::Paper;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // From URL
  /// let paper = Paper::new("https://arxiv.org/abs/2301.07041").await?;
  ///
  /// // From identifier
  /// let paper = Paper::new("2301.07041").await?;
  ///
  /// // From DOI
  /// let paper = Paper::new("10.1145/1327452.1327492").await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn new(input: &str) -> Result<Self> {
    lazy_static! {
        // arXiv patterns
        static ref ARXIV_NEW: Regex = Regex::new(r"^(\d{4}\.\d{4,5})$").unwrap();
        static ref ARXIV_OLD: Regex = Regex::new(r"^([a-zA-Z-]+/\d{7})$").unwrap();

        // IACR pattern
        static ref IACR: Regex = Regex::new(r"^(\d{4}/\d+)$").unwrap();

        // DOI pattern
        static ref DOI: Regex = Regex::new(r"^10\.\d{4,9}/[-._;()/:\w]+$").unwrap();
    }

    // First try to parse as URL
    if let Ok(url) = Url::parse(input) {
      return match url.host_str() {
        Some("arxiv.org") => {
          let id = extract_arxiv_id(&url)?;
          ArxivClient::new().fetch_paper(&id).await
        },
        Some("eprint.iacr.org") => {
          let id = extract_iacr_id(&url)?;
          IACRClient::new().fetch_paper(&id).await
        },
        Some("doi.org") => {
          let doi = extract_doi(&url)?;
          DOIClient::new().fetch_paper(&doi).await
        },
        _ => Err(LearnerError::InvalidIdentifier),
      };
    }

    // If not a URL, try to match against known patterns
    match input {
      // arXiv patterns
      id if ARXIV_NEW.is_match(id) || ARXIV_OLD.is_match(id) =>
        ArxivClient::new().fetch_paper(id).await,

      // IACR pattern
      id if IACR.is_match(id) => IACRClient::new().fetch_paper(id).await,

      // DOI pattern
      id if DOI.is_match(id) => DOIClient::new().fetch_paper(id).await,

      // No pattern matched
      _ => Err(LearnerError::InvalidIdentifier),
    }
  }

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

impl Display for Source {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Source::Arxiv => write!(f, "Arxiv"),
      Source::IACR => write!(f, "IACR"),
      Source::DOI => write!(f, "DOI"),
    }
  }
}

impl FromStr for Source {
  type Err = LearnerError;

  fn from_str(s: &str) -> Result<Self> {
    match &s.to_lowercase() as &str {
      "arxiv" => Ok(Source::Arxiv),
      "iacr" => Ok(Source::IACR),
      "doi" => Ok(Source::DOI),
      s => Err(LearnerError::InvalidSource(s.to_owned())),
    }
  }
}

// TODO (autoparallel): These three functions should really be some simple generic alongside the
// rest of the stuff we have in here
/// Extracts the arXiv identifier from a URL.
///
/// Parses URLs like "https://arxiv.org/abs/2301.07041" to extract "2301.07041".
fn extract_arxiv_id(url: &Url) -> Result<String> {
  let path = url.path();
  let re = regex::Regex::new(r"abs/([^/]+)$").unwrap();
  re.captures(path)
    .and_then(|cap| cap.get(1))
    .map(|m| m.as_str().to_string())
    .ok_or(LearnerError::InvalidIdentifier)
}

/// Extracts the IACR identifier from a URL.
///
/// Parses URLs like "https://eprint.iacr.org/2016/260" to extract "2016/260".
fn extract_iacr_id(url: &Url) -> Result<String> {
  let path = url.path();
  let re = regex::Regex::new(r"(\d{4}/\d+)$").unwrap();
  re.captures(path)
    .and_then(|cap| cap.get(1))
    .map(|m| m.as_str().to_string())
    .ok_or(LearnerError::InvalidIdentifier)
}

/// Extracts the DOI from a URL.
///
/// Parses URLs like "https://doi.org/10.1145/1327452.1327492" to extract the DOI.
fn extract_doi(url: &Url) -> Result<String> {
  url.path().strip_prefix('/').map(|s| s.to_string()).ok_or(LearnerError::InvalidIdentifier)
}

#[cfg(test)]
mod tests {

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_arxiv_paper_from_id() {
    let paper = Paper::new("2301.07041").await.unwrap();
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::Arxiv);
    dbg!(paper);
  }

  #[traced_test]
  #[tokio::test]
  async fn test_arxiv_paper_from_url() {
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
    assert_eq!(paper.source, Source::Arxiv);
    assert_eq!(paper.source_identifier, "2301.07041");
  }

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_paper_from_id() {
    let paper = Paper::new("2016/260").await.unwrap();
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::IACR);
  }

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_paper_from_url() {
    let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::IACR);
  }

  #[traced_test]
  #[tokio::test]
  async fn test_doi_paper_from_id() {
    let paper = Paper::new("10.1145/1327452.1327492").await.unwrap();
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::DOI);
  }

  #[traced_test]
  #[tokio::test]
  async fn test_doi_paper_from_url() {
    let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::DOI);
  }

  #[traced_test]
  #[tokio::test]
  async fn test_arxiv_pdf_from_paper() {
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path()).await.unwrap();
    let formatted_title = format::format_title("Verifiable Fully Homomorphic Encryption", Some(50));
    let path = dir.into_path().join(format!("{}.pdf", formatted_title));
    assert!(path.exists());
  }

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_pdf_from_paper() {
    let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path()).await.unwrap();
    let formatted_title =
      format::format_title("On the Size of Pairing-based Non-interactive Arguments", Some(50));
    let path = dir.into_path().join(format!("{}.pdf", formatted_title));
    assert!(path.exists());
  }

  // TODO (autoparallel): This technically passes, but it is not actually getting a PDF from this
  // site.
  #[ignore]
  #[traced_test]
  #[tokio::test]
  async fn test_doi_pdf_from_paper() {
    let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
    dbg!(&paper);
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path()).await.unwrap();
    let path = dir.into_path().join(paper.filename());
    assert!(path.exists());
  }

  #[traced_test]
  #[tokio::test]
  async fn test_broken_api_link() {
    assert!(Paper::new("https://arxiv.org/abs/2401.00000").await.is_err());
  }

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
