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

use clap::{arg, Args};
use dialoguer::{Confirm, Input};
use interaction::{ResponseContent, UserInteraction, INFO};
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

  /// Remove a paper from the database by its source and identifier
  Remove {
    /// Source system (arxiv, doi, iacr)
    #[arg(value_enum)]
    source: String,

    /// Paper identifier in the source system
    identifier: String,
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
      ResponseContent::Paper(paper, detailed) => {
        println!("\n{} Paper details:", style(PAPER).green());
        println!("   {} {}", style("Title:").green().bold(), style(&paper.title).white());
        println!(
          "   {} {}",
          style("Authors:").green().bold(),
          style(paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
            .white()
        );

        if detailed {
          println!(
            "   {} {}",
            style("Abstract:").green().bold(),
            style(&paper.abstract_text).white()
          );
          println!(
            "   {} {}",
            style("Published:").green().bold(),
            style(&paper.publication_date).white()
          );
          if let Some(url) = &paper.pdf_url {
            println!("   {} {}", style("PDF URL:").green().bold(), style(url).blue().underlined());
          }
          if let Some(doi) = &paper.doi {
            println!("   {} {}", style("DOI:").green().bold(), style(doi).blue().underlined());
          }
        }
      },
      ResponseContent::Papers(papers) => {
        if papers.is_empty() {
          println!("{} No papers found", style(WARNING).yellow());
          return Ok(());
        }

        println!("\n{} Found {} papers:", style(SUCCESS).green(), style(papers.len()).yellow());
        for (i, paper) in papers.iter().enumerate() {
          println!("\n{}. {}", style(i + 1).yellow(), style(&paper.title).white().bold());

          let authors = paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();
          let author_display = if authors.is_empty() {
            style("No authors listed").red().italic().to_string()
          } else {
            style(authors.join(", ")).white().to_string()
          };
          println!("   {} {}", style("Authors:").green(), author_display);

          // Show a preview of the abstract
          if !paper.abstract_text.is_empty() {
            let preview = paper.abstract_text.chars().take(100).collect::<String>();
            let preview =
              if paper.abstract_text.len() > 100 { format!("{}...", preview) } else { preview };
            println!("   {} {}", style("Abstract:").green(), style(preview).white().italic());
          }
        }
      },
      ResponseContent::Success(message) => {
        println!("{} {}", style(SUCCESS).green(), style(message).white());
      },
      ResponseContent::Error(error) => {
        println!("{} {}", style(ERROR).red(), style(error).red());
      },
      ResponseContent::Info(message) => {
        println!("{} {}", style(INFO).blue(), style(message).white());
      },
    }
    Ok(())
  }
}
