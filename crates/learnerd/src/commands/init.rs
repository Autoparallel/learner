//! Module for setting up a [`learner`] environment

use super::*;

/// Arguments that can be used for the [`Commands::Add`]
#[derive(Args, Clone)]
pub struct InitArgs {
  /// Path to use to store [`Database`].
  /// Defaults to [`Database::default_path`].
  #[arg(long)]
  pub db_path: Option<PathBuf>,

  /// Path to use to store documents.
  /// Defaults to [`Database::default_storage_path`].
  #[arg(long)]
  pub storage_path: Option<PathBuf>,

  /// Whether to use the default set of retrievier configurations.
  /// Defaults to `true`.
  #[arg(long, action=ArgAction::SetTrue)]
  pub default_retrievers: bool,
}

/// Function for the [`Commands::Init`] in the CLI.
pub async fn init<I: UserInteraction>(interaction: &mut I, init_args: InitArgs) -> Result<()> {
  let InitArgs { db_path, storage_path, default_retrievers } = init_args;
  // Throughout, assume we are using default config path (`~/.learner`)

  // Set database storage location
  let config = if let Some(db_path) = db_path {
    Config::default().with_database_path(&db_path)
  } else if !interaction.confirm(&format!(
    "Would you like to use the default path {:?} for storing the Learner database?",
    Database::default_path(),
  ))? {
    interaction.reply(ResponseContent::Info(
      "Please pass in your intended database storage path using --db-path",
    ))?;
    return Ok(());
  } else {
    Config::default()
  };

  if config.database_path.exists()
    && !interaction.confirm(
      "Database already exists at this location, do you want to overwrite this database?",
    )?
  {
    interaction.reply(ResponseContent::Info(
      "Please choose a different location for this new Learner database using --db-path",
    ))?;
    return Ok(());
  }

  // Set document storage location
  let config = if let Some(storage_path) = storage_path {
    config.with_storage_path(&storage_path)
  } else if !interaction.confirm(&format!(
    "Would you like to use the default path {:?} for storing documents?",
    Database::default_storage_path(),
  ))? {
    interaction.reply(ResponseContent::Info(
      "Please pass in your intended database storage path using --storage-path",
    ))?;
    return Ok(());
  } else {
    config
  };

  // Create learner with this configuration and with the default retrievers (arXiv and DOI)
  if default_retrievers {
    interaction
      .reply(ResponseContent::Info("Using the default set of retrievers (arXiv and DOI)."))?;
    const ARXIV_CONFIG: &str = include_str!("../../config/retrievers/arxiv.toml");
    const DOI_CONFIG: &str = include_str!("../../config/retrievers/doi.toml");

    std::fs::write(config.retrievers_path.join("arxiv.toml"), ARXIV_CONFIG)?;
    std::fs::write(config.retrievers_path.join("doi.toml"), DOI_CONFIG)?;
  }
  Learner::builder().with_config(config.clone()).build().await?;
  interaction.reply(ResponseContent::Success(&format!(
    "Created Learner configuration with\nConfig path: {:?}\nDatabase path: {:?}\nDocument storage \
     path: {:?}",
    Config::default_path(),
    config.database_path,
    config.storage_path,
  )))?;
  Ok(())
}
