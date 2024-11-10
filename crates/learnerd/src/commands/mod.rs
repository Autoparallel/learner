use super::*;

pub mod add;
pub mod clean;
pub mod daemon;
pub mod download;
pub mod get;
pub mod init;
pub mod remove;
pub mod search;

pub use add::add;
pub use clean::clean;
pub use daemon::daemon;
pub use download::download;
pub use get::get;
pub use init::init;
pub use remove::remove;
pub use search::search;

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
