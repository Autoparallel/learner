// use std::{env::temp_dir, error::Error};

// use learner::{format, llm::send_single_request, paper::Paper, pdf::PDFContentBuilder};
// use tempfile::tempdir;

// #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
// #[tokio::test]
// async fn test_send_pdf_summary_request() -> Result<(), Box<dyn Error>> {
//   let mut prompt = "Please act like a researcher and digest this text from a PDF for me and give
// \                     me an excellent summary. I will send you it in a convenient form here.\n"
//     .to_owned();
//   let pdf_content =
// PDFContentBuilder::new().path("tests/data/test_paper.pdf").analyze().unwrap();
//   prompt.push_str(&serde_json::to_string(&pdf_content).unwrap());
//   let max_length = 1024;
//   let top_k = 50;
//   let top_p = 0.95;
//   let temperature = 0.7; // Lowered temperature for more focused responses

//   let llama_response = send_request(&prompt, max_length, top_k, top_p, temperature).await?;
//   dbg!(llama_response.message);
//   Ok(())
// }

// #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
// #[tokio::test]
// async fn test_download_then_send_pdf() -> Result<(), Box<dyn Error>> {
//   let mut prompt = "Please act like a researcher and digest this text from a PDF for me and give
// \                     me an excellent summary. The summary can be long and descriptive. \n"
//     .to_owned();
//   let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
//   let dir = tempdir().unwrap();
//   paper.download_pdf(dir.path().to_path_buf()).await.unwrap();
//   let formatted_title = format::format_title(&paper.title, None); // use default 50
//   let path = dir.into_path().join(format!("{}.pdf", formatted_title));
//   let pdf_content = PDFContentBuilder::new().path(path).analyze().unwrap();
//   dbg!(&pdf_content);
//   prompt.push_str(&serde_json::to_string(&pdf_content.metadata).unwrap());
//   prompt.push_str(&serde_json::to_string(&pdf_content.pages[0..5]).unwrap());
//   let max_length = 128000;
//   let top_k = 50;
//   let top_p = 0.95;
//   let temperature = 0.7; // Lowered temperature for more focused responses

//   let llama_response = send_request(&prompt, max_length, top_k, top_p, temperature).await?;
//   dbg!(llama_response.message);
//   Ok(())
// }
