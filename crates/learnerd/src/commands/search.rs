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
  // Get initial result set from text search
  let mut papers = Query::text(query).execute(&mut learner.database).await?;

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
        interaction.reply(ResponseContent::Paper(paper, true))?;

        // Check PDF status
        let pdf_path = learner.database.get_storage_path().await?.join(paper.filename());
        if pdf_path.exists() {
          interaction.reply(ResponseContent::Info(&format!(
            "   PDF available at: {}",
            pdf_path.display()
          )))?;
        } else {
          interaction.reply(ResponseContent::Info("   PDF not downloaded"))?;
        }
      }
    } else {
      // Show summary view
      interaction.reply(ResponseContent::Papers(&papers))?;
    }

    if papers.len() > 1 {
      interaction.reply(ResponseContent::Info(
        "Tip: Use --author, --source, or --before together to further refine results",
      ))
    } else {
      Ok(())
    }
  }
}

fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
  let parsed = if date_str.len() == 4 {
    // Just year provided
    DateTime::parse_from_str(&format!("{}-01-01 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  } else if date_str.len() == 7 {
    // Year and month provided
    DateTime::parse_from_str(&format!("{}-01 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  } else {
    // Full date provided
    DateTime::parse_from_str(&format!("{} 00:00:00 +0000", date_str), "%Y-%m-%d %H:%M:%S %z")
  }
  .map_err(|e| LearnerdError::Interaction(format!("Invalid date format: {}", e)))?;

  Ok(parsed.with_timezone(&Utc))
}
