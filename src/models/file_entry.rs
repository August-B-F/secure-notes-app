use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FileEntry {
    pub id: Uuid,
    pub name: String,
    pub original_path: String,
    pub size_bytes: u64,
    pub encrypted: bool,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl FileEntry {
    pub fn new(name: String, original_path: String, size_bytes: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            original_path,
            size_bytes,
            encrypted: false,
            created_at: Utc::now(),
        }
    }

    pub fn size_display(&self) -> String {
        let b = self.size_bytes;
        if b < 1024 { format!("{b} B") }
        else if b < 1024 * 1024 { format!("{:.1} KB", b as f64 / 1024.0) }
        else if b < 1024 * 1024 * 1024 { format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)) }
        else { format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0)) }
    }
}
