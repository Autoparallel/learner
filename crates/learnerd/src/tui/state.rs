use crossterm::event::KeyCode;
use learner::{database::Database, format::format_title, paper::Paper};
use ratatui::widgets::ListState;

#[derive(Debug, PartialEq)]
pub enum FocusedPane {
  List,
  Details,
}

#[derive(Debug)]
pub enum DialogType {
  None,
  ExitConfirm,
  PDFNotFound,
}

pub struct UIState {
  pub papers:          Vec<Paper>,
  pub selected:        ListState,
  pub dialog:          DialogType,
  pub focused_pane:    FocusedPane,
  pub scroll_position: usize,
  pub max_scroll:      Option<usize>,
  pub needs_redraw:    bool,
}

impl UIState {
  pub fn new(papers: Vec<Paper>) -> Self {
    let mut selected = ListState::default();
    selected.select(Some(0));
    Self {
      papers,
      selected,
      dialog: DialogType::None,
      focused_pane: FocusedPane::List,
      scroll_position: 0,
      max_scroll: None,
      needs_redraw: true,
    }
  }

  pub fn selected_paper(&self) -> Option<&Paper> {
    self.selected.selected().map(|i| &self.papers[i])
  }

  pub fn handle_input(&mut self, key: KeyCode) -> bool {
    match self.dialog {
      DialogType::ExitConfirm => self.handle_exit_dialog(key),
      DialogType::PDFNotFound => self.handle_pdf_not_found_dialog(key),
      DialogType::None => self.handle_normal_input(key),
    }
  }

  fn handle_exit_dialog(&mut self, key: KeyCode) -> bool {
    match key {
      KeyCode::Char('y') => true,
      KeyCode::Char('n') | KeyCode::Esc => {
        self.dialog = DialogType::None;
        self.needs_redraw = true;
        false
      },
      _ => false,
    }
  }

  fn handle_pdf_not_found_dialog(&mut self, key: KeyCode) -> bool {
    if key == KeyCode::Enter {
      self.dialog = DialogType::None;
      self.needs_redraw = true;
    }
    false
  }

  fn handle_normal_input(&mut self, key: KeyCode) -> bool {
    match key {
      KeyCode::Char('q') => {
        self.dialog = DialogType::ExitConfirm;
        self.needs_redraw = true;
        false
      },
      // Pane switching
      KeyCode::Left | KeyCode::Char('h') => {
        if self.focused_pane == FocusedPane::Details {
          self.focused_pane = FocusedPane::List;
          self.needs_redraw = true;
        }
        false
      },
      KeyCode::Right | KeyCode::Char('l') => {
        if self.focused_pane == FocusedPane::List {
          self.focused_pane = FocusedPane::Details;
          self.needs_redraw = true;
        }
        false
      },
      // Navigation
      KeyCode::Up | KeyCode::Char('k') => {
        self.handle_up_navigation();
        false
      },
      KeyCode::Down | KeyCode::Char('j') => {
        self.handle_down_navigation();
        false
      },
      KeyCode::Char('o') => {
        self.handle_open_pdf();
        false
      },
      _ => false,
    }
  }

  fn handle_up_navigation(&mut self) {
    match self.focused_pane {
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
    }
  }

  fn handle_down_navigation(&mut self) {
    match self.focused_pane {
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
    }
  }

  fn handle_open_pdf(&mut self) {
    if let Some(paper) = self.selected_paper() {
      let pdf_path = format!(
        "{}/{}.pdf",
        Database::default_pdf_path().display(),
        format_title(&paper.title, Some(50))
      );

      if std::path::Path::new(&pdf_path).exists() {
        self.open_pdf_with_system_viewer(&pdf_path);
      } else {
        self.dialog = DialogType::PDFNotFound;
        self.needs_redraw = true;
      }
    }
  }

  #[cfg(target_os = "windows")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", path]).spawn();
  }

  #[cfg(target_os = "macos")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("open").arg(path).spawn();
  }

  #[cfg(target_os = "linux")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
  }

  pub fn update_max_scroll(&mut self, available_lines: usize, visible_lines: usize) {
    self.max_scroll = Some(available_lines.saturating_sub(visible_lines));
  }
}
