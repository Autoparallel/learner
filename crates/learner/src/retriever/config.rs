use resource::Resource;

use super::*;

/// Configuration for a specific paper source retriever.
///
/// This struct defines how to interact with a particular paper source's API,
/// including URL patterns, authentication, and response parsing rules.
///
/// # Examples
///
/// Example TOML configuration:
///
/// ```toml
/// name = "arxiv"
/// base_url = "http://export.arxiv.org/api/query"
/// pattern = "^\\d{4}\\.\\d{4,5}$"
/// source = "arxiv"
/// endpoint_template = "http://export.arxiv.org/api/query?id_list={identifier}"
///
/// [response_format]
/// type = "xml"
/// strip_namespaces = true
///
/// [response_format.field_maps]
/// title = { path = "entry/title" }
/// abstract = { path = "entry/summary" }
/// publication_date = { path = "entry/published" }
/// authors = { path = "entry/author/name" }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct RetrieverConfig {
  /// Name of this retriever configuration
  pub name:              String,
  // TODO (autoparallel): Ultimately this will have to peer into the `Resources` to be useful
  /// The type of resource this retriever should yield
  pub resource:          String,
  /// Base URL for API requests
  pub base_url:          String,
  /// Regex pattern for matching and extracting paper identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:           Regex,
  /// Source identifier for papers from this retriever
  pub source:            String,
  /// Template for constructing API endpoint URLs
  pub endpoint_template: String,
  /// Format and parsing configuration for API responses
  pub response_format:   ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:           HashMap<String, String>,
}

impl Identifiable for RetrieverConfig {
  fn name(&self) -> String { self.name.clone() }
}

impl RetrieverConfig {
  /// Extracts the canonical identifier from an input string.
  ///
  /// Uses the configured regex pattern to extract the standardized
  /// identifier from various input formats (URLs, DOIs, etc.).
  ///
  /// # Arguments
  ///
  /// * `input` - Input string containing a paper identifier
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The extracted identifier as a string slice
  /// - A LearnerError if the input doesn't match the pattern
  pub fn extract_identifier<'a>(&self, input: &'a str) -> Result<&'a str> {
    self
      .pattern
      .captures(input)
      .and_then(|cap| cap.get(1))
      .map(|m| m.as_str())
      .ok_or(LearnerError::InvalidIdentifier)
  }

  /// Retrieves a paper using this configuration.
  ///
  /// This method:
  /// 1. Extracts the canonical identifier
  /// 2. Constructs the API URL
  /// 3. Makes the HTTP request
  /// 4. Processes the response
  ///
  /// # Arguments
  ///
  /// * `input` - Paper identifier or URL
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - The retrieved Paper object
  /// - A LearnerError if any step fails
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - The identifier cannot be extracted
  /// - The HTTP request fails
  /// - The response cannot be parsed
  pub async fn retrieve_paper(&self, input: &str) -> Result<Paper> {
    let identifier = self.extract_identifier(input)?;
    let url = self.endpoint_template.replace("{identifier}", identifier);

    debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;

    trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    let response_processor = match &self.response_format {
      ResponseFormat::Xml(config) => config as &dyn ResponseProcessor,
      ResponseFormat::Json(config) => config as &dyn ResponseProcessor,
    };
    let mut paper = response_processor.process_response(&data).await?;
    paper.source = self.source.clone();
    paper.source_identifier = identifier.to_string();
    Ok(paper)
  }

  // TODO: perhaps this just isn't even implemented here and is instead implemented on `Learner`.
  // Could consider an `api.rs` module to extend more learner functionality there.
  #[allow(missing_docs)]
  pub async fn retrieve_resource(
    &self,
    input: &str,
    resource_config: &ResourceConfig,
  ) -> Result<Resource> {
    let identifier = self.extract_identifier(input)?;

    // Send request and get response
    let url = self.endpoint_template.replace("{identifier}", identifier);
    debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;
    trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    // Process the response into a generic Value first
    let _response_processor = match &self.response_format {
      ResponseFormat::Xml(config) => config as &dyn ResponseProcessor,
      ResponseFormat::Json(config) => config as &dyn ResponseProcessor,
    };

    todo!();
  }
}
