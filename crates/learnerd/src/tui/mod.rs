//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive TUI for managing papers in the learner database.
//! It is enabled through the "tui" feature flag and provides a keyboard-driven interface
//! for viewing, searching, and managing papers.
use super::*;

mod app;

/// Runs the Terminal User Interface.
///
/// This function initializes the terminal, sets up event handling,
/// and manages the main application loop. It restores the terminal
/// state when exiting.
///
/// # Errors
///
/// Returns an error if:
/// - Terminal initialization fails
/// - Event handling fails
/// - Drawing the UI fails
use std::io::stdout;

pub use app::App;
use crossterm::{
  event::{DisableMouseCapture, EnableMouseCapture},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub async fn run() -> Result<(), LearnerdErrors> {
  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

  // Create app and run it
  let mut app = App::new()?;
  let result = app.run().await;

  // Restore terminal
  disable_raw_mode()?;
  execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

  result
}
