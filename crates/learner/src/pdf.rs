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

    Ok(PDFContent { metadata, pages: vec![] })
  }

  fn extract_metadata(&self, doc: &Document) -> Result<PDFMetadata, LearnerError> {
    let trailer = &doc.trailer;
    dbg!(&trailer);

    let info_ref = trailer.get(b"Info").ok().and_then(|o| o.as_reference().ok());
    dbg!(&info_ref);

    let info = match info_ref {
      Some(reference) => {
        let dic = doc.get_object(reference).and_then(|obj| obj.as_dict())?;
        dbg!(&dic);
        dic
      },
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
        // let bytes = &bytes[2..];
        let (cow, ..) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        cow.into_owned()
      } else {
        // Regular string decoding
        String::from_utf8_lossy(bytes).into_owned()
      }
    })
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn test_pdf_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let test_pdf = PathBuf::from("tests/data/test_paper.pdf");

    let analyzer = PDFAnalyzer::new();
    let content = analyzer.analyze(test_pdf)?;

    // Test metadata
    let metadata = content.metadata;
    assert_eq!(metadata.title.unwrap(), "Analysis of PDF Extraction Methods");
    assert_eq!(metadata.author.unwrap(), "Alice Researcher and Bob Scholar");
    assert_eq!(metadata.subject.unwrap(), "PDF Content Analysis");
    assert_eq!(
      metadata.keywords.unwrap(),
      "PDF analysis, text extraction, metadata, academic papers"
    );

    // Test page content
    // assert!(!content.pages.is_empty(), "PDF should contain at least one page");

    // for page in &content.pages {
    //   println!("Page {}: {} characters of text", page.page_number, page.text.len());
    //   println!("First 100 chars: {:?}", &page.text[..page.text.len().min(100)]);
    // }

    Ok(())
  }
}
