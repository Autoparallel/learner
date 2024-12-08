use std::collections::HashSet;

use super::*;

// Type alias for clarity and consistency
pub type TemplatedItem = BTreeMap<String, Value>;

#[derive(Debug, Clone, Serialize)]
pub struct Template {
  pub name:        String,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub fields:      Vec<FieldDefinition>,
}
impl<'de> Deserialize<'de> for Template {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where D: serde::Deserializer<'de> {
    // Helper struct to capture the raw TOML structure
    #[derive(Deserialize)]
    struct TemplateHelper {
      name:        String,
      #[serde(default)]
      description: Option<String>,
      #[serde(flatten)]
      fields:      BTreeMap<String, FieldDefinition>,
    }

    // Deserialize into our helper first
    let helper = TemplateHelper::deserialize(deserializer)?;

    // Convert the field map into a Vec, setting the name from the key
    // Filter out the metadata fields we don't want to treat as FieldDefinitions
    let fields = helper
      .fields
      .into_iter()
      .filter(|(key, _)| key != "name" && key != "description")
      .map(|(key, mut field_def)| {
        field_def.name = key;
        field_def
      })
      .collect();

    Ok(Template { name: helper.name, description: helper.description, fields })
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
  /// Name of the field
  #[serde(skip_deserializing)]
  pub name:        String,
  /// Type of the field (should be a JSON Value type)
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

  pub type_definition: Option<TypeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
  // For array types, defines the structure of elements
  pub element_type: Option<Box<FieldDefinition>>,
  // For table types, defines the fields
  pub fields:       Option<Vec<FieldDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
  pub datetime:    Option<bool>,        // Validates RFC3339 format
}

impl Template {
  // TODO: Make this just return a `Result<()>`
  pub fn validate(&self, resource: &TemplatedItem) -> Result<()> {
    // Check required fields
    for field in &self.fields {
      if field.required && !resource.contains_key(&field.name) {
        return Err(LearnerError::TemplateInvalidation(format!(
          "Missing required field: {}",
          field.name
        )));
      }
    }

    // Validate each provided field
    for (name, value) in resource {
      if let Some(field) = self.fields.iter().find(|f| f.name == *name) {
        // Validate field value against its definition
        self.validate_field(field, value)?;
      }
    }

    Ok(())
  }

  /// Validates a single field value against its definition
  fn validate_field(&self, field: &FieldDefinition, value: &Value) -> Result<()> {
    match (field.field_type.as_str(), value) {
      // String validation - handles both basic type checking and string-specific rules
      ("string", Value::String(v)) => {
        if let Some(rules) = &field.validation {
          // Length constraints
          if let Some(min_length) = rules.min_length {
            if v.len() < min_length {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' must be at least {} characters",
                field.name, min_length
              )));
            }
          }
          if let Some(max_length) = rules.max_length {
            if v.len() > max_length {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' cannot exceed {} characters",
                field.name, max_length
              )));
            }
          }

          // Pattern matching via regex
          if let Some(pattern) = &rules.pattern {
            let re = Regex::new(pattern)
              .map_err(|_| LearnerError::TemplateInvalidation("Invalid regex pattern".into()))?;
            if !re.is_match(v) {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' must match pattern: {}",
                field.name, pattern
              )));
            }
          }

          // Datetime validation if specified
          if rules.datetime == Some(true) && DateTime::parse_from_rfc3339(v).is_err() {
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{}' must be a valid RFC3339 datetime",
              field.name
            )));
          }

          // Enumerated values check
          if let Some(allowed) = &rules.enum_values {
            if !allowed.contains(v) {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' must be one of: {:?}",
                field.name, allowed
              )));
            }
          }
        }
        Ok(())
      },

      // Numeric validations - handle both number types
      ("number", Value::Number(n)) => {
        if let Some(rules) = &field.validation {
          if let Some(num) = n.as_f64() {
            validate_numeric(field, num, rules)?;
          }
        }
        Ok(())
      },

      // Array validation - handles array-specific rules
      ("array", Value::Array(v)) => {
        if let Some(rules) = &field.validation {
          if let Some(min_items) = rules.min_items {
            if v.len() < min_items {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' must have at least {} items",
                field.name, min_items
              )));
            }
          }

          if let Some(max_items) = rules.max_items {
            if v.len() > max_items {
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' cannot exceed {} items",
                field.name, max_items
              )));
            }
          }

          if rules.unique_items == Some(true) {
            let mut seen = HashSet::new();
            for item in v {
              let item_str = serde_json::to_string(item).map_err(|_| {
                LearnerError::TemplateInvalidation("Failed to serialize array item".into())
              })?;
              if !seen.insert(item_str) {
                return Err(LearnerError::TemplateInvalidation(format!(
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
      ("boolean", Value::Bool(_)) => Ok(()),
      ("object", Value::Object(_)) => Ok(()),
      ("null", Value::Null) => Ok(()),

      // Type mismatch - provide a clear error message
      _ => Err(LearnerError::TemplateInvalidation(format!(
        "Field '{}' expected type '{}' but got '{}'",
        field.name,
        field.field_type,
        match value {
          Value::String(_) => "string",
          Value::Number(_) => "number",
          Value::Bool(_) => "boolean",
          Value::Array(_) => "array",
          Value::Object(_) => "object",
          Value::Null => "null",
        }
      ))),
    }
  }
}

fn validate_numeric(field: &FieldDefinition, value: f64, rules: &ValidationRules) -> Result<()> {
  if let Some(min) = rules.minimum {
    if value < min {
      return Err(LearnerError::TemplateInvalidation(format!(
        "Field '{}' must be at least {}",
        field.name, min
      )));
    }
  }

  if let Some(max) = rules.maximum {
    if value > max {
      return Err(LearnerError::TemplateInvalidation(format!(
        "Field '{}' cannot exceed {}",
        field.name, max
      )));
    }
  }

  if let Some(multiple) = rules.multiple_of {
    let ratio = value / multiple;
    if (ratio - ratio.round()).abs() > f64::EPSILON {
      return Err(LearnerError::TemplateInvalidation(format!(
        "Field '{}' must be a multiple of {}",
        field.name, multiple
      )));
    }
  }

  Ok(())
}

/// Convert DateTime to RFC3339 string for JSON storage
pub fn datetime_to_json(dt: DateTime<Utc>) -> String { dt.to_rfc3339() }

/// Parse RFC3339 string from JSON into DateTime
pub fn datetime_from_json(s: &str) -> Result<DateTime<Utc>> {
  DateTime::parse_from_rfc3339(s)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| LearnerError::TemplateInvalidation(format!("Invalid datetime format: {}", e)))
}
#[cfg(test)]
mod tests {
  use chrono::TimeZone;
  use serde_json::json;

  use super::*;

  #[test]
  fn validate_paper_configuration() {
    let template = include_str!("../config/resources/paper.toml");
    let template: Template = toml::from_str(template).unwrap();

    let date = datetime_to_json(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    // Create a valid paper resource
    let paper_resource = BTreeMap::from([
      ("title".into(), json!("Understanding Quantum Computing")),
      (
        "authors".into(),
        json!([{
            "name": "Alice Researcher",
            "affiliation": "Tech University"
        }]),
      ),
      ("publication_date".into(), json!(date)),
      ("doi".into(), json!("10.1234/example.123")),
    ]);

    // Validate the paper
    template.validate(&paper_resource).unwrap();

    // Test required field validation
    let invalid_paper = BTreeMap::from([
      ("authors".into(), json!([])), // Missing title
    ]);
    assert!(template.validate(&invalid_paper).is_err());
  }

  #[test]
  fn validate_book_configuration() {
    let template = include_str!("../config/resources/book.toml");
    let template: Template = toml::from_str(template).unwrap();

    let date = datetime_to_json(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    let book_resource = BTreeMap::from([
      ("title".into(), json!("Advanced Quantum Computing")),
      ("authors".into(), json!(["Alice Writer", "Bob Author"])),
      ("isbn".into(), json!("978-0-12-345678-9")),
      ("publisher".into(), json!("Academic Press")),
      ("publication_date".into(), json!(date)),
    ]);

    template.validate(&book_resource).unwrap();
  }

  #[test]
  fn validate_thesis_configuration() {
    let template = include_str!("../config/resources/thesis.toml");
    let template: Template = toml::from_str(template).unwrap();

    let date = datetime_to_json(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().unwrap());

    let thesis_resource = BTreeMap::from([
      ("title".into(), json!("Novel Approaches to Quantum Error Correction")),
      ("author".into(), json!("Alice Researcher")),
      ("degree".into(), json!("PhD")),
      ("institution".into(), json!("Tech University")),
      ("completion_date".into(), json!(date)),
      ("advisors".into(), json!(["Prof. Bob Supervisor"])),
    ]);

    template.validate(&thesis_resource).unwrap();

    // Test degree enum validation
    let mut invalid_thesis = thesis_resource.clone();
    invalid_thesis.insert("degree".into(), json!("InvalidDegree"));
    assert!(template.validate(&invalid_thesis).is_err());
  }

  #[test]
  fn test_datetime_validation() {
    let template = Template {
      name:        "Test Template".to_string(),
      description: None,
      fields:      vec![FieldDefinition {
        name:            "timestamp".into(),
        field_type:      "string".into(),
        required:        true,
        description:     None,
        default:         None,
        validation:      Some(ValidationRules { datetime: Some(true), ..Default::default() }),
        type_definition: None,
      }],
    };

    let valid_resource = BTreeMap::from([("timestamp".into(), json!("2024-01-01T00:00:00Z"))]);
    template.validate(&valid_resource).unwrap();

    let invalid_resource = BTreeMap::from([
      ("timestamp".into(), json!("2024-01-01")), // Not RFC3339
    ]);
    assert!(template.validate(&invalid_resource).is_err());
  }
}
