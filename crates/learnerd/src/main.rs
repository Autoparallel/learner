//! Command line interface and daemon for the learner paper management system.
//!
//! This crate provides a CLI tool for managing academic papers using the `learner` library.
//! It supports operations like:
//! - Database initialization and management
//! - Paper addition and retrieval
//! - Full-text search across papers
//! - Database maintenance and cleanup
//!
//! # Usage
//!
//! ```bash
//! # Initialize a new database
//! learnerd init
//!
//! # Add a paper by its identifier
//! learnerd add 2301.07041
//!
//! # Retrieve a paper
//! learnerd get arxiv 2301.07041
//!
//! # Search for papers
//! learnerd search "neural networks"
//!
//! # Clean up the database
//! learnerd clean
//! ```
//!
//! The CLI provides colored output and interactive confirmations for destructive
//! operations. It also supports various verbosity levels for debugging through
//! the `-v` flag.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use std::path::PathBuf;

use clap::{builder::ArgAction, Parser, Subcommand};
use console::style;
use error::LearnerdError;
use learner::{database::Database, error::LearnerError, paper::Paper, prelude::*, Config, Learner};
use tracing_subscriber::EnvFilter;

pub mod commands;
pub mod daemon;
pub mod error;
pub mod interaction;
#[cfg(feature = "tui")] pub mod tui;

use crate::{commands::*, daemon::*, error::*};

/// Prefix for information messages
static INFO_PREFIX: &str = "ℹ ";
/// Prefix for success messages
static SUCCESS_PREFIX: &str = "✓ ";
/// Prefix for warning messages
static WARNING_PREFIX: &str = "⚠️ ";
/// Prefix for error messages
static ERROR_PREFIX: &str = "✗ ";
/// Prefix for user prompts
static PROMPT_PREFIX: &str = "❯ ";
/// Continuation line for tree structure
static CONTINUE_PREFIX: &str = "│  ";
/// Vertical line for tree structure
static TREE_VERT: &str = "│";
/// Branch character for tree structure
static TREE_BRANCH: &str = "├";
/// Leaf character for tree structure (end of branch)
static TREE_LEAF: &str = "└";

/// Command line interface configuration and argument parsing
#[derive(Parser)]
#[command(author, version, about = "Daemon and CLI for the learner paper management system")]
pub struct CliArgs {
  /// Verbose mode (-v, -vv, -vvv) for different levels of logging detail
  #[arg(
        short,
        long,
        action = ArgAction::Count,
        global = true,
        help = "Increase logging verbosity"
    )]
  verbose: u8,

  /// Path to the database file. This is where the database will be created or referenced from. If
  /// not specified, uses the default platform-specific data directory.
  #[arg(long, short, global = true)]
  path: Option<PathBuf>,

  /// The subcommand to execute
  #[command(subcommand)]
  command: Option<Commands>,

  /// Skip all prompts and accept defaults (mostly for testing)
  #[arg(long, hide = true, global = true)]
  accept_defaults: bool,
}

pub struct Cli {
  args:    CliArgs,
  learner: Option<Learner>,
}

/// Configures the logging system based on the verbosity level
///
/// # Arguments
///
/// * `verbosity` - Number of times the verbose flag was used (0-3)
///
/// The verbosity levels are:
/// - 0: warn (default)
/// - 1: info
/// - 2: debug
/// - 3+: trace
fn setup_logging(verbosity: u8) {
  let filter = match verbosity {
    0 => "error",
    1 => "warn",
    2 => "info",
    3 => "debug",
    _ => "trace",
  };

  let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

  tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_file(true)
    .with_line_number(true)
    .with_thread_ids(true)
    .with_target(true)
    .init();
}

/// Entry point for the learnerd CLI application
///
/// Handles command line argument parsing, sets up logging, and executes
/// the requested command. All commands provide colored output and
/// interactive confirmations for destructive operations.
///
/// # Errors
///
/// Returns `LearnerdErrors` for various failure conditions including:
/// - Database operations failures
/// - Paper fetching failures
/// - File system errors
/// - User interaction errors
#[tokio::main]
async fn main() -> Result<()> {
  let args = CliArgs::parse();

  // Handle the command, using TUI as default when enabled
  let command = args.command.clone().unwrap_or_else(|| {
    #[cfg(feature = "tui")]
    return Commands::Tui;

    #[cfg(not(feature = "tui"))]
    {
      println!("Please specify a command. Use --help for usage information.");
      std::process::exit(1);
    }
  });

  if let Commands::Daemon { .. } = command {
  } else {
    setup_logging(args.verbose);
  }

  let mut cli = Cli { args, learner: None };
  // Initialize learner unless it's an init command
  if !matches!(command, Commands::Init(_)) {
    cli.learner = Some(Learner::from_path(Config::default_path()?).await?);
  }

  match command {
    Commands::Init(init_options) => init(&mut cli, init_options).await,
    Commands::Add(add_options) => add(&mut cli, add_options).await,
    Commands::Remove(remove_options) => remove(&mut cli, remove_options).await,
    Commands::Search(search_options) => search(&mut cli, search_options).await,
    Commands::Daemon { cmd } => daemon(cmd).await,
    #[cfg(feature = "tui")]
    Commands::Tui =>
      if let Some(learner) = cli.learner.take() {
        tui::run(learner).await
      } else {
        Err(LearnerdError::from(LearnerError::Config("Failed to initialize learner".to_string())))
      },
  }
}
