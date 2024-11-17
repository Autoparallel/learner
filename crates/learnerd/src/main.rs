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

use std::{path::PathBuf, str::FromStr};

use clap::{builder::ArgAction, Parser, Subcommand};
use console::{style, Emoji};
use error::LearnerdError;
use learner::{
  database::Database,
  error::LearnerError,
  paper::{Paper, Source},
  prelude::*,
};
use tracing::{debug, trace};
use tracing_subscriber::EnvFilter;

pub mod commands;
pub mod daemon;
pub mod error;
#[cfg(feature = "tui")] pub mod tui;

use crate::{commands::*, daemon::*, error::*};

// Emoji constants for prettier output
/// Search operation indicator
static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍 ", "");
/// Database/library operations indicator
static BOOKS: Emoji<'_, '_> = Emoji("📚 ", "");
/// Initialization/startup indicator
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
/// Paper details indicator
static PAPER: Emoji<'_, '_> = Emoji("📄 ", "");
/// Save operation indicator
static SAVE: Emoji<'_, '_> = Emoji("💾 ", "");
/// Warning indicator
static WARNING: Emoji<'_, '_> = Emoji("⚠️  ", "");
/// Success indicator
static SUCCESS: Emoji<'_, '_> = Emoji("✨ ", "");

/// Command line interface configuration and argument parsing
#[derive(Parser)]
#[command(author, version, about = "Daemon and CLI for the learner paper management system")]
pub struct Cli {
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
  let cli = Cli::parse();

  // Handle the command, using TUI as default when enabled
  let command = cli.command.clone().unwrap_or_else(|| {
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
    setup_logging(cli.verbose);
  }

  match command {
    Commands::Init => init(cli).await,
    Commands::Add { identifier, no_pdf } => add(cli, identifier, no_pdf).await,
    Commands::Remove { source, identifier } => remove(cli, source, identifier).await,
    Commands::Get { source, identifier } => get(cli, source, identifier).await,
    Commands::Search { query } => search(cli, query).await,
    Commands::Clean => clean(cli).await,
    Commands::Download { source, identifier } => download(cli, source, identifier).await,
    Commands::Daemon { cmd } => daemon(cmd).await,
    #[cfg(feature = "tui")]
    Commands::Tui => tui::run().await,
  }
}
