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

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::{database::Database, format::format_title};
use ratatui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Modifier, Style, Stylize},
  text::{Line, Span},
  widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
  Terminal,
};

use crate::errors::LearnerdErrors;

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
enum DialogState {
  /// No dialog is currently active
  None,
  /// Exit confirmation dialog is being shown
  ExitConfirm,
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
      // Create main horizontal split
      let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
          Constraint::Percentage(30), // List of papers
          Constraint::Percentage(70), // Paper details
        ])
        .split(f.area());

      // Create vertical layout for left pane (list + help)
      let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(main_chunks[0]);

      // Paper list
      let items: Vec<ListItem> = papers.iter().map(|p| ListItem::new(p.title.clone())).collect();

      let list = List::new(items)
        .block(
          Block::default()
            .title(Line::from(vec![
              Span::styled("📚 ", Style::default().fg(Color::LightBlue)),
              Span::styled("Papers", TITLE_STYLE),
              Span::styled(format!(" ({})", papers.len()), NORMAL_TEXT),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(HIGHLIGHT_STYLE)
        .highlight_symbol("▶ ");
      f.render_stateful_widget(list, left_chunks[0], &mut selected);

      // Help text
      let help = Paragraph::new(Line::from(vec![
        Span::styled("↑/k", HIGHLIGHT_KEY),
        Span::styled(": up", HELP_STYLE),
        Span::raw(" • "),
        Span::styled("↓/j", HIGHLIGHT_KEY),
        Span::styled(": down", HELP_STYLE),
        Span::raw(" • "),
        Span::styled("q", HIGHLIGHT_KEY),
        Span::styled(": quit", HELP_STYLE),
      ]));
      f.render_widget(help, left_chunks[1]);

      // Paper details (right pane)
      if let Some(i) = selected.selected() {
        let paper = &papers[i];
        let pdf_path = format!(
          "{}/{}.pdf",
          Database::default_pdf_path().display(),
          format_title(&paper.title, Some(50))
        );

        let pdf_exists = std::path::Path::new(&pdf_path).exists();
        let content = vec![
          Line::from(vec![
            Span::styled("Title: ", LABEL_STYLE),
            Span::styled(&paper.title, Style::default().fg(Color::White)),
          ]),
          Line::from(""),
          Line::from(vec![
            Span::styled("Authors: ", LABEL_STYLE),
            Span::styled(
              paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
              NORMAL_TEXT,
            ),
          ]),
          Line::from(""),
          Line::from(vec![
            Span::styled("Source: ", LABEL_STYLE),
            Span::styled(paper.source.to_string(), Style::default().fg(Color::LightYellow)),
            Span::raw(" ("),
            Span::styled(&paper.source_identifier, Style::default().fg(Color::LightYellow)),
            Span::raw(")"),
          ]),
          Line::from(""),
          Line::from(vec![Span::styled("Abstract:", LABEL_STYLE)]),
          Line::from(""),
          Line::from(Span::styled(&paper.abstract_text, NORMAL_TEXT)),
          Line::from(""),
          Line::from(vec![
            Span::styled("PDF Status: ", LABEL_STYLE),
            Span::styled(
              if pdf_exists {
                format!("✓ Available: {}", pdf_path)
              } else {
                "✗ Not downloaded".to_string()
              },
              if pdf_exists {
                Style::default().fg(Color::Green)
              } else {
                Style::default().fg(Color::Red)
              },
            ),
          ]),
        ];

        let details = Paragraph::new(content)
          .block(
            Block::default()
              .title(Line::from(vec![
                Span::styled("📄 ", Style::default().fg(Color::LightBlue)),
                Span::styled("Paper Details", TITLE_STYLE),
              ]))
              .borders(Borders::ALL)
              .border_style(Style::default().fg(Color::Blue)),
          )
          .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(details, main_chunks[1]);
      }

      // Render exit confirmation if active
      if let DialogState::ExitConfirm = dialog {
        let dialog_box =
          create_dialog_box("Exit Confirmation", "Are you sure you want to quit? (y/n)", f.area());
        f.render_widget(Clear, dialog_box);
        f.render_widget(
          Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(Span::styled("Exit Confirmation", Style::default().fg(Color::Red).bold())),
          dialog_box,
        );
        f.render_widget(
          Paragraph::new(vec![
            Line::from(Span::styled(
              "Are you sure you want to quit?",
              Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![
              Span::styled("y", HIGHLIGHT_KEY),
              Span::styled(": yes", HELP_STYLE),
              Span::raw(" • "),
              Span::styled("n", HIGHLIGHT_KEY),
              Span::styled(": no", HELP_STYLE),
            ]),
          ])
          .alignment(ratatui::layout::Alignment::Center),
          dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
        );
        f.render_widget(
          Paragraph::new("Are you sure you want to quit? (y/n)").style(Style::default()),
          dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
        );
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

/// Creates a centered dialog box with the given dimensions.
///
/// This helper function calculates the appropriate layout for a centered
/// dialog box based on the given title, message, and available screen space.
///
/// # Arguments
///
/// * `title` - The title to display at the top of the dialog
/// * `message` - The message to display in the dialog body
/// * `r` - The available screen area to center within
///
/// # Returns
///
/// Returns a `Rect` defining the position and size of the dialog box
fn create_dialog_box(title: &str, message: &str, r: Rect) -> Rect {
  let width = title.len().max(message.len()).max(40) as u16 + 4;
  let height = 3;
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length((r.height - height) / 2),
      Constraint::Length(height),
      Constraint::Length((r.height - height) / 2),
    ])
    .split(r);

  Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Length((r.width - width) / 2),
      Constraint::Length(width),
      Constraint::Length((r.width - width) / 2),
    ])
    .split(popup_layout[1])[1]
}
