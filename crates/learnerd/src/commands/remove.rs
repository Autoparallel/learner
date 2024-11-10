use super::*;

pub async fn remove(cli: Cli, source: Source, identifier: String) -> Result<()> {
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
}
