use super::*;

pub async fn init(cli: Cli) -> Result<()> {
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
        .with_prompt("Do you want to reinitialize this database? This will erase all existing data")
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
}
