mod validate;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
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

#[derive(Clone, Debug, ValueEnum)]
enum ValidationType {
  Retriever,
  Resource,
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
