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
use event::KeyModifiers;
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
  /// Dialog used when entering a command
  CommandInput,
  /// Dialog used when confirming a PDF download
  PDFConfirm {
    /// The paper to download a PDF for
    paper: Paper,
  },
  /// Dialog used when removing papers
  RemoveConfirm {
    /// The list of papers that will be removed
    papers: Vec<Paper>,
    /// The additional arguments used for removal
    args:   RemoveArgs,
  },
  /// The results of search made
  SearchResults {
    /// The search term used
    query:    String,
    /// The papers that match the search
    papers:   Vec<Paper>,
    /// Which paper in the list is currently selected
    selected: ListState,
  },
  /// Message displayed when an operation has occured successfully
  Success {
    /// The message to display
    message: String,
  },
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
  /// Status message to display
  pub status_message:  Option<String>,
  /// Stores the current state of the entered command
  pub command_buffer:  CommandBuffer,
  /// The command that is to be executed
  pub pending_command: Option<Commands>,
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
      status_message: None,
      command_buffer: CommandBuffer::new(),
      pending_command: None,
    }
  }

  /// Sets a status message to display
  pub fn set_status_message(&mut self, message: String) {
    self.status_message = Some(message);
    self.needs_redraw = true;
  }

  /// Returns a reference to the currently selected paper.
  ///
  /// Returns None if no paper is selected (should never happen in practice
  /// as we always maintain a selection).
  pub fn selected_paper(&self) -> Option<&Paper> {
    self.selected.selected().map(|i| &self.papers[i])
  }

  /// Handles button inputs in the home page
  pub fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
    match &self.dialog {
      DialogType::ExitConfirm => self.handle_exit_dialog(key),
      DialogType::PDFNotFound => self.handle_pdf_not_found_dialog(key),
      DialogType::CommandInput { .. } => self.handle_command_input(key, modifiers),
      DialogType::RemoveConfirm { .. } => self.handle_remove_confirm(key),
      DialogType::SearchResults { .. } => self.handle_search_results(key),
      DialogType::PDFConfirm { .. } => self.handle_pdf_confirm(key),
      DialogType::Success { .. } => {
        if matches!(key, KeyCode::Enter | KeyCode::Esc) {
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        }
        false
      },
      DialogType::None => self.handle_normal_input(key),
    }
  }

  /// Handles the search result pop up
  fn handle_search_results(&mut self, key: KeyCode) -> bool {
    if let DialogType::SearchResults { papers, selected, .. } = &mut self.dialog {
      match key {
        KeyCode::Up | KeyCode::Char('k') => {
          // Move selection up
          if let Some(i) = selected.selected() {
            if i > 0 {
              selected.select(Some(i - 1));
              self.needs_redraw = true;
            }
          }
        },
        KeyCode::Down | KeyCode::Char('j') => {
          // Move selection down
          if let Some(i) = selected.selected() {
            if i < papers.len() - 1 {
              selected.select(Some(i + 1));
              self.needs_redraw = true;
            }
          }
        },
        KeyCode::Enter => {
          // Find the selected paper in the main list and focus on it
          if let Some(selected_idx) = selected.selected() {
            let selected_paper = &papers[selected_idx];

            // Find this paper in the main list
            if let Some(main_idx) = self.papers.iter().position(|p| {
              p.source == selected_paper.source
                && p.source_identifier == selected_paper.source_identifier
            }) {
              // Focus on the paper in the main list
              self.selected.select(Some(main_idx));
            }
          }
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        KeyCode::Esc => {
          // Cancel search
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        _ => {},
      }
    }
    false
  }

  /// Handles the remove paper pop up
  fn handle_remove_confirm(&mut self, key: KeyCode) -> bool {
    if let DialogType::RemoveConfirm { args, .. } = &self.dialog {
      match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
          // Clone args and set force to true to bypass further confirmations
          let mut confirmed_args = args.clone();
          confirmed_args.force = true;
          self.pending_command = Some(Commands::Remove(confirmed_args));
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        _ => {},
      }
    }
    false
  }

  /// Handles inputs when typing a command
  fn handle_command_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
    match (key, modifiers) {
      (key, KeyModifiers::NONE) => match key {
        KeyCode::Esc => {
          self.command_buffer.reset();
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        KeyCode::Enter => {
          if let Some(cmd) = self.command_buffer.try_execute() {
            // Set the pending command and exit command mode
            self.pending_command = Some(cmd);
            self.command_buffer.reset();
            self.dialog = DialogType::None;
          }
          self.needs_redraw = true;
        },
        KeyCode::Char(c) => {
          self.command_buffer.insert_char(c);
          self.needs_redraw = true;
        },
        KeyCode::Backspace => {
          self.command_buffer.backspace();
          self.needs_redraw = true;
        },
        KeyCode::Left => {
          self.command_buffer.move_cursor_left();
          self.needs_redraw = true;
        },
        KeyCode::Right => {
          self.command_buffer.move_cursor_right();
          self.needs_redraw = true;
        },
        KeyCode::Up => {
          self.command_buffer.previous_history();
          self.needs_redraw = true;
        },
        KeyCode::Down => {
          self.command_buffer.next_history();
          self.needs_redraw = true;
        },
        KeyCode::Tab => {
          // Get completions
          let completions = self.command_buffer.get_completions();
          if completions.len() == 1 {
            // Single completion - use it
            let parts: Vec<&str> = self.command_buffer.input.split_whitespace().collect();
            let new_input = if parts.len() <= 1 {
              // Completing command
              format!("{} ", completions[0])
            } else {
              // Completing flag
              let base =
                &self.command_buffer.input[..self.command_buffer.input.rfind(' ').unwrap() + 1];
              format!("{}{} ", base, completions[0])
            };
            self.command_buffer.input = new_input;
            self.command_buffer.cursor_position = self.command_buffer.input.len();
          }
          self.needs_redraw = true;
        },
        _ => {},
      },
      (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
        self.command_buffer.delete_word();
        self.needs_redraw = true;
      },

      _ => {},
    }
    false
  }

  /// Handles the confirmation popup when asked to download a PDF
  fn handle_pdf_confirm(&mut self, key: KeyCode) -> bool {
    if let DialogType::PDFConfirm { paper } = &self.dialog {
      match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
          // Store paper for command execution
          self.pending_command = Some(Commands::Add(AddArgs {
            identifier: paper.source_identifier.clone(),
            pdf:        true,
            no_pdf:     false,
          }));
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
          self.dialog = DialogType::None;
          self.needs_redraw = true;
        },
        _ => {},
      }
    }
    false
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
        self.dialog = DialogType::CommandInput;
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

/// The buffer for writing commands into
#[derive(Default, Debug)]
pub struct CommandBuffer {
  /// Current command text
  pub input:            String,
  /// Cursor position within the text
  pub cursor_position:  usize,
  /// Command history
  pub history:          Vec<String>,
  /// Current position when navigating history (-1 means current input)
  pub history_position: isize,
  /// Saves current input when navigating history
  pub current_input:    String,
  /// Current error message, if any
  pub error:            Option<String>,
}

impl CommandBuffer {
  /// Creates a new buffer for commands ran in the TUI
  pub fn new() -> Self {
    Self {
      input:            String::new(),
      cursor_position:  0,
      history:          Vec::new(),
      history_position: -1,
      current_input:    String::new(),
      error:            None,
    }
  }

  /// Try to execute the current command
  pub fn try_execute(&mut self) -> Option<Commands> {
    self.error = None;

    // Trim the input to remove any leading/trailing whitespace
    let input = self.input.trim();

    // Skip empty commands
    if input.is_empty() {
      return None;
    }

    // Parse the command
    match Commands::from_str(input) {
      Ok(cmd) => {
        // Add to history only if successful
        if !input.is_empty() {
          self.history.push(input.to_string());
        }
        self.reset();
        Some(cmd)
      },
      Err(e) => {
        self.error = Some(e);
        None
      },
    }
  }

  /// Try to get command completion suggestions
  pub fn get_completions(&self) -> Vec<String> {
    let input = self.input.trim();

    // No input - show all commands
    if input.is_empty() {
      return Commands::command_list().iter().map(|&s| s.to_string()).collect();
    }

    // Split into command and current word
    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.first() {
      // If we just have a partial command, complete the command
      Some(&cmd) if parts.len() == 1 => Commands::command_list()
        .iter()
        .filter(|c| c.starts_with(cmd))
        .map(|&s| s.to_string())
        .collect(),
      // If we have a command and are starting a flag
      Some(&cmd) if parts.last().unwrap().starts_with("--") => {
        let current = parts.last().unwrap();
        Commands::flags_for_command(cmd)
          .iter()
          .filter(|f| f.starts_with(current))
          .map(|&s| s.to_string())
          .collect()
      },
      // Otherwise no completions
      _ => Vec::new(),
    }
  }

  /// Insert character at current cursor position
  pub fn insert_char(&mut self, c: char) {
    self.error = None;
    self.input.insert(self.cursor_position, c);
    self.cursor_position += 1;
  }

  /// Delete character before cursor
  pub fn backspace(&mut self) {
    self.error = None;
    if self.cursor_position > 0 {
      self.cursor_position -= 1;
      self.input.remove(self.cursor_position);
    }
  }

  /// Move cursor left
  pub fn move_cursor_left(&mut self) {
    if self.cursor_position > 0 {
      self.cursor_position -= 1;
    }
  }

  /// Move cursor right
  pub fn move_cursor_right(&mut self) {
    if self.cursor_position < self.input.len() {
      self.cursor_position += 1;
    }
  }

  /// Delete word before cursor
  pub fn delete_word(&mut self) {
    self.error = None;
    // Find the start of the current word
    let mut word_start = self.cursor_position;
    while word_start > 0 && !self.input[..word_start].chars().last().unwrap().is_whitespace() {
      word_start -= 1;
    }
    // Remove from word start to cursor
    self.input.replace_range(word_start..self.cursor_position, "");
    self.cursor_position = word_start;
  }

  /// Navigate to previous command in history
  pub fn previous_history(&mut self) {
    if self.history.is_empty() {
      return;
    }

    // Save current input if just starting history navigation
    if self.history_position == -1 {
      self.current_input = self.input.clone();
    }

    // Move up in history if possible
    if self.history_position < (self.history.len() as isize - 1) {
      self.history_position += 1;
      self.input = self.history[self.history.len() - 1 - self.history_position as usize].clone();
      self.cursor_position = self.input.len();
    }
  }

  /// Navigate to next command in history
  pub fn next_history(&mut self) {
    if self.history_position >= 0 {
      self.history_position -= 1;
      if self.history_position == -1 {
        self.input = self.current_input.clone();
      } else {
        self.input = self.history[self.history.len() - 1 - self.history_position as usize].clone();
      }
      self.cursor_position = self.input.len();
    }
  }

  /// Reset the command buffer
  pub fn reset(&mut self) {
    self.input.clear();
    self.cursor_position = 0;
    self.history_position = -1;
    self.current_input.clear();
    self.error = None;
  }
}
