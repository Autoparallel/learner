//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive TUI for managing papers in the learner database.
//! It is enabled through the "tui" feature flag and provides a keyboard-driven interface
//! for viewing, searching, and managing papers.

use std::io;

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
use tokio::signal::unix::{signal, SignalKind};

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
  // Create app state
  let db = Database::open(Database::default_path()).await?;

  // For now, let's just get all papers from the database
  let papers = db.list_papers("title", true).await?;

  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

  // Create terminal backend
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Set up signal handlers
  let mut sigint = signal(SignalKind::interrupt())?;
  let mut sigterm = signal(SignalKind::terminate())?;

  // Main loop
  'main: loop {
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
      let help = ratatui::widgets::Paragraph::new("q: quit  |  Ctrl-c: exit");
      f.render_widget(help, chunks[1]);
    })?;

    // Handle input events with a timeout
    if event::poll(std::time::Duration::from_millis(100))? {
      if let Event::Key(key) = event::read()? {
        match (key.code, key.modifiers) {
          (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            break 'main;
          },
          _ => {},
        }
      }
    }

    // Check for signals
    tokio::select! {
        _ = sigint.recv() => break 'main,
        _ = sigterm.recv() => break 'main,
        else => {}
    }
  }

  // Cleanup
  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
  terminal.show_cursor()?;

  Ok(())
}
