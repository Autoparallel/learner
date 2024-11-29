//! Resource abstraction and configuration for the learner library.
//!
//! This module provides the core abstractions for working with different types of academic
//! and research resources. It defines:
//!
//! - A [`Resource`] trait that all resource types must implement
//! - A flexible [`ResourceConfig`] for runtime-configured resource types
//! - Common utility types and functions for resource management
//!
//! The design allows for both statically defined resource types (like papers and books)
//! and dynamically configured resources that can be defined through configuration files.
//!
//! # Examples
//!
//! ```rust,no_run
//! use std::collections::BTreeMap;
//!
//! use learner::{
//!   resource::{Paper, Resource, ResourceConfig},
//!   Learner,
//! };
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Using a built-in resource type
//! let learner = Learner::builder().build().await?;
//! let paper = learner.retriever.get_paper("2301.07041").await?;
//!
//! // Access resource fields
//! let fields = paper.fields()?;
//! println!("Paper type: {}", paper.resource_type());
//!
//! // Or create a custom resource type at runtime
//! let mut fields = BTreeMap::new();
//! fields.insert("title".into(), json!("My Thesis"));
//! fields.insert("university".into(), json!("Tech University"));
//!
//! let thesis = ResourceConfig { type_name: "thesis".to_string(), fields };
//! # Ok(())
//! # }
//! ```

use serde_json::Value;

use super::*;

mod paper;
mod shared;

pub use paper::*;
pub use shared::*;

/// Core trait that defines the behavior of a resource in the system.
///
/// This trait provides a common interface for all resource types, whether they are
/// statically defined (like [`Paper`]) or dynamically configured through [`ResourceConfig`].
/// It requires that implementing types can be serialized and deserialized, which enables
/// persistent storage and retrieval.
///
/// The trait provides two key capabilities:
/// - Identification of the resource type
/// - Access to the resource's fields in a uniform way
///
/// # Examples
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use learner::resource::Resource;
/// #[derive(Serialize, Deserialize)]
/// struct Book {
///   title:  String,
///   author: String,
///   isbn:   String,
/// }
///
/// impl Resource for Book {
///   fn resource_type(&self) -> String { "book".to_string() }
/// }
/// ```
pub trait Resource: Serialize + for<'de> Deserialize<'de> {
  /// Returns the type identifier for this resource.
  ///
  /// This identifier is used to distinguish between different types of resources
  /// in the system. For example, "paper", "book", or "thesis".
  fn resource_type(&self) -> String;

  /// Returns a map of field names to their values for this resource.
  ///
  /// This method provides a uniform way to access a resource's fields regardless
  /// of the concrete type. The default implementation uses serde to serialize
  /// the resource to JSON and extract its fields.
  ///
  /// # Errors
  ///
  /// Returns [`LearnerError::InvalidResource`] if the resource cannot be serialized
  /// to a JSON object.
  fn fields(&self) -> Result<BTreeMap<String, Value>> {
    let mut output = BTreeMap::new();
    let map = serde_json::to_value(self)?
      .as_object()
      .cloned()
      .ok_or_else(|| LearnerError::InvalidResource)?;
    map.into_iter().for_each(|(k, v)| {
      let _v = output.insert(k, v).unwrap();
    });
    Ok(output)
  }
}

/// A dynamically configured resource type.
///
/// This struct enables the creation of new resource types at runtime through
/// configuration files. It provides a flexible way to extend the system without
/// requiring code changes.
///
/// The type consists of:
/// - A type identifier string
/// - A map of field names to their values
///
/// # Examples
///
/// ```rust
/// use std::collections::BTreeMap;
///
/// use learner::resource::ResourceConfig;
/// use serde_json::json;
///
/// let mut fields = BTreeMap::new();
/// fields.insert("title".into(), json!("Understanding Type Systems"));
/// fields.insert("university".into(), json!("Tech University"));
///
/// let thesis = ResourceConfig { type_name: "thesis".to_string(), fields };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
  /// The type identifier for this resource configuration
  pub type_name: String,
  /// Map of field names to their values
  pub fields:    BTreeMap<String, Value>,
}

impl Resource for ResourceConfig {
  fn resource_type(&self) -> String { self.type_name.clone() }

  fn fields(&self) -> Result<BTreeMap<String, Value>> { Ok(self.fields.clone()) }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn test_thesis_resource() -> Result<()> {
    // Create a thesis resource
    let mut fields = BTreeMap::new();
    fields.insert("title".into(), json!("Understanding Quantum Computing Effects"));
    fields.insert("author".into(), json!(["Alice Researcher", "Bob Scientist"]));
    fields.insert("university".into(), json!("Tech University"));
    fields.insert("department".into(), json!("Computer Science"));
    fields.insert("defense_date".into(), json!("2024-06-15T14:00:00Z"));
    fields.insert(
      "committee".into(),
      json!(["Prof. Carol Chair", "Dr. David Member", "Dr. Eve External"]),
    );
    fields
      .insert("keywords".into(), json!(["quantum computing", "decoherence", "error correction"]));

    let thesis = ResourceConfig { type_name: "thesis".to_string(), fields };

    // Test resource_type
    assert_eq!(thesis.resource_type(), "thesis");

    // Test fields method
    let fields = thesis.fields()?;

    // Verify we can access specific fields with proper types
    assert!(fields.get("title").unwrap().is_string());
    assert!(fields.get("author").unwrap().as_array().unwrap().len() == 2);

    // Test JSON serialization/deserialization roundtrip
    let serialized = serde_json::to_string(&thesis)?;
    let deserialized: ResourceConfig = serde_json::from_str(&serialized)?;
    assert_eq!(thesis.fields.get("title"), deserialized.fields.get("title"));

    Ok(())
  }

  #[test]
  fn test_thesis_from_toml() -> Result<()> {
    let toml_str = include_str!("../../config/resources/thesis.toml");
    let config: ResourceConfig = toml::from_str(toml_str)?;
    dbg!(&config);

    assert_eq!(config.resource_type(), "thesis");

    // Test that we can access the field definitions
    let fields = config.fields()?;
    dbg!(&fields);
    assert!(fields.contains_key("title"));
    assert!(fields.contains_key("author"));
    assert!(fields.contains_key("university"));

    Ok(())
  }
}
