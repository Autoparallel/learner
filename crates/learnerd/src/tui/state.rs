//! State management for the Terminal User Interface.
//!
//! This module handles the application's state, including:
//! - Paper list management
//! - Selection and focus tracking
//! - Dialog management
//! - Input handling
//! - Scrolling state
//!
//! The state is designed to be self-contained and manages all user interactions
//! and view updates through a clean state transition system.

use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

use super::*;

/// Represents which pane currently has focus in the UI.
///
/// Used to determine which pane receives keyboard input and
/// how to style the UI elements.
#[derive(Debug, PartialEq)]
pub enum FocusedPane {
  /// The paper list on the left side is focused
  List,
  /// The paper details on the right side is focused
  Details,
}

/// Represents the current active dialog in the UI.
///
/// Used to manage modal dialogs and their specific input handling.
#[derive(Debug)]
pub enum DialogType {
  /// No dialog is currently active
  None,
  /// Showing the exit confirmation dialog
  ExitConfirm,
  /// Showing the PDF not found error dialog
  PDFNotFound,
  CommandInput {
    input: String,
  },
}

/// Represents which mode the TUI is currently in
#[derive(Debug, PartialEq)]
pub enum Mode {
  /// Normal mode for navigation and viewing
  Normal,
  /// Command mode for entering commands
  Command,
}

/// Maintains the complete state of the terminal interface.
pub struct UIState {
  /// List of papers from the database
  pub papers:          Vec<Paper>,
  /// Current selection state in the paper list
  pub selected:        ListState,
  /// Current active dialog (if any)
  pub dialog:          DialogType,
  /// Which pane currently has focus
  pub focused_pane:    FocusedPane,
  /// Current scroll position in the details view
  pub scroll_position: usize,
  /// Maximum scroll position based on content
  pub max_scroll:      Option<usize>,
  /// Whether the UI needs to be redrawn
  pub needs_redraw:    bool,
  /// Current UI mode
  pub mode:            Mode,
  /// Status message to display
  pub status_message:  Option<String>,
}

impl UIState {
  /// Creates a new UI state with the given papers.
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
      mode: Mode::Normal,
      status_message: None,
    }
  }

  /// Sets a status message to display
  pub fn set_status_message(&mut self, message: String) {
    self.status_message = Some(message);
    self.needs_redraw = true;
  }

  /// Clears the current status message
  pub fn clear_status_message(&mut self) {
    self.status_message = None;
    self.needs_redraw = true;
  }

  /// Returns a reference to the currently selected paper.
  ///
  /// Returns None if no paper is selected (should never happen in practice
  /// as we always maintain a selection).
  pub fn selected_paper(&self) -> Option<&Paper> {
    self.selected.selected().map(|i| &self.papers[i])
  }

  pub fn handle_input(&mut self, key: KeyCode) -> bool {
    match &self.dialog {
      DialogType::ExitConfirm => self.handle_exit_dialog(key),
      DialogType::PDFNotFound => self.handle_pdf_not_found_dialog(key),
      DialogType::CommandInput { .. } => self.handle_command_input(key),
      DialogType::None => self.handle_normal_input(key),
    }
  }

  fn handle_command_input(&mut self, key: KeyCode) -> bool {
    match key {
      KeyCode::Esc => {
        self.dialog = DialogType::None;
        self.needs_redraw = true;
        false
      },
      KeyCode::Char(c) => {
        if let DialogType::CommandInput { input } = &mut self.dialog {
          input.push(c);
          self.needs_redraw = true;
        }
        false
      },
      KeyCode::Backspace => {
        if let DialogType::CommandInput { input } = &mut self.dialog {
          input.pop();
          self.needs_redraw = true;
        }
        false
      },
      _ => false,
    }
  }

  /// Handles input while the exit confirmation dialog is active.
  ///
  /// Returns true only if user confirms exit.
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

  /// Handles input while the PDF not found dialog is active.
  fn handle_pdf_not_found_dialog(&mut self, key: KeyCode) -> bool {
    if key == KeyCode::Enter {
      self.dialog = DialogType::None;
      self.needs_redraw = true;
    }
    false
  }

  /// Handles input during normal operation (no dialog active).
  ///
  /// Supports:
  /// - Vim-style navigation (h,j,k,l)
  /// - Arrow key navigation
  /// - Pane switching
  /// - PDF opening
  /// - Quit command
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
      KeyCode::Char(':') => {
        self.dialog = DialogType::CommandInput { input: String::new() };
        self.needs_redraw = true;
        false
      },
      _ => false,
    }
  }

  /// Handles upward navigation in both list and details views.
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

  /// Handles downward navigation in both list and details views.
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

  /// Attempts to open the selected paper's PDF with the system viewer.
  ///
  /// Shows an error dialog if the PDF file is not found.
  fn handle_open_pdf(&mut self) {
    if let Some(paper) = self.selected_paper() {
      let pdf_path = format!(
        "{}/{}.pdf",
        Database::default_storage_path().display(),
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

  /// Updates the maximum scroll position for the details view.
  ///
  /// # Arguments
  ///
  /// * `available_lines` - Total number of lines in the content
  /// * `visible_lines` - Number of lines that can be displayed at once
  pub fn update_max_scroll(&mut self, available_lines: usize, visible_lines: usize) {
    self.max_scroll = Some(available_lines.saturating_sub(visible_lines));
  }

  /// Opens a PDF file using the Windows system viewer.
  #[cfg(target_os = "windows")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", path]).spawn();
  }

  /// Opens a PDF file using the macOS system viewer.
  #[cfg(target_os = "macos")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("open").arg(path).spawn();
  }

  /// Opens a PDF file using the Linux system viewer.
  #[cfg(target_os = "linux")]
  fn open_pdf_with_system_viewer(&self, path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
  }
}
