use super::*;

/// A complete view of a resource with all associated data
#[derive(Debug)]
pub struct ResourceRecord<R: Resource> {
  pub resource:  R,
  pub state:     ResourceState,
  pub tags:      Vec<String>,
  pub storage:   Option<StorageData>,
  pub retrieval: Option<RetrievalData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadStatus {
  Unread,
  Reading {
    progress: f32,
    // last_read:  DateTime<Utc>, // Track when reading sessions occur
    // total_time: Duration,      // Accumulate reading time
  },
  Completed {
    finished_at: DateTime<Utc>,
    // times_referenced: u32, // Track how often it's been revisited
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
  pub read_status:     ReadStatus,
  pub starred:         bool,
  pub rating:          Option<u8>,
  pub last_accessed:   Option<DateTime<Utc>>,
  pub notes:           Option<String>,
  pub citation_key:    Option<String>,
  // pub importance:      Option<u8>, // Different from rating - how crucial is this?
  pub tags_updated_at: Option<DateTime<Utc>>, // Track tag management
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalData {
  pub source:            Option<String>,
  pub source_identifier: Option<String>,
  pub urls:              BTreeMap<String, String>,
  pub doi:               Option<String>,
  pub last_checked:      Option<DateTime<Utc>>, // When we last verified URLs
  pub access_type:       Option<String>,        // "open", "subscription", "institutional"
  pub verified:          bool,                  // Whether we've confirmed this data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageData {
  pub files:              BTreeMap<String, PathBuf>,
  pub original_filenames: BTreeMap<String, String>,
  pub added_at:           BTreeMap<String, DateTime<Utc>>,
  pub file_sizes:         BTreeMap<String, u64>, // Track file sizes
  pub checksums:          BTreeMap<String, String>, // For integrity checking
  pub last_verified:      DateTime<Utc>,         // When we last checked files exist
}
