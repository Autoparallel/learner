//! Module for abstracting the "remove" functionality to the [`learner`] database.

use std::fs;

use learner::database::Remove;

use super::*;

/// Arguments that can be used for the [`Commands::Remove`]
#[derive(Args, Clone, Debug)]
pub struct RemoveArgs {
  /// Paper identifier or search terms
  pub query: String,

  /// Search filters
  #[command(flatten)]
  pub filter: SearchFilter,

  /// Show what would be removed without actually removing
  #[arg(long)]
  pub dry_run: bool,

  /// Skip confirmation prompts
  #[arg(long)]
  pub force: bool,

  /// Remove associated PDFs
  #[arg(long, group = "pdf_behavior")]
  pub remove_pdf: bool,

  /// Keep PDFs when removing papers
  #[arg(long, group = "pdf_behavior")]
  pub keep_pdf: bool,
}

// TODO (autoparallel): Address this lint
#[allow(clippy::too_many_arguments)]
/// Function for the [`Commands::Remove`] in the CLI.
pub async fn remove<I: UserInteraction>(
  interaction: &mut I,
  remove_args: RemoveArgs,
) -> Result<()> {
  let RemoveArgs { query, filter, dry_run, force, remove_pdf, keep_pdf } = remove_args;

  // First find matching papers
  let mut papers = Query::text(&query).execute(&mut interaction.learner().database).await?;

  // Apply filters
  if let Some(author) = &filter.author {
    let author_papers =
      Query::by_author(author).execute(&mut interaction.learner().database).await?;
    papers.retain(|p| author_papers.contains(p));
  }

  if let Some(source) = &filter.source {
    papers.retain(|p| p.source == *source);
  }

  if let Some(date_str) = &filter.before {
    let date = parse_date(date_str)?;
    papers.retain(|p| p.publication_date < date);
  }

  if papers.is_empty() {
    interaction.reply(ResponseContent::Info("No papers found matching criteria"))?;
    return Ok(());
  }

  // Show matching papers and their PDF status
  interaction.reply(ResponseContent::Papers(&papers))?;

  // For dry run, stop here
  if dry_run {
    interaction
      .reply(ResponseContent::Info(&format!("Dry run: would remove {} papers", papers.len())))?;
    return Ok(());
  }

  // TODO: This is absurd with alloc... lol
  let reply: String;
  if !force
    && !interaction.confirm(if papers.len() == 1 {
      reply = "Are you sure you want to remove this paper?".to_string();
      &reply
    } else {
      reply = format!("Are you sure you want to remove these {} papers?", papers.len());
      &reply
    })?
  {
    interaction.reply(ResponseContent::Info("Operation cancelled"))?;
    return Ok(());
  }

  // Determine PDF handling
  let should_remove_pdfs = if remove_pdf {
    true
  } else if keep_pdf {
    false
  } else {
    let storage_path = &interaction.learner().config.storage_path;
    let has_pdfs = papers.iter().any(|p| storage_path.join(p.filename()).exists());
    has_pdfs && interaction.confirm("Do you also want to remove associated PDFs?")?
  };

  // Remove papers and optionally their PDFs
  for paper in &papers {
    // Remove paper from database
    if let Err(e) = Remove::by_source(&paper.source, &paper.source_identifier)
      .execute(&mut interaction.learner().database)
      .await
    {
      interaction.reply(ResponseContent::Error(e.into()))?;
      continue;
    }

    interaction.reply(ResponseContent::Success(&format!("Removed paper: {}", paper.title)))?;

    // Handle PDF removal if requested
    if should_remove_pdfs {
      let pdf_path = interaction.learner().config.storage_path.join(paper.filename());
      if pdf_path.exists() {
        fs::remove_file(&pdf_path)?;
        interaction
          .reply(ResponseContent::Success(&format!("Removed PDF: {}", pdf_path.display())))?;
      }
    }
  }

  Ok(())
}
