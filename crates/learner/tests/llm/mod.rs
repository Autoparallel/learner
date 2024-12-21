// use learner::database::Add;

// use super::*;

// #[ignore = "Can't run this in general -- relies on local LLM endpoint."]
// #[tokio::test]
// #[traced_test]
// async fn test_download_then_send_pdf() -> Result<(), Box<dyn Error>> {
//   // Download a PDF
//   let (mut learner, _cfg_dir, _db_dir, _strg_dir) = create_test_learner().await;
//   let paper = learner.retrievers.get_paper("https://eprint.iacr.org/2016/260").await?;
//   // let paper = Paper::new("https://eprint.iacr.org/2016/260").await.unwrap();

//   // paper.download_pdf(dir.path()).await.unwrap();
//   Add::complete(&paper).execute(&mut learner.database).await?;

//   // Get the content of the PDF

//   let path = learner.database.get_storage_path().await?.join(paper.filename());
//   let pdf_content = PDFContentBuilder::new().path(path).analyze()?;

//   let mut message =
//     "Please act like a researcher and digest this text from a PDF for me and give  me an \
//      excellent summary. The summary can be long and descriptive. \n"
//       .to_owned();

//   message.push_str(&serde_json::to_string(&pdf_content.metadata).unwrap());
//   message.push_str(&serde_json::to_string(&pdf_content.pages[0..5]).unwrap());

//   let response =
//     LlamaRequest::new().with_model(Model::Llama3p2c3b).with_message(&message).send().await?;
//   dbg!(response.message);
//   Ok(())
// }
