# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build          # Debug build (ONLY use this)
cargo run            # Debug run
```

**CrowdStrike antivirus blocks `cargo build --release`** on this machine (flags palette crate build scripts). Always use debug builds.

## Architecture

Rust desktop app using **iced 0.13** GUI framework in **daemon mode** (multi-window). Follows the Elm architecture: `Message` enum -> `update()` -> `view()`.

### Key Files

- **`src/main.rs`** — Entry point, `iced::daemon` setup with transparent window styling
- **`src/app.rs`** — Core app state (~4000+ lines): `App` struct, `Message` enum (200+ variants), `update()`, `view()`, multi-window state swapping
- **`src/ui/line_editor.rs`** — Per-line markdown editor (Obsidian-like: rendered lines hide `**`/`##` markers, clicking a line reveals raw markdown for editing)
- **`src/ui/editor.rs`** — Editor panel orchestrator (dispatches to line_editor, canvas_editor, or password_editor based on NoteType)
- **`src/ui/md_highlight.rs`** — Markdown parser producing highlight spans (Bold, Italic, Heading, Code, Marker, etc.)
- **`src/ui/notes_list.rs`** — Center panel: note list with drag-drop, sort, search, folder tree
- **`src/ui/tags_panel.rs`** — Left sidebar: folder navigation, favorites, All Notes
- **`src/ui/theme.rs`** — Dark theme colors and widget styles
- **`src/ui/icons.rs`** — Inline SVG icon definitions
- **`src/db/`** — SQLite layer (rusqlite): `mod.rs` (connection, vault meta), `notes.rs`, `folders.rs`, `schema.rs`
- **`src/crypto/`** — `key_derivation.rs` (Argon2id, 64MB memory, 3 iterations), `encryption.rs` (AES-256-GCM), `secure_memory.rs` (zeroize)
- **`src/models/`** — `Note`, `NotePreview`, `Folder`, `PasswordData`, `NoteType` (Text/Password/Canvas), `FolderColor` (20 colors)

### Multi-Window

Uses `iced::daemon` with `window::open()` for secondary windows. `WindowState` struct stores per-window editor state. On focus switch: snapshot current state into `other_windows: HashMap<window::Id, WindowState>`, restore the focused window's state. All windows share the same DB, vault, and folder tree.

Window close uses `std::process::exit(0)` because `iced::exit()` doesn't reliably terminate the daemon on Windows.

### Per-Line Markdown Editor

`LineEditorState` splits note body into `Vec<String>` lines. Non-active lines render as `rich_text` with styled spans (markers hidden). Active line uses `text_input` with `text_input::focus(Id)` for reliable focus. A transparent `mouse_area` overlay (via `stack![]`) captures clicks on rendered lines since `rich_text` consumes mouse events.

Enter splits lines, Backspace at col 0 merges with previous line.

### Vault & Encryption

Two-level encryption:
1. **Vault password** — gates app access. Argon2id derives key, encrypts literal `"notes-app-verify"` with AES-256-GCM. Salt + nonce + ciphertext stored in `vault_meta` table.
2. **Per-note encryption** — optional, separate password per note. Encrypted body stored as base64 in `notes.body`. Session-decrypted notes cached in `session_decrypted: HashMap<Uuid, Vec<u8>>`.

### Data Flow

View switch (`SelectView`) loads data **synchronously** via `refresh_data_sync()` to avoid flicker (no async gap). Note content loads synchronously too. Auto-save triggers after configurable delay (default 2s) via subscription timer.

### Keyboard Shortcuts

Ctrl+N: new note, Ctrl+Shift+N: new folder, Ctrl+Shift+W: new window, Ctrl+S: save, Ctrl+B: bold, Ctrl+I: italic, Ctrl+F: find, Alt+click note: open in new window. Shortcuts use `keyboard::on_key_press` which only fires for `Status::Ignored` keys (doesn't conflict with focused text inputs).

## Patterns to Follow

- Messages that should NOT close context menus are listed in the passthrough block near line 920 in app.rs
- All `editor_content.text()` calls have been replaced with `{ self.line_editor.sync_active_to_lines(); self.line_editor.to_body() }`
- Sort operations must sort by `is_pinned DESC` first, then by sort mode
- Favorites view shows all favorited notes + notes inside favorited folders (with deduplication)
- Dialog overlays use `mouse_area` backdrop with `on_press(CloseDialog)` to block interaction behind them
