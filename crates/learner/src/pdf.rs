use std::path::Path;

use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PDFContent {
  metadata: PDFMetadata,
  pages:    Vec<PageContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PDFMetadata {
  title:    Option<String>,
  author:   Option<String>,
  subject:  Option<String>,
  keywords: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageContent {
  page_number: u32,
  text:        String,
}

pub struct PDFAnalyzer;

impl PDFAnalyzer {
  pub fn new() -> Self { Self }

  /// Analyzes a PDF file and extracts basic content
  pub fn analyze<P: AsRef<Path>>(&self, path: P) -> Result<PDFContent, LearnerError> {
    let doc = Document::load(path)
      .map_err(|e| LearnerError::PDFError(format!("Failed to load PDF: {}", e)))?;
    let metadata = self.extract_metadata(&doc)?;
    let pages = self.extract_pages(&doc)?;

    Ok(PDFContent { metadata, pages })
  }

  fn extract_metadata(&self, doc: &Document) -> Result<PDFMetadata, LearnerError> {
    // Get the trailer dictionary which might contain the info reference
    let trailer = &doc.trailer;

    // Get the Info dictionary reference from the trailer
    let info_ref = trailer.get(b"Info").ok().and_then(|o| o.as_reference().ok());

    let info = match info_ref {
      Some(reference) => doc
        .get_object(reference)
        .ok()
        .and_then(|obj| obj.as_dict().ok())
        .ok_or_else(|| LearnerError::PDFError("Could not get Info dictionary".into()))?,
      None =>
        return Ok(PDFMetadata { title: None, author: None, subject: None, keywords: None }),
    };

    Ok(PDFMetadata {
      title:    self.get_text_from_dict(&info, "Title"),
      author:   self.get_text_from_dict(&info, "Author"),
      subject:  self.get_text_from_dict(&info, "Subject"),
      keywords: self.get_text_from_dict(&info, "Keywords"),
    })
  }

  fn extract_pages(&self, doc: &Document) -> Result<Vec<PageContent>, LearnerError> {
    let mut pages = Vec::new();

    for (page_num, page_id) in doc.page_iter().enumerate() {
      let page = doc
        .get_object(page_id)
        .map_err(|e| LearnerError::PDFError(format!("Failed to get page {}: {}", page_num, e)))?;

      let page_dict = page.as_dict().map_err(|e| {
        LearnerError::PDFError(format!("Page {} is not a dictionary: {}", page_num, e))
      })?;
      dbg!(&page_dict);
      // Get Contents object(s)
      let contents = page_dict
        .get(b"Contents")
        .ok()
        .and_then(|contents| self.extract_text_from_contents(doc, contents));

      pages.push(PageContent {
        page_number: page_num as u32 + 1,
        text:        contents.unwrap_or_default(),
      });
    }

    Ok(pages)
  }

  fn extract_text_from_contents(&self, doc: &Document, contents: &Object) -> Option<String> {
    match contents {
      Object::Array(array) => {
        // Combine text from multiple content streams
        let mut text = String::new();
        for content_ref in array {
          if let Ok(content_obj) = doc.get_object(content_ref.as_reference().ok()?) {
            if let Ok(content_str) = content_obj.as_str() {
              // This unwrap should never fail
              text.push_str(core::str::from_utf8(content_str).unwrap());
            }
          }
        }
        Some(text)
      },
      Object::Reference(r) => {
        // Single content stream
        doc
          .get_object(*r)
          .ok()?
          .as_str()
          .ok()
          .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
      },
      _ => None,
    }
  }

  fn get_text_from_dict(&self, dict: &lopdf::Dictionary, key: &str) -> Option<String> {
    dict
      .get(key.as_bytes())
      .ok()
      .and_then(|obj| obj.as_str().ok())
      .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn test_pdf_analysis() -> Result<(), Box<dyn std::error::Error>> {
    // Get the test PDF path - adjust this path as needed
    let test_pdf = PathBuf::from("tests/data/test_paper.pdf");

    let analyzer = PDFAnalyzer::new();
    let content = analyzer.analyze(test_pdf)?;

    // Test metadata
    let metadata = content.metadata;
    println!("Title: {:?}", metadata.title);
    println!("Author: {:?}", metadata.author);
    println!("Subject: {:?}", metadata.subject);
    println!("Keywords: {:?}", metadata.keywords);

    // Test page content
    assert!(!content.pages.is_empty(), "PDF should contain at least one page");

    for page in content.pages {
      println!("Page {}: {} characters of text", page.page_number, page.text.len());
    }

    Ok(())
  }
}
