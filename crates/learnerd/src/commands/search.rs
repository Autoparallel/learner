//! Module for abstracting the "search" functionality to the [`learner`] database.

use chrono::{DateTime, Utc};

use super::*;

/// Function for the [`Commands::Search`] in the CLI.
pub async fn search<I: UserInteraction>(
  interaction: &I,
  mut learner: Learner,
  query: &str,
  filter: &SearchFilter,
  detailed: bool,
) -> Result<()> {
  // Build appropriate query based on filters
  let terms: String;
  let db_query = if let Some(author) = &filter.author {
    Query::by_author(author)
  } else if let Some(source) = &filter.source {
    Query::by_source(source, query)
  } else if let Some(date) = &filter.before {
    let date = DateTime::parse_from_str(date, "%Y-%m-%d")
      .map_err(|e| LearnerdError::Interaction(format!("Invalid date format: {}", e)))?
      .with_timezone(&Utc);
    Query::before_date(date)
  } else {
    // Query::
    // Build search terms with field restrictions
    terms = if filter.title_only {
      // Search only in title
      format!("title: {}", query)
    } else if filter.abstract_only {
      // Search only in abstract
      format!("abstract_text: {}", query)
    } else {
      // Search both with OR
      query.split_whitespace().map(|term| term.to_string()).collect::<Vec<_>>().join(" OR ")
    };
    Query::text(&terms)
  };

  interaction.reply(ResponseContent::Info(&format!("Searching for: {}", query)))?;

  let papers = db_query.execute(&mut learner.database).await?;

  if papers.is_empty() {
    interaction.reply(ResponseContent::Info(&format!("No papers found matching: {}", query)))
  } else {
    // Use ResponseContent::Papers for bulk display
    interaction.reply(ResponseContent::Papers(&papers))?;

    // For each paper that needs detailed view
    if detailed {
      for paper in papers.iter() {
        interaction.reply(ResponseContent::Paper(paper, true))?;
      }
    }

    // Show search refinement tip for multiple results
    if papers.len() > 1 {
      interaction.reply(ResponseContent::Info(
        "Tip: Use --author, --source, or --before to refine results. Use quotes for exact \
         phrases, e.g. \"exact phrase\""
          .into(),
      ))
    } else {
      Ok(())
    }
  }
}
