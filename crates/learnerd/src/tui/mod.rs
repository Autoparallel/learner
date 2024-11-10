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
  widgets::{Block, Borders, List, ListItem, ListState},
  Terminal,
};

use crate::errors::LearnerdErrors;

/// Runs the Terminal User Interface.
pub async fn run() -> Result<(), LearnerdErrors> {
  // Create app state
  let db = Database::open(Database::default_path()).await?;
  let papers = db.list_papers("title", true).await?;
  let mut selected = ListState::default();
  selected.select(Some(0));

  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

  // Create terminal backend
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Main loop
  let mut running = true;
  while running {
    // Draw UI
    terminal.draw(|f| {
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

      let items: Vec<ListItem> = papers
        .iter()
        .map(|p| {
          ListItem::new(format!(
            "{} ({}, {})",
            p.title,
            p.source.to_string(),
            p.authors.first().map_or("No author", |a| &a.name)
          ))
        })
        .collect();

      let list = List::new(items)
        .block(Block::default().title("Papers").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray));

      f.render_stateful_widget(list, chunks[0], &mut selected);

      let help = ratatui::widgets::Paragraph::new("↑/k: up  |  ↓/j: down  |  q: quit");
      f.render_widget(help, chunks[1]);
    })?;

    // Handle input
    if event::poll(std::time::Duration::from_millis(50))? {
      if let Event::Key(key) = event::read()? {
        match key.code {
          KeyCode::Char('q') => {
            running = false;
          },
          KeyCode::Up | KeyCode::Char('k') => {
            let i = selected.selected().unwrap_or(0);
            if i > 0 {
              selected.select(Some(i - 1));
            }
          },
          KeyCode::Down | KeyCode::Char('j') => {
            let i = selected.selected().unwrap_or(0);
            if i < papers.len().saturating_sub(1) {
              selected.select(Some(i + 1));
            }
          },
          _ => {},
        }
      }
    }
  }

  // Cleanup
  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
  terminal.show_cursor()?;

  Ok(())
}
