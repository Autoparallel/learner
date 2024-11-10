//! Application state and logic for the TUI.

use std::{io, time::Duration};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use learner::{database::Database, paper::Paper};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::Mutex;

use super::{
  event::{Event, EventHandler},
  *,
};
use crate::errors::LearnerdErrors;

/// Represents the different views available in the application
#[derive(Debug, PartialEq, Eq)]
pub enum View {
  /// List of papers
  List,
  /// Detailed view of a single paper
  Detail,
  /// Search interface
  Search,
  /// Help screen
  Help,
}

/// Holds the application state and handles the main event loop.
pub struct App {
  /// Terminal backend for rendering
  pub terminal:      Terminal<CrosstermBackend<io::Stdout>>,
  /// Event handler for input
  pub events:        EventHandler,
  /// Database connection
  pub db:            Database,
  /// Whether the application should exit
  pub running:       bool,
  /// Current view
  pub view:          View,
  /// List of papers (cached)
  pub papers:        Vec<Paper>,
  /// Currently selected paper index
  pub selected:      Option<usize>,
  /// Search query
  pub search_query:  String,
  /// Whether we're currently in search input mode
  pub search_active: bool,
}

impl App {
  /// Creates a new application instance.
  pub async fn new() -> Result<Self, LearnerdErrors> {
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(Duration::from_millis(100));
    let db = Database::open(Database::default_path()).await?;

    Ok(Self {
      terminal,
      events,
      db,
      running: true,
      view: View::List,
      papers: Vec::new(),
      selected: None,
      search_query: String::new(),
      search_active: false,
    })
  }

  /// Runs the main application loop.
  pub async fn run(&mut self) -> Result<(), LearnerdErrors> {
    // Initial papers load
    self.load_papers().await?;

    while self.running {
      // Draw the UI
      self.terminal.draw(|f| {
        ui::draw(f, self).expect("Error drawing UI");
      })?;

      // Handle events
      if let Some(event) = self.events.next().await {
        match event {
          Event::Key(key) => self.handle_key_event(key).await?,
          Event::Resize(..) => {},
          Event::Tick => {},
        }
      }
    }

    Ok(())
  }

  /// Handles key events based on the current view and state.
  async fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), LearnerdErrors> {
    match key.code {
      KeyCode::Char('q') => self.running = false,
      KeyCode::Char('?') => self.view = View::Help,
      KeyCode::Char('/') => {
        self.view = View::Search;
        self.search_active = true;
      },
      KeyCode::Esc => {
        self.search_active = false;
        self.view = View::List;
      },
      _ => match self.view {
        View::List => self.handle_list_keys(key).await?,
        View::Detail => self.handle_detail_keys(key).await?,
        View::Search => self.handle_search_keys(key).await?,
        View::Help => self.handle_help_keys(key).await?,
      },
    }

    Ok(())
  }

  /// Loads or reloads papers from the database.
  async fn load_papers(&mut self) -> Result<(), LearnerdErrors> {
    if self.search_query.is_empty() {
      // TODO: Implement getting all papers
      self.papers = Vec::new();
    } else {
      self.papers = self.db.search_papers(&self.search_query).await?;
    }
    Ok(())
  }

  // TODO: Implement various key handlers for different views
  async fn handle_list_keys(&mut self, key: KeyEvent) -> Result<(), LearnerdErrors> {
    match key.code {
      KeyCode::Up | KeyCode::Char('k') =>
        if let Some(selected) = self.selected {
          self.selected = Some(selected.saturating_sub(1));
        } else if !self.papers.is_empty() {
          self.selected = Some(0);
        },
      KeyCode::Down | KeyCode::Char('j') =>
        if let Some(selected) = self.selected {
          if selected < self.papers.len().saturating_sub(1) {
            self.selected = Some(selected + 1);
          }
        } else if !self.papers.is_empty() {
          self.selected = Some(0);
        },
      KeyCode::Enter =>
        if self.selected.is_some() {
          self.view = View::Detail;
        },
      _ => {},
    }
    Ok(())
  }

  // Additional handler implementations...
}
