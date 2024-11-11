use app::FocusedPane;
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Style},
  text::{Line, Span},
  widgets::{Block, Borders, Clear, List, Padding, Paragraph, Wrap},
  Frame,
};

use super::*;

pub fn draw_ui(app: &mut AppState, f: &mut Frame) {
  let (left_area, right_area) = app.update_layout(f.area());

  // Left pane for list and help text
  draw_paper_list(app, f, left_area);

  // Right pane for paper details
  if let Some(i) = app.selected.selected() {
    draw_paper_details(i, f, right_area, app); // Updated signature
  }

  // Draw dialog if active
  if let DialogState::ExitConfirm = app.dialog {
    draw_exit_dialog(f);
  }
}

fn draw_paper_list(app: &mut AppState, f: &mut Frame, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(area);

  // Paper list with focus-aware styling
  let items = app.get_list_items();
  let list = List::new(items.to_vec())
    .block(
      Block::default()
        .title(Line::from(vec![
          Span::styled("📚 ", Style::default().fg(Color::LightBlue)),
          Span::styled("Papers", TITLE_STYLE),
          Span::styled(format!(" ({})", app.papers.len()), NORMAL_TEXT),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focused_pane == FocusedPane::List {
          Color::LightBlue
        } else {
          Color::Blue
        })),
    )
    .highlight_style(HIGHLIGHT_STYLE)
    .highlight_symbol("▶ ");

  f.render_stateful_widget(list, chunks[0], &mut app.selected);

  // Updated help text with new controls
  let help = Paragraph::new(Line::from(vec![
    Span::styled("↑/k", HIGHLIGHT_KEY),
    Span::styled(": up", HELP_STYLE),
    Span::raw(" • "),
    Span::styled("↓/j", HIGHLIGHT_KEY),
    Span::styled(": down", HELP_STYLE),
    Span::raw(" • "),
    Span::styled("←/h", HIGHLIGHT_KEY),
    Span::styled(": left pane", HELP_STYLE),
    Span::raw(" • "),
    Span::styled("→/l", HIGHLIGHT_KEY),
    Span::styled(": right pane", HELP_STYLE),
    Span::raw(" • "),
    Span::styled("PgUp/PgDn", HIGHLIGHT_KEY),
    Span::styled(": scroll", HELP_STYLE),
    Span::raw(" • "),
    Span::styled("q", HIGHLIGHT_KEY),
    Span::styled(": quit", HELP_STYLE),
  ]));
  f.render_widget(help, chunks[1]);
}

// TODO: This is a really awkward function signature now with the index being used. We should make
// these function signatures homogeneous.
fn draw_paper_details(paper_idx: usize, f: &mut Frame, area: Rect, app: &mut AppState) {
  let paper = &app.papers[paper_idx];
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints([
      Constraint::Length(3), // Title
      Constraint::Length(2), // Authors
      Constraint::Length(2), // Source
      Constraint::Length(1), // Abstract header
      Constraint::Min(5),    // Abstract content
      Constraint::Length(2), // PDF status
    ])
    .split(area);

  let details_block = Block::default()
    .title(Line::from(vec![
      Span::styled("📄 ", Style::default().fg(Color::LightBlue)),
      Span::styled("Paper Details", TITLE_STYLE),
    ]))
    .borders(Borders::ALL)
    .border_style(Style::default().fg(if app.focused_pane == FocusedPane::Details {
      Color::LightBlue
    } else {
      Color::Blue
    }));

  f.render_widget(details_block.clone(), area);

  // Title (with word wrap)
  let title = Paragraph::new(Line::from(vec![
    Span::styled("Title: ", LABEL_STYLE),
    Span::styled(&paper.title, Style::default().fg(Color::White)),
  ]))
  .wrap(Wrap { trim: true });
  f.render_widget(title, chunks[0]);

  // Authors
  let authors = Paragraph::new(Line::from(vec![
    Span::styled("Authors: ", LABEL_STYLE),
    Span::styled(
      paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
      NORMAL_TEXT,
    ),
  ]))
  .wrap(Wrap { trim: true });
  f.render_widget(authors, chunks[1]);

  // Source
  let source = Paragraph::new(Line::from(vec![
    Span::styled("Source: ", LABEL_STYLE),
    Span::styled(paper.source.to_string(), Style::default().fg(Color::LightYellow)),
    Span::raw(" ("),
    Span::styled(&paper.source_identifier, Style::default().fg(Color::LightYellow)),
    Span::raw(")"),
  ]));
  f.render_widget(source, chunks[2]);

  // Abstract header
  let abstract_header = Paragraph::new(Span::styled("Abstract:", LABEL_STYLE));
  f.render_widget(abstract_header, chunks[3]);

  // Abstract content with scrolling
  let abstract_text = normalize_whitespace(&paper.abstract_text);
  // TODO: should avoid this clone
  let abstract_content = Paragraph::new(abstract_text.clone())
    .style(NORMAL_TEXT)
    .wrap(Wrap { trim: true })
    .block(Block::default().padding(Padding::new(0, 1, 0, 0)))
    .scroll((app.scroll_position as u16, 0));

  // TODO (autoparallel): This doesn't work right as it's just counting new lines, we need to know
  // how long the text actually goes.
  // Calculate max scroll position
  let content_height = abstract_text.chars().filter(|c| *c == '\n').count() + 10;
  app.max_scroll = Some(content_height.saturating_sub(chunks[4].height as usize));

  f.render_widget(abstract_content, chunks[4]);

  // Scroll position indicator (if in details pane and scrollable)
  if app.focused_pane == FocusedPane::Details && app.max_scroll.unwrap_or(0) > 0 {
    let scroll_indicator = format!(" {}/{} ", app.scroll_position + 1, app.max_scroll.unwrap() + 1);
    let indicator_area = Rect {
      x:      area.x + area.width - scroll_indicator.len() as u16 - 1,
      y:      area.y + area.height - 2,
      width:  scroll_indicator.len() as u16,
      height: 1,
    };
    let scroll_text = Paragraph::new(scroll_indicator)
      .alignment(Alignment::Right)
      .style(Style::default().fg(Color::DarkGray));
    f.render_widget(scroll_text, indicator_area);
  }

  // PDF Status
  let pdf_path = format!(
    "{}/{}.pdf",
    Database::default_pdf_path().display(),
    format_title(&paper.title, Some(50))
  );
  let pdf_exists = std::path::Path::new(&pdf_path).exists();

  let status = Paragraph::new(Line::from(vec![
    Span::styled("PDF Status: ", LABEL_STYLE),
    Span::styled(
      if pdf_exists {
        format!("✓ Available: {}", pdf_path)
      } else {
        "✗ Not downloaded".to_string()
      },
      if pdf_exists { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) },
    ),
  ]))
  .wrap(Wrap { trim: true });
  f.render_widget(status, chunks[5]);
}

fn draw_exit_dialog(f: &mut Frame) {
  let area = f.area();
  let dialog_box =
    create_dialog_box("Exit Confirmation", "Are you sure you want to quit? (y/n)", area);

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
      Line::from(Span::styled("Are you sure you want to quit?", Style::default().fg(Color::White))),
      Line::from(""),
      Line::from(vec![
        Span::styled("y", HIGHLIGHT_KEY),
        Span::styled(": yes", HELP_STYLE),
        Span::raw(" • "),
        Span::styled("n", HIGHLIGHT_KEY),
        Span::styled(": no", HELP_STYLE),
      ]),
    ])
    .alignment(Alignment::Center),
    dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
  );
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

// TODO (autoparallel): This should maybe be handled back in the paper impl?
fn normalize_whitespace(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}
