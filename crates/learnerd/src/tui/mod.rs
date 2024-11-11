//! Terminal User Interface for learnerd.
//!
//! Provides an interactive terminal interface for managing and viewing academic papers.

use std::io;

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::{database::Database, paper::Paper};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::*;

mod state;
mod styles;
mod ui;

use state::UIState;
use ui::UIDrawer;

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
