//! PDF parsing and content extraction functionality.
//!
//! This module provides capabilities for extracting structured content from PDF files,
//! including metadata and page-level text content. It's designed to work with academic
//! papers and research documents, handling common PDF features like:
//!
//! - Document metadata (title, author, subject, keywords)
//! - UTF-16BE encoded text content
//! - Page-by-page text extraction
//! - Structured content organization
//!
//! # Examples
//!
//! ```no_run
//! use std::path::PathBuf;
//!
//! use learner::pdf::PDFContentBuilder;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Build and analyze a PDF
//! let content = PDFContentBuilder::new().path("paper.pdf").analyze()?;
//!
//! // Access metadata
//! if let Some(title) = &content.metadata.title {
//!   println!("Title: {}", title);
//! }
//!
//! // Process page content
//! for page in &content.pages {
//!   println!("Page {}: {}", page.page_number, page.text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # PDF Text Extraction
//!
//! Text extraction handles both standard ASCII text and UTF-16BE encoded content,
//! which is common in PDFs containing non-ASCII characters. The extractor:
//!
//! - Preserves document structure through page numbering
//! - Handles Unicode text encoding
//! - Extracts text content while maintaining readability
//!
//! Future improvements may include:
//! - More sophisticated text layout preservation
//! - Section and paragraph detection
//! - Figure and table extraction
//! - Citation extraction

use lopdf::Document;

use super::*;

/// Structured content extracted from a PDF document.
///
/// This structure contains both the document's metadata and the text content
/// of each page. It's designed to facilitate both document organization
/// (through metadata) and full-text search capabilities.
///
/// The content is typically created using [`PDFContentBuilder`], which provides
/// a fluent interface for configuring the extraction process.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
///
/// use learner::pdf::PDFContentBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let content = PDFContentBuilder::new().path("paper.pdf").analyze()?;
///
/// if let Some(author) = &content.metadata.author {
///   println!("Author: {}", author);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PDFContent {
  /// Path to the source PDF file
  pub path:     PathBuf,
  /// Document metadata including title, author, etc.
  pub metadata: PDFMetadata,
  /// Ordered collection of page contents
  pub pages:    Vec<PageContent>,
}

/// Builder for configuring and creating PDFContent instances.
///
/// Provides a fluent interface for specifying options and analyzing PDFs. The builder
/// pattern allows for future extensibility of PDF analysis options without breaking
/// existing code.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
///
/// use learner::pdf::PDFContentBuilder;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let content = PDFContentBuilder::new().path("paper.pdf").analyze()?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct PDFContentBuilder {
  /// Path to the source PDF file
  path: Option<PathBuf>,
}

/// Metadata extracted from a PDF document.
///
/// Represents the standard PDF document information dictionary entries.
/// All fields are optional as not all PDFs contain complete metadata.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PDFMetadata {
  /// Document title
  pub title:    Option<String>,
  /// Document author(s)
  pub author:   Option<String>,
  /// Document subject or description
  pub subject:  Option<String>,
  /// Keywords associated with the document
  pub keywords: Option<String>,
}

/// Content extracted from a single PDF page.
///
/// Associates extracted text content with its page number for
/// proper ordering and reference.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PageContent {
  /// 1-based page number in the document
  pub page_number: u32,
  /// Extracted text content from the page
  pub text:        String,
}

impl PDFContentBuilder {
  /// Creates a new builder instance with default settings.
  ///
  /// # Examples
  ///
  /// ```
  /// use learner::pdf::PDFContentBuilder;
  ///
  /// let builder = PDFContentBuilder::new();
  /// ```
  pub fn new() -> Self { Default::default() }

  /// Sets the path to the PDF file to analyze.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the PDF file, convertible to [`PathBuf`]
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::pdf::PDFContentBuilder;
  /// let builder = PDFContentBuilder::new().path("paper.pdf");
  /// ```
  pub fn path<P: Into<PathBuf>>(mut self, path: P) -> Self {
    self.path = Some(path.into());
    self
  }

  /// Analyzes the PDF file and extracts its content.
  ///
  /// This method performs the actual PDF parsing and content extraction,
  /// including metadata and page content.
  ///
  /// # Returns
  ///
  /// Returns a [`Result`] containing either:
  /// - A [`PDFContent`] with the extracted metadata and text
  /// - A [`LearnerError`] if analysis fails
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - No path has been specified
  /// - The PDF file cannot be read
  /// - The PDF format is invalid
  /// - Text extraction fails
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use learner::pdf::PDFContentBuilder;
  /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let content = PDFContentBuilder::new().path("paper.pdf").analyze()?;
  /// # Ok(())
  /// # }
  /// ```
  pub fn analyze(self) -> Result<PDFContent> {
    let path = self.path.ok_or_else(|| {
      LearnerError::Path(std::io::Error::new(std::io::ErrorKind::NotFound, "No PDF path specified"))
    })?;
    let doc = Document::load(&path)?;
    let metadata = extract_metadata(&doc)?;
    let pages = extract_pages(&doc)?;

    Ok(PDFContent { path, metadata, pages })
  }
}

/// Extracts metadata from a PDF document.
///
/// Processes the PDF's information dictionary to extract standard metadata fields,
/// handling both ASCII and UTF-16BE encoded text.
///
/// # Arguments
///
/// * `doc` - Reference to the parsed PDF document
///
/// # Returns
///
/// Returns a [`Result`] containing either:
/// - A [`PDFMetadata`] structure with the extracted metadata
/// - A [`LearnerError`] if metadata extraction fails
fn extract_metadata(doc: &Document) -> Result<PDFMetadata> {
  let trailer = &doc.trailer;
  let info_ref = trailer.get(b"Info").ok().and_then(|o| o.as_reference().ok());

  let info = match info_ref {
    Some(reference) => doc.get_object(reference).and_then(|obj| obj.as_dict())?,
    None =>
      return Ok(PDFMetadata { title: None, author: None, subject: None, keywords: None }),
  };

  Ok(PDFMetadata {
    title:    get_text_from_dict(info, "Title"),
    author:   get_text_from_dict(info, "Author"),
    subject:  get_text_from_dict(info, "Subject"),
    keywords: get_text_from_dict(info, "Keywords"),
  })
}

/// Extracts text from a PDF dictionary entry.
///
/// Handles both ASCII strings and UTF-16BE encoded text, which is common
/// in PDF files containing non-ASCII characters.
///
/// # Arguments
///
/// * `dict` - Reference to the PDF dictionary
/// * `key` - Key for the dictionary entry to extract
///
/// # Returns
///
/// Returns an [`Option`] containing the extracted text if present
fn get_text_from_dict(dict: &lopdf::Dictionary, key: &str) -> Option<String> {
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

/// Extracts text content from all pages in the PDF.
///
/// Processes each page's content stream to extract text, maintaining page
/// order and associating content with page numbers.
///
/// # Arguments
///
/// * `doc` - Reference to the parsed PDF document
///
/// # Returns
///
/// Returns a [`Result`] containing either:
/// - A [`Vec`] of [`PageContent`] with the extracted text
/// - A [`LearnerError`] if text extraction fails
fn extract_pages(doc: &Document) -> Result<Vec<PageContent>> {
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

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn test_pdf_metadata_extraction() {
    let content =
      PDFContentBuilder::new().path(PathBuf::from("tests/.data/test_paper.pdf")).analyze().unwrap();

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
    let content =
      PDFContentBuilder::new().path(PathBuf::from("tests/.data/test_paper.pdf")).analyze().unwrap();

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
