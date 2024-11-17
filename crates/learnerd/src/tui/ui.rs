//! Drawing and layout management for the Terminal User Interface.
//!
//! This module handles all rendering aspects of the TUI, including:
//! - Layout management
//! - Widget drawing
//! - Dialog rendering
//! - Content formatting
//!
//! The module uses a drawer pattern where each component has its own
//! specialized drawing method, making the code modular and maintainable.
//! Layout is handled through constraint-based positioning, ensuring
//! proper scaling across different terminal sizes.

use learner::format::format_title;
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Style},
  text::{Line, Span},
  widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
  Frame,
};

use super::{
  state::{DialogType, FocusedPane, UIState},
  *,
};

/// Main drawer struct responsible for rendering the UI.
///
/// Holds references to both the frame being drawn and the current UI state.
/// These references are kept separate to avoid borrow checker issues while
/// still maintaining access to all necessary drawing context.
pub struct UIDrawer<'a, 'b> {
  /// Reference to the current frame being rendered.
  ///
  /// This is provided by ratatui's Terminal::draw callback and represents
  /// the current rendering context. It provides methods to render widgets
  /// and maintains the terminal buffer.
  frame: &'a mut Frame<'b>,

  /// Reference to the current UI state.
  ///
  /// Contains all the dynamic state of the UI including:
  /// - Paper list and selection
  /// - Current focus and scroll positions
  /// - Active dialogs
  /// - Redraw flags
  state: &'a mut UIState,
}

impl<'a, 'b> UIDrawer<'a, 'b> {
  /// Creates a new drawer instance.
  pub fn new(frame: &'a mut Frame<'b>, state: &'a mut UIState) -> Self { Self { frame, state } }

  /// Main drawing entry point, handles the entire UI render.
  ///
  /// This method:
  /// 1. Splits the layout into main sections
  /// 2. Draws the paper list and details
  /// 3. Handles any active dialogs
  /// 4. Updates the redraw state
  pub fn draw(&mut self) {
    let frame_size = self.frame.area();
    let (left_area, right_area) = self.split_layout(frame_size);

    self.draw_paper_list(left_area);
    // TODO (autoparallel): prefer not to clone if we can.
    if let Some(paper) = self.state.selected_paper().cloned() {
      self.draw_paper_details(&paper, right_area);
    }

    match self.state.dialog {
      DialogType::ExitConfirm => self.draw_exit_dialog(),
      DialogType::PDFNotFound => self.draw_pdf_not_found_dialog(),
      DialogType::None => {},
    }

    self.state.needs_redraw = false;
  }

  /// Splits the main layout into left and right panes.
  ///
  /// The layout uses a 30/70 split for optimal content display.
  fn split_layout(&self, area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
      .split(area);
    (chunks[0], chunks[1])
  }

  /// Draws the paper list with its help bar.
  ///
  /// Creates a list widget showing all paper titles with:
  /// - Selection highlighting
  /// - Border styling based on focus
  /// - Paper count in the title
  fn draw_paper_list(&mut self, area: Rect) {
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(0), Constraint::Length(1)])
      .split(area);

    let items: Vec<ListItem> =
      self.state.papers.iter().map(|p| ListItem::new(p.title.clone())).collect();

    let list = List::new(items)
      .block(
        Block::default()
          .title(Line::from(vec![
            Span::styled("📚 ", Style::default().fg(Color::LightBlue)),
            Span::styled("Papers", styles::TITLE),
            Span::styled(format!(" ({})", self.state.papers.len()), styles::NORMAL),
          ]))
          .borders(Borders::ALL)
          .border_style(styles::border_style(self.state.focused_pane == FocusedPane::List)),
      )
      .highlight_style(styles::HIGHLIGHT)
      .highlight_symbol("▶ ");

    self.frame.render_stateful_widget(list, chunks[0], &mut self.state.selected);

    self.draw_help_bar(chunks[1]);
  }

  /// Draws the help bar showing available commands.
  ///
  /// Shows keyboard shortcuts and their actions in a
  /// compact, readable format.
  fn draw_help_bar(&mut self, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
      Span::styled("↑↓←→", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
      Span::styled(":nav", styles::HELP),
      Span::styled(" • ", Style::default().fg(Color::Blue)),
      Span::styled("o", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
      Span::styled(":open", styles::HELP),
      Span::styled(" • ", Style::default().fg(Color::Blue)),
      Span::styled("q", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
      Span::styled(":quit", styles::HELP),
    ]));
    self.frame.render_widget(help, area);
  }

  /// Draws the detailed view of a paper.
  ///
  /// Shows:
  /// - Title
  /// - Authors
  /// - Source information
  /// - Abstract (scrollable)
  /// - PDF status
  fn draw_paper_details(&mut self, paper: &learner::paper::Paper, area: Rect) {
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
        Span::styled("Paper Details", styles::TITLE),
      ]))
      .borders(Borders::ALL)
      .border_style(styles::border_style(self.state.focused_pane == FocusedPane::Details));

    self.frame.render_widget(details_block.clone(), area);

    self.draw_title(paper, chunks[0]);
    self.draw_authors(paper, chunks[1]);
    self.draw_source(paper, chunks[2]);
    self.draw_abstract(paper, chunks[3], chunks[4]);
    self.draw_pdf_status(paper, chunks[5]);

    if self.state.focused_pane == FocusedPane::Details {
      self.draw_scroll_indicator(area);
    }
  }

  /// Draws the paper title section.
  ///
  /// Renders the title with:
  /// - A "Title:" label in the defined label style
  /// - The actual title in white
  /// - Word wrapping enabled for long titles
  fn draw_title(&mut self, paper: &learner::paper::Paper, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
      Span::styled("Title: ", styles::LABEL),
      Span::styled(&paper.title, Style::default().fg(Color::White)),
    ]))
    .wrap(Wrap { trim: true });
    self.frame.render_widget(title, area);
  }

  /// Draws the paper authors section.
  ///
  /// Displays all authors as a comma-separated list with:
  /// - An "Authors:" label in the defined label style
  /// - Author names in the normal text style
  /// - Word wrapping for long author lists
  fn draw_authors(&mut self, paper: &learner::paper::Paper, area: Rect) {
    let authors = Paragraph::new(Line::from(vec![
      Span::styled("Authors: ", styles::LABEL),
      Span::styled(
        paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "),
        styles::NORMAL,
      ),
    ]))
    .wrap(Wrap { trim: true });
    self.frame.render_widget(authors, area);
  }

  /// Draws the paper source information.
  ///
  /// Shows the paper's source system and identifier with:
  /// - A "Source:" label
  /// - The source type (e.g., "arXiv", "DOI")
  /// - The source-specific identifier in parentheses
  /// - Both source and identifier in light yellow
  fn draw_source(&mut self, paper: &learner::paper::Paper, area: Rect) {
    let source = Paragraph::new(Line::from(vec![
      Span::styled("Source: ", styles::LABEL),
      Span::styled(paper.source.to_string(), Style::default().fg(Color::LightYellow)),
      Span::raw(" ("),
      Span::styled(&paper.source_identifier, Style::default().fg(Color::LightYellow)),
      Span::raw(")"),
    ]));
    self.frame.render_widget(source, area);
  }

  /// Draws the paper's abstract with header and content.
  ///
  /// Renders the abstract in two parts:
  /// - A header section with the "Abstract:" label
  /// - The main content area with:
  ///   - Normalized whitespace
  ///   - Word wrapping
  ///   - Scrolling support
  ///   - Left padding for better readability
  ///
  /// Also updates the maximum scroll position based on content length.
  ///
  /// # Arguments
  ///
  /// * `paper` - The paper whose abstract is being displayed
  /// * `header_area` - The area for the "Abstract:" label
  /// * `content_area` - The area for the abstract text
  fn draw_abstract(
    &mut self,
    paper: &learner::paper::Paper,
    header_area: Rect,
    content_area: Rect,
  ) {
    let abstract_header = Paragraph::new(Span::styled("Abstract:", styles::LABEL));
    self.frame.render_widget(abstract_header, header_area);

    let abstract_text = normalize_whitespace(&paper.abstract_text);
    let lines = calculate_abstract_lines(&abstract_text, content_area);

    let abstract_content = Paragraph::new(abstract_text)
      .style(styles::NORMAL)
      .wrap(Wrap { trim: true })
      .block(Block::default().padding(Padding::new(0, 1, 0, 0)))
      .scroll((self.state.scroll_position as u16, 0));

    self.state.update_max_scroll(lines, content_area.height as usize);
    self.frame.render_widget(abstract_content, content_area);
  }

  /// Draws the PDF availability status.
  ///
  /// Shows the current status of the paper's PDF:
  /// - A checkmark (✓) and path in green if the PDF is available
  /// - A cross (✗) in red if the PDF is not downloaded
  /// - The full path where the PDF is/would be stored
  /// - Word wrapping for long paths
  fn draw_pdf_status(&mut self, paper: &learner::paper::Paper, area: Rect) {
    let pdf_path = format!(
      "{}/{}.pdf",
      Database::default_storage_path().display(),
      format_title(&paper.title, Some(50))
    );
    let pdf_exists = std::path::Path::new(&pdf_path).exists();

    let status = Paragraph::new(Line::from(vec![
      Span::styled("PDF Status: ", styles::LABEL),
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
    ]))
    .wrap(Wrap { trim: true });
    self.frame.render_widget(status, area);
  }

  /// Draws the scroll position indicator when viewing long content.
  ///
  /// Only appears when:
  /// - The details pane is focused
  /// - The content is long enough to scroll
  ///
  /// Shows the current position in the format "current/total"
  /// aligned to the right side of the details pane.
  fn draw_scroll_indicator(&mut self, area: Rect) {
    if let Some(max_scroll) = self.state.max_scroll {
      if max_scroll > 0 {
        let scroll_indicator = format!(" {}/{} ", self.state.scroll_position + 1, max_scroll + 1);
        let indicator_area = Rect {
          x:      area.x + area.width - scroll_indicator.len() as u16 - 1,
          y:      area.y + area.height - 2,
          width:  scroll_indicator.len() as u16,
          height: 1,
        };
        let scroll_text =
          Paragraph::new(scroll_indicator).alignment(Alignment::Right).style(styles::HELP);
        self.frame.render_widget(scroll_text, indicator_area);
      }
    }
  }

  /// Draws the exit confirmation dialog.
  ///
  /// Shows a centered dialog box asking the user to confirm exit with:
  /// - A clear question
  /// - Instructions for confirming ('y') or canceling ('n')
  /// - Red border to indicate a destructive action
  /// - Proper spacing and alignment
  fn draw_exit_dialog(&mut self) {
    let content = vec![
      Line::from(Span::styled("Are you sure you want to quit?", Style::default().fg(Color::White))),
      Line::from(""),
      Line::from(vec![
        Span::styled("Press ", styles::HELP),
        Span::styled("y", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" to confirm, ", styles::HELP),
        Span::styled("n", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" to cancel", styles::HELP),
      ]),
    ];

    self.draw_dialog("Exit Confirmation", &content, Color::Red);
  }

  /// Draws the PDF not found error dialog.
  ///
  /// Shows a centered dialog explaining that the PDF is not available:
  /// - Clear error message
  /// - Instructions for downloading
  /// - Information about how to dismiss the dialog
  /// - Yellow border to indicate a warning state
  fn draw_pdf_not_found_dialog(&mut self) {
    let content = vec![
      Line::from(Span::styled(
        "The PDF file for this paper has not been downloaded.",
        Style::default().fg(Color::White),
      )),
      Line::from(""),
      Line::from(Span::styled("Use the download command to get the PDF first.", styles::HELP)),
      Line::from(""),
      Line::from(vec![
        Span::styled("Press ", styles::HELP),
        Span::styled("Enter", styles::KEY_HIGHLIGHT.add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" to continue", styles::HELP),
      ]),
    ];

    self.draw_dialog("PDF Not Found", &content, Color::Yellow);
  }

  /// Draws a centered dialog box with the given content.
  ///
  /// # Arguments
  ///
  /// * `title` - Dialog title
  /// * `content` - Vector of lines to display
  /// * `color` - Color theme for the dialog
  fn draw_dialog(&mut self, title: &str, content: &[Line], color: Color) {
    let area = self.frame.area();
    let dialog_box = create_dialog_box(title, content, area);

    self.frame.render_widget(Clear, dialog_box);
    self.frame.render_widget(
      Block::default().borders(Borders::ALL).border_style(Style::default().fg(color)).title(
        Span::styled(
          title,
          Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD),
        ),
      ),
      dialog_box,
    );

    self.frame.render_widget(
      Paragraph::new(content.to_vec()).alignment(Alignment::Center),
      dialog_box.inner(Margin { vertical: 1, horizontal: 2 }),
    );
  }
}

/// Creates a centered dialog box with appropriate dimensions.
///
/// Calculates the size based on content and positions the dialog
/// in the center of the screen.
fn create_dialog_box(title: &str, content: &[Line], r: Rect) -> Rect {
  let content_width = content.iter().map(|line| line.width()).max().unwrap_or(0);
  let width = title.len().max(content_width).max(40) as u16 + 4;
  let height = (content.len() as u16) + 2;

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

/// Normalizes whitespace in text for consistent display.
///
/// Converts multiple spaces and newlines into single spaces
/// for clean presentation.
fn normalize_whitespace(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Calculates how many lines a text will occupy given a width constraint.
///
/// Used for determining scroll limits and content positioning.
///
/// # Arguments
///
/// * `text` - The text to calculate lines for
/// * `area` - The rectangle defining the available space
fn calculate_abstract_lines(text: &str, area: Rect) -> usize {
  text
    .lines()
    .flat_map(|line| {
      let mut wrapped_lines = Vec::new();
      let mut current_line = String::new();
      let available_width = area.width.saturating_sub(2) as usize;

      for word in line.split_whitespace() {
        if current_line.len() + word.len() < available_width {
          if !current_line.is_empty() {
            current_line.push(' ');
          }
          current_line.push_str(word);
        } else {
          if !current_line.is_empty() {
            wrapped_lines.push(current_line);
          }
          current_line = word.to_string();
        }
      }
      if !current_line.is_empty() {
        wrapped_lines.push(current_line);
      }
      wrapped_lines
    })
    .count()
}
