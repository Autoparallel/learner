mod validate;

use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, Subcommand};
use tracing::{debug, error, info};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct LearnerSdk {
  #[command(subcommand)]
  command: Commands,

  /// Verbose mode (-v, -vv, -vvv) for different levels of logging detail
  #[arg(
      short,
      long,
      action = ArgAction::Count,
      global = true,
      help = "Increase logging verbosity"
  )]
  verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
  /// Validate a resource config
  ValidateResource {
    /// Path to the configuration file
    path: PathBuf,
  },
  /// Validate a retriever config for an optional given input
  ValidateRetriever {
    /// Path to the configuration file
    path: PathBuf,

    /// Identifier or URL
    input: Option<String>,
  },
}

/// Attempts to find the root config directory by walking up the path.
/// Returns the config directory and its relation to the input path.
fn find_config_dir(path: &Path) -> Option<(PathBuf, String)> {
  // Convert path to absolute for clearer error messages
  let abs_path = path.canonicalize().ok()?;
  let mut current = abs_path.as_path();

  // Walk up the directory tree
  while let Some(parent) = current.parent() {
    // Check if this is a config directory by looking for expected structure
    if parent.ends_with("config")
      && parent.join("resources").is_dir()
      && parent.join("retrievers").is_dir()
    {
      // Calculate the relationship to the original path
      let relation = if abs_path.starts_with(parent) {
        format!(
          "Found config directory {} levels up from input path",
          abs_path.strip_prefix(parent).ok()?.components().count() - 1
        )
      } else {
        "Found config directory".to_string()
      };

      return Some((parent.to_path_buf(), relation));
    }
    current = parent;
  }
  None
}

#[tokio::main]
async fn main() {
  let cli = LearnerSdk::parse();
  let filter = match cli.verbose {
    0 => "error",
    1 => "warn",
    2 => "info",
    3 => "debug",
    _ => "trace",
  };
  tracing_subscriber::fmt()
    .with_env_filter(filter)
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_target(false)
    .with_max_level(tracing::Level::TRACE)
    .init();

  // Get the path from the command
  let path = match &cli.command {
    Commands::ValidateRetriever { path, .. } | Commands::ValidateResource { path } => path,
  };

  // First check if the input path exists
  if !path.exists() {
    error!("Input path does not exist: {}", path.display());
    error!("Please provide a valid path to a configuration file");
    return;
  }

  // Try to find the config directory
  let (config_dir, message) = match find_config_dir(path) {
    Some((dir, msg)) => (dir, msg),
    None => {
      error!("Could not find a valid configuration directory!");
      error!("Looking for a directory named 'config' containing:");
      error!("  - resources/ directory");
      error!("  - retrievers/ directory");
      error!("Input path was: {}", path.display());
      error!("Tip: Make sure you're running this command from a location where");
      error!("     the config directory structure is accessible");
      return;
    },
  };

  // Initialize the environment
  info!("{}", message);
  debug!("Using config directory: {}", config_dir.display());

  // Proceed with validation based on command
  match &cli.command {
    Commands::ValidateRetriever { path, input } => {
      info!("Validating retriever configuration...");
      debug!("Config file: {}", path.display());
      if let Some(input) = input {
        debug!("Testing with input: {}", input);
      }
      validate::validate_retriever(path, input).await;
    },
    Commands::ValidateResource { path } => {
      info!("Validating resource configuration...");
      debug!("Config file: {}", path.display());
      validate::validate_resource(path);
    },
  }
}
