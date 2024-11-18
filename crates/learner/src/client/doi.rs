//! Client implementation for fetching papers using Digital Object Identifiers (DOIs).
//!
//! This module provides functionality to resolve DOIs and fetch paper metadata using
//! the Crossref API. It handles the conversion of Crossref's rich metadata format
//! into the common [`Paper`] structure.
//!
//! The client uses Crossref's REST API (https://api.crossref.org/) and follows their
//! best practices for API access.
//!
//! # Examples
//!
//! ```no_run
//! use learner::clients::DOIClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = DOIClient::new();
//! let paper = client.fetch_paper("10.1145/1327452.1327492").await?;
//!
//! println!("Title: {}", paper.title);
//! println!("DOI: {}", paper.doi.unwrap());
//! # Ok(())
//! # }
//! ```

use super::*;

/// Response structure from the Crossref API.
#[derive(Debug, Deserialize)]
struct CrossrefResponse {
  /// The main work metadata container
  message: CrossrefWork,
}

/// Metadata about an academic work from Crossref.
#[derive(Debug, Deserialize)]
struct CrossrefWork {
  /// Paper titles (usually contains one item)
  title:            Vec<String>,
  /// List of paper authors with their details
  author:           Vec<CrossrefAuthor>,
  /// Paper abstract, which may not be available for all works
  #[serde(rename = "abstract")]
  abstract_text:    Option<String>,
  /// Print publication date, if available
  published_print:  Option<CrossrefDate>,
  /// Online publication date, if available
  published_online: Option<CrossrefDate>,
  /// URL to the paper (may be the publisher's page)
  #[serde(rename = "URL")]
  url:              Option<String>,
  /// The paper's DOI
  #[serde(rename = "DOI")]
  doi:              String,
  /// Creation date in Crossref's system (fallback for publication date)
  created:          Option<CrossrefDate>,
}

/// Author information from Crossref.
#[derive(Debug, Deserialize)]
struct CrossrefAuthor {
  /// Author's given (first) name
  given:       Option<String>,
  /// Author's family (last) name
  family:      Option<String>,
  /// List of author's affiliations
  affiliation: Vec<CrossrefAffiliation>,
}

/// Institution affiliation information from Crossref.
#[derive(Debug, Deserialize)]
struct CrossrefAffiliation {
  /// Name of the affiliated institution
  name: Option<String>,
}

/// Date representation in Crossref's API.
#[derive(Debug, Deserialize)]
struct CrossrefDate {
  /// Date parts in the format [[year, month, day]]
  /// where month and day are optional
  #[serde(rename = "date-parts")]
  date_parts: Vec<Vec<i32>>,
}

/// Client for fetching paper metadata using DOIs via the Crossref API.
///
/// This client provides methods to resolve DOIs and fetch associated metadata
/// using Crossref's REST API. It handles authentication, request formatting,
/// and conversion of Crossref's rich metadata format to the common [`Paper`] structure.
///
/// The client follows Crossref's best practices including:
/// - Proper user agent identification
/// - Rate limiting consideration
/// - Fallback date handling
pub struct DOIClient {
  /// Internal web client used to connect to the API.
  client:   reqwest::Client,
  /// The base URL to use for the client.
  base_url: String,
}

impl DOIClient {
  /// Creates a new DOI client instance.
  ///
  /// Initializes an HTTP client with appropriate headers for Crossref API access.
  /// The client will identify itself to Crossref with a user agent string as
  /// required by their API terms of service.
  pub fn new() -> Self {
    Self {
      client:   reqwest::Client::builder()
                .user_agent("YourApp/1.0 (mailto:your@email.com)")  // Required by Crossref
                .build()
                .unwrap(),
      base_url: "https://api.crossref.org/works".to_string(),
    }
  }

  /// Parses a Crossref date structure into a DateTime.
  ///
  /// Handles Crossref's date-parts format which may include:
  /// - Full dates: [year, month, day]
  /// - Partial dates: [year, month] or [year]
  ///
  /// Returns None if the date cannot be parsed.
  fn parse_date(&self, date: &CrossrefDate) -> Option<DateTime<Utc>> {
    let parts = date.date_parts.first()?;
    debug!("Date parts: {:?}", parts);

    let year = *parts.first()?;
    let month = parts.get(1).copied().unwrap_or(1);
    let day = parts.get(2).copied().unwrap_or(1);

    debug!("Parsed year: {}, month: {}, day: {}", year, month, day);

    Utc.with_ymd_and_hms(year, month as u32, day as u32, 0, 0, 0).single()
  }

  /// Fetches paper metadata from Crossref using a DOI.
  ///
  /// # Arguments
  ///
  /// * `doi` - A Digital Object Identifier (e.g., "10.1145/1327452.1327492")
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
  /// - Required metadata fields are missing
  /// - No valid publication date can be determined
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::clients::DOIClient;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let client = DOIClient::new();
  /// let paper = client.fetch_paper("10.1145/1327452.1327492").await?;
  ///
  /// // Access metadata
  /// println!("Title: {}", paper.title);
  /// println!("Authors: {}", paper.authors.len());
  /// if let Some(url) = paper.pdf_url {
  ///   println!("Available at: {}", url);
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub async fn fetch_paper(&self, doi: &str) -> Result<Paper> {
    let url = format!("{}/{}", self.base_url, doi);
    debug!("Fetching from Crossref via: {}", url);

    let response = self.client.get(&url).send().await?;
    let status = response.status();
    debug!("Crossref response status: {}", status);

    let text = response.text().await?;
    debug!("Crossref response: {}", text);

    let response: CrossrefResponse = serde_json::from_str(&text)
      .map_err(|e| LearnerError::ApiError(format!("Failed to parse JSON: {}", e)))?;

    let work = response.message;

    debug!("Published print: {:?}", work.published_print);
    debug!("Published online: {:?}", work.published_online);
    debug!("Created: {:?}", work.created);

    // Get the first title or return an error
    let title =
      work.title.first().ok_or_else(|| LearnerError::ApiError("No title found".into()))?.clone();

    // Convert Crossref authors to our Author type
    let authors = work
      .author
      .into_iter()
      .map(|author| {
        let name = match (author.given, author.family) {
          (Some(given), Some(family)) => format!("{} {}", given, family),
          (Some(given), None) => given,
          (None, Some(family)) => family,
          (None, None) => "Unknown".to_string(),
        };

        let affiliation = author.affiliation.first().and_then(|aff| aff.name.clone());

        Author { name, affiliation, email: None }
      })
      .collect();

    // Try to get publication date, with multiple fallbacks
    let publication_date = work
      .published_print
      .as_ref()
      .and_then(|d| self.parse_date(d))
      .or_else(|| work.published_online.as_ref().and_then(|d| self.parse_date(d)))
      .or_else(|| work.created.as_ref().and_then(|d| self.parse_date(d)))
      .ok_or_else(|| {
        LearnerError::ApiError(format!(
          "No valid publication date found. Print: {:?}, Online: {:?}, Created: {:?}",
          work.published_print, work.published_online, work.created
        ))
      })?;

    Ok(Paper {
      title,
      authors,
      abstract_text: work.abstract_text.unwrap_or_default(),
      publication_date,
      source: Source::DOI,
      source_identifier: doi.to_string(),
      pdf_url: work.url,
      doi: Some(work.doi),
    })
  }
}

impl Default for DOIClient {
  fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
  use tracing_test::traced_test;

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_crossref_parse() -> anyhow::Result<()> {
    let doi = "10.1145/1327452.1327492";
    let client = DOIClient::new();
    let paper = client.fetch_paper(doi).await.unwrap();

    dbg!(&paper);

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::DOI);
    assert_eq!(paper.source_identifier, doi);

    Ok(())
  }
}
