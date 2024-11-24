//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive terminal interface for managing and viewing academic papers,
//! built using the `ratatui` library. The interface offers:
//!
//! - A split-pane view with paper list and details
//! - Keyboard-driven navigation
//! - Real-time paper information display
//! - PDF availability status and opening
//! - Vim-style navigation controls
//! - Command mode for paper management
//!
//! # Navigation
//!
//! The interface supports both arrow keys and vim-style navigation:
//! - `↑`/`k`: Move selection up
//! - `↓`/`j`: Move selection down
//! - `←`/`h`: Focus left pane
//! - `→`/`l`: Focus right pane
//! - `:`: Enter command mode
//! - `o`: Open PDF (if available)
//! - `q`: Quit application

use std::io::{self, Stdout};

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::{
  database::{OrderField, Query},
  format::format_title,
  Config, Learner,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::*;

mod state;
mod styles;
mod ui;

use interaction::{ResponseContent, UserInteraction};
use state::UIState;
use ui::UIDrawer;

/// Main TUI application struct that handles the interface and interactions
pub struct Tui {
  /// Terminal interface handler
  terminal: Terminal<CrosstermBackend<Stdout>>,
  /// Application state
  state:    UIState,
  /// Learner instance for paper management
  learner:  Learner,
}

impl Tui {
  /// Creates a new TUI instance
  pub async fn new() -> Result<Self> {
    // Initialize learner
    let mut learner = Learner::from_path(Config::default_path()?).await?;

    // Get initial paper list
    let papers =
      Query::list_all().order_by(OrderField::Title).execute(&mut learner.database).await?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    Ok(Self { terminal, state: UIState::new(papers), learner })
  }

  /// Runs the TUI main loop
  pub async fn run(&mut self) -> Result<()> {
    loop {
      // Draw UI if needed
      if self.state.needs_redraw {
        self.terminal.draw(|f| UIDrawer::new(f, &mut self.state).draw())?;
      }

      // Handle events
      if event::poll(std::time::Duration::from_millis(5))? {
        match event::read()? {
          Event::Key(key) =>
            if self.state.handle_input(key.code) {
              break;
            },
          Event::Resize(..) => self.state.needs_redraw = true,
          _ => {},
        }
      }
    }

    // Cleanup and restore terminal
    self.cleanup()?;
    Ok(())
  }

  /// Cleans up the terminal state
  fn cleanup(&mut self) -> Result<()> {
    disable_raw_mode()?;
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    self.terminal.show_cursor()?;
    Ok(())
  }
}

impl UserInteraction for Tui {
  fn confirm(&mut self, message: &str) -> Result<bool> {
    // For now, default to true - we'll implement proper UI confirmation later
    Ok(true)
  }

  fn prompt(&mut self, message: &str) -> Result<String> {
    // For now, return empty string - we'll implement proper UI prompting later
    Ok(String::new())
  }

  fn reply(&mut self, content: ResponseContent) -> Result<()> {
    // For now, just set a status message - we'll improve feedback later
    match content {
      ResponseContent::Success(msg) | ResponseContent::Info(msg) => {
        self.state.set_status_message(msg.to_string());
      },
      ResponseContent::Error(e) => {
        self.state.set_status_message(format!("Error: {}", e));
      },
      _ => {}, // Ignore Paper/Papers content for now
    }
    Ok(())
  }
}

/// Runs the Terminal User Interface.
pub async fn run() -> Result<()> {
  let mut tui = Tui::new().await?;
  tui.run().await
}
