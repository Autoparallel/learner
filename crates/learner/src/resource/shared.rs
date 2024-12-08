use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
  /// Author's full name
  pub name:        String,
  /// Optional institutional affiliation
  pub affiliation: Option<String>,
  /// Optional contact email
  pub email:       Option<String>,
}
