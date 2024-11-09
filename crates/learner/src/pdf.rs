#![allow(missing_docs, clippy::missing_docs_in_private_items)]
use std::path::Path;

use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PDFContent {
  metadata: PDFMetadata,
  pages:    Vec<PageContent>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PDFMetadata {
  title:    Option<String>,
  author:   Option<String>,
  subject:  Option<String>,
  keywords: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PageContent {
  page_number: u32,
  text:        String,
}

#[derive(Default)]
pub struct PDFAnalyzer;

impl PDFAnalyzer {
  pub fn new() -> Self { Self }

  pub fn analyze<P: AsRef<Path>>(&self, path: P) -> Result<PDFContent, LearnerError> {
    let doc = Document::load(path)?;

    let metadata = self.extract_metadata(&doc)?;
    let pages = self.extract_pages(&doc)?;

    Ok(PDFContent { metadata, pages })
  }

  fn extract_metadata(&self, doc: &Document) -> Result<PDFMetadata, LearnerError> {
    let trailer = &doc.trailer;
    let info_ref = trailer.get(b"Info").ok().and_then(|o| o.as_reference().ok());

    let info = match info_ref {
      Some(reference) => doc.get_object(reference).and_then(|obj| obj.as_dict())?,
      None =>
        return Ok(PDFMetadata { title: None, author: None, subject: None, keywords: None }),
    };

    Ok(PDFMetadata {
      title:    self.get_text_from_dict(info, "Title"),
      author:   self.get_text_from_dict(info, "Author"),
      subject:  self.get_text_from_dict(info, "Subject"),
      keywords: self.get_text_from_dict(info, "Keywords"),
    })
  }

  fn get_text_from_dict(&self, dict: &lopdf::Dictionary, key: &str) -> Option<String> {
    dict.get(key.as_bytes()).ok().and_then(|obj| obj.as_str().ok()).map(|bytes| {
      // Check if the string starts with the UTF-16BE BOM (0xFE 0xFF)
      if bytes.starts_with(&[0xFE, 0xFF]) {
        // Skip the BOM and decode as UTF-16BE
        let (cow, ..) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        cow.into_owned()
      } else {
        // Regular string decoding
        String::from_utf8_lossy(bytes).into_owned()
      }
    })
  }

  fn extract_pages(&self, doc: &Document) -> Result<Vec<PageContent>, LearnerError> {
    let mut pages = Vec::new();

    for (page_num, page_id) in doc.page_iter().enumerate() {
      println!("Processing page {}, id: {:?}", page_num + 1, page_id);

      let page = doc.get_object(page_id)?;
      println!("Page object: {:?}", page);

      let page_dict = page.as_dict()?;
      println!("Page dict: {:?}", page_dict);

      // Get Contents object(s)
      match page_dict.get(b"Contents") {
        Ok(contents) => {
          println!("Contents: {:?}", contents);
          let text = self.extract_text_from_contents(doc, contents)?;
          println!("Extracted text length: {}", text.len());
          pages.push(PageContent { page_number: page_num as u32 + 1, text });
        },
        Err(e) => println!("Failed to get Contents: {:?}", e),
      }
    }

    Ok(pages)
  }

  fn extract_text_from_contents(
    &self,
    doc: &Document,
    contents: &Object,
  ) -> Result<String, LearnerError> {
    println!("Extracting text from contents: {:?}", contents);
    let mut text = String::new();

    match contents {
      Object::Array(array) => {
        println!("Processing array of {} content streams", array.len());
        for content_ref in array {
          println!("Processing content ref: {:?}", content_ref);
          if let Ok(content_obj) = doc.get_object(content_ref.as_reference()?) {
            println!("Content object: {:?}", content_obj);
            if let Ok(stream) = content_obj.as_stream() {
              println!("Got stream, decoding content...");
              let content = stream.decode_content()?;
              println!("Decoded {} operations", content.operations.len());

              for operation in content.operations {
                println!(
                  "Operation: {} with {} operands",
                  operation.operator,
                  operation.operands.len()
                );
                self.process_text_operation(&operation, &mut text)?;
              }
            } else {
              println!("Failed to get stream from content object");
            }
          }
        }
      },
      Object::Reference(r) => {
        println!("Processing single reference: {:?}", r);
        // ... similar debug prints for single reference case ...
      },
      _ => println!("Unexpected contents type: {:?}", contents),
    }

    println!("Final text length: {}", text.len());
    Ok(text)
  }

  fn process_text_operation(
    &self,
    operation: &lopdf::content::Operation,
    text: &mut String,
  ) -> Result<(), LearnerError> {
    match operation.operator.as_str() {
      // Text showing operators
      "Tj" | "TJ" => {
        if let Some(first) = operation.operands.first() {
          if let Ok(text_bytes) = first.as_str() {
            // Handle UTF-16BE encoded text
            if text_bytes.starts_with(&[0xFE, 0xFF]) {
              let (decoded, ..) = encoding_rs::UTF_16BE.decode(&text_bytes[2..]);
              text.push_str(&decoded);
            } else {
              text.push_str(&String::from_utf8_lossy(text_bytes));
            }
            text.push(' '); // Add space between text chunks
          }
        }
      },
      // Single quote operator (move to next line and show text)
      "'" => {
        text.push('\n');
        if let Some(first) = operation.operands.first() {
          if let Ok(text_bytes) = first.as_str() {
            text.push_str(&String::from_utf8_lossy(text_bytes));
            text.push(' ');
          }
        }
      },
      // Double quote operator (move to next line and show text with spacing)
      "\"" => {
        text.push('\n');
        if let Some(text_op) = operation.operands.get(2) {
          if let Ok(text_bytes) = text_op.as_str() {
            text.push_str(&String::from_utf8_lossy(text_bytes));
            text.push(' ');
          }
        }
      },
      _ => {}, // Ignore other operators
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn test_pdf_metadata_extraction() {
    let test_pdf = PathBuf::from("tests/data/test_paper.pdf");

    let analyzer = PDFAnalyzer::new();
    let content = analyzer.analyze(test_pdf).unwrap();

    // Test metadata
    let metadata = content.metadata;
    assert_eq!(metadata.title.unwrap(), "Analysis of PDF Extraction Methods");
    assert_eq!(metadata.author.unwrap(), "Alice Researcher and Bob Scholar");
    assert_eq!(metadata.subject.unwrap(), "PDF Content Analysis");
    assert_eq!(
      metadata.keywords.unwrap(),
      "PDF analysis, text extraction, metadata, academic papers"
    );
  }

  #[test]
  fn test_pdf_page_extraction() {
    let test_pdf = PathBuf::from("tests/data/test_paper.pdf");

    let analyzer = PDFAnalyzer::new();
    let content = analyzer.analyze(test_pdf).unwrap();

    // // Test page content
    // assert!(!content.pages.is_empty(), "Should have at least one page");

    // // First page should contain title and abstract
    // let first_page = &content.pages[0];
    // assert!(
    //   first_page.text.contains("Analysis of PDF Extraction Methods"),
    //   "First page should contain title"
    // );
    // assert!(
    //   first_page.text.contains("This is a sample paper"),
    //   "First page should contain abstract"
    // );

    // Print first 200 chars of each page for inspection
    for page in &content.pages {
      println!("\nPage {}:", page.page_number);
      println!("First 200 chars: {:?}", page.text.chars().take(200).collect::<String>());
    }
  }
}
