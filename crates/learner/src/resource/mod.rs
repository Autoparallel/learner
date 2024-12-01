use std::{collections::HashSet, str::FromStr};

use super::*;

mod paper;
mod shared;

pub use paper::*;
pub use shared::*;
use toml::Value;

// TODO (autoparallel): We almost need something like `Resource` to be given by these
// `ResourceConfig`s. Or, even renaming these like `ResourceTemplates` or something so a `Resource`
// has to fit into the `ResourceTemplate` (now that I type this out, `ResourceConfig` is still a
// reasonable name). But when we want to retrieve a resource, we need to actually get back a
// resource. Perhaps its just:
pub type Resource = BTreeMap<String, Value>;

#[derive(Debug, Clone, Default)]
pub struct Resources {
  configs: BTreeMap<String, ResourceConfig>,
}

impl Resources {
  pub fn new() -> Self { Self::default() }
}

impl Configurable for Resources {
  type Config = ResourceConfig;

  fn as_map(&mut self) -> &mut BTreeMap<String, Self::Config> { &mut self.configs }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
  /// The type identifier for this resource
  pub name:        String,
  /// Optional description of this resource type
  #[serde(default)]
  pub description: Option<String>,
  /// Field definitions with optional metadata
  #[serde(default)]
  pub fields:      Vec<FieldDefinition>,
}

impl Identifiable for ResourceConfig {
  fn name(&self) -> String { self.name.clone() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
  /// Name of the field
  pub name:        String,
  /// Type of the field (should be a TOML Value)
  pub field_type:  String,
  /// Whether this field must be present
  #[serde(default)]
  pub required:    bool,
  /// Human-readable description
  #[serde(default)]
  pub description: Option<String>,
  /// Default value if field is absent
  #[serde(default)]
  pub default:     Option<Value>,
  /// Optional validation rules
  #[serde(default)]
  pub validation:  Option<ValidationRules>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
  // String validations
  pub pattern:    Option<String>, // Regex pattern to match
  pub min_length: Option<usize>,  // Minimum string length
  pub max_length: Option<usize>,  // Maximum string length

  // Numeric validations
  pub minimum:     Option<f64>, // Minimum value
  pub maximum:     Option<f64>, // Maximum value
  pub multiple_of: Option<f64>, // Must be multiple of this value

  // Array validations
  pub min_items:    Option<usize>, // Minimum array length
  pub max_items:    Option<usize>, // Maximum array length
  pub unique_items: Option<bool>,  // Whether items must be unique

  // General validations
  pub enum_values: Option<Vec<String>>, // List of allowed values
}

impl ResourceConfig {
  /// Validates a set of values against this resource configuration
  pub fn validate(&self, values: &toml::value::Table) -> Result<bool> {
    // Check required fields
    for field in &self.fields {
      if field.required {
        if !values.contains_key(&field.name) {
          return Err(LearnerError::InvalidResource(format!(
            "Missing required field: {}",
            field.name
          )));
        }
      }
    }

    // Validate each provided field
    for (name, value) in values {
      if let Some(field) = self.fields.iter().find(|f| f.name == *name) {
        // Validate field value against its definition
        self.validate_field(field, value)?;
      }
    }

    Ok(true)
  }

  /// Validates a single field value against its definition
  fn validate_field(&self, field: &FieldDefinition, value: &toml::Value) -> Result<()> {
    // First validate that the provided value matches the declared type
    match (field.field_type.as_str(), value) {
      // String validation - handles both basic type checking and string-specific rules
      ("string", toml::Value::String(v)) => {
        if let Some(rules) = &field.validation {
          // Length constraints
          if let Some(min_length) = rules.min_length {
            if v.len() < min_length {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' must be at least {} characters",
                field.name, min_length
              )));
            }
          }
          if let Some(max_length) = rules.max_length {
            if v.len() > max_length {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' cannot exceed {} characters",
                field.name, max_length
              )));
            }
          }

          // Pattern matching via regex
          if let Some(pattern) = &rules.pattern {
            dbg!(&pattern);
            let re = Regex::new(pattern)
              .map_err(|_| LearnerError::InvalidResource("Invalid regex pattern".into()))?;
            if !re.is_match(v) {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' must match pattern: {}",
                field.name, pattern
              )));
            }
          }

          // Enumerated values check
          if let Some(allowed) = &rules.enum_values {
            if !allowed.contains(v) {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' must be one of: {:?}",
                field.name, allowed
              )));
            }
          }
        }
        Ok(())
      },

      // Numeric validations - handle both integer and float values
      ("integer", toml::Value::Integer(v)) => {
        if let Some(rules) = &field.validation {
          validate_numeric(field, *v as f64, rules)?;
        }
        Ok(())
      },

      ("float", toml::Value::Float(v)) => {
        if let Some(rules) = &field.validation {
          validate_numeric(field, *v, rules)?;
        }
        Ok(())
      },

      // Array validation - handles array-specific rules
      ("array", toml::Value::Array(v)) => {
        if let Some(rules) = &field.validation {
          if let Some(min_items) = rules.min_items {
            if v.len() < min_items {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' must have at least {} items",
                field.name, min_items
              )));
            }
          }

          if let Some(max_items) = rules.max_items {
            if v.len() > max_items {
              return Err(LearnerError::InvalidResource(format!(
                "Field '{}' cannot exceed {} items",
                field.name, max_items
              )));
            }
          }

          if rules.unique_items == Some(true) {
            let mut seen = HashSet::new();
            for item in v {
              let item_str = toml::to_string(item).map_err(|_| {
                LearnerError::InvalidResource("Failed to serialize array item".into())
              })?;
              if !seen.insert(item_str) {
                return Err(LearnerError::InvalidResource(format!(
                  "Field '{}' contains duplicate items",
                  field.name
                )));
              }
            }
          }
        }
        Ok(())
      },

      // Simple type validations - just ensure type matches
      ("boolean", toml::Value::Boolean(_)) => Ok(()),
      ("datetime", toml::Value::Datetime(_)) => Ok(()),
      ("table", toml::Value::Table(_)) => Ok(()),

      // Type mismatch - provide a clear error message
      _ => Err(LearnerError::InvalidResource(format!(
        "Field '{}' expected type '{}' but got '{}'",
        field.name,
        field.field_type,
        match value {
          toml::Value::String(_) => "string",
          toml::Value::Integer(_) => "integer",
          toml::Value::Float(_) => "float",
          toml::Value::Boolean(_) => "boolean",
          toml::Value::Datetime(_) => "datetime",
          toml::Value::Array(_) => "array",
          toml::Value::Table(_) => "table",
        }
      ))),
    }
  }
}

fn validate_numeric(field: &FieldDefinition, value: f64, rules: &ValidationRules) -> Result<()> {
  if let Some(min) = rules.minimum {
    if value < min {
      return Err(LearnerError::InvalidResource(format!(
        "Field '{}' must be at least {}",
        field.name, min
      )));
    }
  }

  if let Some(max) = rules.maximum {
    if value > max {
      return Err(LearnerError::InvalidResource(format!(
        "Field '{}' cannot exceed {}",
        field.name, max
      )));
    }
  }

  if let Some(multiple) = rules.multiple_of {
    let ratio = value / multiple;
    if (ratio - ratio.round()).abs() > f64::EPSILON {
      return Err(LearnerError::InvalidResource(format!(
        "Field '{}' must be a multiple of {}",
        field.name, multiple
      )));
    }
  }

  Ok(())
}
// Convert from chrono DateTime to TOML Datetime
pub fn chrono_to_toml_datetime(dt: DateTime<Utc>) -> toml::value::Datetime {
  // TOML datetime is stored as seconds since Unix epoch
  toml::value::Datetime::from_str(&dt.to_rfc3339()).unwrap()
}

// Convert from TOML Datetime to chrono DateTime
pub fn toml_to_chrono_datetime(dt: toml::value::Datetime) -> DateTime<Utc> {
  // Create DateTime from Unix timestamp
  DateTime::parse_from_rfc3339(&dt.to_string()).unwrap().to_utc()
}

#[cfg(test)]
mod tests {
  use chrono::TimeZone;

  use super::*;

  #[test]
  fn validate_paper_configuration() {
    let config = include_str!("../../config/resources/paper.toml");
    let config: ResourceConfig = toml::from_str(config).unwrap();

    let date = chrono_to_toml_datetime(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    // Create a valid paper
    let paper_values = toml::value::Table::from_iter([
      ("title".into(), toml::Value::String("Understanding Quantum Computing".into())),
      (
        "authors".into(),
        toml::Value::Array(vec![toml::Value::Table(toml::value::Table::from_iter([
          ("name".into(), toml::Value::String("Alice Researcher".into())),
          ("affiliation".into(), toml::Value::String("Tech University".into())),
        ]))]),
      ),
      ("publication_date".into(), toml::Value::Datetime(date)),
      ("doi".into(), toml::Value::String("10.1234/example.123".into())),
    ]);

    // Validate the paper
    assert!(config.validate(&paper_values).unwrap());

    // Test required field validation
    let invalid_paper = toml::value::Table::from_iter([
      ("authors".into(), toml::Value::Array(vec![])), // Missing title
    ]);
    assert!(config.validate(&invalid_paper).is_err());
  }

  #[test]
  fn validate_book_configuration() {
    let config = include_str!("../../config/resources/book.toml");
    let config: ResourceConfig = toml::from_str(config).unwrap();

    let date = chrono_to_toml_datetime(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    let book_values = toml::value::Table::from_iter([
      ("title".into(), toml::Value::String("Advanced Quantum Computing".into())),
      (
        "authors".into(),
        toml::Value::Array(vec![
          toml::Value::String("Alice Writer".into()),
          toml::Value::String("Bob Author".into()),
        ]),
      ),
      ("isbn".into(), toml::Value::String("978-0-12-345678-9".into())),
      ("publisher".into(), toml::Value::String("Academic Press".into())),
      ("publication_date".into(), toml::Value::Datetime(date)),
    ]);

    assert!(config.validate(&book_values).unwrap());
  }

  #[test]
  fn validate_thesis_configuration() {
    let config = include_str!("../../config/resources/thesis.toml");
    let config: ResourceConfig = toml::from_str(config).unwrap();

    let date = chrono_to_toml_datetime(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    let thesis_values = toml::value::Table::from_iter([
      ("title".into(), toml::Value::String("Novel Approaches to Quantum Error Correction".into())),
      ("author".into(), toml::Value::String("Alice Researcher".into())),
      ("degree".into(), toml::Value::String("PhD".into())),
      ("institution".into(), toml::Value::String("Tech University".into())),
      ("completion_date".into(), toml::Value::Datetime(date)),
      (
        "advisors".into(),
        toml::Value::Array(vec![toml::Value::String("Prof. Bob Supervisor".into())]),
      ),
    ]);

    assert!(config.validate(&thesis_values).unwrap());

    // Test degree enum validation
    let mut invalid_thesis = thesis_values.clone();
    invalid_thesis.insert("degree".into(), toml::Value::String("InvalidDegree".into()));
    assert!(config.validate(&invalid_thesis).is_err());
  }
}
