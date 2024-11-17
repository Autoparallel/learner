//! Module for abstracting the "search" functionality to the [`learner`] database.

use super::*;

/// Function for the [`Commands::Search`] in the CLI.
pub async fn search(cli: Cli, query: String) -> Result<()> {
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

  println!("{} Searching for: {}", style(LOOKING_GLASS).cyan(), style(&query).yellow());

  // Modify query to use FTS5 syntax for better matching
  let search_query = query.split_whitespace().collect::<Vec<_>>().join(" OR ");
  debug!("Modified search query: {}", search_query);

  let papers = Query::text(&search_query).execute(&mut db).await?;
  if papers.is_empty() {
    println!("{} No papers found matching: {}", style(WARNING).yellow(), style(&query).yellow());
  } else {
    println!("\n{} Found {} papers:", style(SUCCESS).green(), style(papers.len()).yellow());

    for (i, paper) in papers.iter().enumerate() {
      debug!("Paper details: {:?}", paper);
      println!("\n{}. {}", style(i + 1).yellow(), style(&paper.title).white().bold());

      let authors = paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();

      let author_display = if authors.is_empty() {
        style("No authors listed").red().italic().to_string()
      } else {
        style(authors.join(", ")).white().to_string()
      };

      println!("   {} {}", style("Authors:").green(), author_display);

      if let Some(doi) = &paper.doi {
        println!("   {} {}", style("DOI:").green(), style(doi).blue().underlined());
      }

      println!(
        "   {} {} {}",
        style("Source:").green(),
        style(&paper.source).cyan(),
        style(&paper.source_identifier).yellow()
      );

      // Show a preview of the abstract
      if !paper.abstract_text.is_empty() {
        let preview = paper.abstract_text.chars().take(100).collect::<String>();
        let preview =
          if paper.abstract_text.len() > 100 { format!("{}...", preview) } else { preview };
        println!("   {} {}", style("Abstract:").green(), style(preview).white().italic());
      }
    }

    // If we have multiple results, show a tip about refining the search
    if papers.len() > 1 {
      println!(
        "\n{} Tip: Use quotes for exact phrases, e.g. {}",
        style("💡").yellow(),
        style("\"exact phrase\"").yellow().italic()
      );
    }
  }
  Ok(())
}
