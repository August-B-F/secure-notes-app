use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::FolderColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteType {
    Text,
    Password,
    Canvas,
    File,
}

impl NoteType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Password => "Password",
            Self::Canvas => "Canvas",
            Self::File => "File",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Password" => Self::Password,
            "Canvas" => Self::Canvas,
            "File" => Self::File,
            _ => Self::Text,
        }
    }
}

/// Password entry data stored as JSON in the body field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PasswordData {
    pub website: String,
    pub username: String,
    pub password: String,
    pub notes: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub custom_fields: Vec<CustomField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub label: String,
    pub value: String,
    pub hidden: bool,
}

/// Options for password generation.
#[derive(Debug, Clone)]
pub struct PasswordGenOptions {
    pub length: u32,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
}

impl Default for PasswordGenOptions {
    fn default() -> Self {
        Self { length: 20, uppercase: true, lowercase: true, numbers: true, symbols: true }
    }
}

impl PasswordGenOptions {
    pub fn generate(&self) -> String {
        use rand::Rng;
        let mut charset = Vec::new();
        if self.lowercase { charset.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz"); }
        if self.uppercase { charset.extend_from_slice(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"); }
        if self.numbers { charset.extend_from_slice(b"0123456789"); }
        if self.symbols { charset.extend_from_slice(b"!@#$%^&*-_+=?"); }
        if charset.is_empty() { charset.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz"); }
        let mut rng = rand::thread_rng();
        (0..self.length).map(|_| charset[rng.gen_range(0..charset.len())] as char).collect()
    }
}

impl PasswordData {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Self {
        serde_json::from_str(s).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub note_type: NoteType,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub is_encrypted: bool,
    pub color: FolderColor,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Note {
    pub fn new(folder_id: Option<Uuid>, color: FolderColor, note_type: NoteType) -> Self {
        let now = Utc::now();
        let body = match note_type {
            NoteType::Password => PasswordData::default().to_json(),
            NoteType::Canvas => String::from(r#"{"nodes":[],"edges":[]}"#),
            NoteType::Text | NoteType::File => String::new(),
        };
        Self {
            id: Uuid::new_v4(),
            folder_id,
            title: String::new(),
            body,
            tags: Vec::new(),
            note_type,
            is_favorite: false,
            is_pinned: false,
            is_encrypted: false,
            color,
            created_at: now,
            modified_at: now,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NotePreview {
    pub id: Uuid,
    pub title: String,
    pub snippet: String,
    pub note_type: NoteType,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub is_encrypted: bool,
    pub color: FolderColor,
    pub modified_at: DateTime<Utc>,
}
