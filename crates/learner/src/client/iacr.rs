//! Client implementation for fetching papers from the IACR Cryptology ePrint Archive.
//!
//! This module provides functionality to fetch papers from the International Association
//! for Cryptologic Research (IACR) ePrint Archive using their OAI-PMH interface. It handles
//! the conversion of Dublin Core metadata into the common [`Paper`] structure.
//!
//! The client uses IACR's OAI-PMH endpoint (https://eprint.iacr.org/oai) which provides
//! standardized access to the ePrint archive.
//!
//! # Examples
//!
//! ```no_run
//! use learner::clients::IACRClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = IACRClient::new();
//! let paper = client.fetch_paper("2023/123").await?;
//!
//! println!("Title: {}", paper.title);
//! println!("Authors: {}", paper.authors.len());
//! # Ok(())
//! # }
//! ```

use super::*;

/// Root response structure for the OAI-PMH protocol.
#[derive(Debug, Deserialize)]
#[serde(rename = "OAI-PMH")]
struct OAIPMHResponse {
  /// The requested record, if found
  #[serde(rename = "GetRecord")]
  get_record: Option<GetRecord>,
  /// Error details, if the request failed
  error:      Option<OAIError>,
}

/// Error information from the OAI-PMH response.
#[derive(Debug, Deserialize)]
struct OAIError {
  /// Standard OAI-PMH error code
  #[serde(rename = "@code")]
  code:    String,
  /// Human-readable error message
  #[serde(rename = "$text")]
  message: String,
}

/// Container for a single record in the OAI-PMH response.
#[derive(Debug, Deserialize)]
struct GetRecord {
  /// The actual record data
  record: Record,
}

/// Metadata record container.
#[derive(Debug, Deserialize)]
struct Record {
  /// The metadata in Dublin Core format
  metadata: Metadata,
}

/// Container for Dublin Core metadata.
#[derive(Debug, Deserialize)]
struct Metadata {
  /// The Dublin Core elements
  #[serde(rename = "dc")]
  dublin_core: DublinCore,
}

/// Dublin Core metadata elements for a paper.
///
/// This follows the Dublin Core Metadata Element Set, Version 1.1,
/// but only includes the elements used by IACR's ePrint archive.
#[derive(Debug, Deserialize)]
struct DublinCore {
  /// Paper title
  #[serde(rename = "title")]
  title:       String,
  /// List of author names
  #[serde(rename = "creator")]
  creators:    Vec<String>,
  /// Paper abstract
  #[serde(rename = "description")]
  description: String,
  /// Associated dates (typically submission/last update)
  #[serde(rename = "date")]
  dates:       Vec<String>,
  /// Various identifiers (URLs, DOIs, etc.)
  #[serde(rename = "identifier")]
  identifiers: Vec<String>,
}

/// Client for fetching papers from the IACR Cryptology ePrint Archive.
///
/// This client provides methods to fetch paper metadata from IACR using their
/// OAI-PMH interface. It handles XML parsing, namespace management, and conversion
/// of Dublin Core metadata to the common [`Paper`] format.
///
/// Papers in the IACR ePrint Archive are identified by a year and number in the
/// format "YYYY/NNNN".
pub struct IACRClient {
  /// Internal web client used to connect to the API.
  client:   reqwest::Client,
  /// The base URL to use for the client.
  base_url: String,
}

impl IACRClient {
  /// Creates a new IACR client instance.
  ///
  /// Initializes an HTTP client for making requests to IACR's OAI-PMH endpoint.
  pub fn new() -> Self {
    Self { client: reqwest::Client::new(), base_url: "https://eprint.iacr.org/oai".to_string() }
  }

  /// Fetches paper metadata from IACR using its identifier.
  ///
  /// # Arguments
  ///
  /// * `identifier` - An IACR paper identifier in the format "YYYY/NNNN" where YYYY is the
  ///   submission year and NNNN is the paper number (e.g., "2023/123")
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
  /// - The identifier format is invalid
  /// - The network request fails
  /// - The XML response cannot be parsed
  /// - The OAI-PMH response contains an error
  /// - Required metadata fields are missing
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::clients::IACRClient;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let client = IACRClient::new();
  /// let paper = client.fetch_paper("2023/123").await?;
  ///
  /// // Access metadata
  /// println!("Title: {}", paper.title);
  /// if let Some(pdf_url) = paper.pdf_url {
  ///   println!("PDF available at: {}", pdf_url);
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub async fn fetch_paper(&self, identifier: &str) -> Result<Paper> {
    // IACR identifiers are in the format "YYYY/NNNN"
    let parts: Vec<&str> = identifier.split('/').collect();
    if parts.len() != 2 {
      return Err(LearnerError::InvalidIdentifier);
    }

    let url = format!(
      "{}?verb=GetRecord&identifier=oai:eprint.iacr.org:{}&metadataPrefix=oai_dc",
      self.base_url, identifier
    );

    debug!("Fetching from IACR via OAI-PMH: {url}");

    let response = self.client.get(&url).send().await?;

    let text = response.text().await?;
    debug!("IACR OAI-PMH response: {}", text);

    // Clean up the XML to handle namespaces
    let text = text
            .replace("xmlns:oai_dc=\"http://www.openarchives.org/OAI/2.0/oai_dc/\"", "")
            .replace("xmlns:dc=\"http://purl.org/dc/elements/1.1/\"", "")
            .replace("xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"", "")
            .replace("xsi:schemaLocation=\"http://www.openarchives.org/OAI/2.0/oai_dc/ http://www.openarchives.org/OAI/2.0/oai_dc.xsd\"", "")
            .replace("oai_dc:", "")
            .replace("dc:", "");

    debug!("Cleaned XML: {}", text);

    let oai_response: OAIPMHResponse =
      from_str(&text).map_err(|e| LearnerError::ApiError(format!("Failed to parse XML: {}", e)))?;

    if let Some(error) = oai_response.error {
      return Err(LearnerError::ApiError(format!(
        "OAI-PMH error: {} - {}",
        error.code, error.message
      )));
    }

    let record = oai_response
      .get_record
      .ok_or_else(|| LearnerError::ApiError("No record found".to_string()))?
      .record;

    let dc = record.metadata.dublin_core;

    // Try to find a URL-style identifier starting with https://eprint.iacr.org/
    let doi = dc.identifiers.iter().find(|id| id.starts_with("https://eprint.iacr.org/")).cloned();

    // Parse the earliest date (creation date)
    let publication_date = dc
      .dates
      .first()
      .and_then(|date_str| DateTime::parse_from_rfc3339(date_str).ok())
      .map(|dt| dt.with_timezone(&Utc))
      .ok_or_else(|| LearnerError::ApiError("Invalid date format".to_string()))?;

    Ok(Paper {
      title: dc.title,
      authors: dc
        .creators
        .into_iter()
        .map(|name| Author { name, affiliation: None, email: None })
        .collect(),
      abstract_text: dc.description,
      publication_date,
      source: Source::IACR,
      source_identifier: identifier.to_string(),
      pdf_url: Some(format!("https://eprint.iacr.org/{}/{}.pdf", parts[0], parts[1])),
      doi,
    })
  }
}

impl Default for IACRClient {
  fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
  use tracing_test::traced_test;

  use super::*;

  #[traced_test]
  #[tokio::test]
  async fn test_iacr_entry_fetch() {
    let client = IACRClient::new();
    let paper = client.fetch_paper("2016/260").await.unwrap();

    dbg!(&paper);

    assert!(!paper.title.is_empty());
    assert!(!paper.authors.is_empty());
    assert_eq!(paper.source, Source::IACR);
    assert_eq!(paper.source_identifier, "2016/260");
  }
}
