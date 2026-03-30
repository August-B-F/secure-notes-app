use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::db::DbError;
use crate::models::{Folder, FolderColor};

pub fn list_folders(conn: &Connection) -> Result<Vec<Folder>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, color, sort_order, is_favorite FROM folders ORDER BY sort_order, name",
    )?;
    let rows = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        let parent_str: Option<String> = row.get(1)?;
        let name: String = row.get(2)?;
        let color_str: String = row.get(3)?;
        let sort_order: i32 = row.get(4)?;
        let is_favorite: bool = row.get::<_, i32>(5).unwrap_or(0) != 0;
        Ok(Folder {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            parent_id: parent_str.and_then(|s| Uuid::parse_str(&s).ok()),
            name,
            color: FolderColor::from_str(&color_str),
            sort_order,
            collapsed: false,
            is_favorite,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn insert_folder(conn: &Connection, folder: &Folder) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO folders (id, parent_id, name, color, sort_order) VALUES (?, ?, ?, ?, ?)",
        params![
            folder.id.to_string(),
            folder.parent_id.map(|id| id.to_string()),
            folder.name,
            folder.color.label(),
            folder.sort_order,
        ],
    )?;
    Ok(())
}

pub fn update_folder(conn: &Connection, folder: &Folder) -> Result<(), DbError> {
    conn.execute(
        "UPDATE folders SET parent_id=?, name=?, color=?, sort_order=?, is_favorite=? WHERE id=?",
        params![
            folder.parent_id.map(|id| id.to_string()),
            folder.name,
            folder.color.label(),
            folder.sort_order,
            folder.is_favorite as i32,
            folder.id.to_string(),
        ],
    )?;
    Ok(())
}

pub fn toggle_folder_favorite(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    conn.execute(
        "UPDATE folders SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE id = ?",
        params![id.to_string()],
    )?;
    Ok(())
}

pub fn delete_folder(conn: &Connection, id: Uuid) -> Result<(), DbError> {
    let fid = id.to_string();
    conn.execute(
        "WITH RECURSIVE descendants(id) AS (
            SELECT ?1
            UNION ALL
            SELECT f.id FROM folders f JOIN descendants d ON f.parent_id = d.id
        )
        DELETE FROM notes WHERE folder_id IN (SELECT id FROM descendants)",
        params![fid],
    )?;
    conn.execute(
        "WITH RECURSIVE descendants(id) AS (
            SELECT ?1
            UNION ALL
            SELECT f.id FROM folders f JOIN descendants d ON f.parent_id = d.id
        )
        DELETE FROM folders WHERE id IN (SELECT id FROM descendants)",
        params![fid],
    )?;
    Ok(())
}

pub fn count_notes_in_folder(conn: &Connection, folder_id: Uuid) -> Result<usize, DbError> {
    let fid = folder_id.to_string();
    let count: i64 = conn.query_row(
        "WITH RECURSIVE descendants(id) AS (
            SELECT ?
            UNION ALL
            SELECT f.id FROM folders f JOIN descendants d ON f.parent_id = d.id
        )
        SELECT COUNT(*) FROM notes WHERE folder_id IN (SELECT id FROM descendants)",
        params![fid],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

pub fn count_all_notes(conn: &Connection) -> Result<usize, DbError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
    Ok(count as usize)
}

#[allow(dead_code)]
pub fn list_subfolders(conn: &Connection, parent_id: Uuid) -> Result<Vec<Folder>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, color, sort_order, is_favorite FROM folders WHERE parent_id = ? ORDER BY sort_order, name",
    )?;
    let rows = stmt.query_map(params![parent_id.to_string()], |row| {
        let id_str: String = row.get(0)?;
        let parent_str: Option<String> = row.get(1)?;
        let name: String = row.get(2)?;
        let color_str: String = row.get(3)?;
        let sort_order: i32 = row.get(4)?;
        let is_favorite: bool = row.get::<_, i32>(5).unwrap_or(0) != 0;
        Ok(Folder {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            parent_id: parent_str.and_then(|s| Uuid::parse_str(&s).ok()),
            name,
            color: FolderColor::from_str(&color_str),
            sort_order,
            collapsed: false,
            is_favorite,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn reparent_folder(conn: &Connection, folder_id: Uuid, new_parent_id: Option<Uuid>) -> Result<(), DbError> {
    conn.execute(
        "UPDATE folders SET parent_id = ? WHERE id = ?",
        params![new_parent_id.map(|id| id.to_string()), folder_id.to_string()],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn count_favorites(conn: &Connection) -> Result<usize, DbError> {
    let note_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notes WHERE is_favorite = 1", [], |row| row.get(0),
    )?;
    let folder_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM folders WHERE is_favorite = 1", [], |row| row.get(0),
    )?;
    Ok((note_count + folder_count) as usize)
}
