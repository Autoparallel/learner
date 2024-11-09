use serde_json::{self, json};
use tiktoken_rs::cl100k_base;

use super::*;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
  pub role:    String,
  pub content: String,
}

pub async fn send_request(
  prompt: &str,
  max_length: i32,
  top_k: i32,
  top_p: f64,
  temperature: f64,
) -> Result<LlamaResponse, LearnerError> {
  let url = "http://localhost:11434/api/chat";
  let payload = json!({
      "model": "llama3.2:3b",
      "messages": [
          {
              "role": "user",
              "content": prompt
          }
      ],
      "stream": false,
      "options": {
          "num_predict": max_length,
          "top_k": top_k,
          "top_p": top_p,
          "temperature": temperature
      }
  });

  let response = reqwest::Client::new().post(url).json(&payload).send().await?;
  let llama_response: LlamaResponse = response.json().await?;
  Ok(llama_response)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pdf::PDFContentBuilder;

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
