//! Module for abstracting the "download" functionality to the [`learner`] database.

use super::*;

/// Function for the [`Commands::Download`] in the CLI.
pub async fn download(cli: Cli, source: Source, identifier: String) -> Result<()> {
  let path = cli.path.unwrap_or_else(|| {
    let default_path = Database::default_path();
    println!(
      "{} Using default database path: {}",
      style(BOOKS).cyan(),
      style(default_path.display()).yellow()
    );
    default_path
  });
  let db = Database::open(&path).await?;

  let paper = match db.get_paper_by_source_id(&source, &identifier).await? {
    Some(p) => p,
    None => {
      println!(
        "{} Paper not found in database. Add it first with: {} {}",
        style(WARNING).yellow(),
        style("learnerd add").yellow(),
        style(&identifier).cyan()
      );
      return Ok(());
    },
  };

  if paper.pdf_url.is_none() {
    println!("{} No PDF URL available for this paper", style(WARNING).yellow());
    return Ok(());
  };

  let pdf_dir = match db.get_config("pdf_dir").await? {
    Some(dir) => PathBuf::from(dir),
    None => {
      println!(
        "{} PDF directory not configured. Run {} first",
        style(WARNING).yellow(),
        style("learnerd init").cyan()
      );
      return Ok(());
    },
  };

  if !pdf_dir.exists() {
    println!(
      "{} Creating PDF directory: {}",
      style(LOOKING_GLASS).cyan(),
      style(&pdf_dir.display()).yellow()
    );
    std::fs::create_dir_all(&pdf_dir)?;
  }

  let formatted_title = learner::format::format_title(&paper.title, Some(50));
  let pdf_path = pdf_dir.join(format!("{}.pdf", formatted_title));

  let should_download = if pdf_path.exists() && !cli.accept_defaults {
    println!(
      "{} PDF already exists at: {}",
      style("ℹ").blue(),
      style(&pdf_path.display()).yellow()
    );

    dialoguer::Confirm::new()
      .with_prompt("Download fresh copy? (This will overwrite the existing file)")
      .default(false)
      .interact()?
  } else {
    true
  };

  if should_download {
    if pdf_path.exists() {
      println!("{} Downloading fresh copy...", style(LOOKING_GLASS).cyan());
    } else {
      println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());
    }

    match paper.download_pdf(pdf_dir.clone()).await {
      Ok(_) => {
        println!("{} PDF downloaded successfully!", style(SUCCESS).green());
        println!("   {} Saved to: {}", style("📄").cyan(), style(&pdf_path.display()).yellow());
      },
      Err(e) => {
        println!(
          "{} Failed to download PDF: {}",
          style(WARNING).yellow(),
          style(e.to_string()).red()
        );

        match e {
          LearnerError::ApiError(ref msg) if msg.contains("403") => {
            println!("   {} This PDF might require institutional access", style("Note:").blue());
            println!(
              "   {} You may need to download this paper directly from the publisher's website",
              style("Tip:").blue()
            );
          },
          LearnerError::Network(_) => {
            println!("   {} Check your internet connection and try again", style("Tip:").blue());
          },
          LearnerError::Path(_) => {
            println!(
              "   {} Check if you have write permissions for: {}",
              style("Tip:").blue(),
              style(&pdf_dir.display()).yellow()
            );
          },
          _ => {
            println!(
              "   {} Try using {} to skip prompts",
              style("Tip:").blue(),
              style("--accept-defaults").yellow()
            );
          },
        }
      },
    }
  }

  Ok(())
}
