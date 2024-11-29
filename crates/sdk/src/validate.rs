use std::fs::read_to_string;

use learner::{
  resource::ResourceConfig,
  retriever::{ResponseFormat, RetrieverConfig},
};

use super::*;

pub fn validate_resource(path: &PathBuf) {
  todo!()
  // Check all required fields are present
  // for field in &self.type_config.required_fields {
  //   if !self.values.contains_key(field) {
  //     return Err(LearnerError::InvalidResource(format!("Missing required field: {}", field)));
  //   }
  // }

  // // Validate all present fields match their declared types
  // for (field, value) in &self.values {
  //   if let Some(field_type) = self.type_config.field_types.get(field) {
  //     if !self.type_config.validate_value_type(value, field_type) {
  //       return Err(LearnerError::InvalidResource(format!("Field {} has invalid type", field)));
  //     }
  //   } else {
  //     return Err(LearnerError::InvalidResource(format!("Unknown field: {}", field)));
  //   }
  // }

  // Ok(())
}

pub async fn validate_retriever(path: &PathBuf, input: &Option<String>) {
  let config_str = match read_to_string(path) {
    Ok(str) => str,
    Err(e) => {
      error!("Failed to read config to string due to: {e:?}");
      return;
    },
  };

  let retriever: RetrieverConfig = match toml::from_str(&config_str) {
    Ok(config) => config,
    Err(e) => {
      error!("Failed to parse config to string due to: {e:?}");
      return;
    },
  };

  match &retriever.response_format {
    ResponseFormat::Xml(config) => {
      debug!("Retriever is configured for: XML\n{config:#?}")
    },
    ResponseFormat::Json(config) => {
      debug!("Retriever is configured for: JSON\n{config:#?}")
    },
  }

  if let Some(input) = input {
    info!("Attempting to match against pattern...");
    match retriever.extract_identifier(input) {
      Ok(identifier) => info!("Retriever extracted input into: {identifier}"),
      Err(e) => {
        error!("Retriever failed to extract input due to: {e:?}");
        return;
      },
    }

    info!("Attempting to fetch paper using retriever...");
    let paper = match retriever.retrieve_paper(input).await {
      Ok(paper) => {
        info!("Paper retrieved!\n{paper:#?}");
        paper
      },
      Err(e) => {
        error!("Retriever failed to retriever paper due to: {e:?}");
        return;
      },
    };

    if paper.pdf_url.is_some() {
      info!("Attempting to download associated pdf");
      let tempdir = tempfile::tempdir().unwrap();
      match paper.download_pdf(tempdir.path()).await {
        Ok(filename) => {
          let pdf_filepath = tempdir.path().join(filename);
          if pdf_filepath.exists() {
            let bytes = std::fs::read(path).unwrap();
            if bytes.is_empty() {
              error!("PDF download was empty.");
            } else {
              info!("Non-empty PDF downloaded successfully.");
            }
          } else {
            error!("PDF path did not end up getting written.")
          }
        },
        Err(e) => {
          error!("PDF was unable to be downloaded due to: {e:?}")
        },
      }
    } else {
      warn!(
        "PDF URL was not determined. Please check your configuration against the server response."
      );
    }
  } else {
    warn!(
      "No input string provided to further debug your `RetrieverConfig`. If you want to test \
       identifier pattern matching and online fetching, please pass in an input string with an \
       additional input, e.g., `2301.07041`."
    );
  }
}
