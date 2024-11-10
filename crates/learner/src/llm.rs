use std::fmt::Display;

use tracing::warn;
use url::Url;

use super::*;

#[derive(Debug, Clone)]
pub enum OllamaEndpoint {
  Chat,
  Generate,
  Embed,
  Pull,
  Push,
  Create,
  Copy,
  Delete,
  Show,
  ListRunning,
  ListLocal,
}

impl OllamaEndpoint {
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

#[derive(Serialize)]
pub enum Model {
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
#[derive(Serialize, Default)]
pub struct LlamaRequest {
  model:    Option<Model>,
  messages: Vec<Message>,
  stream:   bool,
  options:  Options,
  #[serde(skip)]
  url:      Option<Url>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
  pub role:    String,
  pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
  num_predict: u64,
  top_k:       u64,
  top_p:       f64,
  temperature: f64,
}

// NOTE (autoparallel): Chosen somewhat arbitrarily.
impl Default for Options {
  fn default() -> Self { Self { num_predict: 16384, top_k: 50, top_p: 0.95, temperature: 0.7 } }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaResponse {
  pub model:                String,
  pub created_at:           String,
  pub message:              Message,
  pub done_reason:          String,
  pub done:                 bool,
  pub total_duration:       u64,
  pub load_duration:        u64,
  pub prompt_eval_count:    u64,
  pub prompt_eval_duration: u64,
  pub eval_count:           u64,
  pub eval_duration:        u64,
}

impl LlamaRequest {
  pub fn new() -> Self { Self::default() }

  pub fn with_host(mut self, host: &str) -> Self {
    self.url = Url::parse(host).ok();
    self
  }

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

  pub fn with_model(mut self, model: Model) -> Self {
    self.model.replace(model);
    self
  }

  pub fn with_message(mut self, content: &str) -> Self {
    self.messages.push(Message { role: "user".to_string(), content: content.to_string() });
    self
  }

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
