use learner::paper::Paper;

use super::*;

/// Application state and UI caching
pub struct AppState {
  /// List of papers from the database
  pub papers:            Vec<Paper>,
  /// Current selection state in the paper list
  pub selected:          ListState,
  /// Current dialog state
  pub dialog:            DialogState,
  /// Whether the UI needs redrawing
  pub needs_redraw:      bool,
  /// Cache for main layout
  pub cached_layout:     Option<(Rect, Vec<Rect>, Vec<Rect>)>,
  /// Cached list items to avoid recreation
  pub cached_list_items: Option<Vec<ListItem<'static>>>,
  /// Last known terminal size
  pub last_size:         Rect,
  pub focused_pane:      FocusedPane,
  pub scroll_position:   usize,
  pub max_scroll:        Option<usize>,
}

#[derive(Debug, PartialEq)]
pub enum FocusedPane {
  List,
  Details,
}

impl AppState {
  pub fn new(papers: Vec<Paper>) -> Self {
    let mut selected = ListState::default();
    selected.select(Some(0));
    Self {
      papers,
      selected,
      dialog: DialogState::None,
      needs_redraw: true,
      cached_layout: None,
      cached_list_items: None,
      last_size: Rect::default(),
      focused_pane: FocusedPane::List,
      scroll_position: 0,
      max_scroll: None,
    }
  }

  pub fn handle_input(&mut self, key: KeyCode) -> bool {
    match self.dialog {
      DialogState::ExitConfirm => match key {
        KeyCode::Char('y') => return true,
        KeyCode::Char('n') | KeyCode::Esc => {
          self.dialog = DialogState::None;
          self.needs_redraw = true;
        },
        _ => {},
      },
      DialogState::None => match key {
        KeyCode::Char('q') => {
          self.dialog = DialogState::ExitConfirm;
          self.needs_redraw = true;
        },
        // Pane switching
        KeyCode::Left | KeyCode::Char('h') =>
          if self.focused_pane == FocusedPane::Details {
            self.focused_pane = FocusedPane::List;
            self.needs_redraw = true;
          },
        KeyCode::Right | KeyCode::Char('l') =>
          if self.focused_pane == FocusedPane::List {
            self.focused_pane = FocusedPane::Details;
            self.needs_redraw = true;
          },
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => match self.focused_pane {
          FocusedPane::List => {
            let i = self.selected.selected().unwrap_or(0);
            if i > 0 {
              self.selected.select(Some(i - 1));
              self.needs_redraw = true;
            }
          },
          FocusedPane::Details =>
            if self.scroll_position > 0 {
              self.scroll_position -= 1;
              self.needs_redraw = true;
            },
        },
        KeyCode::Down | KeyCode::Char('j') => match self.focused_pane {
          FocusedPane::List => {
            let i = self.selected.selected().unwrap_or(0);
            if i < self.papers.len().saturating_sub(1) {
              self.selected.select(Some(i + 1));
              self.needs_redraw = true;
            }
          },
          FocusedPane::Details =>
            if let Some(max) = self.max_scroll {
              if self.scroll_position < max {
                self.scroll_position += 1;
                self.needs_redraw = true;
              }
            },
        },
        // Page up/down for faster scrolling
        KeyCode::PageUp =>
          if self.focused_pane == FocusedPane::Details {
            self.scroll_position = self.scroll_position.saturating_sub(10);
            self.needs_redraw = true;
          },
        KeyCode::PageDown =>
          if self.focused_pane == FocusedPane::Details {
            if let Some(max) = self.max_scroll {
              self.scroll_position = (self.scroll_position + 10).min(max);
              self.needs_redraw = true;
            }
          },
        _ => {},
      },
    }
    false
  }

  /// Update layouts if terminal size changed
  pub fn update_layout(&mut self, frame_size: Rect) -> (Rect, Rect) {
    if self.last_size != frame_size {
      self.needs_redraw = true;
      self.last_size = frame_size;

      // Main split
      let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame_size);

      // Left pane split
      let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(main_chunks[0]);

      // Store all layouts
      self.cached_layout = Some((
        frame_size,           // Original frame size
        main_chunks.to_vec(), // Main horizontal split
        left_chunks.to_vec(), // Left pane vertical split
      ));

      (main_chunks[0], main_chunks[1])
    } else if let Some((_, main_chunks, _)) = &self.cached_layout {
      (main_chunks[0], main_chunks[1])
    } else {
      self.update_layout(frame_size)
    }
  }

  /// Get cached list items or create them
  pub fn get_list_items(&mut self) -> &[ListItem<'static>] {
    if self.cached_list_items.is_none() {
      self.cached_list_items =
        Some(self.papers.iter().map(|p| ListItem::new(p.title.clone())).collect());
    }
    self.cached_list_items.as_ref().unwrap()
  }
}
