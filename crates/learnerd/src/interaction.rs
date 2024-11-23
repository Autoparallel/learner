//! User interaction handling and response formatting.
//!
//! This module provides the core abstractions for handling user interactions and
//! displaying content in a consistent way. It defines:
//!
//! - A trait for implementing different interaction styles (CLI, TUI, API)
//! - Response content types for different kinds of output
//! - A consistent way to handle user input and display feedback

use super::*;

/// Content types for user interaction responses.
///
/// This enum represents different types of content that can be displayed to users,
/// from paper information to status messages. All string content uses borrowed
/// references to avoid unnecessary allocations.
#[derive(Debug)]
pub enum ResponseContent<'a> {
  /// Single paper with its details
  Paper(&'a Paper),
  /// Collection of papers (e.g., search results)
  Papers(&'a [Paper]),
  /// Success message
  Success(&'a str),
  /// Error with details
  Error(LearnerdError),
  /// Informational message
  Info(&'a str),
}

/// Trait for implementing user interactions.
///
/// This trait defines the interface for handling user interactions. Implementations
/// of this trait can provide different ways of interacting with users (CLI, TUI, API)
/// while maintaining consistent behavior.
///
/// The trait handles three main types of interactions:
/// - Confirmation prompts (`confirm`)
/// - Text input prompts (`prompt`)
/// - Content display (`reply`)
pub trait UserInteraction {
  /// Request confirmation from the user.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to display in the confirmation prompt
  ///
  /// # Returns
  ///
  /// Returns `Ok(true)` if the user confirms, `Ok(false)` if they decline,
  /// or an error if the interaction fails.
  fn confirm(&self, message: &str) -> Result<bool>;

  /// Request text input from the user.
  ///
  /// # Arguments
  ///
  /// * `message` - The prompt message to display to the user
  ///
  /// # Returns
  ///
  /// Returns the user's input as a String, or an error if the interaction fails.
  fn prompt(&self, message: &str) -> Result<String>;

  /// Display content to the user.
  ///
  /// This method handles formatting and displaying different types of content
  /// defined in [`ResponseContent`].
  ///
  /// # Arguments
  ///
  /// * `content` - The content to display
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the content was displayed successfully, or an error
  /// if the display operation fails.
  fn reply(&self, content: ResponseContent) -> Result<()>;
}
