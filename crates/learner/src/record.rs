use template::TemplatedItem;

use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
  pub resource:  Resource,
  pub state:     State,
  pub storage:   Storage,
  pub retrieval: Retrieval,
}

// Resource requirements that every resource must have
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Resource {
  pub title:    String,
  // Extension point for additional fields
  #[serde(flatten, default)]
  pub extended: TemplatedItem,
}

//  State tracking that every record needs
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct State {
  pub progress:      Progress, // enum: Unopened, Reading, Completed
  pub starred:       bool,
  pub tags:          Vec<String>,
  pub last_accessed: Option<DateTime<Utc>>,
  // Extension point
  #[serde(flatten, default)]
  pub extended:      TemplatedItem,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub enum Progress {
  #[default]
  Unopened,
  Opened(Option<f64>),
  Completed,
}

//  Storage information every record should track
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Storage {
  pub files:     BTreeMap<String, PathBuf>, // key could be "primary", "supplementary", etc.
  pub added_at:  Option<DateTime<Utc>>,
  pub checksums: BTreeMap<String, String>, // For integrity
  // Extension point
  #[serde(flatten, default)]
  pub extended:  TemplatedItem,
}

//  Retrieval data every record should have
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Retrieval {
  pub source:            String,                   // Where it came from
  pub source_identifier: Option<String>,           // Original ID in that source
  pub urls:              BTreeMap<String, String>, // Various URLs (HTML, PDF, etc)
  pub last_checked:      Option<DateTime<Utc>>,
  // Extension point
  #[serde(flatten, default)]
  pub extended:          TemplatedItem,
}
