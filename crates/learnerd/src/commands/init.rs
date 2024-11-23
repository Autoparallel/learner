//! Module for setting up a [`learner`] environment

use super::*;

#[derive(Args, Clone)]
pub struct InitOptions {
  #[arg(long)]
  db_path:            Option<PathBuf>,
  #[arg(long)]
  storage_path:       Option<PathBuf>,
  #[arg(long)]
  default_retrievers: bool,
}

/// Function for the [`Commands::Init`] in the CLI.
pub async fn init<I: UserInteraction>(
  interaction: &I,
  mut learner: Learner,
  init_options: InitOptions,
) -> Result<()> {
  todo!();
  // if papers.is_empty() {
  //   interaction.reply(ResponseContent::Info(&format!("Fetching paper: {}", identifier)))?;
  //   let paper = learner.retriever.get_paper(identifier).await?;
  //   interaction.reply(ResponseContent::Paper(&paper))?;

  //   let with_pdf = paper.pdf_url.is_some()
  //     && if pdf {
  //       true
  //     } else if no_pdf {
  //       false
  //     } else {
  //       interaction.confirm("Download PDF?")?
  //     };

  //   match if with_pdf {
  //     Add::complete(&paper).execute(&mut learner.database).await
  //   } else {
  //     Add::paper(&paper).execute(&mut learner.database).await
  //   } {
  //     Ok(_) => interaction.reply(ResponseContent::Success("Paper added successfully")),
  //     Err(e) => interaction.reply(ResponseContent::Error(LearnerdError::from(e))),
  //   }
  // } else {
  //   let paper = &papers[0];
  //   interaction.reply(ResponseContent::Info("Paper already exists in database"))?;

  //   let pdf_dir = learner.database.get_storage_path().await?;
  //   let pdf_path = pdf_dir.join(paper.filename());

  //   if pdf_path.exists() {
  //     interaction.reply(ResponseContent::Info(&format!("PDF exists at: {}", pdf_path.display())))
  //   } else if paper.pdf_url.is_some() {
  //     let should_download = if pdf {
  //       true
  //     } else if no_pdf {
  //       false
  //     } else {
  //       interaction.confirm("PDF not found. Download it now?")?
  //     };

  //     if should_download {
  //       match Add::complete(paper).execute(&mut learner.database).await {
  //         Ok(_) => interaction.reply(ResponseContent::Success("PDF downloaded successfully")),
  //         Err(e) => interaction.reply(ResponseContent::Error(LearnerdError::from(e))),
  //       }
  //     } else {
  //       Ok(())
  //     }
  //   } else {
  //     Ok(())
  //   }
  // }
}
