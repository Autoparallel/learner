//! Command implementations for the learner CLI and TUI.
//!
//! This module organizes all available commands for managing academic papers and the
//! learner system. It provides functionality for:
//!
//! - Paper Management
//!   - Adding papers from various sources (arXiv, DOI, IACR)
//!   - Retrieving paper details
//!   - Searching across papers
//!   - Removing papers
//!
//! - PDF Management
//!   - Downloading PDFs
//!   - Managing PDF storage
//!
//! - System Management
//!   - Database initialization
//!   - Database cleanup
//!   - Daemon control
//!
//! # Usage
//!
//! Commands can be used both from the CLI and the TUI (when enabled):
//!
//! ```bash
//! # Initialize database
//! learner init
//!
//! # Add a paper
//! learner add 2301.07041
//!
//! # Search papers
//! learner search "quantum computing"
//!
//! # Get paper details
//! learner get arxiv 2301.07041
//! ```
//!
//! When the TUI feature is enabled, running `learner` with no commands launches
//! the interactive terminal interface.
//!
//! # Command Organization
//!
//! Each command is implemented in its own module, with the main command enum
//! [`Commands`] providing the interface between the CLI parser and the individual
//! implementations. Commands are designed to be usable both from the CLI and
//! the TUI contexts.
//!
//! # Feature Flags
//!
//! - `tui`: Enables the Terminal User Interface and makes it the default when no command is
//!   specified.

use super::*;

pub mod add;

pub mod daemon;

pub mod init;
pub mod remove;
pub mod search;

use chrono::{DateTime, Utc};
use clap::{arg, Args};
use dialoguer::{Confirm, Input};
use interaction::*;
use learner::database::{Add, Query};

pub use self::{add::add, daemon::daemon, init::init, remove::remove, search::search};

// TODO: Make these take `&str` and hold a lifetime here?
/// Available commands for the CLI
#[derive(Subcommand, Clone)]
pub enum Commands {
  /// Launch the Terminal User Interface (default when no command specified)
  #[cfg(feature = "tui")]
  #[clap(hide = true)] // Hide from help since it's the default
  Tui,

  /// Initialize a new learner database
  Init,

  /// Add a paper to the database by its identifier
  Add {
    identifier: String,
    #[arg(long, group = "pdf_behavior")]
    pdf:        bool,
    #[arg(long, group = "pdf_behavior")]
    no_pdf:     bool,
  },

  /// Remove papers from the database
  Remove {
    /// Paper identifier or search terms
    query: String,

    #[command(flatten)]
    filter: SearchFilter,

    /// Show what would be removed without actually removing
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    force: bool,

    /// PDF handling
    #[arg(long, group = "pdf_behavior")]
    remove_pdf: bool,

    /// Keep PDFs when removing papers
    #[arg(long, group = "pdf_behavior")]
    keep_pdf: bool,
  },

  Search {
    /// Search query - supports full text search
    query: String,

    /// Show detailed paper information
    #[arg(long)]
    detailed: bool,

    #[command(flatten)]
    filter: SearchFilter,
  },

  /// Manage the learnerd daemon
  Daemon {
    /// The set of commands specifically for managing the [`Daemon`].
    #[command(subcommand)]
    cmd: DaemonCommands,
  },
}

#[derive(Args, Clone)]
pub struct SearchFilter {
  /// Filter by author name
  #[arg(long)]
  author: Option<String>,

  /// Filter by paper source (arxiv, doi, iacr)
  #[arg(long)]
  source: Option<String>,

  /// Filter by publication date (YYYY-MM-DD)
  #[arg(long)]
  before: Option<String>,
  // TODO (autoparallel): Allow for proper scoped searches
  // /// Search only titles
  // #[arg(long, group = "search_scope")]
  // title_only: bool,

  // /// Search only abstracts
  // #[arg(long, group = "search_scope")]
  // abstract_only: bool,
}

impl UserInteraction for Cli {
  fn confirm(&self, message: &str) -> Result<bool> {
    // Check if pdf flags are present
    if let Some(Commands::Add { pdf, no_pdf, .. }) = self.command {
      return Ok(if pdf {
        true
      } else if no_pdf {
        false
      } else if self.accept_defaults {
        false
      } else {
        Confirm::new().with_prompt(message).default(false).interact()?
      });
    }

    // Default behavior for other commands
    if self.accept_defaults {
      return Ok(false);
    }

    Confirm::new()
      .with_prompt(message)
      .default(false)
      .interact()
      .map_err(|e| LearnerdError::Interaction(e.to_string()))
  }

  fn prompt(&self, message: &str) -> Result<String> {
    Input::new()
      .with_prompt(message)
      .interact_text()
      .map_err(|e| LearnerdError::Interaction(e.to_string()))
  }

  fn reply(&self, content: ResponseContent) -> Result<()> {
    match content {
      ResponseContent::Papers(papers) => {
        if papers.is_empty() {
          println!("{} {} No papers found", style(TREE_LEAF).cyan(), style(ERROR_PREFIX).red());
          return Ok(());
        }

        // println!("{} Searching for: {}", style(TREE_VERT).cyan(), style(&query).white());
        println!(
          "{} {} Found {} papers:",
          style(TREE_VERT).cyan(),
          style(SUCCESS_PREFIX).green(),
          style(papers.len()).yellow()
        );

        for (i, paper) in papers.iter().enumerate() {
          let prefix = if i == papers.len() - 1 { TREE_LEAF } else { TREE_BRANCH };
          println!("\n{} {}", style(prefix).cyan(), style(&paper.title).white().bold());
          println!(
            "{}   {}: {}",
            style(TREE_VERT).cyan(),
            style("Authors").green().bold(),
            style(&paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
              .white()
          );
        }
      },
      ResponseContent::Paper(paper) => {
        println!("{} Paper details:", style(TREE_VERT).cyan());
        println!("{} {}", style(TREE_LEAF).cyan(), style(&paper.title).white().bold());
        println!(
          "{}   {}: {}",
          style(TREE_VERT).cyan(),
          style("Authors").green().bold(),
          style(&paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
            .white()
        );

        println!("{}   {}:", style(TREE_VERT).cyan(), style("Abstract").green().bold());
        println!("{}   {}", style(TREE_VERT).cyan(), style(&paper.abstract_text).white());
        println!(
          "{}   {}: {}",
          style(TREE_VERT).cyan(),
          style("Published").green().bold(),
          style(&paper.publication_date).white()
        );

        // PDF information - show regardless of detailed flag since it's important metadata
        if let Some(url) = &paper.pdf_url {
          println!(
            "{}   {}: {}",
            style(TREE_VERT).cyan(),
            style("PDF URL").green().bold(),
            style(url).blue().underlined()
          );

          // Check PDF status
          // TODO (autoparallel): This is not good, we need to modify this to use a path known from
          // the command
          let pdf_path = Database::default_storage_path().join(paper.filename());
          if pdf_path.exists() {
            println!(
              "{}   {} PDF available at: {}",
              style(TREE_VERT).cyan(),
              style(SUCCESS_PREFIX).green(),
              style(pdf_path.display()).white()
            );
          } else {
            println!(
              "{}   {} PDF not downloaded",
              style(TREE_VERT).cyan(),
              style(ERROR_PREFIX).yellow()
            );
          }
        } else {
          println!(
            "{}   {} No PDF available",
            style(TREE_VERT).cyan(),
            style(ERROR_PREFIX).yellow()
          );
        }
      },
      ResponseContent::Success(message) => {
        println!("{} {}", style(SUCCESS_PREFIX).green(), style(message).white());
      },
      ResponseContent::Error(error) => {
        println!("{} {}", style(ERROR_PREFIX).red(), style(error).red());
      },
      ResponseContent::Info(message) => {
        println!("{} {}", style(INFO_PREFIX).cyan(), style(message).white());
      },
    }
    Ok(())
  }
}

fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
  let parsed = if date_str.len() == 4 {
    // Just year provided
    DateTime::parse_from_str(&format!("{}-01-01 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  } else if date_str.len() == 7 {
    // Year and month provided
    DateTime::parse_from_str(&format!("{}-01 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  } else {
    // Full date provided
    DateTime::parse_from_str(&format!("{} 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  }
  .map_err(|e| LearnerdError::Interaction(format!("Invalid date format: {}", e)))?;

  Ok(parsed.with_timezone(&Utc))
}
