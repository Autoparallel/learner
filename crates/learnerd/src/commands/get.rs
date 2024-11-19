//! Module for abstracting the "get" functionality to the [`learner`] database.

use super::*;

/// Function for the [`Commands::Get`] in the CLI.
pub async fn get(cli: Cli, source: String, identifier: String) -> Result<()> {
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
  let mut db = Database::open(&path).await?;

  println!(
    "{} Fetching paper from {} with ID {}",
    style(LOOKING_GLASS).cyan(),
    style(&source).cyan(),
    style(&identifier).yellow()
  );

  let papers = Query::by_source(&source, &identifier).execute(&mut db).await?;
  match papers.first() {
    Some(paper) => {
      debug!("Found paper: {:?}", paper);
      println!("\n{} Paper details:", style(PAPER).green());
      println!("   {} {}", style("Title:").green().bold(), style(&paper.title).white());
      println!(
        "   {} {}",
        style("Authors:").green().bold(),
        style(paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")).white()
      );
      println!("   {} {}", style("Abstract:").green().bold(), style(&paper.abstract_text).white());
      println!(
        "   {} {}",
        style("Published:").green().bold(),
        style(&paper.publication_date).white()
      );
      if let Some(url) = &paper.pdf_url {
        println!("   {} {}", style("PDF URL:").green().bold(), style(url).blue().underlined());
      }
      if let Some(doi) = &paper.doi {
        println!("   {} {}", style("DOI:").green().bold(), style(doi).blue().underlined());
      }
    },
    None => {
      println!("{} Paper not found", style(WARNING).yellow());
    },
  }
  Ok(())
}
