// use resource::Resource;

use record::RetrievalData;
use resource::ResourceTemplate;

use super::*;

// TODO: fix all the stuff that had to do with `Retriever.name`

#[derive(Debug, Clone, Deserialize)]
pub struct Retriever {
  pub resource:       ResourceTemplate,
  #[serde(skip_deserializing)]
  #[serde(default)]
  pub retrieval_data: RetrievalData,

  // TODO: Should own a `Record`
  /// Base URL for API requests
  pub base_url:           String,
  /// Regex pattern for matching and extracting paper identifiers
  #[serde(deserialize_with = "deserialize_regex")]
  pub pattern:            Regex,
  /// Source identifier for papers from this retriever
  pub source:             String,
  /// Template for constructing API endpoint URLs
  pub endpoint_template:  String,
  // TODO: This is now more like "how to get the thing to map into the resource"
  // #[serde(flatten)]
  pub response_format:    ResponseFormat,
  /// Optional HTTP headers for API requests
  #[serde(default)]
  pub headers:            BTreeMap<String, String>,
  // TODO: need to have these be associated somehow, actually resource should probably be in record
  #[serde(default)]
  pub resource_mappings:  BTreeMap<String, FieldMap>,
  #[serde(default)]
  pub retrieval_mappings: BTreeMap<String, FieldMap>,
}

// impl Identifiable for Retriever {
//   fn name(&self) -> String { self.name.clone() }
// }

impl Retriever {
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

  // TODO: perhaps this just isn't even implemented here and is instead implemented on `Learner`.
  // Could consider an `api.rs` module to extend more learner functionality there.
  #[allow(missing_docs)]
  pub async fn retrieve_resource(&self, input: &str) -> Result<Resource> {
    let identifier = self.extract_identifier(input)?;

    // Send request and get response
    let url = self.endpoint_template.replace("{identifier}", identifier);
    // debug!("Fetching from {} via: {}", self.name, url);

    let client = reqwest::Client::new();
    let mut request = client.get(&url);

    // Add any configured headers
    for (key, value) in &self.headers {
      request = request.header(key, value);
    }

    let response = request.send().await?;
    let data = response.bytes().await?;

    // trace!("{} response: {}", self.name, String::from_utf8_lossy(&data));

    // Process the response using configured processor
    let json = match &self.response_format {
      ResponseFormat::Xml { strip_namespaces } => xml::convert_to_json(&data, *strip_namespaces),
      ResponseFormat::Json => serde_json::from_slice(&data)?,
    };

    // Process response and get resource
    // TODO: this should probably be a method
    let mut resource = process_json_value(&json, &self.resource_mappings, &self.resource)?;

    // Add source metadata
    resource.insert("source".into(), Value::String(self.source.clone()));
    resource.insert("source_identifier".into(), Value::String(identifier.to_string()));

    // Validate full resource against config
    self.resource.validate(&resource)?;
    Ok(resource)

    // todo!()

    // Ok(Record {
    //   resource,
    //   resource_config: resource_config.clone(),
    //   retrieval: None,
    //   state: ResourceState::default(),
    //   storage: None,
    //   tags: Vec::new(),
    // })
  }
}
