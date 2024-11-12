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
//!
//! # Navigation
//!
//! The interface supports both arrow keys and vim-style navigation:
//! - `↑`/`k`: Move selection up
//! - `↓`/`j`: Move selection down
//! - `←`/`h`: Focus left pane
//! - `→`/`l`: Focus right pane
//! - `o`: Open PDF (if available)
//! - `q`: Quit application
//!
//! # Layout
//!
//! The interface is divided into two main sections:
//! - Left: List of papers with title and count
//! - Right: Detailed view of the selected paper including:
//!   - Title
//!   - Authors
//!   - Source information
//!   - Abstract (scrollable)
//!   - PDF status
//!
//! # Implementation Notes
//!
//! The UI is built using a modular approach with:
//! - State management ([`state::UIState`])
//! - Consistent styling ([`styles`])
//! - Drawing logic ([`ui::UIDrawer`])
//!
//! This separation allows for clear responsibility boundaries and easier maintenance.
use std::io;

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::format::format_title;
use ratatui::{backend::CrosstermBackend, Terminal};

use super::*;

mod state;
mod styles;
mod ui;

use state::UIState;
use ui::UIDrawer;

/// Runs the Terminal User Interface.
///
/// This function initializes the terminal, sets up the display, and manages the main event loop.
/// It handles:
/// - Terminal setup and cleanup
/// - State initialization
/// - Event processing
/// - Screen rendering
///
/// The interface is restored to its original state when the function returns,
/// regardless of how it exits.
///
/// # Terminal Setup
///
/// The function configures the terminal for full-screen operation by:
/// - Enabling raw mode for direct input handling
/// - Entering alternate screen to preserve the original terminal content
/// - Setting up mouse capture for potential future mouse support
///
/// # Event Loop
///
/// The main loop handles:
/// - Redraw requests through `needs_redraw` flag
/// - Input events (keyboard, resize)
/// - Graceful shutdown on quit command
///
/// # Errors
///
/// Returns an error if:
/// - Database operations fail
/// - Terminal initialization fails
/// - Event handling fails
/// - Screen drawing fails
///
/// # Cleanup
///
/// On exit (either through error or normal termination), the function:
/// - Disables raw mode
/// - Restores the original screen
/// - Shows the cursor
/// - Disables mouse capture
pub async fn run() -> Result<()> {
  // Initialize state
  let db = Database::open(Database::default_path()).await?;
  let papers = db.list_papers("title", false).await?;
  let mut state = UIState::new(papers);

  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Event loop
  loop {
    if state.needs_redraw {
      terminal.draw(|f| UIDrawer::new(f, &mut state).draw())?;
    }

    if event::poll(std::time::Duration::from_millis(5))? {
      match event::read()? {
        Event::Key(key) =>
          if state.handle_input(key.code) {
            break;
          },
        Event::Resize(..) => state.needs_redraw = true,
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
