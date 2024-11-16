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
pub mod clean;
pub mod daemon;
pub mod download;
pub mod get;
pub mod init;
pub mod remove;
pub mod search;

pub use self::{
  add::add, clean::clean, daemon::daemon, download::download, get::get, init::init, remove::remove,
  search::search,
};

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
