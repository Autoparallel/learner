use super::*;

pub fn draw_ui(app: &mut AppState, f: &mut ratatui::Frame) {
  let (left_area, right_area) = app.update_layout(f.area());

  // Left pane split for list and help text
  let left_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(left_area);

  // Paper list
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
        .border_style(Style::default().fg(Color::Blue)),
    )
    .highlight_style(HIGHLIGHT_STYLE)
    .highlight_symbol("▶ ");
  f.render_stateful_widget(list, left_chunks[0], &mut app.selected);

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
  if let Some(i) = app.selected.selected() {
    let paper = &app.papers[i];
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
    f.render_widget(details, right_area);
  }

  // Render exit confirmation if active
  if let DialogState::ExitConfirm = app.dialog {
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
  }
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
