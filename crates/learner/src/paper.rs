//! Paper management and metadata types for the learner library.
//!
//! This module provides types and functionality for working with academic papers from
//! various sources including arXiv, IACR, and DOI-based repositories. It handles paper
//! metadata, author information, and source-specific identifier parsing.
//!
//! # Examples
//!
//! ```no_run
//! use learner::paper::Paper;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a paper from an arXiv URL
//! let paper = Paper::new("https://arxiv.org/abs/2301.07041").await?;
//! println!("Title: {}", paper.title);
//!
//! // Or from a DOI
//! let paper = Paper::new("10.1145/1327452.1327492").await?;
//!
//! // Save to database
//! let db = learner::database::Database::open("papers.db").await?;
//! paper.save(&db).await?;
//! # Ok(())
//! # }
//! ```

use super::*;

/// The source repository or system from which a paper originates.
///
/// This enum represents the supported academic paper sources, each with its own
/// identifier format and access patterns.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Source {
  /// Papers from arxiv.org, using either new-style (2301.07041) or
  /// old-style (math.AG/0601001) identifiers
  Arxiv,
  /// Papers from the International Association for Cryptologic Research (eprint.iacr.org)
  IACR,
  /// Papers identified by a Digital Object Identifier (DOI)
  DOI,
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

/// Represents an author of an academic paper.
///
/// Contains the author's name and optional affiliation and contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
  /// The author's full name
  pub name:        String,
  /// The author's institutional affiliation, if available
  pub affiliation: Option<String>,
  /// The author's email address, if available
  pub email:       Option<String>,
}

/// A complete academic paper with its metadata.
///
/// This struct represents a paper from any supported source (arXiv, IACR, DOI)
/// along with its metadata including title, authors, abstract, and identifiers.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Fetch a paper from arXiv
/// let paper = learner::paper::Paper::new("2301.07041").await?;
///
/// // Access metadata
/// println!("Title: {}", paper.title);
/// println!("Authors: {}", paper.authors.len());
/// println!("Abstract: {}", paper.abstract_text);
///
/// // Download the PDF if available
/// if let Some(pdf_url) = &paper.pdf_url {
///   paper.download_pdf("paper.pdf".into()).await?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
  /// The paper's title
  pub title:             String,
  /// List of the paper's authors
  pub authors:           Vec<Author>,
  /// The paper's abstract text
  pub abstract_text:     String,
  /// When the paper was published or last updated
  pub publication_date:  DateTime<Utc>,
  /// The source system (arXiv, IACR, DOI)
  pub source:            Source,
  /// The source-specific identifier (e.g., arXiv ID, DOI)
  pub source_identifier: String,
  /// URL to the paper's PDF, if available
  pub pdf_url:           Option<String>,
  /// The paper's DOI, if available
  pub doi:               Option<String>,
}

impl Paper {
  /// Create a new paper from a URL, identifier, or DOI.
  ///
  /// This method accepts various formats for paper identification and automatically
  /// determines the appropriate source and fetches the paper's metadata.
  ///
  /// # Arguments
  ///
  /// * `input` - One of the following:
  ///   - An arXiv URL (e.g., "https://arxiv.org/abs/2301.07041")
  ///   - An arXiv ID (e.g., "2301.07041" or "math.AG/0601001")
  ///   - An IACR URL (e.g., "https://eprint.iacr.org/2016/260")
  ///   - An IACR ID (e.g., "2023/123")
  ///   - A DOI URL (e.g., "https://doi.org/10.1145/1327452.1327492")
  ///   - A DOI (e.g., "10.1145/1327452.1327492")
  ///
  /// # Returns
  ///
  /// Returns a `Result<Paper>` which is:
  /// - `Ok(Paper)` - Successfully fetched paper with metadata
  /// - `Err(LearnerError)` - Failed to parse input or fetch paper
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::paper::Paper;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// // From arXiv URL
  /// let paper1 = Paper::new("https://arxiv.org/abs/2301.07041").await?;
  ///
  /// // From arXiv ID
  /// let paper2 = Paper::new("2301.07041").await?;
  ///
  /// // From DOI
  /// let paper3 = Paper::new("10.1145/1327452.1327492").await?;
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

  /// Download the paper's PDF to a specified path.
  ///
  /// # Arguments
  ///
  /// * `path` - The filesystem path where the PDF should be saved
  ///
  /// # Errors
  ///
  /// Returns `LearnerError` if:
  /// - The paper has no PDF URL available
  /// - The download fails
  /// - Writing to the specified path fails
  pub async fn download_pdf(&self, dir: PathBuf) -> Result<()> {
    // unimplemented!("Work in progress -- needs integrated with `Database`");
    let Some(pdf_url) = &self.pdf_url else {
      return Err(LearnerError::ApiError("No PDF URL available".into()));
    };

    let response = reqwest::get(pdf_url).await?;
    trace!("{} pdf_url response: {response:?}", self.source);
    let bytes = response.bytes().await?;

    // TODO (autoparallel): uses a fixed max output filename length, should make this configurable
    // in the future.
    let formatted_title = format::format_title(&self.title, Some(50));
    let path = dir.join(format!("{}.pdf", formatted_title));
    debug!("Writing PDF to path: {path:?}");
    std::fs::write(path, bytes)?;
    Ok(())
  }

  /// Save the paper to a database.
  ///
  /// # Arguments
  ///
  /// * `db` - Reference to an open database connection
  ///
  /// # Returns
  ///
  /// Returns the database ID of the saved paper on success.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let paper = learner::paper::Paper::new("2301.07041").await?;
  /// let db = learner::database::Database::open("papers.db").await?;
  /// let id = paper.save(&db).await?;
  /// println!("Saved paper with ID: {}", id);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn save(&self, db: &Database) -> Result<i64> { db.save_paper(self).await }
}

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
  async fn test_iacr_paper_from_id() -> anyhow::Result<()> {
    let paper = Paper::new("2016/260").await?;
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::IACR);
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_paper_from_url() -> anyhow::Result<()> {
    let paper = Paper::new("https://eprint.iacr.org/2016/260").await?;
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::IACR);
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_doi_paper_from_id() -> anyhow::Result<()> {
    let paper = Paper::new("10.1145/1327452.1327492").await?;
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::DOI);
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_doi_paper_from_url() -> anyhow::Result<()> {
    let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await?;
    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::DOI);
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_arxiv_pdf_from_paper() -> anyhow::Result<()> {
    let paper = Paper::new("https://arxiv.org/abs/2301.07041").await.unwrap();
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path().to_path_buf()).await.unwrap();
    let formatted_title = format::format_title("Verifiable Fully Homomorphic Encryption", Some(50));
    let path = dir.into_path().join(format!("{}.pdf", formatted_title));
    assert!(path.exists());
    Ok(())
  }

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_pdf_from_paper() -> anyhow::Result<()> {
    let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path().to_path_buf()).await.unwrap();
    let formatted_title =
      format::format_title("On the Size of Pairing-based Non-interactive Arguments", Some(50));
    let path = dir.into_path().join(format!("{}.pdf", formatted_title));
    assert!(path.exists());
    Ok(())
  }

  // TODO (autoparallel): This technically passes, but it is not actually getting a PDF from this
  // site.
  #[ignore]
  #[traced_test]
  #[tokio::test]
  async fn test_doi_pdf_from_paper() -> anyhow::Result<()> {
    let paper = Paper::new("https://doi.org/10.1145/1327452.1327492").await.unwrap();
    dbg!(&paper);
    let dir = tempdir().unwrap();
    paper.download_pdf(dir.path().to_path_buf()).await.unwrap();
    let formatted_title =
      format::format_title("MapReduce: simplified data processing on large clusters", Some(50));
    let path = dir.into_path().join(format!("{}.pdf", formatted_title));
    assert!(path.exists());
    Ok(())
  }

  //  TODO (autoparallel): Convenient entrypoint to try seeing if the PDF comes out correct. What I
  // have tried now is using a `reqwest` client with ```
  // let _ = client.get("https://dl.acm.org/").send().await?;
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
