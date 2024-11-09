use lopdf::Document;

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
        String::from_utf16be_lossy(&bytes[2..])
      } else {
        // Regular string decoding
        String::from_utf8_lossy(bytes).to_string()
      }
    })
  }

  fn extract_pages(&self, doc: &Document) -> Result<Vec<PageContent>, LearnerError> {
    let mut pages = Vec::new();
    lazy_static! {
      static ref PDF_TEXT_REGEX: Regex = Regex::new(r"\(([^)]+)\)").unwrap();
    };

    for (page_num, page_id) in doc.page_iter().enumerate() {
      debug!("Processing page {}, id: {:?}", page_num + 1, page_id);

      let page = doc.get_object(page_id)?;
      let page_dict = page.as_dict()?;

      match page_dict.get(b"Contents") {
        Ok(contents) => {
          let mut text = String::new();
          let text_ref = contents.as_reference()?;
          let plain_content = doc.get_object(text_ref)?.as_stream()?.get_plain_content()?;
          for cap in PDF_TEXT_REGEX.captures_iter(&String::from_utf8_lossy(&plain_content)) {
            text.push_str(&cap[1]);
            text.push(' '); // TODO (autoparallel): This adds space between text segments, but it
                            // does so too aggressively
          }
          trace!("text for page {}: {}", page_num, text);
          pages.push(PageContent { page_number: page_num as u32 + 1, text });
        },
        Err(e) => println!("Failed to get Contents: {:?}", e),
      }
    }

    Ok(pages)
  }
}

#[cfg(test)]
mod tests {

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

    // Test page content
    assert!(!content.pages.is_empty(), "Should have at least one page");

    // First page should contain title and abstract
    let first_page = &content.pages[0];
    assert!(
      first_page.text.contains("Analysis of PDF Extraction Methods"),
      "First page should contain title"
    );
    assert!(
      first_page.text.contains("Abstract \\227This is a sam ple paper"),
      "First page should contain abstract"
    );
  }
}
