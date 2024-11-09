use reqwest;
use serde_json::{self, json};

async fn send_request(
  prompt: &str,
  max_length: i32,
  top_k: i32,
  top_p: f64,
  temperature: f64,
) -> Result<String, reqwest::Error> {
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
  let text = response.text().await?;
  Ok(text)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pdf::{PDFContent, PDFContentBuilder};

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
      Ok(text) => println!("{}", text),
      Err(e) => println!("Error: {}", e),
    }
  }

  #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
  #[tokio::test]
  async fn test_send_pdf_summary_request() {
    let mut prompt = "Please act like a researcher and digest this text from a PDF for me and \
                      give me an excellent summary. I will send you it in a convenient form \
                      here.\n"
      .to_owned();
    let pdf_content = PDFContentBuilder::new().path("tests/data/test_paper.pdf").analyze().unwrap();
    prompt.push_str(&serde_json::to_string(&pdf_content).unwrap());
    let max_length = 1024;
    let top_k = 50;
    let top_p = 0.95;
    let temperature = 0.7; // Lowered temperature for more focused responses

    let result = send_request(&prompt, max_length, top_k, top_p, temperature).await;
    match result {
      Ok(text) => println!("{}", text),
      Err(e) => println!("Error: {}", e),
    }
  }
}
