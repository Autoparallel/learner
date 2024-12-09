use std::collections::HashSet;

use serde_json::{Map, Number};

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

// Custom deserialization to handle the flattened field structure
impl<'de> Deserialize<'de> for Template {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where D: serde::Deserializer<'de> {
    #[derive(Deserialize)]
    struct TemplateHelper {
      name:        String,
      #[serde(default)]
      description: Option<String>,
      #[serde(flatten)]
      fields:      BTreeMap<String, FieldDefinition>,
    }

    let helper = TemplateHelper::deserialize(deserializer)?;

    // Filter out metadata fields and set field names
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
  pub name: String,

  /// Whether this field must be present
  #[serde(default)]
  pub required: bool,

  /// Human-readable description
  #[serde(default)]
  pub description: Option<String>,

  /// The base type of this field (string, number, array, object)
  pub base_type: String,

  /// Validation rules for this type
  #[serde(default)]
  pub validation: Option<ValidationRules>,

  /// Element type if this is an array type
  #[serde(default)]
  pub items: Option<Box<FieldDefinition>>,

  /// Fields if this is an object type
  #[serde(default)]
  pub fields: Option<Vec<FieldDefinition>>,
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
  pub fn validate(&self, resource: &TemplatedItem) -> Result<()> {
    // Check required fields are present
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
        field.validate_with_path(value, &field.name)?;
      }
    }

    Ok(())
  }
}

impl FieldDefinition {
  fn validate_with_path(&self, value: &Value, path: &str) -> Result<()> {
    match (self.base_type.as_str(), value) {
      ("string", Value::String(s)) => self.validate_string(s, path),
      ("number", Value::Number(n)) => self.validate_number(n, path),
      ("array", Value::Array(items)) => self.validate_array(items, path),
      ("object", Value::Object(obj)) => self.validate_object(obj, path),
      ("boolean", Value::Bool(_)) => Ok(()),
      ("null", Value::Null) => Ok(()),
      _ => Err(LearnerError::TemplateInvalidation(format!(
        "Field '{}' expected type '{}' but got '{}'",
        path,
        self.base_type,
        type_name_of_value(value)
      ))),
    }
  }

  fn validate_string(&self, value: &str, path: &str) -> Result<()> {
    if let Some(rules) = &self.validation {
      // Length constraints
      if let Some(min_length) = rules.min_length {
        if value.len() < min_length {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must be at least {} characters",
            path, min_length
          )));
        }
      }
      if let Some(max_length) = rules.max_length {
        if value.len() > max_length {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' cannot exceed {} characters",
            path, max_length
          )));
        }
      }

      // Pattern matching
      if let Some(pattern) = &rules.pattern {
        let re = Regex::new(pattern)
          .map_err(|_| LearnerError::TemplateInvalidation("Invalid regex pattern".into()))?;
        if !re.is_match(value) {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must match pattern: {}",
            path, pattern
          )));
        }
      }

      // DateTime validation
      if rules.datetime == Some(true) && DateTime::parse_from_rfc3339(value).is_err() {
        return Err(LearnerError::TemplateInvalidation(format!(
          "Field '{}' must be a valid RFC3339 datetime",
          path
        )));
      }

      // Enum validation
      if let Some(allowed) = &rules.enum_values {
        if !allowed.contains(&value.to_string()) {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must be one of: {:?}",
            path, allowed
          )));
        }
      }
    }
    Ok(())
  }

  fn validate_number(&self, value: &Number, path: &str) -> Result<()> {
    if let Some(rules) = &self.validation {
      if let Some(num) = value.as_f64() {
        if let Some(min) = rules.minimum {
          if num < min {
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{}' must be at least {}",
              path, min
            )));
          }
        }
        if let Some(max) = rules.maximum {
          if num > max {
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{}' cannot exceed {}",
              path, max
            )));
          }
        }
        if let Some(multiple) = rules.multiple_of {
          let ratio = num / multiple;
          if (ratio - ratio.round()).abs() > f64::EPSILON {
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{}' must be a multiple of {}",
              path, multiple
            )));
          }
        }
      }
    }
    Ok(())
  }

  fn validate_array(&self, items: &[Value], path: &str) -> Result<()> {
    if let Some(rules) = &self.validation {
      if let Some(min_items) = rules.min_items {
        if items.len() < min_items {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must have at least {} items",
            path, min_items
          )));
        }
      }
      if let Some(max_items) = rules.max_items {
        if items.len() > max_items {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' cannot exceed {} items",
            path, max_items
          )));
        }
      }
      if rules.unique_items == Some(true) {
        let mut seen = HashSet::new();
        for item in items {
          let item_str = serde_json::to_string(item).map_err(|_| {
            LearnerError::TemplateInvalidation("Failed to serialize array item".into())
          })?;
          if !seen.insert(item_str) {
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{}' contains duplicate items",
              path
            )));
          }
        }
      }
    }

    // Validate each item if we have an item type definition
    if let Some(item_type) = &self.items {
      for (index, item) in items.iter().enumerate() {
        item_type.validate_with_path(item, &format!("{}[{}]", path, index)).map_err(|e| {
          LearnerError::TemplateInvalidation(format!(
            "Invalid item at index {} in array '{}': {}",
            index, path, e
          ))
        })?;
      }
    }

    Ok(())
  }

  fn validate_object(&self, obj: &Map<String, Value>, path: &str) -> Result<()> {
    if let Some(fields) = &self.fields {
      for field in fields {
        if let Some(value) = obj.get(&field.name) {
          field.validate_with_path(value, &format!("{}.{}", path, field.name))?;
        } else if field.required {
          return Err(LearnerError::TemplateInvalidation(format!(
            "Missing required field '{}' in object '{}'",
            field.name, path
          )));
        }
      }
    }
    Ok(())
  }
}

fn type_name_of_value(value: &Value) -> &'static str {
  match value {
    Value::String(_) => "string",
    Value::Number(_) => "number",
    Value::Bool(_) => "boolean",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
    Value::Null => "null",
  }
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
    todo!("Fix this")
    // let template = Template {
    //   name:        "Test Template".to_string(),
    //   description: None,
    //   fields:      vec![FieldDefinition {
    //     name:            "timestamp".into(),
    //     field_type:      "string".into(),
    //     required:        true,
    //     description:     None,
    //     default:         None,
    //     validation:      Some(ValidationRules { datetime: Some(true), ..Default::default() }),
    //     type_definition: None,
    //   }],
    // };

    // let valid_resource = BTreeMap::from([("timestamp".into(), json!("2024-01-01T00:00:00Z"))]);
    // template.validate(&valid_resource).unwrap();

    // let invalid_resource = BTreeMap::from([
    //   ("timestamp".into(), json!("2024-01-01")), // Not RFC3339
    // ]);
    // assert!(template.validate(&invalid_resource).is_err());
  }
}
