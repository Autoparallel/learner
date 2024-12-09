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

    // Convert map into vec and set top-level field names
    let mut fields: Vec<FieldDefinition> = helper
      .fields
      .into_iter()
      .filter(|(key, _)| key != "name" && key != "description")
      .map(|(key, mut field_def)| {
        field_def.name = key.clone();
        field_def.process_nested_names();
        field_def
      })
      .collect();

    Ok(Template { name: helper.name, description: helper.description, fields })
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
  /// Name of the field
  #[serde(default)]
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
  #[instrument(
      skip(self, resource),
      fields(
          template_name = %self.name,
          field_count = %self.fields.len()
      ),
      level = "debug"
  )]
  pub fn validate(&self, resource: &TemplatedItem) -> Result<()> {
    info!("Starting template validation");

    // First validate all required fields are present
    for field in &self.fields {
      if field.required && !resource.contains_key(&field.name) {
        error!(
            field = %field.name,
            required = true,
            validation_type = "required_field",
            "Validation failed: missing required field"
        );
        return Err(LearnerError::TemplateInvalidation(format!(
          "Missing required field: {}",
          field.name
        )));
      }
    }

    // Then validate each provided field
    for (name, value) in resource {
      if let Some(field) = self.fields.iter().find(|f| f.name == *name) {
        debug!(
            field = %name,
            field_type = %field.base_type,
            required = %field.required,
            "Validating field"
        );

        if let Err(e) = field.validate_with_path(value, &field.name) {
          error!(
              field = %name,
              error = %e,
              "Field validation failed"
          );
          return Err(e);
        }
      } else {
        warn!(
            field = %name,
            "Found unexpected field in resource"
        );
      }
    }

    info!("Template validation completed successfully");
    Ok(())
  }
}

impl FieldDefinition {
  #[instrument(
    skip(self),
    fields(
        field_name = %self.name,
        field_type = %self.base_type,
        has_items = %self.items.is_some(),
        has_fields = %self.fields.is_some()
    ),
    level = "debug"
)]
  fn process_nested_names(&mut self) {
    if let Some(items) = &mut self.items {
      items.process_nested_names();
    }

    if let Some(fields) = &mut self.fields {
      for (index, field) in fields.iter_mut().enumerate() {
        if field.name.is_empty() {
          error!(
              parent_field = %self.name,
              field_index = index,
              field_type = %field.base_type,
              "Found empty field name in template definition"
          );
        }
        field.process_nested_names();
      }
    }
  }

  #[instrument(
      skip(self, value),
      fields(
          field_name = %self.name,
          field_type = %self.base_type,
          required = %self.required,
          has_validation = %self.validation.is_some(),
          has_items = %self.items.is_some(),
          has_fields = %self.fields.is_some()
      )
  )]
  fn validate_with_path(&self, value: &Value, path: &str) -> Result<()> {
    debug!(
        path = %path,
        value = ?value,
        "Starting field validation"
    );

    let result = match (self.base_type.as_str(), value) {
      ("string", Value::String(s)) => self.validate_string(s, path),
      ("number", Value::Number(n)) => self.validate_number(n, path),
      ("array", Value::Array(items)) => self.validate_array(items, path),
      ("object", Value::Object(obj)) => self.validate_object(obj, path),
      ("boolean", Value::Bool(_)) | ("null", Value::Null) => Ok(()),
      _ => {
        error!(
            path = %path,
            expected_type = %self.base_type,
            actual_type = %type_name_of_value(value),
            value = ?value,
            "Type mismatch in field validation"
        );
        Err(LearnerError::TemplateInvalidation(format!(
          "Field '{}' expected type '{}' but got '{}'",
          path,
          self.base_type,
          type_name_of_value(value)
        )))
      },
    };

    if let Err(ref e) = result {
      error!(
          path = %path,
          error = %e,
          "Field validation failed"
      );
    }
    result
  }

  #[instrument(
      skip(self, value),
      fields(
          field_name = %self.name,
          string_length = %value.len(),
          has_validation = %self.validation.is_some()
      )
  )]
  fn validate_string(&self, value: &str, path: &str) -> Result<()> {
    debug!(
        path = %path,
        value = %value,
        "Starting string validation"
    );

    if let Some(rules) = &self.validation {
      // Length constraints
      if let Some(min_length) = rules.min_length {
        if value.len() < min_length {
          error!(
              path = %path,
              min_required = min_length,
              actual_length = value.len(),
              validation_type = "min_length",
              "String validation failed: too short"
          );
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must be at least {} characters (found {})",
            path,
            min_length,
            value.len()
          )));
        }
      }

      if let Some(max_length) = rules.max_length {
        if value.len() > max_length {
          error!(
              path = %path,
              max_allowed = max_length,
              actual_length = value.len(),
              validation_type = "max_length",
              "String validation failed: too long"
          );
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' cannot exceed {} characters (found {})",
            path,
            max_length,
            value.len()
          )));
        }
      }

      // Pattern matching
      if let Some(pattern) = &rules.pattern {
        match Regex::new(pattern) {
          Ok(re) =>
            if !re.is_match(value) {
              error!(
                  path = %path,
                  pattern = %pattern,
                  value = %value,
                  validation_type = "pattern",
                  "String validation failed: pattern mismatch"
              );
              return Err(LearnerError::TemplateInvalidation(format!(
                "Field '{}' must match pattern: {}",
                path, pattern
              )));
            },
          Err(e) => {
            error!(
                path = %path,
                pattern = %pattern,
                error = %e,
                validation_type = "pattern",
                "Invalid regex pattern"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Invalid regex pattern for field '{}': {}",
              path, e
            )));
          },
        }
      }

      // DateTime validation
      if rules.datetime == Some(true) {
        match DateTime::parse_from_rfc3339(value) {
          Ok(_) => {},
          Err(e) => {
            error!(
                path = %path,
                value = %value,
                error = %e,
                validation_type = "datetime",
                "Invalid datetime format"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{path}' must be a valid RFC3339 datetime: {e}",
            )));
          },
        }
      }

      // Enum validation
      if let Some(allowed) = &rules.enum_values {
        if !allowed.contains(&value.to_string()) {
          error!(
              path = %path,
              value = %value,
              allowed_values = ?allowed,
              validation_type = "enum",
              "Invalid enum value"
          );
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{path}' must be one of: {allowed:?}",
          )));
        }
      }
    }
    Ok(())
  }

  #[instrument(
      skip(self, value),
      fields(
          field_name = %self.name,
          has_validation = %self.validation.is_some(),
          number_type = ?value.as_f64().map(|_| "f64").or_else(|| value.as_i64().map(|_| "i64")).or_else(|| value.as_u64().map(|_| "u64"))
      )
  )]
  fn validate_number(&self, value: &Number, path: &str) -> Result<()> {
    debug!(
        path = %path,
        value = %value,
        "Starting number validation"
    );

    if let Some(rules) = &self.validation {
      if let Some(num) = value.as_f64() {
        if let Some(min) = rules.minimum {
          if num < min {
            error!(
                path = %path,
                min_required = min,
                actual = num,
                validation_type = "minimum",
                "Number validation failed: too small"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{path}' must be at least {min} (found {num})",
            )));
          }
        }

        if let Some(max) = rules.maximum {
          if num > max {
            error!(
                path = %path,
                max_allowed = max,
                actual = num,
                validation_type = "maximum",
                "Number validation failed: too large"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{path}' cannot exceed {max} (found {num})",
            )));
          }
        }

        if let Some(multiple) = rules.multiple_of {
          let ratio = num / multiple;
          if (ratio - ratio.round()).abs() > f64::EPSILON {
            error!(
                path = %path,
                multiple = multiple,
                value = num,
                validation_type = "multiple_of",
                "Number validation failed: not a multiple"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Field '{path}' must be a multiple of {multiple} (found {num})",
            )));
          }
        }
      } else {
        warn!(
            path = %path,
            value = %value,
            "Number could not be converted to f64 for validation"
        );
      }
    }
    Ok(())
  }

  #[instrument(
      skip(self, items),
      fields(
          field_name = %self.name,
          array_length = %items.len(),
          has_validation = %self.validation.is_some(),
          has_item_def = %self.items.is_some()
      )
  )]
  fn validate_array(&self, items: &[Value], path: &str) -> Result<()> {
    debug!(
        path = %path,
        "Starting array validation"
    );

    // Validate array-level rules
    if let Some(rules) = &self.validation {
      if let Some(min_items) = rules.min_items {
        if items.len() < min_items {
          error!(
              path = %path,
              min_required = min_items,
              actual = items.len(),
              validation_type = "min_items",
              "Array validation failed: too few items"
          );
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' must have at least {} items (found {})",
            path,
            min_items,
            items.len()
          )));
        }
      }

      if let Some(max_items) = rules.max_items {
        if items.len() > max_items {
          error!(
              path = %path,
              max_allowed = max_items,
              actual = items.len(),
              validation_type = "max_items",
              "Array validation failed: too many items"
          );
          return Err(LearnerError::TemplateInvalidation(format!(
            "Field '{}' cannot exceed {} items (found {})",
            path,
            max_items,
            items.len()
          )));
        }
      }

      if rules.unique_items == Some(true) {
        let mut seen = HashSet::new();
        for (idx, item) in items.iter().enumerate() {
          match serde_json::to_string(item) {
            Ok(item_str) =>
              if !seen.insert(item_str.clone()) {
                error!(
                    path = %path,
                    index = idx,
                    value = %item_str,
                    validation_type = "unique_items",
                    "Array validation failed: duplicate item"
                );
                return Err(LearnerError::TemplateInvalidation(format!(
                  "Field '{path}' contains duplicate item at index {idx}",
                )));
              },
            Err(e) => {
              error!(
                  path = %path,
                  index = idx,
                  error = %e,
                  validation_type = "unique_items",
                  "Failed to serialize array item"
              );
              return Err(LearnerError::TemplateInvalidation(format!(
                "Failed to check uniqueness for item at index {idx}: {e}",
              )));
            },
          }
        }
      }
    }

    // Validate individual items if we have an item definition
    if let Some(item_def) = &self.items {
      for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{index}]");

        match (item_def.base_type.as_str(), item) {
          ("object", Value::Object(obj)) => {
            if let Err(e) = item_def.validate_object(obj, &item_path) {
              error!(
                  path = %item_path,
                  error = %e,
                  validation_type = "object",
                  "Array item validation failed"
              );
              return Err(e);
            }
          },
          (expected, got) => {
            error!(
                path = %item_path,
                expected_type = %expected,
                actual_type = %type_name_of_value(got),
                validation_type = "type_check",
                "Array item type mismatch"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Item at index {} in '{}' expected type '{}' but got '{}'",
              index,
              path,
              expected,
              type_name_of_value(got)
            )));
          },
        }
      }
    }
    Ok(())
  }

  #[instrument(
    skip(self, obj),
    fields(
        field_name = %self.name,
        field_count = %obj.len(),
        has_fields = %self.fields.is_some()
    )
)]
  fn validate_object(&self, obj: &Map<String, Value>, path: &str) -> Result<()> {
    debug!(
        path = %path,
        fields = ?obj.keys().collect::<Vec<_>>(),
        "Starting object validation"
    );

    if let Some(fields) = &self.fields {
      for field in fields {
        match obj.get(&field.name) {
          Some(value) => {
            let field_path = format!("{}.{}", path, field.name);
            if let Err(e) = field.validate_with_path(value, &field_path) {
              error!(
                  path = %field_path,
                  field = %field.name,
                  error = %e,
                  "Object field validation failed"
              );
              return Err(e);
            }
          },
          None if field.required => {
            // Field is missing but required
            error!(
                path = %path,
                field = %field.name,
                validation_type = "required_field",
                "Missing required field in object"
            );
            return Err(LearnerError::TemplateInvalidation(format!(
              "Missing required field '{}' in object '{}'",
              field.name, path
            )));
          },
          None => {},
        }
      }

      // Log any extra fields that weren't in our field definitions
      let defined_fields: HashSet<_> = fields.iter().map(|f| &f.name).collect();
      let extra_fields: Vec<_> = obj.keys().filter(|k| !defined_fields.contains(k)).collect();

      if !extra_fields.is_empty() {
        warn!(
            path = %path,
            extra_fields = ?extra_fields,
            "Object contains undefined fields"
        );
      }
    }
    Ok(())
  }
}

const fn type_name_of_value(value: &Value) -> &'static str {
  match value {
    Value::String(_) => "string",
    Value::Number(_) => "number",
    Value::Bool(_) => "boolean",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
    Value::Null => "null",
  }
}

// TODO: Not sure we really need this...
pub fn datetime_to_json(dt: DateTime<Utc>) -> String { dt.to_rfc3339() }

/// Parse RFC3339 string from JSON into DateTime
pub fn datetime_from_json(s: &str) -> Result<DateTime<Utc>> {
  DateTime::parse_from_rfc3339(s)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| LearnerError::TemplateInvalidation(format!("Invalid datetime format: {e}")))
}
#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  #[traced_test]
  fn test_array_object_validation() {
    let template_str = r#"
        name = "test"
        description = "Test template"
        
        [authors]
        base_type = "array"
        required = true
        validation = { min_items = 1 }
        
        [authors.items]
        base_type = "object"
        
        [[authors.items.fields]]
        name = "name"
        base_type = "string"
        required = true
        validation = { min_length = 1 }
        
        [[authors.items.fields]]
        name = "affiliation"
        base_type = "string"
        required = false
    "#;

    let template: Template = toml::from_str(template_str).unwrap();

    // Test valid case
    let valid_resource = BTreeMap::from([(
      "authors".into(),
      json!([
          {"name": "John Doe", "affiliation": "University"},
          {"name": "Jane Smith"}
      ]),
    )]);

    if let Err(e) = template.validate(&valid_resource) {
      error!(
          error = %e,
          template = ?template,
          data = ?valid_resource,
          "Validation failed unexpectedly"
      );
      panic!("Validation should have succeeded: {}", e);
    }
  }

  #[test]
  #[traced_test]
  fn test_datetime_validation() {
    let template_str = r#"
        name = "test"
        description = "Test template"
        
        [dates]
        base_type = "object"
        required = true
        
        [[dates.fields]]
        name = "created"
        base_type = "string"
        required = true
        validation = { datetime = true }
        
        [[dates.fields]]
        name = "updated"
        base_type = "string"
        required = false
        validation = { datetime = true }
    "#;

    let template: Template = toml::from_str(template_str).unwrap();

    // Test valid dates
    let valid_dates = BTreeMap::from([(
      "dates".into(),
      json!({
          "created": "2024-01-01T00:00:00Z",
          "updated": "2024-02-01T00:00:00Z"
      }),
    )]);

    if let Err(e) = template.validate(&valid_dates) {
      error!(
          error = %e,
          template = ?template,
          data = ?valid_dates,
          "Validation failed unexpectedly"
      );
      panic!("Validation should have succeeded: {}", e);
    }
  }
}
