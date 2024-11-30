use std::fs::read_to_string;

use learner::{
  resource::ResourceConfig,
  retriever::{ResponseFormat, RetrieverConfig},
};

use super::*;

pub fn validate_resource(path: &PathBuf) {
  info!("Validating resource configuration from: {}", path.display());

  // Read and parse the configuration
  let config_str = match read_to_string(path) {
    Ok(str) => str,
    Err(e) => {
      error!("Failed to read config file: {}", e);
      error!("Please ensure the file exists and has proper permissions");
      return;
    },
  };

  let resource: ResourceConfig = match toml::from_str(&config_str) {
    Ok(config) => config,
    Err(e) => {
      error!("Failed to parse TOML configuration: {}", e);
      error!("Common issues:");
      error!("- Missing or malformed fields");
      error!("- Incorrect data types");
      error!("- TOML syntax errors");
      return;
    },
  };

  info!("Found resource type: {}", resource.type_name);
  if let Some(desc) = &resource.description {
    info!("Description: {}", desc);
  }

  // Validate field definitions
  info!("Validating {} field definitions...", resource.fields.len());
  for field in &resource.fields {
    // Check field type validity
    match field.field_type.as_str() {
      "string" | "integer" | "float" | "boolean" | "datetime" | "array" | "table" => {
        info!("Field '{}' ({}):", field.name, field.field_type);
        if let Some(desc) = &field.description {
          info!("  Description: {}", desc);
        }
        info!("  Required: {}", field.required);

        // Validate default values match declared type
        if let Some(default) = &field.default {
          match (field.field_type.as_str(), default) {
            ("string", toml::Value::String(_))
            | ("integer", toml::Value::Integer(_))
            | ("float", toml::Value::Float(_))
            | ("boolean", toml::Value::Boolean(_))
            | ("datetime", toml::Value::Datetime(_))
            | ("array", toml::Value::Array(_))
            | ("table", toml::Value::Table(_)) => {
              info!("  Default value: valid");
            },
            _ => {
              error!("  Default value type doesn't match field type!");
              error!("  Expected {}, got {}", field.field_type, default.type_str());
            },
          }
        }

        // Validate validation rules
        if let Some(rules) = &field.validation {
          info!("  Validation rules:");
          match field.field_type.as_str() {
            "string" => {
              if let Some(pattern) = &rules.pattern {
                match regex::Regex::new(pattern) {
                  Ok(_) => info!("    - Valid regex pattern"),
                  Err(e) => error!("    - Invalid regex pattern: {}", e),
                }
              }
              if let Some(min) = rules.min_length {
                info!("    - Minimum length: {}", min);
              }
              if let Some(max) = rules.max_length {
                info!("    - Maximum length: {}", max);
              }
            },
            "array" => {
              if let Some(min) = rules.min_items {
                info!("    - Minimum items: {}", min);
              }
              if let Some(max) = rules.max_items {
                info!("    - Maximum items: {}", max);
              }
              if rules.unique_items == Some(true) {
                info!("    - Items must be unique");
              }
            },
            _ => {},
          }
        }
      },
      invalid_type => {
        error!("Field '{}' has invalid type: {}", field.name, invalid_type);
        error!("Valid types are: string, integer, float, boolean, datetime, array, table");
      },
    }
  }

  info!("Resource configuration validation complete!");
}

pub async fn validate_retriever(path: &PathBuf, input: &Option<String>) {
  info!("Validating retriever configuration from: {}", path.display());

  let config_str = match read_to_string(path) {
    Ok(str) => str,
    Err(e) => {
      error!("Failed to read config file: {}", e);
      error!("Please ensure the file exists and has proper permissions");
      return;
    },
  };

  let retriever: RetrieverConfig = match toml::from_str(&config_str) {
    Ok(config) => config,
    Err(e) => {
      error!("Failed to parse TOML configuration: {}", e);
      error!("Common issues:");
      error!("- Missing required fields");
      error!("- Incorrect field types");
      error!("- Malformed URLs or patterns");
      return;
    },
  };

  // Validate basic configuration
  info!("Validating retriever '{}'", retriever.name);

  // Check URL validity
  if let Err(e) = reqwest::Url::parse(&retriever.base_url) {
    error!("Invalid base URL: {}", e);
    return;
  }

  // Validate endpoint template
  if !retriever.endpoint_template.contains("{identifier}") {
    error!("Endpoint template must contain {{identifier}} placeholder");
    return;
  }

  // Check response format configuration
  match &retriever.response_format {
    ResponseFormat::Xml(config) => {
      info!("XML configuration:");
      info!("- Namespace stripping: {}", config.strip_namespaces);
      info!("Validating field mappings:");
      for (field, map) in &config.field_maps {
        info!("- {}: {}", field, map.path);
        if let Some(transform) = &map.transform {
          info!("  with transformation: {:?}", transform);
        }
      }
    },
    ResponseFormat::Json(config) => {
      info!("JSON configuration:");
      info!("Validating field mappings:");
      for (field, map) in &config.field_maps {
        info!("- {}: {}", field, map.path);
        if let Some(transform) = &map.transform {
          info!("  with transformation: {:?}", transform);
        }
      }
    },
  }

  // Test pattern matching if input provided
  if let Some(input) = input {
    info!("Testing identifier pattern matching...");
    match retriever.extract_identifier(input) {
      Ok(identifier) => {
        info!("✓ Successfully extracted identifier: {}", identifier);

        // Try fetching
        info!("Testing retrieval...");
        match retriever.retrieve_paper(input).await {
          Ok(paper) => {
            info!("✓ Successfully retrieved paper:");
            info!("Title: {}", paper.title);
            info!("Authors: {:?}", paper.authors);

            // Test PDF download if available
            if let Some(url) = &paper.pdf_url {
              info!("Testing PDF download from: {}", url);
              let tempdir = tempfile::tempdir().unwrap();
              match paper.download_pdf(tempdir.path()).await {
                Ok(filename) => {
                  let pdf_path = tempdir.path().join(filename);
                  if pdf_path.exists() {
                    let metadata = std::fs::metadata(&pdf_path).unwrap();
                    info!("✓ PDF downloaded successfully ({} bytes)", metadata.len());
                  } else {
                    error!("PDF download failed - file not created");
                  }
                },
                Err(e) => error!("PDF download failed: {}", e),
              }
            } else {
              warn!("No PDF URL available");
            }
          },
          Err(e) => error!("Retrieval failed: {}", e),
        }
      },
      Err(e) => error!("Pattern matching failed: {}", e),
    }
  } else {
    info!("No test input provided - skipping retrieval tests");
    info!("To test retrieval, provide an identifier like: 2301.07041");
  }
}
