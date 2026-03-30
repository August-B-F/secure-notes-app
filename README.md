# Notes

A secure encrypted desktop notes app built with Rust and [iced](https://github.com/iced-rs/iced).

## Features

- **Vault encryption** — all data protected behind a master password (Argon2id + AES-256-GCM)
- **Text notes** — full markdown editor with live formatting, headings, bold/italic, code blocks, tables, checklists, images, and word wrapping
- **Password notes** — structured password storage with generator and strength meter
- **Canvas notes** — node-based canvas with draggable cards, each containing a full markdown editor, connected by bezier edges
- **File notes** — drag & drop any file to encrypt and store it, chunked processing for large files
- **Multi-window** — open notes in separate windows
- **Folders** — nested folder tree with drag-drop, favorites, color coding
- **Per-note encryption** — optionally encrypt individual notes with separate passwords
- **Search** — full-text search with case-sensitive toggle and match navigation
- **Dark theme** — custom dark UI throughout

## Build

Requires Rust toolchain. Debug builds only (release builds may be blocked by antivirus on some systems).

```bash
cargo build
cargo run
```

### Linux dependencies

```bash
sudo apt install libgtk-3-dev xclip
```

## Install (Windows)

Creates a Start Menu shortcut so the app appears in Windows search:

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

## Architecture

Rust desktop app using iced 0.13 in daemon mode (multi-window). Follows the Elm architecture: `Message` enum → `update()` → `view()`.

| File | Purpose |
|------|---------|
| `src/app.rs` | core state, message handlers, view |
| `src/ui/md_widget.rs` | custom markdown editor widget |
| `src/ui/canvas_editor.rs` | canvas with embedded card editors |
| `src/ui/editor.rs` | editor panel dispatcher |
| `src/ui/theme.rs` | dark theme colors and styles |
| `src/crypto/` | AES-256-GCM encryption, Argon2id key derivation |
| `src/db/` | SQLite storage layer |

## Security

- Vault password derived with Argon2id (64 MB memory, 3 iterations)
- All images and files encrypted with AES-256-GCM using the vault key
- File attachments stored in encrypted chunks (4 MB per chunk)
- Vault key cleared from memory on lock
- Database permissions restricted to owner on Linux (0600)
- No network access — fully offline

## License

Free to use for personal and non-commercial purposes. Commercial use requires written approval from the author.
