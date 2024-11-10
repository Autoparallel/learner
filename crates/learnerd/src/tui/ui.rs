//! UI rendering for the TUI.
//!
//! This module handles the layout and rendering of the terminal user interface.

use app::View;
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
  Frame,
};

use super::*;
use crate::errors::LearnerdErrors;

/// Draws the user interface.
///
/// # Arguments
///
/// * `f` - The frame to draw on
/// * `app` - The application state
pub fn draw(f: &mut Frame, app: &App) -> Result<(), LearnerdErrors> {
  // Create the layout
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(1), // Status line
      Constraint::Min(0),    // Main content
      Constraint::Length(1), // Help line
    ])
    .split(f.size());

  // Draw status line
  let status = Line::from(vec![
    Span::raw("Learner TUI • "),
    Span::styled(
      match app.view {
        View::List => "List",
        View::Detail => "Detail",
        View::Search => "Search",
        View::Help => "Help",
      },
      Style::default().fg(Color::Cyan),
    ),
  ]);
  f.render_widget(Paragraph::new(status), chunks[0]);

  // Draw main content
  match app.view {
    View::List => draw_list(f, app, chunks[1])?,
    _ => todo!(),
  }

  // Draw help line
  let help = Line::from(vec![
    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
    Span::raw(" quit • "),
    Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
    Span::raw(" search • "),
    Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
    Span::raw(" help"),
  ]);
  f.render_widget(Paragraph::new(help), chunks[2]);

  Ok(())
}

/// Draws the paper list view.
fn draw_list(f: &mut Frame, app: &App, area: Rect) -> Result<(), LearnerdErrors> {
  let items: Vec<ListItem> = app
    .papers
    .iter()
    .map(|p| {
      ListItem::new(Line::from(vec![
        Span::raw(&p.title),
        Span::raw(" • "),
        Span::styled(
          p.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
          Style::default().fg(Color::Gray),
        ),
      ]))
    })
    .collect();

  let list = List::new(items)
    .block(Block::default().title("Papers").borders(Borders::ALL))
    .highlight_style(Style::default().bg(Color::DarkGray));

  f.render_stateful_widget(list, area, &mut app.selected.map(ListState::default));
  Ok(())
}

// TODO: Implement other view drawing functions
