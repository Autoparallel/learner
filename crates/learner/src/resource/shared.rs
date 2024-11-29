//! Shared types for various academic resources.
//!
//! This module contains common data structures used across different types of
//! resources in the learner library. These shared types help maintain consistency
//! and reduce duplication in how we represent common academic concepts like
//! authorship, publication details, and citations.

use super::*;

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
