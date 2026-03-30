use chrono::{NaiveDateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::db::DbError;
use crate::models::FileEntry;

#[allow(dead_code)]
fn parse_dt(s: &str) -> chrono::DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| ndt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

pub fn create_table(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS vault_files (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            original_path TEXT NOT NULL DEFAULT '',
            size_bytes INTEGER NOT NULL DEFAULT 0,
            encrypted INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn list_files(conn: &Connection) -> Result<Vec<FileEntry>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, original_path, size_bytes, encrypted, created_at FROM vault_files ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        let name: String = row.get(1)?;
        let original_path: String = row.get(2)?;
        let size_bytes: i64 = row.get(3)?;
        let encrypted: bool = row.get(4)?;
        let created_str: String = row.get(5)?;
        Ok(FileEntry {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            name,
            original_path,
            size_bytes: size_bytes as u64,
            encrypted,
            created_at: parse_dt(&created_str),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[allow(dead_code)]
pub fn insert_file(conn: &Connection, entry: &FileEntry) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO vault_files (id, name, original_path, size_bytes, encrypted, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        params![
            entry.id.to_string(),
            entry.name,
            entry.original_path,
            entry.size_bytes as i64,
            entry.encrypted,
            entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn delete_file(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    conn.execute("DELETE FROM vault_files WHERE id = ?", params![id.to_string()])?;
    Ok(())
}
