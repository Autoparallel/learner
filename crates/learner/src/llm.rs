use std::error::Error;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tiktoken_rs::{cl100k_base, CoreBPE};
use url::Url;

use super::*;

pub struct TokenCounter {
  bpe:            CoreBPE,
  context_window: usize,
}

impl Default for TokenCounter {
  fn default() -> Self { Self::new(1024).unwrap() }
}

impl TokenCounter {
  // TODO: this returning a result is stupid
  pub fn new(context_window: usize) -> Result<Self, Box<dyn Error>> {
    Ok(Self { bpe: cl100k_base()?, context_window })
  }

  pub fn count_tokens(&self, text: &str) -> usize {
    self.bpe.encode_with_special_tokens(text).len()
  }

  pub fn get_max_completion_tokens(&self, prompt: &str, buffer: usize) -> usize {
    let prompt_tokens = self.count_tokens(prompt);
    self.context_window.saturating_sub(prompt_tokens).saturating_sub(buffer)
  }
}

#[derive(Debug)]
pub enum ProcessingMode {
  Single,
  Chunked { max_completion_tokens: usize, buffer_tokens: usize },
}

impl Default for ProcessingMode {
  fn default() -> Self { ProcessingMode::Single }
}

#[derive(Serialize)]
pub struct LlamaRequestBuilder {
  model:           Option<String>,
  messages:        Option<Vec<Message>>,
  stream:          bool,
  options:         Options,
  #[serde(skip)]
  url:             Option<String>,
  #[serde(skip)]
  processing_mode: ProcessingMode,
  #[serde(skip)]
  token_counter:   TokenCounter,
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

impl Default for Options {
  fn default() -> Self { Self { num_predict: 1024, top_k: 50, top_p: 0.95, temperature: 0.7 } }
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

impl LlamaRequestBuilder {
  pub fn new() -> Self {
    Self {
      model:           None,
      messages:        None,
      stream:          false,
      options:         Default::default(),
      url:             None,
      token_counter:   Default::default(),
      processing_mode: Default::default(),
    }
  }

  async fn send_single_request(
    &self,
    prompt: &str,
    max_tokens: usize,
  ) -> Result<LlamaResponse, LearnerError> {
    // let url = "http://localhost:11434/api/chat";
    // let payload = json!({
    //     "model": "llama2:3b",
    //     "messages": [
    //         {
    //             "role": "user",
    //             "content": prompt
    //         }
    //     ],
    //     "stream": false,
    //     "options": {
    //         "num_predict": max_tokens as i32,
    //         "top_k": self.top_k,
    //         "top_p": self.top_p,
    //         "temperature": self.temperature
    //     }
    // });

    let client = reqwest::Client::new();
    // TODO: this unwrap won't fail if we check the shit outside of this function
    let response = client.post(self.url.as_ref().unwrap()).json(&json!(&self)).send().await?;
    let llama_response: LlamaResponse = response.json().await?;
    Ok(llama_response)
  }

  pub async fn process(
    &self,
    content: &str,
    system_prompt: &str,
  ) -> Result<Vec<LlamaResponse>, LearnerError> {
    // TODO: check that the necessary fields are filled here and return error otherwise.

    match &self.processing_mode {
      ProcessingMode::Single => {
        let full_prompt = format!("{}\n{}", system_prompt, content);
        let max_tokens = self.token_counter.get_max_completion_tokens(&full_prompt, 100);

        if max_tokens == 0 {
          return Err(LearnerError::LLMContentTooLong);
        }

        let response = self.send_single_request(&full_prompt, max_tokens).await?;
        Ok(vec![response])
      },

      ProcessingMode::Chunked { max_completion_tokens, buffer_tokens } => {
        let base_tokens = self.token_counter.count_tokens(system_prompt);
        let available_for_content =
          self.token_counter.context_window - base_tokens - max_completion_tokens - buffer_tokens;

        // Split content into chunks
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;

        // Simple splitting strategy - could be improved based on your needs
        for line in content.lines() {
          let line_content = format!("{}\n", line);
          let line_tokens = self.token_counter.count_tokens(&line_content);

          if current_tokens + line_tokens > available_for_content && !current_chunk.is_empty() {
            chunks.push(current_chunk);
            current_chunk = String::new();
            current_tokens = 0;
          }

          current_chunk.push_str(&line_content);
          current_tokens += line_tokens;
        }

        if !current_chunk.is_empty() {
          chunks.push(current_chunk);
        }

        // Process each chunk
        let mut responses = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
          let chunk_prompt =
            format!("{} [Part {}/{}]\n{}", system_prompt, i + 1, chunks.len(), chunk);

          let response = self.send_single_request(&chunk_prompt, *max_completion_tokens).await?;

          responses.push(response);
        }

        Ok(responses)
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
  #[tokio::test]
  async fn test_send_request() {
    let prompt = "Please tell me what is the capital of France?";
    let max_length = 1024;
    let top_k = 50;
    let top_p = 0.95;
    let temperature = 0.7; // Lowered temperature for more focused responses

    let result = send_request(prompt, max_length, top_k, top_p, temperature).await;
    match result {
      Ok(text) => println!("{:?}", text),
      Err(e) => println!("Error: {}", e),
    }
  }
}
