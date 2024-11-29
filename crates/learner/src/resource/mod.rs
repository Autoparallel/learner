use serde_json::{Map, Value};

use super::*;

mod paper;

pub use paper::*;

pub trait Resource: Serialize + for<'de> Deserialize<'de> {
  fn resource_type(&self) -> &'static str;

  fn fields(&self) -> Result<Map<String, Value>> {
    Ok(
      serde_json::to_value(self)?
        .as_object()
        .cloned()
        .ok_or_else(|| LearnerError::InvalidResource)?,
    )
  }
}

/// Author information for academic papers.
///
/// Represents a single author of a paper, including their name and optional
/// institutional details. This struct supports varying levels of author
/// information availability across different sources.
///
/// # Examples
///
/// ```
/// use learner::resource::Author;
///
/// let author = Author {
///   name:        "Alice Researcher".to_string(),
///   affiliation: Some("Example University".to_string()),
///   email:       Some("alice@example.edu".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
  /// Author's full name
  pub name:        String,
  /// Optional institutional affiliation
  pub affiliation: Option<String>,
  /// Optional contact email
  pub email:       Option<String>,
}
