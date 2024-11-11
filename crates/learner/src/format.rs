//! Text formatting utilities for standardizing document titles and filenames.
//!
//! This module provides functionality for cleaning and standardizing text strings,
//! particularly focused on converting document titles into filesystem-friendly
//! filenames. It handles common transformations like converting to lowercase,
//! replacing spaces with underscores, and enforcing length limits while preserving
//! word boundaries.
//!
//! # Examples
//!
//! ```
//! use learner::format;
//!
//! let title = "This Is A Very Long Document Title";
//!
//! // Default formatting (50 char limit)
//! let formatted = format::format_title(title, None);
//! assert_eq!(formatted, "this_is_a_very_long_document_title");
//!
//! // Custom length limit
//! let formatted = format::format_title(title, Some(20));
//! assert_eq!(formatted, "this_is_a_very_long");
//! ```

/// Formats a title string for use as a filename or identifier.
///
/// This function performs several transformations to make titles more suitable for
/// use as filenames or identifiers:
/// - Converts the text to lowercase
/// - Replaces whitespace (including multiple spaces) with single underscores
/// - Truncates to a maximum length while preserving word boundaries
///
/// # Arguments
///
/// * `title` - The input title string to format
/// * `max_length` - Optional maximum length limit. If `None`, defaults to 50 characters. The
///   function will truncate at word boundaries to stay within this limit.
///
/// # Returns
///
/// Returns a `String` containing the formatted title.
///
/// # Examples
///
/// ```
/// use learner::format;
///
/// // Basic formatting
/// assert_eq!(format::format_title("Hello World", None), "hello_world");
///
/// // Handling multiple spaces
/// assert_eq!(format::format_title("No    Extra    Spaces", None), "no_extra_spaces");
///
/// // Length limiting
/// assert_eq!(
///   format::format_title("This Is A Very Long Title Indeed", Some(20)),
///   "this_is_a_very_long"
/// );
/// ```
pub fn format_title(title: &str, max_length: Option<usize>) -> String {
  // Convert to lowercase and collapse multiple spaces into one, then replace with underscore
  let formatted = title
      .to_lowercase()
      .chars()
      .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
      .collect::<String>()
      .split_whitespace() // This splits on any whitespace and removes empty strings
      .collect::<Vec<&str>>()
      .join("_");

  let max_length = max_length.unwrap_or(50);

  // If the string is already within length limit, return it
  if formatted.len() <= max_length {
    return formatted;
  }

  // Split into words
  let words: Vec<&str> = formatted.split('_').collect();
  let mut result = String::new();

  // Build string word by word until we hit the limit
  for (i, word) in words.iter().enumerate() {
    if i > 0 {
      // Check if adding underscore + word would exceed limit
      if result.len() + word.len() + 1 > max_length {
        break;
      }
      result.push('_');
    }

    // Check if adding just the word would exceed limit
    if result.len() + word.len() > max_length {
      break;
    }
    result.push_str(word);
  }

  result
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_format_title() {
    assert_eq!(format_title("Hello World", None), "hello_world");
    assert_eq!(
      format_title("This Is A Very Long Title Indeed", None),
      "this_is_a_very_long_title_indeed"
    );
    assert_eq!(format_title("This Is A Very Long Title Indeed", Some(20)), "this_is_a_very_long");
    assert_eq!(
      format_title("This Is A Very Long Title Indeed", Some(30)),
      "this_is_a_very_long_title"
    );
    assert_eq!(format_title("short", None), "short");
    assert_eq!(format_title("UPPERCASE TEXT", None), "uppercase_text");
    assert_eq!(format_title("No    Extra    Spaces", None), "no_extra_spaces");
    assert_eq!(format_title("Title with: </weird things\\>", None), "title_with_weird_things");
  }
}
