pub const CREATE_TABLES: &str = "
CREATE TABLE IF NOT EXISTS folders (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL DEFAULT 'Blue',
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS notes (
    id               TEXT PRIMARY KEY,
    folder_id        TEXT REFERENCES folders(id) ON DELETE SET NULL,
    title            TEXT NOT NULL DEFAULT '',
    body             TEXT NOT NULL DEFAULT '',
    tags             TEXT NOT NULL DEFAULT '[]',
    is_favorite      INTEGER NOT NULL DEFAULT 0,
    is_pinned        INTEGER NOT NULL DEFAULT 0,
    is_encrypted     INTEGER NOT NULL DEFAULT 0,
    color            TEXT NOT NULL DEFAULT 'Green',
    encryption_nonce TEXT,
    encryption_salt  TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    modified_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS vault_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS note_images (
    id      TEXT PRIMARY KEY,
    data    BLOB NOT NULL,
    format  TEXT NOT NULL DEFAULT 'image/png'
);

CREATE TABLE IF NOT EXISTS note_files (
    id          TEXT PRIMARY KEY,
    note_id     TEXT NOT NULL,
    filename    TEXT NOT NULL,
    size        INTEGER NOT NULL DEFAULT 0,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS note_file_chunks (
    file_id     TEXT NOT NULL,
    chunk_idx   INTEGER NOT NULL,
    data        BLOB NOT NULL,
    PRIMARY KEY (file_id, chunk_idx)
);

CREATE INDEX IF NOT EXISTS idx_notes_folder ON notes(folder_id);
CREATE INDEX IF NOT EXISTS idx_notes_favorite ON notes(is_favorite);
CREATE INDEX IF NOT EXISTS idx_notes_modified ON notes(modified_at DESC);
";

/// Migration for databases created before color/is_pinned were added.
pub const MIGRATIONS: &[&str] = &[
    "ALTER TABLE notes ADD COLUMN color TEXT NOT NULL DEFAULT 'Green'",
    "ALTER TABLE notes ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE notes ADD COLUMN note_type TEXT NOT NULL DEFAULT 'Text'",
    "ALTER TABLE folders ADD COLUMN parent_id TEXT",
    "ALTER TABLE folders ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE note_files ADD COLUMN chunk_count INTEGER NOT NULL DEFAULT 0",
];
