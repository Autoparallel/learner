mod validate;

use std::path::PathBuf;

use clap::{arg, Parser, Subcommand, ValueEnum};
use tracing::{error, info};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct LearnerSdk {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Validate different types of configurations
  Validate {
    /// Type of validation to perform: retriever or resource
    #[arg(value_enum)]
    validation_type: ValidationType,

    /// Path to the configuration file
    path: PathBuf,
  },
}

#[derive(Clone, Debug, ValueEnum)]
enum ValidationType {
  Retriever,
  Resource,
}

fn main() {
  tracing_subscriber::fmt()
    .without_time()
    .with_file(false)
    .with_line_number(false)
    .with_target(false)
    .init();

  let cli = LearnerSdk::parse();

  match &cli.command {
    Commands::Validate { validation_type, path } => match validation_type {
      ValidationType::Retriever => {
        if !path.exists() {
          error!("Path to retriever config was invalid.\nPath used: {path:?}");
          return;
        }
        info!("Validating retriever config at {:?}", path);
        validate::validate_retriever(path);
      },
      ValidationType::Resource => {
        if !path.exists() {
          error!("Path to resource config was invalid.\nPath used: {path:?}");
          return;
        }
        info!("Validating resource config at {:?}", path);
        validate::validate_resource(path);
      },
    },
  }
}
