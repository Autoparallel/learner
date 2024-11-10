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
use errors::LearnerdErrors;
use learner::{
  database::Database,
  errors::LearnerError,
  paper::{Paper, Source},
};
use tracing::{debug, trace};
use tracing_subscriber::EnvFilter;

pub mod daemon;
pub mod errors;
#[cfg(feature = "tui")] pub mod tui;

use daemon::*;

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
struct Cli {
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

/// Available commands for the CLI
#[derive(Subcommand)]
enum Commands {
  /// Launch the Terminal User Interface (default when no command specified)
  #[cfg(feature = "tui")]
  #[clap(hide = true)] // Hide from help since it's the default
  Tui,

  /// Initialize a new learner database
  Init,

  /// Add a paper to the database by its identifier
  Add {
    /// Paper identifier (arXiv ID, DOI, or IACR ID)
    /// Examples: "2301.07041", "10.1145/1327452.1327492"
    identifier: String,

    /// Skip PDF download prompt
    #[arg(long)]
    no_pdf: bool,
  },

  /// Download the PDF for a given entry, replacing an existing PDF if desired.
  Download {
    /// Source system (arxiv, doi, iacr)
    #[arg(value_enum)]
    source: Source,

    /// Paper identifier in the source system
    /// Example: "2301.07041" for arXiv
    identifier: String,
  },

  /// Remove a paper from the database by its source and identifier
  Remove {
    /// Source system (arxiv, doi, iacr)
    #[arg(value_enum)]
    source: Source,

    /// Paper identifier in the source system
    identifier: String,
  },

  /// Retrieve and display a paper's details
  Get {
    /// Source system (arxiv, doi, iacr)
    #[arg(value_enum)]
    source: Source,

    /// Paper identifier in the source system
    identifier: String,
  },

  /// Search papers in the database
  Search {
    /// Search query - supports full text search
    query: String,
  },

  /// Removes the entire database after confirmation
  Clean,

  /// Manage the learnerd daemon
  Daemon {
    /// The set of commands specifically for managing the [`Daemon`].
    #[command(subcommand)]
    cmd: DaemonCommands,
  },
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
    0 => "warn",
    1 => "info",
    2 => "debug",
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
async fn main() -> Result<(), LearnerdErrors> {
  let cli = Cli::parse();

  // Handle the command, using TUI as default when enabled
  let command = cli.command.unwrap_or_else(|| {
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
    Commands::Init => {
      let db_path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });

      if db_path.exists() {
        println!(
          "{} Database already exists at: {}",
          style(WARNING).yellow(),
          style(db_path.display()).yellow()
        );

        // Handle reinitialize confirmation
        let should_reinit = if cli.accept_defaults {
          false // Default to not reinitializing in automated mode
        } else {
          dialoguer::Confirm::new()
            .with_prompt(
              "Do you want to reinitialize this database? This will erase all existing data",
            )
            .default(false)
            .interact()?
        };

        if !should_reinit {
          println!("{} Keeping existing database", style("ℹ").blue());
          return Ok(());
        }

        // Handle INIT confirmation
        let should_proceed = if cli.accept_defaults {
          false // Default to not proceeding in automated mode
        } else {
          let input = dialoguer::Input::<String>::new()
            .with_prompt(format!(
              "{} Type {} to confirm reinitialization",
              style("⚠️").red(),
              style("INIT").red().bold()
            ))
            .interact_text()?;
          input == "INIT"
        };

        if !should_proceed {
          println!("{} Operation cancelled, keeping existing database", style("ℹ").blue());
          return Ok(());
        }

        // Remove existing database
        println!("{} Removing existing database", style(WARNING).yellow());
        std::fs::remove_file(&db_path)?;

        // Also remove any FTS auxiliary files
        let fts_files = glob::glob(&format!("{}*", db_path.display()))?;
        for file in fts_files.flatten() {
          std::fs::remove_file(file)?;
        }
      }

      // Create parent directories if they don't exist
      if let Some(parent) = db_path.parent() {
        trace!("Creating parent directories: {}", parent.display());
        std::fs::create_dir_all(parent)?;
      }

      println!(
        "{} Initializing database at: {}",
        style(ROCKET).cyan(),
        style(db_path.display()).yellow()
      );

      let db = Database::open(&db_path).await?;

      // Set up PDF directory
      let pdf_dir = Database::default_pdf_path();
      println!(
        "\n{} PDF files will be stored in: {}",
        style(PAPER).cyan(),
        style(pdf_dir.display()).yellow()
      );

      // TODO (autoparallel): I think we need this `allow` because though the returns are the same,
      // the initial `if` bypasses interaction
      #[allow(clippy::if_same_then_else)]
      let pdf_dir = if cli.accept_defaults {
        pdf_dir // Use default in automated mode
      } else if dialoguer::Confirm::new()
        .with_prompt("Use this location for PDF storage?")
        .default(true)
        .interact()?
      {
        pdf_dir
      } else {
        let input: String =
          dialoguer::Input::new().with_prompt("Enter path for PDF storage").interact_text()?;
        PathBuf::from_str(&input).unwrap() // TODO (autoparallel): fix this unwrap
      };

      std::fs::create_dir_all(&pdf_dir)?;
      db.set_config("pdf_dir", &pdf_dir.to_string_lossy()).await?;

      println!("{} Database initialized successfully!", style(SUCCESS).green());
      Ok(())
    },

    Commands::Add { identifier, no_pdf } => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      trace!("Using database at: {}", path.display());
      let db = Database::open(&path).await?;

      println!("{} Fetching paper: {}", style(LOOKING_GLASS).cyan(), style(&identifier).yellow());

      let paper = Paper::new(&identifier).await?;
      debug!("Paper details: {:?}", paper);

      println!("\n{} Found paper:", style(SUCCESS).green());
      println!("   {} {}", style("Title:").green().bold(), style(&paper.title).white());
      println!(
        "   {} {}",
        style("Authors:").green().bold(),
        style(paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")).white()
      );

      match paper.save(&db).await {
        Ok(id) => {
          println!("\n{} Saved paper with ID: {}", style(SAVE).green(), style(id).yellow());

          // Handle PDF download for newly added paper
          if paper.pdf_url.is_some() && !no_pdf {
            let should_download = if cli.accept_defaults {
              true // Default to downloading in automated mode
            } else {
              dialoguer::Confirm::new().with_prompt("Download PDF?").default(true).interact()?
            };

            if should_download {
              println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());

              let pdf_dir = match db.get_config("pdf_dir").await? {
                Some(dir) => PathBuf::from(dir),
                None => {
                  println!(
                    "{} PDF directory not configured. Run {} first",
                    style(WARNING).yellow(),
                    style("learnerd init").cyan()
                  );
                  return Ok(());
                },
              };

              match paper.download_pdf(pdf_dir).await {
                Ok(_) => {
                  println!("{} PDF downloaded successfully!", style(SUCCESS).green());
                },
                Err(e) => {
                  println!(
                    "{} Failed to download PDF: {}",
                    style(WARNING).yellow(),
                    style(e.to_string()).red()
                  );
                  println!(
                    "   {} You can try downloading it later using: {} {} {}",
                    style("Tip:").blue(),
                    style("learnerd download").yellow(),
                    style(&paper.source.to_string()).cyan(),
                    style(&paper.source_identifier).yellow(),
                  );
                },
              }
            }
          } else if paper.pdf_url.is_none() {
            println!("\n{} No PDF URL available for this paper", style(WARNING).yellow());
          }
        },
        Err(e) if e.is_duplicate_error() => {
          println!("\n{} This paper is already in your database", style("ℹ").blue());

          // Check existing PDF status
          if paper.pdf_url.is_some() && !no_pdf {
            if let Ok(Some(dir)) = db.get_config("pdf_dir").await {
              let pdf_dir = PathBuf::from(dir);
              let formatted_title = learner::format::format_title(&paper.title, Some(50));
              let pdf_path = pdf_dir.join(format!("{}.pdf", formatted_title));

              if pdf_path.exists() {
                println!(
                  "   {} PDF exists at: {}",
                  style("📄").cyan(),
                  style(pdf_path.display()).yellow()
                );

                let should_redownload = if cli.accept_defaults {
                  false // Default to not redownloading in automated mode
                } else {
                  dialoguer::Confirm::new()
                    .with_prompt("Download fresh copy? (This will overwrite the existing file)")
                    .default(false)
                    .interact()?
                };

                if should_redownload {
                  println!("{} Downloading fresh copy of PDF...", style(LOOKING_GLASS).cyan());
                  match paper.download_pdf(pdf_dir).await {
                    Ok(_) => println!("{} PDF downloaded successfully!", style(SUCCESS).green()),
                    Err(e) => println!(
                      "{} Failed to download PDF: {}",
                      style(WARNING).yellow(),
                      style(e.to_string()).red()
                    ),
                  }
                }
              } else {
                let should_download = if cli.accept_defaults {
                  true // Default to downloading in automated mode
                } else {
                  dialoguer::Confirm::new()
                    .with_prompt("PDF not found. Download it now?")
                    .default(true)
                    .interact()?
                };

                if should_download {
                  println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());
                  match paper.download_pdf(pdf_dir).await {
                    Ok(_) => println!("{} PDF downloaded successfully!", style(SUCCESS).green()),
                    Err(e) => println!(
                      "{} Failed to download PDF: {}",
                      style(WARNING).yellow(),
                      style(e.to_string()).red()
                    ),
                  }
                }
              }
            }
          }
        },
        Err(e) => return Err(LearnerdErrors::Learner(e)),
      }

      Ok(())
    },

    Commands::Remove { source, identifier } => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      trace!("Using database at: {}", path.display());
      let _db = Database::open(&path).await?;

      println!("{} Remove functionality not yet implemented", style(WARNING).yellow());
      println!(
        "Would remove paper from {} with ID {}",
        style(source).cyan(),
        style(identifier).yellow()
      );
      Ok(())
    },

    Commands::Get { source, identifier } => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      trace!("Using database at: {}", path.display());
      let db = Database::open(&path).await?;

      println!(
        "{} Fetching paper from {} with ID {}",
        style(LOOKING_GLASS).cyan(),
        style(&source).cyan(),
        style(&identifier).yellow()
      );

      match db.get_paper_by_source_id(&source, &identifier).await? {
        Some(paper) => {
          debug!("Found paper: {:?}", paper);
          println!("\n{} Paper details:", style(PAPER).green());
          println!("   {} {}", style("Title:").green().bold(), style(&paper.title).white());
          println!(
            "   {} {}",
            style("Authors:").green().bold(),
            style(paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))
              .white()
          );
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
        },
        None => {
          println!("{} Paper not found", style(WARNING).yellow());
        },
      }
      Ok(())
    },

    Commands::Search { query } => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      trace!("Using database at: {}", path.display());
      let db = Database::open(&path).await?;

      println!("{} Searching for: {}", style(LOOKING_GLASS).cyan(), style(&query).yellow());

      // Modify query to use FTS5 syntax for better matching
      let search_query = query.split_whitespace().collect::<Vec<_>>().join(" OR ");
      debug!("Modified search query: {}", search_query);

      let papers = db.search_papers(&search_query).await?;
      if papers.is_empty() {
        println!(
          "{} No papers found matching: {}",
          style(WARNING).yellow(),
          style(&query).yellow()
        );
      } else {
        println!("\n{} Found {} papers:", style(SUCCESS).green(), style(papers.len()).yellow());

        for (i, paper) in papers.iter().enumerate() {
          debug!("Paper details: {:?}", paper);
          println!("\n{}. {}", style(i + 1).yellow(), style(&paper.title).white().bold());

          let authors = paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();

          let author_display = if authors.is_empty() {
            style("No authors listed").red().italic().to_string()
          } else {
            style(authors.join(", ")).white().to_string()
          };

          println!("   {} {}", style("Authors:").green(), author_display);

          if let Some(doi) = &paper.doi {
            println!("   {} {}", style("DOI:").green(), style(doi).blue().underlined());
          }

          println!(
            "   {} {} {}",
            style("Source:").green(),
            style(&paper.source).cyan(),
            style(&paper.source_identifier).yellow()
          );

          // Show a preview of the abstract
          if !paper.abstract_text.is_empty() {
            let preview = paper.abstract_text.chars().take(100).collect::<String>();
            let preview =
              if paper.abstract_text.len() > 100 { format!("{}...", preview) } else { preview };
            println!("   {} {}", style("Abstract:").green(), style(preview).white().italic());
          }
        }

        // If we have multiple results, show a tip about refining the search
        if papers.len() > 1 {
          println!(
            "\n{} Tip: Use quotes for exact phrases, e.g. {}",
            style("💡").yellow(),
            style("\"exact phrase\"").yellow().italic()
          );
        }
      }
      Ok(())
    },

    Commands::Clean => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      if path.exists() {
        println!(
          "{} Database found at: {}",
          style(WARNING).yellow(),
          style(path.display()).yellow()
        );

        // Skip confirmations if force flag is set
        if !cli.accept_defaults {
          // First confirmation
          if !dialoguer::Confirm::new()
            .with_prompt("Are you sure you want to delete this database?")
            .default(false)
            .wait_for_newline(true)
            .interact()?
          {
            println!("{} Operation cancelled", style("✖").red());
            return Ok(());
          }

          // Require typing DELETE for final confirmation
          let input = dialoguer::Input::<String>::new()
            .with_prompt(format!(
              "{} Type {} to confirm deletion",
              style("⚠️").red(),
              style("DELETE").red().bold()
            ))
            .interact_text()?;

          if input != "DELETE" {
            println!("{} Operation cancelled", style("✖").red());
            return Ok(());
          }
        }

        // Proceed with deletion
        println!(
          "{} Removing database: {}",
          style(WARNING).yellow(),
          style(path.display()).yellow()
        );
        std::fs::remove_file(&path)?;

        // Also remove any FTS auxiliary files
        let fts_files = glob::glob(&format!("{}*", path.display()))?;
        for file in fts_files.flatten() {
          std::fs::remove_file(file)?;
        }
        println!("{} Database files cleaned", style(SUCCESS).green());
      } else {
        println!(
          "{} No database found at: {}",
          style(WARNING).yellow(),
          style(path.display()).yellow()
        );
      }
      Ok(())
    },

    Commands::Download { source, identifier } => {
      let path = cli.path.unwrap_or_else(|| {
        let default_path = Database::default_path();
        println!(
          "{} Using default database path: {}",
          style(BOOKS).cyan(),
          style(default_path.display()).yellow()
        );
        default_path
      });
      let db = Database::open(&path).await?;

      let paper = match db.get_paper_by_source_id(&source, &identifier).await? {
        Some(p) => p,
        None => {
          println!(
            "{} Paper not found in database. Add it first with: {} {}",
            style(WARNING).yellow(),
            style("learnerd add").yellow(),
            style(&identifier).cyan()
          );
          return Ok(());
        },
      };

      if paper.pdf_url.is_none() {
        println!("{} No PDF URL available for this paper", style(WARNING).yellow());
        return Ok(());
      };

      let pdf_dir = match db.get_config("pdf_dir").await? {
        Some(dir) => PathBuf::from(dir),
        None => {
          println!(
            "{} PDF directory not configured. Run {} first",
            style(WARNING).yellow(),
            style("learnerd init").cyan()
          );
          return Ok(());
        },
      };

      if !pdf_dir.exists() {
        println!(
          "{} Creating PDF directory: {}",
          style(LOOKING_GLASS).cyan(),
          style(&pdf_dir.display()).yellow()
        );
        std::fs::create_dir_all(&pdf_dir)?;
      }

      let formatted_title = learner::format::format_title(&paper.title, Some(50));
      let pdf_path = pdf_dir.join(format!("{}.pdf", formatted_title));

      let should_download = if pdf_path.exists() && !cli.accept_defaults {
        println!(
          "{} PDF already exists at: {}",
          style("ℹ").blue(),
          style(&pdf_path.display()).yellow()
        );

        dialoguer::Confirm::new()
          .with_prompt("Download fresh copy? (This will overwrite the existing file)")
          .default(false)
          .interact()?
      } else {
        true
      };

      if should_download {
        if pdf_path.exists() {
          println!("{} Downloading fresh copy...", style(LOOKING_GLASS).cyan());
        } else {
          println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());
        }

        match paper.download_pdf(pdf_dir.clone()).await {
          Ok(_) => {
            println!("{} PDF downloaded successfully!", style(SUCCESS).green());
            println!("   {} Saved to: {}", style("📄").cyan(), style(&pdf_path.display()).yellow());
          },
          Err(e) => {
            println!(
              "{} Failed to download PDF: {}",
              style(WARNING).yellow(),
              style(e.to_string()).red()
            );

            match e {
              LearnerError::ApiError(ref msg) if msg.contains("403") => {
                println!(
                  "   {} This PDF might require institutional access",
                  style("Note:").blue()
                );
                println!(
                  "   {} You may need to download this paper directly from the publisher's website",
                  style("Tip:").blue()
                );
              },
              LearnerError::Network(_) => {
                println!(
                  "   {} Check your internet connection and try again",
                  style("Tip:").blue()
                );
              },
              LearnerError::Path(_) => {
                println!(
                  "   {} Check if you have write permissions for: {}",
                  style("Tip:").blue(),
                  style(&pdf_dir.display()).yellow()
                );
              },
              _ => {
                println!(
                  "   {} Try using {} to skip prompts",
                  style("Tip:").blue(),
                  style("--accept-defaults").yellow()
                );
              },
            }
          },
        }
      }

      Ok(())
    },

    Commands::Daemon { cmd } => {
      let daemon = daemon::Daemon::new();

      match cmd {
        DaemonCommands::Start => {
          println!("{} Starting daemon...", style(ROCKET).cyan());
          match daemon.start() {
            Ok(_) => println!("{} Daemon started successfully", style(SUCCESS).green()),
            Err(e) => {
              println!("{} Failed to start daemon: {}", style(WARNING).yellow(), style(&e).red());
              return Err(e);
            },
          }
        },
        DaemonCommands::Stop => {
          println!("{} Stopping daemon...", style(WARNING).yellow());
          match daemon.stop() {
            Ok(_) => println!("{} Daemon stopped", style(SUCCESS).green()),
            Err(e) => {
              println!("{} Failed to stop daemon: {}", style(WARNING).yellow(), style(&e).red());
              return Err(e);
            },
          }
        },
        DaemonCommands::Restart => {
          println!("{} Restarting daemon...", style(ROCKET).cyan());
          match daemon.restart() {
            Ok(_) => println!("{} Daemon restarted successfully", style(SUCCESS).green()),
            Err(e) => {
              println!("{} Failed to restart daemon: {}", style(WARNING).yellow(), style(&e).red());
              return Err(e);
            },
          }
        },
        DaemonCommands::Install => {
          println!("{} Installing daemon service...", style(ROCKET).cyan());
          match daemon.install() {
            Ok(_) => {
              println!("{} Daemon service installed", style(SUCCESS).green());
              daemon_install_prompt(&daemon);
            },
            Err(e) => {
              println!("{} Failed to install daemon: {}", style(WARNING).yellow(), style(&e).red());
              return Err(e);
            },
          }
        },
        DaemonCommands::Uninstall => {
          println!("{} Removing daemon service...", style(WARNING).yellow());
          match daemon.uninstall() {
            Ok(_) => {
              println!("{} Daemon service removed", style(SUCCESS).green());

              #[cfg(target_os = "linux")]
              println!(
                "\n{} Run {} to apply changes",
                style("Next step:").blue(),
                style("sudo systemctl daemon-reload").yellow()
              );
            },
            Err(e) => {
              println!(
                "{} Failed to uninstall daemon: {}",
                style(WARNING).yellow(),
                style(&e).red()
              );
              return Err(e);
            },
          }
        },
        DaemonCommands::Status => {
          if let Ok(pid) = std::fs::read_to_string(&daemon.pid_file) {
            let pid = pid.trim();
            println!(
              "{} Daemon is running with PID: {}",
              style(SUCCESS).green(),
              style(pid).yellow()
            );

            // Show log file location
            println!("\n{} Log files:", style("📄").cyan());
            println!(
              "   Main log: {}",
              style(daemon.log_dir.join("learnerd.log").display()).yellow()
            );
            println!("   Stdout: {}", style(daemon.log_dir.join("stdout.log").display()).yellow());
            println!("   Stderr: {}", style(daemon.log_dir.join("stderr.log").display()).yellow());

            // Show service status if installed
            #[cfg(target_os = "linux")]
            println!(
              "\n{} For detailed status, run: {}",
              style("Tip:").blue(),
              style("sudo systemctl status learnerd").yellow()
            );

            #[cfg(target_os = "macos")]
            println!(
              "\n{} For detailed status, run: {}",
              style("Tip:").blue(),
              style("sudo launchctl list | grep learnerd").yellow()
            );
          } else {
            println!("{} Daemon is not running", style(WARNING).yellow());
          }
        },
      }
      Ok(())
    },
    #[cfg(feature = "tui")]
    Commands::Tui => {
      todo!()
    },
  }
}
