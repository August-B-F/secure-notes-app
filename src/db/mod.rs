pub mod files;
pub mod folders;
pub mod notes;
pub mod schema;

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rusqlite::{params, Connection};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Note not found: {0}")]
    NoteNotFound(Uuid),
    #[error("Folder not found: {0}")]
    FolderNotFound(Uuid),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encryption failed")]
    EncryptionFailed,
}

pub fn db_path() -> PathBuf {
    let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("notes-app")
}

pub fn open_connection(path: &Path) -> Result<Connection, DbError> {
    std::fs::create_dir_all(path.parent().unwrap_or(path))?;
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;",
    )?;
    // Restrict file permissions on Unix (owner-only read/write)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(conn)
}

pub fn initialize(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(schema::CREATE_TABLES)?;
    for migration in schema::MIGRATIONS {
        let _ = conn.execute_batch(migration);
    }
    let _ = files::create_table(conn);
    Ok(())
}


pub fn has_vault_password(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT value FROM vault_meta WHERE key = 'vault_salt'",
        [],
        |_| Ok(()),
    )
    .is_ok()
}

/// Stores salt and verification ciphertext (nonce || ciphertext) in vault_meta.
pub fn set_vault_password(
    conn: &Connection,
    salt: &[u8; 16],
    verify_nonce: &[u8; 12],
    verify_ciphertext: &[u8],
) -> Result<(), DbError> {
    let salt_b64 = BASE64.encode(salt);
    let mut verify_data = Vec::with_capacity(12 + verify_ciphertext.len());
    verify_data.extend_from_slice(verify_nonce);
    verify_data.extend_from_slice(verify_ciphertext);
    let verify_b64 = BASE64.encode(&verify_data);

    conn.execute(
        "INSERT OR REPLACE INTO vault_meta (key, value) VALUES ('vault_salt', ?)",
        params![salt_b64],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO vault_meta (key, value) VALUES ('vault_verify', ?)",
        params![verify_b64],
    )?;
    Ok(())
}

pub fn get_vault_salt(conn: &Connection) -> Option<[u8; 16]> {
    let b64: String = conn
        .query_row(
            "SELECT value FROM vault_meta WHERE key = 'vault_salt'",
            [],
            |row| row.get(0),
        )
        .ok()?;
    let bytes = BASE64.decode(&b64).ok()?;
    if bytes.len() == 16 {
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&bytes);
        Some(salt)
    } else {
        None
    }
}

/// Returns (nonce, ciphertext) for password verification.
pub fn get_vault_verify(conn: &Connection) -> Option<([u8; 12], Vec<u8>)> {
    let b64: String = conn
        .query_row(
            "SELECT value FROM vault_meta WHERE key = 'vault_verify'",
            [],
            |row| row.get(0),
        )
        .ok()?;
    let bytes = BASE64.decode(&b64).ok()?;
    if bytes.len() > 12 {
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[..12]);
        let ciphertext = bytes[12..].to_vec();
        Some((nonce, ciphertext))
    } else {
        None
    }
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM vault_meta WHERE key = ?",
        params![key],
        |row| row.get(0),
    ).ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO vault_meta (key, value) VALUES (?, ?)",
        params![key, value],
    )?;
    Ok(())
}


#[allow(dead_code)]
pub fn save_image(conn: &Connection, id: &str, data: &[u8], format: &str) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR REPLACE INTO note_images (id, data, format) VALUES (?, ?, ?)",
        params![id, data, format],
    )?;
    Ok(())
}

/// Save image with encryption
pub fn save_image_encrypted(conn: &Connection, id: &str, data: &[u8], format: &str, key: &[u8; 32]) -> Result<(), DbError> {
    use crate::crypto::encryption;
    match encryption::encrypt(key, data) {
        Ok((ciphertext, nonce)) => {
            let mut blob = Vec::with_capacity(12 + ciphertext.len());
            blob.extend_from_slice(&nonce);
            blob.extend_from_slice(&ciphertext);
            conn.execute(
                "INSERT OR REPLACE INTO note_images (id, data, format) VALUES (?, ?, ?)",
                params![id, blob, format],
            )?;
        }
        Err(e) => {
            eprintln!("Encryption failed for image {}: {:?}", id, e);
            return Err(DbError::EncryptionFailed);
        }
    }
    Ok(())
}

/// Load image with decryption
pub fn load_image_encrypted(conn: &Connection, id: &str, key: &[u8; 32]) -> Option<(Vec<u8>, String)> {
    use crate::crypto::encryption;
    let (blob, format): (Vec<u8>, String) = conn.query_row(
        "SELECT data, format FROM note_images WHERE id = ?",
        params![id],
        |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
    ).ok()?;

    if blob.len() > 12 {
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&blob[..12]);
        if let Ok(plaintext) = encryption::decrypt(key, &nonce, &blob[12..]) {
            return Some((plaintext, format));
        }
    }
    // Fallback: return raw (for unencrypted legacy images)
    Some((blob, format))
}

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB per chunk — faster than 1MB

/// Save file with chunked encryption — reads in chunks to keep RAM low.
/// Progress is reported via the atomic (0-1000 = 0%-100%).
pub fn save_file_chunked(
    conn: &Connection, id: &str, note_id: &str, filename: &str,
    path: &std::path::Path, key: &[u8; 32],
    progress: &std::sync::Arc<std::sync::atomic::AtomicU32>,
) -> Result<usize, DbError> {
    use crate::crypto::encryption;
    use std::io::Read;
    use std::sync::atomic::Ordering;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS note_files (
            id TEXT PRIMARY KEY, note_id TEXT NOT NULL, filename TEXT NOT NULL,
            size INTEGER NOT NULL DEFAULT 0, chunk_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS note_file_chunks (
            file_id TEXT NOT NULL, chunk_idx INTEGER NOT NULL, data BLOB NOT NULL,
            PRIMARY KEY (file_id, chunk_idx)
        );"
    )?;
    // Migrate old note_files table that may lack chunk_count
    conn.execute("ALTER TABLE note_files ADD COLUMN chunk_count INTEGER NOT NULL DEFAULT 0", []).ok();

    let file = std::fs::File::open(path).map_err(|e| DbError::Io(e))?;
    let file_size: usize = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
    let mut reader = std::io::BufReader::with_capacity(CHUNK_SIZE, file);
    let mut chunk_idx = 0usize;
    let mut bytes_written = 0usize;
    let mut buf = vec![0u8; CHUNK_SIZE];

    // Optimize SQLite for bulk writes
    conn.execute_batch("PRAGMA synchronous = NORMAL; BEGIN")?;

    loop {
        let bytes_read = reader.read(&mut buf).unwrap_or(0);
        if bytes_read == 0 { break; }

        let chunk_data = &buf[..bytes_read];
        match encryption::encrypt(key, chunk_data) {
            Ok((ciphertext, nonce)) => {
                let mut blob = Vec::with_capacity(12 + ciphertext.len());
                blob.extend_from_slice(&nonce);
                blob.extend_from_slice(&ciphertext);
                conn.execute(
                    "INSERT OR REPLACE INTO note_file_chunks (file_id, chunk_idx, data) VALUES (?, ?, ?)",
                    params![id, chunk_idx as i64, blob],
                )?;
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK").ok();
                return Err(DbError::EncryptionFailed);
            }
        }
        bytes_written += bytes_read;
        chunk_idx += 1;
        if file_size > 0 {
            let pct = ((bytes_written as f64 / file_size as f64) * 1000.0) as u32;
            progress.store(pct.min(1000), Ordering::Relaxed);
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO note_files (id, note_id, filename, size, chunk_count, data) VALUES (?, ?, ?, ?, ?, X'')",
        params![id, note_id, filename, file_size as i64, chunk_idx as i64],
    )?;

    conn.execute_batch("COMMIT")?;
    conn.execute_batch("PRAGMA synchronous = FULL").ok(); // non-fatal
    progress.store(1000, std::sync::atomic::Ordering::Relaxed);
    Ok(file_size)
}

/// Load and decrypt a file — streams chunks and writes to destination.
pub fn export_file_chunked(conn: &Connection, id: &str, dest: &std::path::Path, key: &[u8; 32]) -> Result<(), DbError> {
    use crate::crypto::encryption;
    use std::io::Write;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS note_file_chunks (file_id TEXT NOT NULL, chunk_idx INTEGER NOT NULL, data BLOB NOT NULL, PRIMARY KEY (file_id, chunk_idx))").ok();

    let chunk_count: i64 = conn.query_row(
        "SELECT chunk_count FROM note_files WHERE id = ?",
        params![id], |row| row.get(0),
    ).unwrap_or(0);

    let mut file = std::fs::File::create(dest).map_err(|e| DbError::Io(e))?;

    for ci in 0..chunk_count {
        let blob: Vec<u8> = conn.query_row(
            "SELECT data FROM note_file_chunks WHERE file_id = ? AND chunk_idx = ?",
            params![id, ci], |row| row.get(0),
        ).unwrap_or_default();

        if blob.len() > 12 {
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&blob[..12]);
            if let Ok(plaintext) = encryption::decrypt(key, &nonce, &blob[12..]) {
                file.write_all(&plaintext).ok();
                continue;
            }
        }
        // If decryption failed, abort export rather than writing encrypted garbage
        return Err(DbError::EncryptionFailed);
    }
    Ok(())
}

/// Delete a file attachment and its chunks
pub fn delete_file(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM note_file_chunks WHERE file_id = ?", params![id])?;
    conn.execute("DELETE FROM note_files WHERE id = ?", params![id])?;
    Ok(())
}

/// Load file into memory (for backward compat / small files). Prefer export_file_chunked for large files.
pub fn load_file_encrypted(conn: &Connection, id: &str, key: &[u8; 32]) -> Option<Vec<u8>> {
    use crate::crypto::encryption;
    let chunk_count: i64 = conn.query_row(
        "SELECT chunk_count FROM note_files WHERE id = ?",
        params![id], |row| row.get(0),
    ).unwrap_or(0);

    if chunk_count > 0 {
        let mut result = Vec::new();
        for ci in 0..chunk_count {
            let blob: Vec<u8> = conn.query_row(
                "SELECT data FROM note_file_chunks WHERE file_id = ? AND chunk_idx = ?",
                params![id, ci], |row| row.get(0),
            ).ok()?;
            if blob.len() > 12 {
                let mut nonce = [0u8; 12];
                nonce.copy_from_slice(&blob[..12]);
                if let Ok(plaintext) = encryption::decrypt(key, &nonce, &blob[12..]) {
                    result.extend_from_slice(&plaintext);
                    continue;
                }
            }
            result.extend_from_slice(&blob);
        }
        return Some(result);
    }
    None
}

#[allow(dead_code)]
pub fn load_image(conn: &Connection, id: &str) -> Option<(Vec<u8>, String)> {
    conn.query_row(
        "SELECT data, format FROM note_images WHERE id = ?",
        params![id],
        |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
    ).ok()
}

#[allow(dead_code)]
pub fn delete_image(conn: &Connection, id: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM note_images WHERE id = ?", params![id])?;
    Ok(())
}
