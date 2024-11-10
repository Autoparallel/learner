//! Module for abstracting the "clean" functionality to the [`learner`] database.

use super::*;

/// Function for the [`Commands::Clean`] in the CLI.
pub async fn clean(cli: Cli) -> Result<()> {
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
    println!("{} Database found at: {}", style(WARNING).yellow(), style(path.display()).yellow());

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
    println!("{} Removing database: {}", style(WARNING).yellow(), style(path.display()).yellow());
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
}
