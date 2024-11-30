mod validate;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use learner::{environment::Environment, prelude::*};
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct LearnerSdk {
  #[command(subcommand)]
  command: Commands,
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

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_target(false)
    .with_max_level(tracing::Level::TRACE)
    .init();

  let cli = LearnerSdk::parse();

  // Get the path from the command
  let path = match &cli.command {
    Commands::ValidateRetriever { path, .. } | Commands::ValidateResource { path } => path,
  };

  // // Set up environment from the config directory in the path
  // if let Some(config_dir) = path.parent().and_then(|p| p.parent()) {
  //   debug!("Setting config directory to: {}", config_dir.display());
  //   if let Err(e) = Environment::set_global(config_dir.to_path_buf()) {
  //     error!("Failed to set global environment: {}", e);
  //     return;
  //   }
  // } else {
  //   error!("Could not determine config directory from path: {}", path.display());
  //   return;
  // }

  match &cli.command {
    Commands::ValidateRetriever { path, input } => {
      info!("Validating retriever...");
      if !path.exists() {
        error!("Path to retriever config was invalid.\nPath used: {path:?}");
        return;
      }
      debug!("Validating retriever config at {:?}", path);
      validate::validate_retriever(path, input).await;
    },
    Commands::ValidateResource { path } => {
      info!("Validating resource...");
      if !path.exists() {
        error!("Path to resource config was invalid.\nPath used: {path:?}");
        return;
      }
      debug!("Validating resource config at {:?}", path);
      validate::validate_resource(path);
    },
  }
}
