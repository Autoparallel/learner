//! UI styling constants and helper functions for the TUI.
//!
//! This module provides a consistent set of styles for the terminal interface, including:
//! - Text colors and formatting for different UI elements
//! - Focus state indicators
//! - Border styles for panels and dialogs
//!
//! The styles are designed to provide clear visual hierarchy and feedback while
//! maintaining readability in different terminal color schemes.

use ratatui::style::{Color, Modifier, Style};

/// Style for help text and secondary information.
/// Uses a muted gray color to indicate supplementary content.
pub const HELP: Style = Style::new().fg(Color::DarkGray);

/// Style for the currently selected item.
/// Combines background, text color, and bold formatting to make the
/// selection clearly visible.
pub const HIGHLIGHT: Style =
  Style::new().bg(Color::DarkGray).fg(Color::LightCyan).add_modifier(Modifier::BOLD);

/// Style for keyboard shortcuts and interactive elements.
/// Uses yellow to draw attention to actionable items.
pub const KEY_HIGHLIGHT: Style = Style::new().fg(Color::Yellow);

/// Style for field labels and categories.
/// Uses light blue to distinguish labels from their values.
pub const LABEL: Style = Style::new().fg(Color::LightBlue);

/// Style for regular text content.
/// Uses a neutral gray that works well for main content.
pub const NORMAL: Style = Style::new().fg(Color::Gray);

/// Style for section titles and headers.
/// Combines cyan color with bold text to create visual hierarchy.
pub const TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

/// Returns the appropriate border style based on focus state.
///
/// # Arguments
///
/// * `focused` - Whether the panel is currently focused
pub fn border_style(focused: bool) -> Style {
  if focused {
    Style::default().fg(Color::LightBlue)
  } else {
    Style::default().fg(Color::Blue)
  }
}
