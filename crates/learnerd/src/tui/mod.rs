//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive TUI for managing papers in the learner database.
//! It is enabled through the "tui" feature flag and provides a keyboard-driven interface
//! for viewing, searching, and managing papers.

use std::io;

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::database::Database;
use ratatui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  style::{Color, Style},
  widgets::{Block, Borders, List, ListItem},
  Terminal,
};

use crate::errors::LearnerdErrors;

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
pub async fn run() -> Result<(), LearnerdErrors> {
  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

  // Create terminal backend
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Create app state
  let db = Database::open(Database::default_path()).await?;
  let papers = db.search_papers("").await?; // Get all papers for now
                                            //   let mut selected = None;

  // Main loop
  loop {
    // Draw UI
    terminal.draw(|f| {
      // Create main layout
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

      // Create list items
      let items: Vec<ListItem> = papers
        .iter()
        .map(|p| {
          ListItem::new(format!("{} ({})", p.title, p.authors.first().map_or("", |a| &a.name)))
        })
        .collect();

      // Create and render list widget
      let list = List::new(items)
        .block(Block::default().title("Papers").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray));

      f.render_widget(list, chunks[0]);

      // Render help text at bottom
      let help = ratatui::widgets::Paragraph::new("q: quit");
      f.render_widget(help, chunks[1]);
    })?;

    // Handle input
    if event::poll(std::time::Duration::from_millis(100))? {
      if let Event::Key(key) = event::read()? {
        if key.code == KeyCode::Char('q') {
          break;
        }
        // We'll add more key handlers later
      }
    }
  }

  // Cleanup
  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

  Ok(())
}
