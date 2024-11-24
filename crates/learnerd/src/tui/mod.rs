//! Terminal User Interface for learnerd.
//!
//! This module provides an interactive terminal interface for managing and viewing academic papers,
//! built using the `ratatui` library. The interface offers:
//!
//! - A split-pane view with paper list and details
//! - Keyboard-driven navigation
//! - Real-time paper information display
//! - PDF availability status and opening
//! - Vim-style navigation controls
//! - Command mode for paper management
//!
//! # Navigation
//!
//! The interface supports both arrow keys and vim-style navigation:
//! - `↑`/`k`: Move selection up
//! - `↓`/`j`: Move selection down
//! - `←`/`h`: Focus left pane
//! - `→`/`l`: Focus right pane
//! - `:`: Enter command mode
//! - `o`: Open PDF (if available)
//! - `q`: Quit application

use std::io::{self, Stdout};

use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use learner::{
  database::{OrderField, Query},
  format::format_title,
  Config, Learner,
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use super::*;

mod state;
mod styles;
mod ui;

use interaction::{ResponseContent, UserInteraction};
use state::{DialogType, UIState};
use ui::UIDrawer;

/// Main TUI application struct that handles the interface and interactions
pub struct Tui {
  /// Terminal interface handler
  terminal: Terminal<CrosstermBackend<Stdout>>,
  /// Application state
  state:    UIState,
  /// Learner instance for paper management
  learner:  Learner,
}

impl Tui {
  /// Creates a new TUI instance
  pub async fn new(mut learner: Learner) -> Result<Self> {
    // Get initial paper list
    let papers =
      Query::list_all().order_by(OrderField::Title).execute(&mut learner.database).await?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    Ok(Self { terminal, state: UIState::new(papers), learner })
  }

  /// Runs the TUI main loop
  pub async fn run(&mut self) -> Result<()> {
    loop {
      if let Some(cmd) = self.state.pending_command.take() {
        if let Err(e) = self.execute_command(cmd).await {
          self.state.set_status_message(format!("Error: {}", e));
        }
      }
      // Draw UI if needed
      if self.state.needs_redraw {
        self.terminal.draw(|f| UIDrawer::new(f, &mut self.state).draw())?;
      }

      // Handle events
      if event::poll(std::time::Duration::from_millis(5))? {
        match event::read()? {
          Event::Key(key) =>
            if self.state.handle_input(key.code, key.modifiers) {
              break;
            },
          Event::Resize(..) => self.state.needs_redraw = true,
          _ => {},
        }
      }
    }

    // Cleanup and restore terminal
    self.cleanup()?;
    Ok(())
  }

  /// Cleans up the terminal state
  fn cleanup(&mut self) -> Result<()> {
    disable_raw_mode()?;
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    self.terminal.show_cursor()?;
    Ok(())
  }

  // TODO (autoparallel): This is definitely just replicating what these commands do to an extent.
  // This abstraction isn't good.
  pub async fn execute_command(&mut self, command: Commands) -> Result<()> {
    match command {
      Commands::Add(args) => {
        // If PDF flag not specified, show confirmation dialog
        if !args.pdf && !args.no_pdf {
          // First add without PDF
          let paper = add(self, AddArgs { no_pdf: true, ..args }).await?;
          self.state.dialog = DialogType::PDFConfirm { paper };
        } else {
          // Execute with specified flags
          add(self, args).await?;
          // Show success dialog
          self.state.dialog =
            DialogType::Success { message: "Paper added successfully".to_string() };
        }
        self.refresh_papers().await?;
      },
      Commands::Remove(args) => {
        // If not forced, show confirmation first
        if !args.force {
          // Find matching papers
          let mut matching_papers = Vec::new();

          // Get papers matching query
          let papers = Query::text(&args.query).execute(&mut self.learner.database).await?;

          // Apply filters if any
          for paper in papers {
            if let Some(author) = &args.filter.author {
              if !paper.authors.iter().any(|a| a.name.contains(author)) {
                continue;
              }
            }
            if let Some(source) = &args.filter.source {
              if paper.source.to_string() != *source {
                continue;
              }
            }
            if let Some(before) = &args.filter.before {
              if !paper.publication_date.to_string().starts_with(before) {
                continue;
              }
            }
            matching_papers.push(paper);
          }

          if matching_papers.is_empty() {
            self.state.set_status_message("No papers found matching criteria".to_string());
          } else {
            // Show confirmation dialog
            self.state.dialog = DialogType::RemoveConfirm {
              query: args.query.clone(),
              papers: matching_papers,
              args,
            };
          }
        } else {
          // Execute removal and show success
          remove(self, args).await?;
          self.state.dialog =
            DialogType::Success { message: "Papers removed successfully".to_string() };
          self.refresh_papers().await?;
        }
      },
      Commands::Search(args) => {
        // Perform the search
        let mut papers = Query::text(&args.query).execute(&mut self.learner.database).await?;

        // Apply filters if any
        if let Some(author) = &args.filter.author {
          papers.retain(|p| p.authors.iter().any(|a| a.name.contains(author)));
        }
        if let Some(source) = &args.filter.source {
          papers.retain(|p| p.source.to_string() == *source);
        }
        if let Some(before) = &args.filter.before {
          papers.retain(|p| p.publication_date.to_string().starts_with(before));
        }

        if papers.is_empty() {
          self.state.set_status_message("No papers found matching criteria".to_string());
        } else {
          // Show search results dialog
          let mut selected = ListState::default();
          selected.select(Some(0));
          self.state.dialog = DialogType::SearchResults { query: args.query, papers, selected };
        }
      },
      _ => return Err(LearnerdError::Daemon("Command not supported in TUI mode".to_string())),
    }
    Ok(())
  }

  async fn refresh_papers(&mut self) -> Result<()> {
    self.state.papers =
      Query::list_all().order_by(OrderField::Title).execute(&mut self.learner.database).await?;
    self.state.needs_redraw = true;
    Ok(())
  }
}

impl UserInteraction for Tui {
  fn learner(&mut self) -> &mut Learner { &mut self.learner }

  fn confirm(&mut self, message: &str) -> Result<bool> {
    // For now, just show the confirmation message and return true
    // TODO: Add proper confirmation dialog
    self.state.set_status_message(format!("Confirm: {}", message));
    Ok(true)
  }

  fn prompt(&mut self, message: &str) -> Result<String> {
    // For now, just show the prompt message and return empty string
    // TODO: Add proper prompt dialog
    self.state.set_status_message(format!("Prompt: {}", message));
    Ok(String::new())
  }

  fn reply(&mut self, content: ResponseContent) -> Result<()> {
    match content {
      ResponseContent::Success(msg) => {
        self.state.set_status_message(msg.to_string());
      },
      ResponseContent::Error(e) => {
        self.state.set_status_message(format!("Error: {}", e));
      },
      ResponseContent::Info(msg) => {
        self.state.set_status_message(msg.to_string());
      },
      ResponseContent::Paper(paper) => {
        // For now, just show paper title in status
        // TODO: Consider showing in a popup or updating the paper list
        self.state.set_status_message(format!("Paper: {}", paper.title));
      },
      ResponseContent::Papers(papers) => {
        // For now, just show count in status
        // TODO: Consider updating the paper list view
        self.state.set_status_message(format!("Found {} papers", papers.len()));
      },
    }
    self.state.needs_redraw = true;
    Ok(())
  }
}

/// Runs the Terminal User Interface.
pub async fn run(learner: Learner) -> Result<()> {
  let mut tui = Tui::new(learner).await?;
  tui.run().await
}
