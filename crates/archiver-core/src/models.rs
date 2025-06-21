use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedProject {
    pub name: String,
    pub path: PathBuf,
    pub last_activity: DateTime<Utc>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedRecord {
    pub name: String,
    pub original_path: PathBuf,
    pub archive_path: PathBuf,
    pub archived_at: DateTime<Utc>,
}