//! Command line interface implementations and user interactions.
//!
//! This module organizes all available commands for managing academic papers and provides
//! a consistent interface for user interactions. It provides functionality for:
//!
//! - Paper Management
//!   - Adding papers from various sources (arXiv, DOI, IACR)
//!   - Searching and filtering papers
//!   - Removing papers
//!
//! - PDF Management
//!   - Configurable PDF downloading
//!   - PDF cleanup during paper removal
//!
//! - System Management
//!   - Database initialization
//!   - Daemon control
//!
//! # Usage
//!
//! ```bash
//! # Initialize database
//! learner init
//!
//! # Add a paper with PDF
//! learner add 2301.07041 --pdf
//!
//! # Search papers with filters
//! learner search "quantum" --author "Alice" --before 2023
//!
//! # Remove papers with confirmation
//! learner remove "quantum computing"
//!
//! # Remove papers without confirmation
//! learner remove "quantum computing" --force --remove-pdf
//! ```
//!
//! # Command Organization
//!
//! The module is structured around two main components:
//!
//! - [`Commands`]: The main command enum providing the interface for CLI parsing
//! - [`UserInteraction`]: Trait implementation for consistent terminal output and user prompts
//!
//! All output uses a consistent visual style with tree structures and color-coded indicators
//! for better readability and user experience.

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

pub use self::{add::*, daemon::*, init::*, remove::*, search::*};

/// Available commands for the CLI
#[derive(Subcommand, Clone)]
pub enum Commands {
  /// Launch the Terminal User Interface (default when no command specified)
  #[cfg(feature = "tui")]
  #[clap(hide = true)]
  Tui,

  /// Initialize a new learner database
  Init(InitArgs),

  /// Add a paper to the database by its identifier
  Add(AddArgs),

  /// Remove papers from the database
  Remove(RemoveArgs),

  /// Search for papers in the database
  Search(SearchArgs),

  /// Manage the learnerd daemon
  Daemon {
    /// Commands for managing the daemon
    #[command(subcommand)]
    cmd: DaemonCommands,
  },
}

impl FromStr for Commands {
  type Err = String;

  /// Parse a command string into a Commands variant
  fn from_str(input: &str) -> std::result::Result<Self, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.first().copied() {
      Some("add") => Self::parse_add(&parts[1..]),
      Some("remove") => Self::parse_remove(&parts[1..]),
      Some("search") => Self::parse_search(&parts[1..]),
      Some(unknown) => Err(format!("Unknown command: {}", unknown)),
      None => Err("No command entered".to_string()),
    }
  }
}

impl Commands {
  /// Parse arguments for the add command
  fn parse_add(args: &[&str]) -> std::result::Result<Self, String> {
    let mut add_args = AddArgs { identifier: String::new(), pdf: false, no_pdf: false };

    let mut i = 0;
    while i < args.len() {
      match args[i] {
        "--pdf" => {
          if add_args.no_pdf {
            return Err("Cannot specify both --pdf and --no-pdf".to_string());
          }
          add_args.pdf = true;
        },
        "--no-pdf" => {
          if add_args.pdf {
            return Err("Cannot specify both --pdf and --no-pdf".to_string());
          }
          add_args.no_pdf = true;
        },
        s if !s.starts_with("--") => {
          if !add_args.identifier.is_empty() {
            return Err("Multiple identifiers specified".to_string());
          }
          add_args.identifier = s.to_string();
        },
        unknown => return Err(format!("Unknown flag: {}", unknown)),
      }
      i += 1;
    }

    if add_args.identifier.is_empty() {
      return Err("Missing paper identifier".to_string());
    }

    Ok(Commands::Add(add_args))
  }

  /// Parse arguments for the remove command
  fn parse_remove(args: &[&str]) -> std::result::Result<Self, String> {
    let mut remove_args = RemoveArgs {
      query:      String::new(),
      filter:     SearchFilter { author: None, source: None, before: None },
      dry_run:    false,
      force:      false,
      remove_pdf: false,
      keep_pdf:   false,
    };

    let mut i = 0;
    while i < args.len() {
      match args[i] {
        "--dry-run" => remove_args.dry_run = true,
        "--force" => remove_args.force = true,
        "--remove-pdf" => {
          if remove_args.keep_pdf {
            return Err("Cannot specify both --remove-pdf and --keep-pdf".to_string());
          }
          remove_args.remove_pdf = true;
        },
        "--keep-pdf" => {
          if remove_args.remove_pdf {
            return Err("Cannot specify both --remove-pdf and --keep-pdf".to_string());
          }
          remove_args.keep_pdf = true;
        },
        "--author" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --author".to_string());
          }
          remove_args.filter.author = Some(args[i].to_string());
        },
        "--source" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --source".to_string());
          }
          remove_args.filter.source = Some(args[i].to_string());
        },
        "--before" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --before".to_string());
          }
          remove_args.filter.before = Some(args[i].to_string());
        },
        s if !s.starts_with("--") => {
          if !remove_args.query.is_empty() {
            return Err("Multiple queries specified".to_string());
          }
          remove_args.query = s.to_string();
        },
        unknown => return Err(format!("Unknown flag: {}", unknown)),
      }
      i += 1;
    }

    if remove_args.query.is_empty() {
      return Err("Missing search query".to_string());
    }

    Ok(Commands::Remove(remove_args))
  }

  /// Parse arguments for the search command
  fn parse_search(args: &[&str]) -> std::result::Result<Self, String> {
    let mut search_args = SearchArgs {
      query:    String::new(),
      detailed: false,
      filter:   SearchFilter { author: None, source: None, before: None },
    };

    let mut i = 0;
    while i < args.len() {
      match args[i] {
        "--detailed" => search_args.detailed = true,
        "--author" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --author".to_string());
          }
          search_args.filter.author = Some(args[i].to_string());
        },
        "--source" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --source".to_string());
          }
          search_args.filter.source = Some(args[i].to_string());
        },
        "--before" => {
          i += 1;
          if i >= args.len() {
            return Err("Missing value for --before".to_string());
          }
          search_args.filter.before = Some(args[i].to_string());
        },
        s if !s.starts_with("--") => {
          if !search_args.query.is_empty() {
            return Err("Multiple queries specified".to_string());
          }
          search_args.query = s.to_string();
        },
        unknown => return Err(format!("Unknown flag: {}", unknown)),
      }
      i += 1;
    }

    if search_args.query.is_empty() {
      return Err("Missing search query".to_string());
    }

    Ok(Commands::Search(search_args))
  }

  /// Get help text for this command
  pub fn help_text(&self) -> &'static str {
    match self {
      Commands::Add(_) =>
        "Usage: add <identifier> [--pdf|--no-pdf]\nAdd a paper to the database by its identifier \
         (arXiv ID, DOI, or IACR ID)",
      Commands::Remove(_) =>
        "Usage: remove <query> [--force] [--dry-run] [--remove-pdf|--keep-pdf] [--author <name>] \
         [--source <source>] [--before <date>]\nRemove papers matching the query from the database",
      Commands::Search(_) =>
        "Usage: search <query> [--detailed] [--author <name>] [--source <source>] [--before \
         <date>]\nSearch for papers in the database",
      _ => "Command help not available",
    }
  }

  /// Get a list of all available commands
  pub fn command_list() -> &'static [&'static str] { &["add", "remove", "search"] }

  /// Get a list of all flags for a command
  pub fn flags_for_command(command: &str) -> &'static [&'static str] {
    match command {
      "add" => &["--pdf", "--no-pdf"],
      "remove" =>
        &["--force", "--dry-run", "--remove-pdf", "--keep-pdf", "--author", "--source", "--before"],
      "search" => &["--detailed", "--author", "--source", "--before"],
      _ => &[],
    }
  }
}

impl UserInteraction for Cli {
  fn learner(&mut self) -> &mut Learner { self.learner.as_mut().expect("Learner not initialized") }

  /// Request confirmation from the user
  ///
  /// Displays a yes/no prompt with the given message and returns the user's choice.
  /// If `accept_defaults` is true, automatically returns false without prompting.
  fn confirm(&mut self, message: &str) -> Result<bool> {
    println!("\n{} {}", style(PROMPT_PREFIX).yellow(), style(message).yellow().bold());

    if self.args.accept_defaults {
      return Ok(false);
    }

    let theme = dialoguer::theme::ColorfulTheme::default();
    Ok(Confirm::with_theme(&theme).with_prompt("").default(false).interact()?)
  }

  /// Request text input from the user
  ///
  /// Displays a prompt for free-form text input and returns the user's response.
  fn prompt(&mut self, message: &str) -> Result<String> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    Ok(Input::with_theme(&theme).with_prompt(message).interact_text()?)
  }

  /// Display content to the user
  ///
  /// Handles different types of content with appropriate formatting:
  /// - Paper listings with tree structure
  /// - Detailed paper information
  /// - Success/error/info messages
  fn reply(&mut self, content: ResponseContent) -> Result<()> {
    match content {
      ResponseContent::Papers(papers) => {
        if papers.is_empty() {
          println!("{} No papers found", style(ERROR_PREFIX).red());
          return Ok(());
        }

        println!(
          "{} Found {} papers:",
          style(SUCCESS_PREFIX).green(),
          style(papers.len()).yellow()
        );

        for (i, paper) in papers.iter().enumerate() {
          let is_last = i == papers.len() - 1;
          let prefix = if is_last { TREE_LEAF } else { TREE_BRANCH };

          println!("{} {}", style(prefix).cyan(), style(&paper.title).white().bold());

          let continuation = if is_last { "   " } else { CONTINUE_PREFIX };
          println!(
            "{}Authors: {}",
            style(continuation).cyan(),
            style(&paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
              .white()
          );
        }

        // Only show tip once for multiple results, without tree line
        if papers.len() > 1 {
          println!("\nTip: Use --author, --source, or --before together to further refine results");
        }
      },
      ResponseContent::Paper(paper) => {
        println!("{} Paper details:", style(TREE_VERT).cyan());
        println!("{} {}", style(TREE_BRANCH).cyan(), style(&paper.title).white().bold());

        println!(
          "{}   Authors: {}",
          style(TREE_BRANCH).cyan(),
          style(&paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
            .white()
        );

        println!("{}   Abstract:", style(TREE_BRANCH).cyan());

        let width = 80;
        let mut current_line = String::new();
        for word in paper.abstract_text.split_whitespace() {
          if current_line.len() + word.len() + 1 > width {
            println!("{}   {}", style(TREE_VERT).cyan(), style(&current_line).white());
            current_line.clear();
          }
          if !current_line.is_empty() {
            current_line.push(' ');
          }
          current_line.push_str(word);
        }
        if !current_line.is_empty() {
          println!("{}   {}", style(TREE_VERT).cyan(), style(&current_line).white());
        }

        println!(
          "{}   Published: {}",
          style(TREE_BRANCH).cyan(), // Changed to TREE_LEAF
          style(&paper.publication_date).white()
        );

        // The following don't use tree characters
        if let Some(url) = &paper.pdf_url {
          println!("{}   PDF URL: {}", style(TREE_BRANCH).cyan(), style(url).blue().underlined());

          let pdf_path = self.learner().config.storage_path.join(paper.filename());
          if pdf_path.exists() {
            println!(
              "{}   {} PDF available at:",
              style(TREE_LEAF).cyan(),
              style(SUCCESS_PREFIX).green()
            );
            println!("      {}", style(pdf_path.display()).white());
          } else {
            println!(
              "{}   {} PDF not downloaded",
              style(TREE_LEAF).cyan(),
              style(ERROR_PREFIX).yellow()
            );
          }
        }
      },
      ResponseContent::Success(message) => {
        println!("{} {}", style(SUCCESS_PREFIX).green(), style(message).white());
      },
      ResponseContent::Info(message) => {
        println!("{} {}", style(INFO_PREFIX).green(), style(message).white());
      },
      ResponseContent::Error(error) => {
        println!("{} {}", style(ERROR_PREFIX).red(), style(error).red());
      },
    }
    Ok(())
  }
}

/// Parse a date string into a UTC DateTime
///
/// Supports multiple date formats:
/// - Year only (YYYY)
/// - Year and month (YYYY-MM)
/// - Full date (YYYY-MM-DD)
///
/// # Arguments
///
/// * `date_str` - Date string to parse
///
/// # Returns
///
/// Returns a Result containing either the parsed UTC DateTime or an error
/// if the date format is invalid.
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
  }?;

  Ok(parsed.with_timezone(&Utc))
}
