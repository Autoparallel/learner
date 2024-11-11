//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive terminal interface for managing and viewing academic papers.
//! It uses the `ratatui` library for rendering and `crossterm` for terminal manipulation and
//! event handling. The TUI offers features including:
//!
//! - Paper list navigation and viewing
//! - Detailed paper information display
//! - PDF status tracking
//! - Keyboard-driven interface
//!
//! The interface is split into two main panes:
//! - Left: List of papers with title and count
//! - Right: Detailed view of the selected paper
//!
//! # Navigation
//!
//! The TUI supports both arrow keys and vim-style navigation:
//! - Up/k: Move selection up
//! - Down/j: Move selection down
//! - q: Quit application
//!
//! # Notes
//! The TUI is enabled through the "tui" feature flag. When enabled, it becomes
//! the default interface when no command is specified.

use std::io;

use app::AppState;
use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::{database::Database, format::format_title};
use ratatui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style, Stylize},
  widgets::{ListItem, ListState},
  Terminal,
};
use ui::draw_ui;

use crate::errors::LearnerdErrors;

mod app;
mod ui;

/// Style for section titles and headers
const TITLE_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
/// Style for the currently selected item
const HIGHLIGHT_STYLE: Style =
  Style::new().bg(Color::DarkGray).fg(Color::LightCyan).add_modifier(Modifier::BOLD);
/// Style for field labels in paper details
const LABEL_STYLE: Style = Style::new().fg(Color::LightBlue);
/// Style for regular text content
const NORMAL_TEXT: Style = Style::new().fg(Color::Gray);
/// Style for help text and secondary information
const HELP_STYLE: Style = Style::new().fg(Color::DarkGray);
/// Style for keyboard shortcuts in help text
const HIGHLIGHT_KEY: Style = Style::new().fg(Color::Yellow);

/// Represents the current dialog state of the application.
///
/// Used to track whether a dialog is currently being displayed
/// and what type of dialog it is.
pub enum DialogState {
  None,
  ExitConfirm,
  PDFNotFound,
}

/// Runs the Terminal User Interface.
///
/// This function initializes the terminal, sets up the display,
/// and manages the main event loop. It handles:
/// - Terminal setup and cleanup
/// - Event processing
/// - User input
/// - Screen rendering
/// - Dialog management
///
/// The interface is restored to its original state when the
/// function returns, regardless of how it exits.
///
/// # Errors
///
/// Returns a `LearnerdErrors` if:
/// - Terminal initialization fails
/// - Database operations fail
/// - Event handling fails
/// - Screen drawing fails
pub async fn run() -> Result<(), LearnerdErrors> {
  // Create app state
  let db = Database::open(Database::default_path()).await?;
  let papers = db.list_papers("title", false).await?;
  let mut app = AppState::new(papers);

  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Event loop
  loop {
    // Draw state if needed
    if app.needs_redraw {
      terminal.draw(|f| draw_ui(&mut app, f))?;
      app.needs_redraw = false;
    }

    // Wait for input with timeout
    if event::poll(std::time::Duration::from_millis(5))? {
      match event::read()? {
        Event::Key(key) =>
          if app.handle_input(key.code) {
            break;
          },
        Event::Resize(..) => {
          app.needs_redraw = true;
        },
        _ => {},
      }
    }
  }

  // Cleanup
  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
  terminal.show_cursor()?;

  Ok(())
}
