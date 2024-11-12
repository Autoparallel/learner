//! Module for abstracting the "add" functionality to the [`learner`] database.

use super::*;

/// Function for the [`Commands::Add`] in the CLI.
pub async fn add(cli: Cli, identifier: String, no_pdf: bool) -> Result<()> {
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
  let db = Database::open(&path).await?;

  println!("{} Fetching paper: {}", style(LOOKING_GLASS).cyan(), style(&identifier).yellow());

  let paper = Paper::new(&identifier).await?;
  debug!("Paper details: {:?}", paper);

  println!("\n{} Found paper:", style(SUCCESS).green());
  println!("   {} {}", style("Title:").green().bold(), style(&paper.title).white());
  println!(
    "   {} {}",
    style("Authors:").green().bold(),
    style(paper.authors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")).white()
  );

  match paper.save(&db).await {
    Ok(id) => {
      println!("\n{} Saved paper with ID: {}", style(SAVE).green(), style(id).yellow());

      // Handle PDF download for newly added paper
      if paper.pdf_url.is_some() && !no_pdf {
        let should_download = if cli.accept_defaults {
          true // Default to downloading in automated mode
        } else {
          dialoguer::Confirm::new().with_prompt("Download PDF?").default(true).interact()?
        };

        if should_download {
          println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());

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

          match paper.download_pdf(pdf_dir).await {
            Ok(_) => {
              println!("{} PDF downloaded successfully!", style(SUCCESS).green());
            },
            Err(e) => {
              println!(
                "{} Failed to download PDF: {}",
                style(WARNING).yellow(),
                style(e.to_string()).red()
              );
              println!(
                "   {} You can try downloading it later using: {} {} {}",
                style("Tip:").blue(),
                style("learnerd download").yellow(),
                style(&paper.source.to_string()).cyan(),
                style(&paper.source_identifier).yellow(),
              );
            },
          }
        }
      } else if paper.pdf_url.is_none() {
        println!("\n{} No PDF URL available for this paper", style(WARNING).yellow());
      }
    },
    Err(e) if e.is_duplicate_error() => {
      println!("\n{} This paper is already in your database", style("ℹ").blue());

      // Check existing PDF status
      if paper.pdf_url.is_some() && !no_pdf {
        if let Ok(Some(dir)) = db.get_config("pdf_dir").await {
          let pdf_dir = PathBuf::from(dir);
          let formatted_title = learner::format::format_title(&paper.title, Some(50));
          let pdf_path = pdf_dir.join(format!("{}.pdf", formatted_title));

          if pdf_path.exists() {
            println!(
              "   {} PDF exists at: {}",
              style("📄").cyan(),
              style(pdf_path.display()).yellow()
            );

            let should_redownload = if cli.accept_defaults {
              false // Default to not redownloading in automated mode
            } else {
              dialoguer::Confirm::new()
                .with_prompt("Download fresh copy? (This will overwrite the existing file)")
                .default(false)
                .interact()?
            };

            if should_redownload {
              println!("{} Downloading fresh copy of PDF...", style(LOOKING_GLASS).cyan());
              match paper.download_pdf(pdf_dir).await {
                Ok(_) => println!("{} PDF downloaded successfully!", style(SUCCESS).green()),
                Err(e) => println!(
                  "{} Failed to download PDF: {}",
                  style(WARNING).yellow(),
                  style(e.to_string()).red()
                ),
              }
            }
          } else {
            let should_download = if cli.accept_defaults {
              true // Default to downloading in automated mode
            } else {
              dialoguer::Confirm::new()
                .with_prompt("PDF not found. Download it now?")
                .default(true)
                .interact()?
            };

            if should_download {
              println!("{} Downloading PDF...", style(LOOKING_GLASS).cyan());
              match paper.download_pdf(pdf_dir).await {
                Ok(_) => println!("{} PDF downloaded successfully!", style(SUCCESS).green()),
                Err(e) => println!(
                  "{} Failed to download PDF: {}",
                  style(WARNING).yellow(),
                  style(e.to_string()).red()
                ),
              }
            }
          }
        }
      }
    },
    Err(e) => return Err(LearnerdError::Learner(e)),
  }

  Ok(())
}
