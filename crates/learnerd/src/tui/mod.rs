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
use learner::{database::Database, format::format_title};
use ratatui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
  Terminal,
};

use crate::errors::LearnerdErrors;

enum DialogState {
  None,
  ExitConfirm,
  PaperDetails(usize),
}

/// Runs the Terminal User Interface.
pub async fn run() -> Result<(), LearnerdErrors> {
  // Create app state
  let db = Database::open(Database::default_path()).await?;
  let papers = db.list_papers("title", true).await?;
  let mut selected = ListState::default();
  selected.select(Some(0));
  let mut dialog = DialogState::None;

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

      let help =
        ratatui::widgets::Paragraph::new("↑/k: up  |  ↓/j: down  |  Enter: details  |  q: quit");
      f.render_widget(help, chunks[1]);

      // Render dialogs if active
      match dialog {
        DialogState::ExitConfirm => {
          let dialog_box = create_dialog_box(
            "Exit Confirmation",
            "Are you sure you want to quit? (y/n)",
            f.size(),
          );
          f.render_widget(Clear, dialog_box); // Clear the background
          f.render_widget(
            Block::default()
              .borders(Borders::ALL)
              .style(Style::default().bg(Color::Black))
              .title("Exit Confirmation"),
            dialog_box,
          );
          f.render_widget(
            Paragraph::new("Are you sure you want to quit? (y/n)").style(Style::default()),
            dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
          );
        },
        DialogState::PaperDetails(index) => {
          let paper = &papers[index];
          let pdf_path = format!(
            "{}/{}.pdf",
            Database::default_pdf_path().display(),
            format_title(&paper.title, Some(50))
          );
          let pdf_status = if std::path::Path::new(&pdf_path).exists() {
            format!("PDF available at: {}", pdf_path)
          } else {
            "PDF not downloaded".to_string()
          };

          let content = vec![
            Line::from(vec![
              Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
              Span::raw(&paper.title),
            ]),
            Line::from(vec![
              Span::styled("Authors: ", Style::default().add_modifier(Modifier::BOLD)),
              Span::raw(
                paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
              ),
            ]),
            Line::from(vec![
              Span::styled("Source: ", Style::default().add_modifier(Modifier::BOLD)),
              Span::raw(format!("{} ({})", paper.source, paper.source_identifier)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
              "Abstract:",
              Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(paper.abstract_text.clone()),
            Line::from(""),
            Line::from(vec![
              Span::styled("PDF Status: ", Style::default().add_modifier(Modifier::BOLD)),
              Span::raw(pdf_status),
            ]),
          ];

          let dialog_box = create_dialog_box("Paper Details", "", f.size());
          f.render_widget(Clear, dialog_box);
          f.render_widget(
            Block::default()
              .borders(Borders::ALL)
              .style(Style::default().bg(Color::Black))
              .title("Paper Details (Esc to close)"),
            dialog_box,
          );
          f.render_widget(
            Paragraph::new(content).wrap(ratatui::widgets::Wrap { trim: true }),
            dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
          );
        },
        DialogState::None => {},
      }
    })?;

    // Handle input
    if event::poll(std::time::Duration::from_millis(50))? {
      if let Event::Key(key) = event::read()? {
        match dialog {
          DialogState::ExitConfirm => match key.code {
            KeyCode::Char('y') => running = false,
            KeyCode::Char('n') | KeyCode::Esc => dialog = DialogState::None,
            _ => {},
          },
          DialogState::PaperDetails(_) =>
            if key.code == KeyCode::Esc {
              dialog = DialogState::None;
            },
          DialogState::None => match key.code {
            KeyCode::Char('q') => dialog = DialogState::ExitConfirm,
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
            KeyCode::Enter =>
              if let Some(i) = selected.selected() {
                dialog = DialogState::PaperDetails(i);
              },
            _ => {},
          },
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

fn create_dialog_box(title: &str, message: &str, r: Rect) -> Rect {
  let width = title.len().max(message.len()).max(40) as u16 + 4;
  let height = if message.is_empty() { 20 } else { 3 };
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length((r.height - height) / 2),
      Constraint::Length(height),
      Constraint::Length((r.height - height) / 2),
    ])
    .split(r);

  let popup_layout = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Length((r.width - width) / 2),
      Constraint::Length(width),
      Constraint::Length((r.width - width) / 2),
    ])
    .split(popup_layout[1])[1];

  popup_layout
}
