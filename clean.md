You are refactoring a Rust desktop notes application built with Iced 0.13. Read every file mentioned before modifying it. Run cargo check after every numbered step. Do not proceed to the next step if the current one does not compile.

CURRENT ARCHITECTURE
Project overview
Framework: Iced 0.13 (Elm architecture: Model → update → view), running as iced::daemon with multi-window support.
Database: SQLite via rusqlite, with AES-GCM encrypted image/file storage and Argon2 key derivation.
Entry point: src/main.rs launches iced::daemon wiring App::title, App::update, App::view, App::theme, App::subscription.
File map — what each file does, key structs, how they connect
src/app.rs (6854 lines) — the god object
Contains everything:

Enums: VaultState (Setup/Login/Loading/Unlocked), ActiveView (AllNotes/Favorites/Folder), EditorSubmenu, ContextMenu (7 variants with Uuid payloads), SortMode (5 variants), ResizeEdge (8 variants), DragItem (Note/Folder), DialogKind (12 variants).
WindowState (80+ fields): complete per-window snapshot used for multi-window support. Fields mirror App 1:1 for editor, canvas, dialog, rename, drag, search, and selection state.
Message enum (~250 variants): every user and system event — vault auth, window management, note/folder CRUD, editor formatting, canvas operations, password manager, file transfers, drag-and-drop, dialogs, settings, clipboard, images.
App struct (~130 fields): groups vault state, DB path, window tracking, folders/notes/counts, view state, context menus, editor state, canvas state, password manager state, dialog state, settings, file transfers, clipboard, multi-selection, drag-and-drop, search.
App::new() (~70 lines): opens DB, reads settings, determines vault state, creates window.
App::update() (~3500 lines): single match on all 250+ Message variants. First block is a passthrough list (~60 message patterns that should not close context menus), then the match body. Handlers mix DB calls, async Task::perform, state mutation, and UI logic.
App::view() (~50 lines): dispatches to view_setup/view_login/view_loading/view_main based on VaultState, wraps in window chrome with resize handles.
App::view_main() (~280 lines): builds title bar + 3-column layout (tags_panel | notes_list | editor/settings), then layers drag ghost, dialog overlay, context menu overlay, and zoom toast on top via stack!.
App::view_context_menu() (~200 lines): renders context menus for tags, notes, empty areas, editor format, tables, images, files.
App::view_dialog() (~120 lines): renders modal dialogs for create note, encrypt/decrypt, create/rename/delete folder, delete note, change password, text color.
App::save_current_note() (~50 lines): serializes note body (handles encrypted re-encryption, password JSON, canvas JSON, or markdown), writes to DB.
App::refresh_data() and refresh_data_sync()**: loads all folders, notes, counts, subfolders from DB.
snapshot_window_state() / restore_window_state() (~160 lines): copies all 80+ fields between App and WindowState using std::mem::take/std::mem::replace.
Free functions: hit_test_position() (converts pixel coords to line/col in the markdown editor), find_word_bounds(), ctx_btn(), ctx_btn_danger(), ctx_btn_hover() (context menu button helpers).
src/ui/md_widget.rs (3314 lines) — custom markdown editor widget
A single-file iced advanced Widget implementing a full markdown editor with formatted rendering.

State: MdEditorState (45 fields) — lines (Vec<String>), cursor (line, col), selection, color annotations (Vec<ColorRange>), search matches, slash command menu state, image cache (HashMap<String, Handle>), image sizes, undo/redo stacks (VecDeque), code language menu, scroll offset/velocity, focus tracking, click tracking.

Actions: MdAction enum (45 variants) — Click, DoubleClick, TripleClick, ShiftClick, DragTo, Release, Insert, Paste, Enter, Backspace, Delete, Indent, Unindent, Move/Select with MdMotion (Left/Right/Up/Down/Home/End/WordLeft/WordRight), SelectAll, Copy, Cut, Undo, Redo, Scroll, RightClick, ToggleCheckbox, Focus, Unfocus, WindowFocus, Tick, SlashSelect/SlashClickSelect/SlashArrow, CopyCodeBlock, CopyPasswordBlock, TogglePasswordVisible, TableAddRow/AddCol/DeleteRow/DeleteCol/Delete, ScrollTo, ImageResize/ResizeStart/ResizeDrag/ResizeEnd/Delete, CodeLangMenuOpen/Select, FileExport/Delete, OpenLink.

Supporting types: ColorRange { line, start_col, end_col, color }, SearchMatch { line, start_col, end_col }, SlashCommand { name, label, description, icon }, MdMotion enum.

Document model methods on MdEditorState: from_body() (parses markdown with color tags), to_body() (serializes back with color tags), collect_images() (finds img:UUID references, migrates inline base64), content_height() (cached), push_undo()/undo()/redo(), insert_char()/insert_text()/insert_newline(), backspace()/delete()/delete_selection(), selection_ordered()/selected_text(), move_left()/move_right()/move_up()/move_down() (with wrapped-line-aware navigation using Paragraph::hit_test), move_home()/move_end()/move_doc_start()/move_doc_end(), select_all(), set_body().

Widget struct: MdEditorWidget<'a, Message> — holds &'a MdEditorState, on_edit callback, font_size, padding, scrollbar flag. Constructor md_editor() returns this. Methods: .size(), .no_scrollbar(), .padding().

draw() (~1200 lines): The rendering method. Performs these passes in order:

Pre-scan all lines to build in_code_block[], code_block_start[], code_block_lang[] vectors.
Pre-scan for %%pass password blocks → in_password_block[], password_block_range[].
Pre-scan for table blocks (lines starting and ending with |) → table_block_range[].
Pre-scan for image lines (![...](...)) → image_lines[].
Draw password block backgrounds (rounded rect with border).
Draw table block backgrounds (rounded rect with border).
Main line loop: for each line, determines line height, skips off-screen lines, then branches:
Divider (---/***/___): draws horizontal rule.
File card ([file:UUID:name:size]): draws a styled card with file icon, filename, size.
Checkbox (- [x]/- [ ]): draws custom checkbox square + optional strikethrough.
Image (![alt](img:UUID)): loads from cache, draws scaled image + resize handle + delete button.
Code block opening (```): draws header bar with language label + copy button + language picker.
Code block content: draws with monospace font, indented, on dark background.
Password block opening (%%pass): draws header with eye toggle + copy button.
Password block content: draws as dots or plain text depending on visibility.
Table row: splits by |, measures columns, draws cell text aligned in grid. Separator rows (|---|---|) are drawn as thin lines.
Blockquote (>): draws left border bar + italicized text.
Normal text: calls build_display_segments() to get styled spans (bold, italic, heading, code, link, colored text), creates a Paragraph with Span array, draws it. Headings get scaled font size (1.8x/1.5x/1.3x/1.15x for h1-h4).
Draw selection highlights (per-line rectangles).
Draw cursor (blinking vertical bar, 500ms cycle).
Draw line numbers (if enabled).
Draw slash command menu (floating autocomplete popup).
Draw scrollbar (custom thin rail + thumb).
on_event() (~600 lines): Handles mouse clicks (with double/triple click detection), drag-to-select, scroll (wheel + trackpad with momentum), keyboard input (character insertion, special keys), checkbox toggle click detection, image resize drag, scrollbar drag, code block button clicks, password block button clicks, file card clicks, link clicks. Dispatches via the on_edit callback as MdAction variants.

Free functions: build_display_segments(line) -> (Vec<Segment>, leading_whitespace) — parses a markdown line into styled segments (handles **bold**, *italic*, `code`, [link](url), {c:color}text{/c} color annotations, heading markers). line_height(), wrapped_line_height() — calculate display height for a line. wrapped_cursor_pos() — returns (x, y) pixel position for a cursor at a given column. wrapped_visual_lines() — counts how many visual lines a logical line occupies when wrapped. measure_text_width() — measures rendered width of text. char_to_byte()/char_len() — UTF-8 helpers. cell_to_raw_col() — maps table cell index to raw string column. strip_color_tags() — removes {c:...}...{/c} wrappers, returning clean text + ColorRange list.

src/ui/canvas_editor.rs (1336 lines) — canvas node editor widget
An iced advanced Widget for a node-based canvas with cards connected by edges.

Data model: CanvasData { nodes: Vec<CanvasNode>, edges: Vec<CanvasEdge> }. CanvasNode { id, x, y, w, h, label, color, bg_color, user_min_h } — cards snap to a 20px grid. CanvasEdge { id, from, to, from_side, to_side } with CardSide enum (Top/Right/Bottom/Left).

Editor state: CanvasEditor — holds CanvasData, selected node/edge IDs, multi-selection set, pan offset, zoom level, viewport size, undo/redo stacks, card editor states (HashMap<String, MdEditorState> — one per card), hover state, context menu info, edge creation state, resize state.

Key methods: new(), load_from_json(), sync_labels() (copies card editor text back to node labels), view() (returns Element wrapping the widget), add_node(), delete_selected(), push_undo()/undo()/redo().

Widget impl draw() (~400 lines): Draws grid background, edges (curved paths with arrowheads), nodes (rounded rects with header color bar), embedded MdEditorWidget for each card's content, selection rectangles, edge creation preview, connection port dots on hover.

Widget impl on_event() (~500 lines): Handles pan (middle-click or empty-area drag), zoom (Ctrl+scroll), node click/drag/select, edge click/select, multi-select (Shift+click or rubber-band), node resize (drag from edges), edge creation (drag from port dots), right-click context menu, card focus (double-click to edit), keyboard (Delete, Ctrl+Z/Y, Ctrl+A, arrow keys).

Helper types: CanvasCtxTarget (Node/Edge/Empty — for context menu), CanvasCtxInfo { pos, target }.

src/ui/editor.rs (543 lines) — editor panel view function
pub fn view() takes ~30 parameters and builds the editor panel based on note type:

Password notes: wraps password_editor::view() in a scrollable.
Canvas notes: wraps canvas_editor.view() with right-side tool buttons (add card, fit view, recenter) + context menu overlay.
File notes: delegates to file_viewer::view().
Text notes: calls md_widget::md_editor() for the main editor area, overlays search bar if open.
All types get a toolbar row (pin, favorite, folder move, encrypt buttons + status text).
Contains duplicated move-to-folder dropdown builder (~80 lines copied for canvas branch and text branch, identical logic).
Helper functions tip(), tip_danger(), ctx_menu_btn() for tooltips and context menu buttons.
src/ui/notes_list.rs (578 lines) — notes list panel
pub fn view() takes 24 parameters: notes slice, selected ID, search query, context menu state, rename state, folder tree state, drag state, sort state, multi-selection state.

Builds search bar + sort button header.
For folder views: calls collect_folder_children() recursively to render nested folder tree with notes.
For non-folder views: renders favorite folders inline, then loose notes.
Sort menu as a floating overlay.
render_note() (~120 lines): renders individual note card with type icon, title, snippet, status icons, selection/hover/drag styling.
collect_folder_children() (~140 lines): recursively renders subfolders with expand/collapse arrows, drag-and-drop targets, inline rename, separator lines.
src/ui/tags_panel.rs (192 lines) — sidebar
pub fn view() renders: All button, Favorites button, separator, folder list (with drag-and-drop reorder, inline rename, context menu targets), empty drop area, Settings button.

src/ui/theme.rs (692 lines) — pure styling
Color constants: BG_PRIMARY (#1F1F1F), BG_SECONDARY (#282828), BG_TERTIARY (#323232), BG_HOVER (#3A3A3A), BG_SELECTED (#404040), TEXT_PRIMARY (#D9D9D9), TEXT_SECONDARY (#8D8D8D), DANGER (#E54D4D), TRANSPARENT.
Style functions for: containers (window, tags panel, notes panel, editor panel, toolbar, dialog overlay/card, context menu), buttons (tag, note, context menu, icon, window control, submit, danger, secondary, color dot, toolbar action, search toggle/nav, transparent), text inputs (search, title, dialog, inline rename, line editor, search transparent), text editor (body, live preview), scrollable (dark), SVG hover, text styles (primary, secondary, danger). All are fn(&Theme) -> Style or fn(&Theme, Status) -> Style.

src/ui/icons.rs (149 lines) — SVG icon handles
icon! macro generates pub fn name() -> svg::Handle from inline SVG byte literals.
~60 icons: window controls, formatting (bold, italic, heading, list, checkbox, code, quote, divider, link, align), navigation (chevrons, arrows), actions (plus, search, save, copy, trash, pencil, palette, move, dice, key, lock/unlock, eye, pin, star, crosshair, fit-view), note type indicators, file icons.
Dynamic functions: folder_icon(), folder_colored(color), note_text_icon(color), note_password_icon(color), note_canvas_icon(color), note_file_icon(color) — generate SVG with runtime color values.

src/ui/password_editor.rs (343 lines) — password entry form
pub fn view() builds: website field, email field, username field, password field (with copy/visibility/generator buttons), password strength meter (0-5 score with color bar), generator panel (length presets, charset toggles, generate button), notes text editor, custom fields list (label + value + copy/hide/delete per field), add custom field button.
Helper functions: ghost_btn(), copyable_field(), theme_input_inner(), len_btn(), toggle_chip(), password_strength().

src/ui/color_picker.rs (171 lines) — HSV color picker
PickerProgram implements canvas::Program<Message> for interactive HSV picking.
draw(): renders saturation-value grid (50×30 cells) + hue bar (72 segments) + indicator circle/line.
update(): handles mouse drag on SV area or hue bar, emits ColorPickerSVChanged or ColorPickerHue.
pub fn view(): builds canvas + preview swatch with hex code + preset color buttons (first 10 from FolderColor::PALETTE).
hsv_to_rgb(), hsl_to_rgb() conversion functions.

src/ui/line_editor.rs (25 lines) — dead shim
Re-exports MdEditorState as LineEditorState, adds 4 no-op methods (sync_active_to_lines, activate, deactivate, sync_to_lines), and a view() that just calls md_widget::md_editor().

src/ui/md_highlight.rs (255 lines) — markdown span highlighter
Highlight enum: Normal, Bold, Italic, BoldItalic, Heading, Code, Marker, Link, Quote, ListMarker. Each variant maps to Format<Font> with specific color + font weight/style.
MdHighlighter implements iced::advanced::text::highlighter::Highlighter.
pub fn highlight_inline(line: &str) -> Vec<(Range<usize>, Highlight)> — parses inline markdown syntax into highlight ranges.

src/ui/create_dialog.rs (106 lines)
New note dialog: type selector (text/password/canvas icon buttons), title input, color picker, cancel/create buttons.

src/ui/empty_state.rs (36 lines)
"No note selected" centered placeholder with document icon.

src/ui/file_viewer.rs (164 lines)
File attachment view: parses [file:UUID:filename:size] body format, renders icon box + extension pill + file info + save/save-as/delete buttons.

src/ui/file_vault.rs (95 lines) — DEAD CODE
References non-existent messages FilePathInputChanged, AddFileToVault, DeleteVaultFile. Not used anywhere.

src/ui/graph_view.rs (123 lines)
Simple graph view: arranges notes in a circle, draws nodes + labels. Minimal, not actively developed.

src/ui/settings_view.rs (458 lines)
Settings panel: framerate (15/30/60), auto-save toggle + delay, font size, line numbers toggle, canvas grid size, GUI scale, change vault password section.

src/ui/dialog/password_dialog.rs (329 lines)
Vault setup screen (create password) and login screen (enter password) with window controls, show/hide password toggle.

src/ui/dialog/confirm_dialog.rs (33 lines)
Generic confirmation dialog with title, message, confirm/cancel buttons.

src/ui/dialog/folder_dialog.rs (33 lines)
Folder creation/rename dialog stub.

src/models/note.rs — Note { id, folder_id, title, body, tags, note_type, is_favorite, is_pinned, is_encrypted, color, created_at, modified_at }, NotePreview (subset for list display), NoteType enum (Text/Password/Canvas/File), PasswordData { website, username, password, notes, email, custom_fields }, CustomField { label, value, hidden }, PasswordGenOptions { length, uppercase, lowercase, numbers, symbols } with generate().
src/models/folder.rs — Folder { id, parent_id, name, color, sort_order, collapsed, is_favorite }, FolderColor enum (20 variants) with PALETTE, ALL, to_iced_color(), label(), from_str(). build_tree() builds depth-first folder hierarchy.
src/models/vault.rs — DerivedKey (32-byte key with zeroize-on-drop), VaultStatus enum.
src/models/file_entry.rs — FileEntry { id, name, original_path, size_bytes, encrypted, created_at } with size_display().
src/db/mod.rs — open_connection() (WAL mode, foreign keys, Unix permissions), initialize() (schema + migrations), has_vault_password(), set_vault_password(), get_vault_salt(), get_vault_verify(), get_setting()/set_setting(), save_image()/save_image_encrypted()/load_image_encrypted(), chunked file storage: save_file_chunked() (4MB chunks, progress via AtomicU32), export_file_chunked(), delete_file(), load_file_encrypted().
src/db/notes.rs — note CRUD: insert_note(), update_note(), get_note(), delete_note(), list_notes(), rename_note(), update_note_encryption().
src/db/folders.rs — folder CRUD: insert_folder(), update_folder(), delete_folder(), list_folders(), update_sort_order().
src/db/schema.rs — SQL: CREATE_TABLES (notes, folders, vault_meta, note_images, note_tags) + MIGRATIONS array.
src/crypto/encryption.rs — encrypt(key, plaintext) -> (ciphertext, nonce), decrypt(key, nonce, ciphertext) -> plaintext using AES-256-GCM.
src/crypto/key_derivation.rs — derive_key(password, salt) -> DerivedKey using Argon2id, generate_salt().
src/crypto/secure_memory.rs — secure memory utilities.
KNOWN BUGS — DO NOT FIX DURING REFACTORING STEPS 1–9
Fix these only in steps 10–15, after the structural refactoring is complete and compiling. Each bug fix is a separate step.

Invisible bold/heading text: In md_widget.rs draw(), build_display_segments() hides markdown markers (**, *, #) by coloring them near-background instead of stripping them from the display text. The markers still take up space in the Paragraph layout, pushing visible text rightward. Bold text in particular appears invisible because the marker color blends with the background but the bold content is offset.

Broken text wrapping: wrapped_line_height() and line_height() estimate wrap lines using line_fs * 0.65 per character instead of actual text measurement. Long lines overflow or wrap at wrong positions because proportional fonts don't have uniform character width.

Code block cursor offset: hit_test_position() (in app.rs) doesn't add the code block's left padding (~12px visual indent from the renderer) to the x-coordinate before hit-testing, so clicks inside code blocks place the cursor at the wrong column.

Non-functional code block buttons: The copy button and language picker in code block headers are drawn at specific pixel positions in draw(), but on_event() checks click coordinates against different bounds. The button rectangles aren't stored during draw, so there's no shared reference for hit-testing.

Broken table rendering: Table cell content is drawn at column positions calculated from pipe | splits, but the column width calculation doesn't account for separator row patterns or variable content widths, so text overlaps between columns.

Password block misalignment: The password field eye/copy buttons are drawn at y-positions calculated from the password block's start line, but on_event() uses a different y-offset calculation, making the buttons unclickable in certain scroll positions.

Color picker HSV/HSL mismatch: The color picker canvas uses HSV (hue, saturation, value) — PickerProgram stores sat and val, and hsv_to_rgb() is called for rendering. But throughout app.rs, the state variables are color_hue, color_sat, color_lit, and ApplyNoteColor/ApplyFolderColor handlers call hsl_to_rgb() with the value as if it were lightness. The applied color doesn't match what the picker shows.

TARGET FILE LAYOUT

src/
  main.rs                   — unchanged
  app/
    mod.rs                  — App struct + new() + title() + theme() + scale_factor() + subscription()
    message.rs              — Message enum, EditorSubmenu, ContextMenu, SortMode, ResizeEdge, DragItem, DialogKind, VaultState, ActiveView
    state.rs                — WindowState struct + new_default() + snapshot_window_state() + restore_window_state()
    update.rs               — App::update() — passthrough logic + dispatch to sub-handlers
    update_vault.rs         — vault auth: Setup/Login/Lock, password change
    update_ui.rs            — window mgmt, resize, drag-and-drop, context menus, dialog open/close, zoom, modifiers, hover, animation tick
    update_data.rs          — note/folder CRUD, sort, search, refresh, file transfers, copy/paste, images, multi-select
    update_editor.rs        — MdEdit dispatch, formatting commands, editor save/auto-save, editor search, canvas messages
    view.rs                 — App::view() + view_main() + view_loading() + window_controls()
    view_overlays.rs        — view_dialog() + view_context_menu() + ctx_btn/ctx_btn_danger/ctx_btn_hover helpers
    persistence.rs          — save_current_note() + refresh_data() + refresh_data_sync() + maybe_save()
  models/                   — unchanged
  db/                       — unchanged
  crypto/                   — unchanged
  ui/
    mod.rs                  — updated module declarations
    theme.rs                — unchanged
    icons.rs                — unchanged
    tags_panel.rs           — unchanged
    notes_list.rs           — unchanged
    editor.rs               — deduplicated move-to-folder dropdown
    empty_state.rs          — unchanged
    create_dialog.rs        — unchanged
    file_viewer.rs          — unchanged
    settings_view.rs        — unchanged
    password_editor.rs      — unchanged
    color_picker.rs         — fixed HSV naming
    md_highlight.rs         — unchanged
    graph_view.rs           — unchanged
    md/
      mod.rs                — re-exports + md_editor() constructor function
      state.rs              — MdEditorState struct + from_body() + to_body() + collect_images() + content_height()
      action.rs             — MdAction, MdMotion, SlashCommand, SLASH_COMMANDS, filter_slash_commands(), SearchMatch, ColorRange
      document.rs           — text manipulation methods on MdEditorState: insert/delete/undo/redo/selection/cursor movement
      layout.rs             — line_height, wrapped_line_height, wrapped_cursor_pos, wrapped_visual_lines, measure_text_width, build_display_segments, char_to_byte, char_len, cell_to_raw_col, strip_color_tags, hit_test_position, find_word_bounds
      widget.rs             — MdEditorWidget struct + Widget impl (size, layout, on_event)
      render.rs             — draw() method + all block-type rendering as private functions
    canvas/
      mod.rs                — re-exports
      data.rs               — CanvasNode, CanvasEdge, CanvasData, CardSide
      editor.rs             — CanvasEditor struct + non-widget methods (add_node, delete_selected, sync_labels, undo/redo, load_from_json, view)
      widget.rs             — CanvasWidget Widget impl (draw + on_event)
    dialog/
      mod.rs                — unchanged
      confirm_dialog.rs     — unchanged
      folder_dialog.rs      — unchanged
      password_dialog.rs    — unchanged
REFACTORING STEPS
Step 1: Delete src/ui/file_vault.rs
Delete the file src/ui/file_vault.rs.
In src/ui/mod.rs, remove the line pub mod file_vault;.
Run cargo check.
Step 2: Inline src/ui/line_editor.rs — remove the shim
In src/ui/md_widget.rs, inside the impl MdEditorState block, add these four no-op methods that were previously in line_editor.rs:

pub fn sync_active_to_lines(&mut self) {}
pub fn activate(&mut self, _index: usize) {}
pub fn deactivate(&mut self) {}
pub fn sync_to_lines(&mut self) {}
In every file that references crate::ui::line_editor::LineEditorState, replace it with crate::ui::md_widget::MdEditorState. Files to update: src/app.rs (multiple occurrences in App struct, WindowState, snapshot_window_state, restore_window_state, new()).
In src/ui/editor.rs, replace crate::ui::line_editor::view(line_editor_state, font_size) with:

crate::ui::md_widget::md_editor(line_editor_state, |action| Message::MdEdit(action))
    .size(font_size as f32)
    .into()
Remove the line_editor import.
In src/ui/editor.rs, update the function parameter type from line_editor_state: &'a crate::ui::line_editor::LineEditorState to line_editor_state: &'a crate::ui::md_widget::MdEditorState.
Delete src/ui/line_editor.rs.
In src/ui/mod.rs, remove pub mod line_editor;.
In src/app.rs, remove use crate::ui::line_editor; and update the use import line for ui:: to no longer include line_editor.
Run cargo check.
Step 3: Extract src/app/message.rs
Create directory src/app/.
Move the entire content of src/app.rs into src/app/mod.rs