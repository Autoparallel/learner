//! Client implementation for interacting with Ollama LLMs.
//!
//! This module provides functionality to interact with locally running Ollama LLMs,
//! particularly focused on the Llama model family. It supports various API endpoints
//! including chat, generation, and embeddings, with primary support for chat-based
//! interactions.
//!
//! The client handles URL management, request building, and response parsing while
//! providing sensible defaults and helpful warnings when using fallback configurations.
//!
//! # Examples
//!
//! ```no_run
//! use learner::llm::{LlamaRequest, Model, OllamaEndpoint};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let request = LlamaRequest::new()
//!   .with_host("http://localhost:11434")
//!   .with_endpoint(OllamaEndpoint::Chat)
//!   .with_model(Model::Llama3p2c3b)
//!   .with_message("What is quantum computing?");
//!
//! let response = request.send().await?;
//! println!("Response: {}", response.message.content);
//! # Ok(())
//! # }
//! ```

use super::*;

/// Available API endpoints for the Ollama service.
///
/// Each variant represents a different API endpoint with specific functionality.
/// Currently, primary support is for the Chat endpoint, with others marked as
/// experimental through runtime warnings.
#[derive(Debug, Clone)]
pub enum OllamaEndpoint {
  /// Chat completion endpoint for conversation-style interactions
  Chat,
  /// Raw text generation endpoint
  Generate,
  /// Vector embedding creation endpoint
  Embed,
  /// Model pulling/downloading endpoint
  Pull,
  /// Model pushing/uploading endpoint
  Push,
  /// Model creation endpoint
  Create,
  /// Model copying endpoint
  Copy,
  /// Model deletion endpoint
  Delete,
  /// Model information endpoint
  Show,
  /// List running models endpoint
  ListRunning,
  /// List available local models endpoint
  ListLocal,
}

impl OllamaEndpoint {
  /// Converts the endpoint variant to its URL path string.
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Chat => "/api/chat",
      Self::Generate => "/api/generate",
      Self::Embed => "/api/embed",
      Self::Pull => "/api/pull",
      Self::Push => "/api/push",
      Self::Create => "/api/create",
      Self::Copy => "/api/copy",
      Self::Delete => "/api/delete",
      Self::Show => "/api/show",
      Self::ListRunning => "/api/ps",
      Self::ListLocal => "/api/tags",
    }
  }
}

/// Available models for use with the Ollama service.
///
/// Currently supports the Llama 3.2 3B model, with plans to expand
/// to support additional models in the future.
#[derive(Serialize)]
pub enum Model {
  /// Llama 3.2 3B model variant
  #[serde(rename = "llama3.2:3b")]
  Llama3p2c3b,
}

impl Display for Model {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Model::Llama3p2c3b => write!(f, "llama3.2:3b"),
    }
  }
}

// TODO (autoparallel): We could make an API like this very nice by having it be a typestate for the
// type of request you're doing so that only the relevant methods appear on a given type.
/// Request builder for Ollama LLM interactions.
///
/// Provides a fluent interface for constructing requests to the Ollama service,
/// handling URL management, message construction, and model configuration.
///
/// # Examples
///
/// ```no_run
/// # use learner::llm::{LlamaRequest, Model, OllamaEndpoint};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request =
///   LlamaRequest::new().with_model(Model::Llama3p2c3b).with_message("Explain how a computer works");
///
/// let response = request.send().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Default)]
pub struct LlamaRequest {
  /// The LLM model to use for generation. If not specified, will result in
  /// an error when sending the request.
  pub model: Option<Model>,

  /// Vector of conversation messages. Must contain at least one message
  /// before sending the request. Messages are processed in order to maintain
  /// conversation context.
  pub messages: Vec<Message>,

  /// Whether to stream the response. Currently not implemented, but when true
  /// will enable token-by-token streaming of the model's response.
  pub stream: bool,

  /// Generation parameters including temperature, top-k, top-p, and maximum
  /// token count. Uses sensible defaults if not explicitly configured.
  pub options: Options,

  /// The target URL for the request. If not specified, defaults to
  /// localhost:11434 with a warning. Skipped during serialization.
  #[serde(skip)]
  pub url: Option<Url>,
}

/// Message structure for LLM interactions.
///
/// Represents a single message in the conversation, containing
/// both the role of the speaker and the content of the message.
///
/// # Examples
///
/// ```
/// use learner::llm::Message;
///
/// let message =
///   Message { role: "user".to_string(), content: "What is the speed of light?".to_string() };
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
  /// The role of the message sender. Typically "user" for
  /// input messages and "assistant" for model responses.
  pub role: String,

  /// The actual content of the message. For user messages,
  /// this is the prompt or question. For assistant messages,
  /// this contains the model's response.
  pub content: String,
}

/// Configuration options for LLM inference.
///
/// Controls various aspects of the model's generation behavior
/// including response length and sampling parameters.
#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
  /// Maximum number of tokens to generate
  num_predict: u64,
  /// Top-k sampling parameter
  top_k:       u64,
  /// Top-p (nucleus) sampling parameter
  top_p:       f64,
  /// Temperature for controlling randomness in generation
  temperature: f64,
}

// NOTE (autoparallel): Chosen somewhat arbitrarily.
impl Default for Options {
  fn default() -> Self { Self { num_predict: 16384, top_k: 50, top_p: 0.95, temperature: 0.7 } }
}

/// Response structure from Ollama LLM requests.
///
/// Contains the generated content along with metadata about
/// the generation process including timing information.
#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaResponse {
  /// Name of the model used
  pub model:                String,
  /// Timestamp of response creation
  pub created_at:           String,
  /// Generated message content
  pub message:              Message,
  /// Reason for completion
  pub done_reason:          String,
  /// Whether generation is complete
  pub done:                 bool,
  /// Total processing time in microseconds
  pub total_duration:       u64,
  /// Model loading time in microseconds
  pub load_duration:        u64,
  /// Number of tokens in the prompt
  pub prompt_eval_count:    u64,
  /// Time spent evaluating prompt in microseconds
  pub prompt_eval_duration: u64,
  /// Number of generated tokens
  pub eval_count:           u64,
  /// Time spent generating tokens in microseconds
  pub eval_duration:        u64,
}

impl LlamaRequest {
  /// Creates a new request with builder-style API with default settings.
  pub fn new() -> Self { Self::default() }

  /// Sets the host URL for the request.
  ///
  /// # Arguments
  ///
  /// * `host` - Base URL for the Ollama service
  pub fn with_host(mut self, host: &str) -> Self {
    self.url = Url::parse(host).ok();
    self
  }

  /// Sets the API endpoint for the request.
  ///
  /// # Arguments
  ///
  /// * `endpoint` - The API endpoint to use
  ///
  /// Note: Currently only the Chat endpoint is fully supported.
  /// Other endpoints will generate warnings about potential limitations.
  pub fn with_endpoint(mut self, endpoint: OllamaEndpoint) -> Self {
    if !matches!(endpoint, OllamaEndpoint::Chat) {
      warn!("Endpoint {:?} is not fully supported yet", endpoint);
    }

    let base = self.url.take().unwrap_or_else(|| {
      warn!("No host set, using localhost");
      Url::parse("http://localhost:11434").unwrap()
    });

    self.url = Some(base.join(endpoint.as_str().trim_start_matches('/')).unwrap_or_else(|_| {
      warn!("Failed to set endpoint, using /api/chat");
      base.join("/api/chat").unwrap()
    }));
    self
  }

  /// Sets the model to use for the request.
  ///
  /// # Arguments
  ///
  /// * `model` - The LLM model to use
  pub fn with_model(mut self, model: Model) -> Self {
    self.model.replace(model);
    self
  }

  /// Adds a message to the conversation.
  ///
  /// # Arguments
  ///
  /// * `content` - The message content
  pub fn with_message(mut self, content: &str) -> Self {
    self.messages.push(Message { role: "user".to_string(), content: content.to_string() });
    self
  }

  /// Sends the request to the Ollama service.
  ///
  /// # Returns
  ///
  /// Returns a Result containing either:
  /// - A `LlamaResponse` with the model's response
  /// - A `LearnerError` if the request fails
  ///
  /// # Errors
  ///
  /// This function will return an error if:
  /// - No model is specified
  /// - No messages are provided
  /// - The network request fails
  /// - The response cannot be parsed
  pub async fn send(&self) -> Result<LlamaResponse, LearnerError> {
    let url = self.url.clone().unwrap_or_else(|| {
      warn!("No URL set, using localhost/chat");
      Url::parse("http://localhost:11434/api/chat").unwrap()
    });

    if self.model.is_none() {
      return Err(LearnerError::LLMMissingModel);
    }

    if self.messages.is_empty() {
      return Err(LearnerError::LLMMissingMessage);
    }

    let client = reqwest::Client::new();
    let response = client.post(url).json(&self).send().await?;
    let llama_response: LlamaResponse = response.json().await?;
    Ok(llama_response)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
  #[tokio::test]
  async fn test_send_request() {
    let host = "http://localhost:11434/";
    let content = "Please tell me what is the capital of France?";
    let request = LlamaRequest::new()
      .with_host(host)
      .with_endpoint(OllamaEndpoint::Chat)
      .with_model(Model::Llama3p2c3b)
      .with_message(content);

    let response = request.send().await.unwrap();
    dbg!(&response);
    assert!(response.message.content.contains("Paris"))
  }

  #[traced_test]
  #[test]
  fn test_warnings() {
    let request = LlamaRequest::new().with_endpoint(OllamaEndpoint::Chat);
    request.with_endpoint(OllamaEndpoint::Create);
    assert!(logs_contain("No host set"));
    assert!(logs_contain("Endpoint Create"));
  }
}
