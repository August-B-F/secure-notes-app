use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::db::DbError;
use crate::models::{FolderColor, Note, NotePreview, NoteType};

fn parse_dt(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

pub fn list_previews(
    conn: &Connection,
    folder_id: Option<Uuid>,
    search: Option<&str>,
) -> Result<Vec<NotePreview>, DbError> {
    let mut sql = String::from(
        "SELECT id, title, body, is_favorite, is_pinned, is_encrypted, color, note_type, modified_at FROM notes WHERE 1=1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(fid) = folder_id {
        sql.push_str(" AND folder_id = ?");
        param_values.push(Box::new(fid.to_string()));
    }
    if let Some(q) = search {
        if !q.is_empty() {
            sql.push_str(" AND (title LIKE ? OR (is_encrypted = 0 AND body LIKE ?))");
            let pattern = format!("%{q}%");
            param_values.push(Box::new(pattern.clone()));
            param_values.push(Box::new(pattern));
        }
    }
    sql.push_str(" ORDER BY is_pinned DESC, modified_at DESC");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_preview)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_favorites(conn: &Connection) -> Result<Vec<NotePreview>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, body, is_favorite, is_pinned, is_encrypted, color, note_type, modified_at
         FROM notes WHERE is_favorite = 1 ORDER BY is_pinned DESC, modified_at DESC",
    )?;
    let rows = stmt.query_map([], map_preview)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn map_preview(row: &rusqlite::Row) -> rusqlite::Result<NotePreview> {
    let id_str: String = row.get(0)?;
    let title: String = row.get(1)?;
    let body: String = row.get(2)?;
    let is_favorite: bool = row.get(3)?;
    let is_pinned: bool = row.get(4)?;
    let is_encrypted: bool = row.get(5)?;
    let color_str: String = row.get(6)?;
    let type_str: String = row.get(7)?;
    let modified_str: String = row.get(8)?;

    let note_type = NoteType::from_str(&type_str);
    let snippet = if is_encrypted {
        String::from("[Encrypted]")
    } else if note_type == NoteType::Password {
        String::from("Password entry")
    } else {
        let mut clean = String::new();
        let mut in_pass = false;
        let mut in_code = false;
        for line in body.lines() {
            let t = line.trim();
            if t == "%%pass" { in_pass = !in_pass; continue; }
            if in_pass { continue; }
            if t.starts_with("![") && (t.contains("](img:") || t.contains("](data:") || t.contains("](rgba:")) && t.ends_with(')') {
                if !clean.is_empty() { clean.push(' '); }
                clean.push_str("Image");
                continue;
            }
            if t.starts_with("```") { in_code = !in_code; continue; }
            if t.starts_with('|') && t.contains('-') && t.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { continue; }
            if t.starts_with('|') && t.ends_with('|') {
                let cells: Vec<&str> = t[1..t.len()-1].split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
                if !cells.is_empty() {
                    if !clean.is_empty() { clean.push(' '); }
                    clean.push_str(&cells.join(" | "));
                    continue;
                }
            }
            let stripped = t
                .trim_start_matches("# ").trim_start_matches("## ").trim_start_matches("### ")
                .trim_start_matches("#### ").trim_start_matches("> ").trim_start_matches("- [ ] ")
                .trim_start_matches("- [x] ").trim_start_matches("- [X] ").trim_start_matches("- ")
                .replace("**", "").replace("``", "").replace("`", "");
            // Strip color tags {c:X}...{/c}
            let mut s = stripped.as_str();
            let mut result = String::new();
            while let Some(start) = s.find("{c:") {
                result.push_str(&s[..start]);
                if let Some(end) = s[start..].find('}') {
                    s = &s[start + end + 1..];
                    if let Some(close) = s.find("{/c}") {
                        result.push_str(&s[..close]);
                        s = &s[close + 4..];
                    }
                } else { break; }
            }
            result.push_str(s);
            let result = result.trim();
            if !result.is_empty() {
                if !clean.is_empty() { clean.push(' '); }
                clean.push_str(result);
            }
            if clean.len() >= 60 { break; }
        }
        clean.chars().take(60).collect::<String>().trim().to_string()
    };

    Ok(NotePreview {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        title,
        snippet,
        note_type,
        is_favorite,
        is_pinned,
        is_encrypted,
        color: FolderColor::from_str(&color_str),
        modified_at: parse_dt(&modified_str),
    })
}

pub fn get_note(conn: &Connection, id: Uuid) -> Result<Option<Note>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, folder_id, title, body, tags, note_type, is_favorite, is_pinned, is_encrypted,
                color, encryption_nonce, encryption_salt, created_at, modified_at
         FROM notes WHERE id = ?",
    )?;
    let mut rows = stmt.query_map(params![id.to_string()], |row| {
        let id_str: String = row.get(0)?;
        let folder_str: Option<String> = row.get(1)?;
        let title: String = row.get(2)?;
        let body: String = row.get(3)?;
        let tags_json: String = row.get(4)?;
        let type_str: String = row.get(5)?;
        let is_favorite: bool = row.get(6)?;
        let is_pinned: bool = row.get(7)?;
        let is_encrypted: bool = row.get(8)?;
        let color_str: String = row.get(9)?;
        let created_str: String = row.get(12)?;
        let modified_str: String = row.get(13)?;

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(Note {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            folder_id: folder_str.and_then(|s| Uuid::parse_str(&s).ok()),
            title,
            body,
            tags,
            note_type: NoteType::from_str(&type_str),
            is_favorite,
            is_pinned,
            is_encrypted,
            color: FolderColor::from_str(&color_str),
            created_at: parse_dt(&created_str),
            modified_at: parse_dt(&modified_str),
        })
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn insert_note(conn: &Connection, note: &Note) -> Result<(), DbError> {
    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO notes (id, folder_id, title, body, tags, note_type, is_favorite, is_pinned, is_encrypted, color, created_at, modified_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            note.id.to_string(),
            note.folder_id.map(|id| id.to_string()),
            note.title,
            note.body,
            tags_json,
            note.note_type.label(),
            note.is_favorite,
            note.is_pinned,
            note.is_encrypted,
            note.color.label(),
            note.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            note.modified_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ],
    )?;
    Ok(())
}

pub fn update_note(conn: &Connection, note: &Note) -> Result<(), DbError> {
    let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE notes SET folder_id=?, title=?, body=?, tags=?, note_type=?, is_favorite=?, is_pinned=?,
         is_encrypted=?, color=?, modified_at=? WHERE id=?",
        params![
            note.folder_id.map(|id| id.to_string()),
            note.title,
            note.body,
            tags_json,
            note.note_type.label(),
            note.is_favorite,
            note.is_pinned,
            note.is_encrypted,
            note.color.label(),
            now,
            note.id.to_string(),
        ],
    )?;
    Ok(())
}

pub fn update_note_encryption(
    conn: &Connection,
    id: Uuid,
    body: &str,
    is_encrypted: bool,
    nonce: Option<&str>,
    salt: Option<&str>,
) -> Result<(), DbError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE notes SET body=?, is_encrypted=?, encryption_nonce=?, encryption_salt=?, modified_at=? WHERE id=?",
        params![body, is_encrypted, nonce, salt, now, id.to_string()],
    )?;
    Ok(())
}

pub fn update_note_color(conn: &Connection, id: Uuid, color: FolderColor) -> Result<(), DbError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE notes SET color=?, modified_at=? WHERE id=?",
        params![color.label(), now, id.to_string()],
    )?;
    Ok(())
}

pub fn rename_note(conn: &Connection, id: Uuid, title: &str) -> Result<(), DbError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE notes SET title=?, modified_at=? WHERE id=?",
        params![title, now, id.to_string()],
    )?;
    Ok(())
}

pub fn toggle_pin(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    conn.execute(
        "UPDATE notes SET is_pinned = NOT is_pinned WHERE id = ?",
        params![id.to_string()],
    )?;
    Ok(())
}

pub fn get_encryption_meta(conn: &Connection, id: Uuid) -> Result<(Option<String>, Option<String>), DbError> {
    let result = conn.query_row(
        "SELECT encryption_nonce, encryption_salt FROM notes WHERE id = ?",
        params![id.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(result)
}

pub fn delete_note(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    conn.execute("DELETE FROM notes WHERE id = ?", params![id.to_string()])?;
    Ok(())
}

pub fn toggle_favorite(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    conn.execute(
        "UPDATE notes SET is_favorite = NOT is_favorite WHERE id = ?",
        params![id.to_string()],
    )?;
    Ok(())
}

pub fn move_to_folder(conn: &Connection, note_id: Uuid, folder_id: Option<Uuid>) -> Result<(), DbError> {
    conn.execute(
        "UPDATE notes SET folder_id = ? WHERE id = ?",
        params![folder_id.map(|id| id.to_string()), note_id.to_string()],
    )?;
    Ok(())
}
