//! Application state and logic for the TUI.

use std::io;

use ratatui::{backend::CrosstermBackend, Terminal};

use super::*;

/// Holds the application state and handles the main event loop.
pub struct App {
  /// Terminal backend for rendering
  terminal: Terminal<CrosstermBackend<io::Stdout>>,
  /// Whether the application should exit
  running:  bool,
}

impl App {
  /// Creates a new application instance.
  pub fn new() -> Result<Self, LearnerdErrors> {
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;

    Ok(Self { terminal, running: true })
  }

  /// Runs the main application loop.
  pub async fn run(&mut self) -> Result<(), LearnerdErrors> {
    todo!("Implement event handling and UI rendering")
  }
}
