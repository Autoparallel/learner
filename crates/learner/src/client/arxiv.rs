//! Client implementation for fetching papers from arXiv.org.
//!
//! This module provides functionality to interact with the arXiv API, fetch paper metadata,
//! and convert it to the common [`Paper`] format. It supports both new-style (2301.07041)
//! and old-style (math.AG/0601001) arXiv identifiers.
//!
//! The client uses arXiv's Atom feed API (http://export.arxiv.org/api/query) to fetch
//! paper metadata in XML format.
//!
//! # Examples
//!
//! ```no_run
//! use learner::clients::ArxivClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArxivClient::new();
//! let paper = client.fetch_paper("2301.07041").await?;
//!
//! println!("Title: {}", paper.title);
//! println!("Authors: {}", paper.authors.len());
//! # Ok(())
//! # }
//! ```

use super::*;

/// Internal representation of the arXiv API's Atom feed response.
#[derive(Debug, Deserialize)]
struct Feed {
  /// A `Feed` from arXiv may contain multiple `Entry`s
  #[serde(rename = "entry")]
  entries: Vec<Entry>,
}

// TODO: Note there are more things we get in a typical response which are probably useful honestly.
// I think we should capture those and also potentially put all of this in the `Source` enum
// variants so that the `Paper` struct contains all the relevant metadata.

/// Internal representation of a paper entry from arXiv's API response.
///
/// Note: The current implementation only captures a subset of the available metadata.
/// Future versions may expand this to include additional fields such as:
/// - Categories/subjects
/// - Comments
/// - Journal references
/// - Primary category
/// - Version information
#[derive(Debug, Deserialize)]
struct Entry {
  /// Paper title (may contain LaTeX markup)
  title:     String,
  /// List of paper authors
  #[serde(rename = "author")]
  authors:   Vec<Author>,
  /// Paper abstract (may contain LaTeX markup)
  summary:   String,
  /// Publication or last update date
  published: DateTime<Utc>,
  /// arXiv URL (e.g., "https://arxiv.org/abs/2301.07041")
  #[serde(rename = "id")]
  arxiv_url: String,
}

/// Internal representation of an author from arXiv's API response.
#[derive(Debug, Deserialize)]
struct Author {
  /// Author's full name
  name: String,
}

/// Client for interacting with the arXiv API.
///
/// This client provides methods to fetch paper metadata from arXiv.org using their
/// public API. It handles the HTTP requests, XML parsing, and conversion to the
/// common [`Paper`] format.
///
/// # Examples
///
/// ```no_run
/// # use learner::clients::arxiv::ArxivClient;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ArxivClient::new();
///
/// // Fetch using new-style ID
/// let paper1 = client.fetch_paper("2301.07041").await?;
///
/// // Fetch using old-style ID
/// let paper2 = client.fetch_paper("math.AG/0601001").await?;
/// # Ok(())
/// # }
/// ```
pub struct ArxivClient {
  /// Internal web client used to connect to the API.
  client: reqwest::Client,
}

impl ArxivClient {
  /// Creates a new arXiv client instance.
  ///
  /// Initializes an HTTP client that will be reused for all requests to the arXiv API.
  pub fn new() -> Self { Self { client: reqwest::Client::new() } }

  /// Fetches paper metadata from arXiv using its identifier.
  ///
  /// # Arguments
  ///
  /// * `identifier` - An arXiv paper identifier in either:
  ///   - New format (e.g., "2301.07041")
  ///   - Old format (e.g., "math.AG/0601001")
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - A [`Paper`] with the fetched metadata
  /// - A [`LearnerError`] if the fetch or parsing fails
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - The network request fails
  /// - The API response cannot be parsed
  /// - The paper is not found
  /// - The API returns an error response
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::clients::ArxivClient;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let client = ArxivClient::new();
  /// let paper = client.fetch_paper("2301.07041").await?;
  ///
  /// // The PDF URL is automatically generated
  /// if let Some(pdf_url) = paper.pdf_url {
  ///   println!("PDF available at: {}", pdf_url);
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub async fn fetch_paper(&self, identifier: &str) -> Result<Paper> {
    let url = format!("http://export.arxiv.org/api/query?id_list={}&max_results=1", identifier);

    debug!("Fetching from arXiv via: {url}");

    let response = self.client.get(&url).send().await?.text().await?;

    trace!("arXiv response: {response}");

    let feed: Feed = from_str(&response)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse XML: {}", e)))?;

    let entry = feed.entries.first().ok_or(LearnerError::NotFound)?;

    // Convert arXiv URL to PDF URL (just need to change /abs/ to /pdf/ and add .pdf)
    let pdf_url = entry.arxiv_url.replace("/abs/", "/pdf/") + ".pdf";

    Ok(Paper {
      title:             entry.title.clone(),
      authors:           entry
        .authors
        .iter()
        .map(|author| crate::Author {
          name:        author.name.clone(),
          affiliation: None,
          email:       None,
        })
        .collect(),
      abstract_text:     entry.summary.clone(),
      publication_date:  entry.published,
      source:            Source::Arxiv,
      source_identifier: identifier.to_string(),
      pdf_url:           Some(pdf_url),
      doi:               None, // We can add DOI extraction if needed
    })
  }
}

impl Default for ArxivClient {
  fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_arxiv_entry_fetch() {
    let client = ArxivClient::new();
    let paper = client.fetch_paper("2301.07041").await.unwrap();

    dbg!(&paper);

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::Arxiv);
    assert_eq!(paper.source_identifier, "2301.07041");
  }
}
