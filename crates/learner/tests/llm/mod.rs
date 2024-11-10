use super::*;

#[ignore = "Can't run this in general -- relies on local LLM endpoint."]
#[tokio::test]
#[traced_test]
async fn test_download_then_send_pdf() -> Result<(), Box<dyn Error>> {
  // Download a PDF
  let dir = tempdir().unwrap();
  let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();
  paper.download_pdf(dir.path().to_path_buf()).await.unwrap();

  // Get the content of the PDF
  let formatted_title = format::format_title(&paper.title, None); // use default 50
  let path = dir.into_path().join(format!("{}.pdf", formatted_title));
  let pdf_content = PDFContentBuilder::new().path(path).analyze().unwrap();

  let mut message = "Please act like a researcher and digest this text from a PDF for me and give \
                     me an excellent summary. The summary can be long and descriptive. \n"
    .to_owned();

  message.push_str(&serde_json::to_string(&pdf_content.metadata).unwrap());
  message.push_str(&serde_json::to_string(&pdf_content.pages[0..5]).unwrap());

  let response =
    LlamaRequest::new().with_model(Model::Llama3p2c3b).with_message(&message).send().await?;
  dbg!(response.message);
  Ok(())
}
