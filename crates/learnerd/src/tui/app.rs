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
    }
  }

  pub fn handle_input(&mut self, key: KeyCode) -> bool {
    // Returns true if should quit
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
        KeyCode::Up | KeyCode::Char('k') => {
          let i = self.selected.selected().unwrap_or(0);
          if i > 0 {
            self.selected.select(Some(i - 1));
            self.needs_redraw = true;
          }
        },
        KeyCode::Down | KeyCode::Char('j') => {
          let i = self.selected.selected().unwrap_or(0);
          if i < self.papers.len().saturating_sub(1) {
            self.selected.select(Some(i + 1));
            self.needs_redraw = true;
          }
        },
        _ => {},
      },
    }
    false
  }

  /// Mark the UI as needing a redraw
  pub fn mark_needs_redraw(&mut self) { self.needs_redraw = true; }

  /// Get cached list items or create them
  pub fn get_list_items(&mut self) -> &[ListItem<'static>] {
    if self.cached_list_items.is_none() {
      self.cached_list_items =
        Some(self.papers.iter().map(|p| ListItem::new(p.title.clone())).collect());
    }
    self.cached_list_items.as_ref().unwrap()
  }

  /// Update layouts if terminal size changed
  pub fn update_layout(&mut self, frame_size: Rect) -> (Rect, Rect) {
    if self.last_size != frame_size {
      self.mark_needs_redraw();
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
}
