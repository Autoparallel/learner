use std::fs::read_to_string;

use console::style;
use learner::{
  resource::ResourceConfig,
  retriever::{ResponseFormat, RetrieverConfig},
};

use super::*;

/// Formats a validation header with consistent styling
fn print_validation_header(message: &str) {
  println!("\n{}", style(message).bold().cyan());
}

/// Formats a validation section header
fn print_section_header(message: &str) {
  println!("\n{}", style(message).bold());
}

/// Prints a success message with a checkmark
fn print_success(message: &str) {
  println!("{} {}", style("✓").bold().green(), message);
}

/// Formats field information consistently
fn print_field_info(name: &str, field_type: &str, indent: usize) {
  let indent_str = " ".repeat(indent);
  println!("{}Field '{}' ({})", indent_str, style(name).bold(), style(field_type).italic());
}

pub fn validate_resource(path: &PathBuf) {
  print_validation_header("Resource Configuration Validation");
  info!("Loading configuration from: {}", path.display());

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
      error!("\nDetailed error: {}", e);
      return;
    },
  };

  // Resource overview
  print_section_header("Resource Overview");
  println!("Name: {}", style(&resource.name).bold());
  if let Some(desc) = &resource.description {
    println!("Description: {}", desc);
  }

  // Field validation
  print_section_header(&format!("Field Definitions ({})", style(resource.fields.len()).bold()));

  for field in &resource.fields {
    match field.field_type.as_str() {
      "string" | "integer" | "float" | "boolean" | "datetime" | "array" | "table" => {
        print_field_info(&field.name, &field.field_type, 0);

        // Show field metadata
        if let Some(desc) = &field.description {
          println!("  Description: {}", desc);
        }

        if field.required {
          println!("  {}", style("Required: true").yellow());
        }

        // Validate and show default values
        if let Some(default) = &field.default {
          match (field.field_type.as_str(), default) {
            ("string", toml::Value::String(_))
            | ("integer", toml::Value::Integer(_))
            | ("float", toml::Value::Float(_))
            | ("boolean", toml::Value::Boolean(_))
            | ("datetime", toml::Value::Datetime(_))
            | ("array", toml::Value::Array(_))
            | ("table", toml::Value::Table(_)) => {
              print_success("Default value has correct type");
            },
            _ => {
              println!("  {}: Default value type mismatch", style("ERROR").red());
              println!(
                "    Expected {}, got {}",
                style(&field.field_type).bold(),
                style(default.type_str()).red()
              );
            },
          }
        }

        // Show validation rules with better formatting
        if let Some(rules) = &field.validation {
          println!("  Validation Rules:");
          match field.field_type.as_str() {
            "string" => {
              if let Some(pattern) = &rules.pattern {
                match regex::Regex::new(pattern) {
                  Ok(_) =>
                    println!("    {} Pattern: {}", style("✓").green(), style(pattern).italic()),
                  Err(e) => println!("    {} Invalid regex pattern: {}", style("✗").red(), e),
                }
              }
              if let Some(min) = rules.min_length {
                println!("    Minimum length: {}", min);
              }
              if let Some(max) = rules.max_length {
                println!("    Maximum length: {}", max);
              }
            },
            "array" => {
              if let Some(min) = rules.min_items {
                println!("    Minimum items: {}", min);
              }
              if let Some(max) = rules.max_items {
                println!("    Maximum items: {}", max);
              }
              if rules.unique_items == Some(true) {
                println!("    {}", style("Items must be unique").yellow());
              }
            },
            _ => {},
          }
        }
        println!(); // Add spacing between fields
      },
      invalid_type => {
        println!("\n{}", style("ERROR").red().bold());
        println!(
          "Field '{}' has invalid type: {}",
          style(&field.name).bold(),
          style(invalid_type).red()
        );
        println!("Valid types are: string, integer, float, boolean, datetime, array, table");
      },
    }
  }

  print_success("Resource configuration validation complete!");
}

pub async fn validate_retriever(path: &PathBuf, input: &Option<String>) {
  print_validation_header("Retriever Configuration Validation");
  info!("Loading configuration from: {}", path.display());

  // Read and parse configuration
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
      error!("\nDetailed error: {}", e);
      return;
    },
  };

  // Basic configuration overview
  print_section_header("Basic Configuration");
  println!("Name: {}", style(&retriever.name).bold());
  println!("Source: {}", retriever.source);

  // URL validation
  print_section_header("URL Configuration");
  match reqwest::Url::parse(&retriever.base_url) {
    Ok(url) => {
      print_success(&format!("Valid base URL: {}", style(url).green()));
    },
    Err(e) => {
      println!("{} Invalid base URL: {}", style("✗").red(), e);
      return;
    },
  }

  // Endpoint template validation
  if retriever.endpoint_template.contains("{identifier}") {
    print_success("Endpoint template contains required {identifier} placeholder");
  } else {
    println!(
      "{} Endpoint template must contain {{identifier}} placeholder",
      style("✗").red().bold()
    );
    println!("Current template: {}", style(&retriever.endpoint_template).italic());
    return;
  }

  // Response format validation
  print_section_header("Response Format Configuration");
  match &retriever.response_format {
    ResponseFormat::Xml(config) => {
      println!("Format: {}", style("XML").cyan());
      println!(
        "Namespace handling: {}",
        if config.strip_namespaces {
          style("Stripping enabled").green()
        } else {
          style("Preserving namespaces").yellow()
        }
      );

      println!("\nField Mappings:");
      for (field, map) in &config.field_maps {
        println!("- {}: {}", style(field).bold(), map.path);
        if let Some(transform) = &map.transform {
          println!("  Transform: {}", style(format!("{:?}", transform)).italic());
        }
      }
    },
    ResponseFormat::Json(config) => {
      println!("Format: {}", style("JSON").cyan());
      println!("\nField Mappings:");
      for (field, map) in &config.field_maps {
        println!("- {}: {}", style(field).bold(), map.path);
        if let Some(transform) = &map.transform {
          println!("  Transform: {}", style(format!("{:?}", transform)).italic());
        }
      }
    },
  }

  // Live testing if input provided
  if let Some(input) = input {
    print_section_header("Live Testing");
    println!("Testing with input: {}", style(input).cyan());

    match retriever.extract_identifier(input) {
      Ok(identifier) => {
        print_success(&format!("Extracted identifier: {}", style(identifier).green()));

        // Paper retrieval test
        println!("\nAttempting paper retrieval...");
        match retriever.retrieve_paper(input).await {
          Ok(paper) => {
            print_success("Paper retrieved successfully");
            println!("\nPaper Details:");
            println!("Title: {}", style(&paper.title).bold());
            println!(
              "Authors: {}",
              paper.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")
            );

            // PDF download test
            if paper.pdf_url.is_some() {
              println!("\nTesting PDF download capability...");
              let tempdir = tempfile::tempdir().unwrap();
              match paper.download_pdf(tempdir.path()).await {
                Ok(filename) => {
                  let pdf_path = tempdir.path().join(filename);
                  if pdf_path.exists() {
                    let metadata = std::fs::metadata(&pdf_path).unwrap();
                    print_success(&format!(
                      "PDF downloaded successfully ({} bytes)",
                      style(metadata.len()).green()
                    ));
                  } else {
                    println!("{} PDF download failed - file not created", style("✗").red());
                  }
                },
                Err(e) => println!("{} PDF download failed: {}", style("✗").red(), e),
              }
            } else {
              println!("{} No PDF URL available for testing", style("!").yellow());
            }
          },
          Err(e) => println!("{} Retrieval failed: {}", style("✗").red(), e),
        }
      },
      Err(e) => println!("{} Pattern matching failed: {}", style("✗").red(), e),
    }
  } else {
    println!("\n{} No test input provided - skipping retrieval tests", style("!").yellow());
    println!("Tip: Provide an identifier (like '2301.07041') to test live retrieval");
  }

  print_success("Retriever configuration validation complete!");
}
