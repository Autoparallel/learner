//! Module for abstracting the "search" functionality to the [`learner`] database.

use super::*;

#[derive(Args, Clone)]
pub struct SearchOptions {
  /// Search query - supports full text search
  pub query: String,

  /// Show detailed paper information
  #[arg(long)]
  pub detailed: bool,

  /// Search filters
  #[command(flatten)]
  pub filter: SearchFilter,
}

/// Filter options for paper searches
#[derive(Args, Clone)]
pub struct SearchFilter {
  /// Filter by author name
  #[arg(long)]
  pub author: Option<String>,

  /// Filter by paper source (arxiv, doi, iacr)
  #[arg(long)]
  pub source: Option<String>,

  /// Filter by publication date (YYYY-MM-DD)
  #[arg(long)]
  pub before: Option<String>,
  // TODO (autoparallel): Allow for proper scoped searches
  // /// Search only titles
  // #[arg(long, group = "search_scope")]
  // title_only: bool,

  // /// Search only abstracts
  // #[arg(long, group = "search_scope")]
  // abstract_only: bool,
}

/// Function for the [`Commands::Search`] in the CLI.
pub async fn search<I: UserInteraction>(
  interaction: &I,
  mut learner: Learner,
  search_options: SearchOptions,
) -> Result<()> {
  let SearchOptions { query, detailed, filter } = search_options;

  // Get initial result set from text search
  let mut papers = Query::text(&query).execute(&mut learner.database).await?;

  // Filter by author if specified
  if let Some(author) = &filter.author {
    let author_papers = Query::by_author(author).execute(&mut learner.database).await?;
    papers.retain(|p| author_papers.contains(p));
  }

  // Filter by source if specified
  if let Some(source) = &filter.source {
    papers.retain(|p| p.source == *source);
  }

  // Filter by date if specified
  if let Some(date_str) = &filter.before {
    let date = parse_date(date_str)?;
    papers.retain(|p| p.publication_date < date);
  }

  interaction.reply(ResponseContent::Info(&format!("Searching for: {}", query)))?;

  // Rest of the display logic remains the same
  if papers.is_empty() {
    interaction.reply(ResponseContent::Info("No papers found matching all criteria"))
  } else {
    if detailed {
      // Only show detailed view
      for paper in papers.iter() {
        interaction.reply(ResponseContent::Paper(paper))?;
      }
    } else {
      // Show summary view
      interaction.reply(ResponseContent::Papers(&papers))?;
    }
    Ok(())
  }
}
