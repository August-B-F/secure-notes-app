pub mod message;
pub mod state;
mod helpers;
mod persistence;
mod view;
mod view_overlays;
mod update_vault;
mod update_editor;
mod update_ui;
mod update;
mod update_data;
pub use message::*;
pub use state::WindowState;
use helpers::*;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use iced::keyboard;
use iced::widget::{button, column, container, mouse_area, row, stack, svg, text, text_editor, text_input, Space};
use iced::window;
use iced::{Element, Length, Point, Subscription, Task, Theme};
use uuid::Uuid;

use crate::crypto;
use crate::db;
use crate::models::*;
use crate::models::note::{CustomField, PasswordGenOptions};
use crate::ui::{canvas_editor, create_dialog, dialog, editor, empty_state, icons, notes_list, settings_view, tags_panel, theme};
use crate::ui::canvas_editor::{CanvasEditor, CanvasNode};

pub struct App {
    db_path: PathBuf,
    #[allow(dead_code)]
    main_window: window::Id,
    focused_window: window::Id,
    cursor_window: window::Id,
    other_windows: std::collections::HashMap<window::Id, WindowState>,
    window_icon: Option<window::Icon>,

    vault_state: VaultState,
    vault_key: Option<[u8; 32]>, // cached vault-derived key for image encryption
    password_input: String,
    confirm_password_input: String,
    pub(crate) auth_error: Option<String>,

    pub(crate) folders: Vec<Folder>,
    notes: Vec<NotePreview>,
    selected_note: Option<Note>,
    pub(crate) all_count: usize,
    fav_count: usize,
    folder_counts: Vec<(Uuid, usize)>,

    active_view: ActiveView,
    search_query: String,

    context_menu: Option<ContextMenu>,
    context_menu_pos: (f32, f32),
    color_submenu_for: Option<Uuid>,  // note or folder ID when color sub-menu is open
    color_submenu_is_folder: bool,   // true if color submenu is for a folder
    move_submenu_for: Option<Uuid>,  // note/folder ID when move submenu is open
    new_note_submenu_for: Option<Uuid>, // folder ID when "New" submenu is open
    editor_submenu: Option<EditorSubmenu>,
    toolbar_move_open: bool,        // move picker dropdown from editor toolbar
    window_size: (f32, f32),         // current window size
    resizing: Option<ResizeEdge>,    // active resize edge/corner
    last_notes_loaded: bool,         // whether saved last-notes have been loaded from DB
    window_controls_hovered: bool,
    hovered_item: Option<Uuid>,
    last_note_per_view: std::collections::HashMap<String, Uuid>,
    pub(crate) is_maximized: bool,
    cursor_pos: (f32, f32),
    #[allow(dead_code)]
    dialog_anim: f32,
    #[allow(dead_code)]
    ctx_menu_anim: f32,
    #[allow(dead_code)]
    page_anim: f32,
    rename_pending: u8, // countdown: retries focus until element exists
    renaming_note: Option<Uuid>,
    rename_buffer: String,
    renaming_folder: Option<Uuid>,
    folder_rename_buffer: String,
    expanded_folders: std::collections::HashSet<Uuid>,
    sort_mode: SortMode,
    sort_menu_open: bool,
    show_sidebar: bool,
    multi_selected: std::collections::HashSet<Uuid>,       // notes
    multi_selected_folders: std::collections::HashSet<Uuid>, // folders
    last_clicked_note: Option<Uuid>,
    ctrl_held: bool,
    shift_held: bool,
    alt_held: bool,
    dragging: Option<DragItem>,
    potential_drag: Option<DragItem>,
    drag_start_pos: (f32, f32),
    subfolders: Vec<Folder>,
    subfolder_notes: Vec<(Uuid, Vec<NotePreview>)>,

    editor_title: String,
    editor_content: text_editor::Content,
    line_editor: crate::ui::md_widget::MdEditorState,
    editor_dirty: bool,
    last_edit_time: Option<Instant>,
    editor_search_open: bool,
    editor_search_query: String,
    editor_search_index: usize,
    editor_search_case_sensitive: bool,
    editor_preview: bool,
    markdown_items: Vec<iced::widget::markdown::Item>,

    file_transfers: Vec<(String, String, std::sync::Arc<std::sync::atomic::AtomicU32>)>,

    password_data: PasswordData,
    password_notes_content: text_editor::Content,
    show_password: bool,
    show_password_gen: bool,
    password_gen_options: PasswordGenOptions,
    copied_field: Option<String>,  // which field just got copied (for checkmark animation)

    create_dialog_title: String,
    create_dialog_type: NoteType,
    create_dialog_color: FolderColor,
    color_hue: f32,
    color_sat: f32,
    color_lit: f32,
    text_color_selection: Option<((usize, usize), (usize, usize))>,
    create_dialog_folder: Option<Uuid>,

    show_graph: bool,
    show_settings: bool,
    canvas_editor: CanvasEditor,
    canvas_color_editing: Option<String>,

    pub setting_framerate: u32,       // FPS (15, 30, 60)
    pub setting_auto_save: bool,
    pub setting_auto_save_delay: u32, // seconds
    pub setting_font_size: u32,       // editor font size in px
    pub gui_scale: f64,               // UI zoom factor (1.0 = 100%)
    pub zoom_toast: Option<Instant>,   // when set, shows zoom overlay until expired
    pub setting_grid_size: u32,       // canvas grid snap in px
    pub setting_line_numbers: bool,

    loading_tick: usize,

    active_dialog: Option<DialogKind>,
    note_password_input: String,
    note_password_confirm: String,
    session_decrypted: std::collections::HashMap<Uuid, Vec<u8>>,  // note ID -> password bytes for re-encryption
    clipboard_notes: Vec<Uuid>,    // copied note IDs
    clipboard_folders: Vec<Uuid>,  // copied folder IDs
    encrypting: bool,  // show loading during crypto
    folder_name_input: String,
    folder_color_input: FolderColor,
    folder_parent_id: Option<Uuid>,

    pub(crate) show_change_password: bool,
    pub(crate) vault_old_password: String,
    pub(crate) vault_new_password: String,
    pub(crate) vault_new_password_confirm: String,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let data_dir = db::db_path();
        let db_path = data_dir.join("notes.db");

        if let Ok(conn) = db::open_connection(&db_path) {
            let _ = db::initialize(&conn);
        }

        let conn_opt = db::open_connection(&db_path).ok();
        let get = |key: &str| conn_opt.as_ref().and_then(|c| db::get_setting(c, key));

        let saved_sort_mode = get("sort_mode").map(|s| SortMode::from_str(&s)).unwrap_or(SortMode::Modified);
        let setting_framerate = get("framerate").and_then(|s| s.parse().ok()).unwrap_or(30u32);
        let setting_auto_save = get("auto_save").map(|s| s == "true").unwrap_or(true);
        let setting_auto_save_delay = get("auto_save_delay").and_then(|s| s.parse().ok()).unwrap_or(2u32);
        let setting_font_size = get("font_size").and_then(|s| s.parse().ok()).unwrap_or(15u32);
        let setting_grid_size = get("grid_size").and_then(|s| s.parse().ok()).unwrap_or(20u32);
        let setting_line_numbers = get("line_numbers").map(|s| s == "true").unwrap_or(false);
        let gui_scale = get("gui_scale").and_then(|s| s.parse().ok()).unwrap_or(1.0f64);
        drop(conn_opt);

        let vault_state = if let Ok(conn) = db::open_connection(&db_path) {
            if db::has_vault_password(&conn) { VaultState::Login } else { VaultState::Setup }
        } else {
            VaultState::Setup
        };

        let window_icon = window::icon::from_file_data(
            include_bytes!("../../assets/logo.png"),
            None,
        ).ok();

        let (main_window_id, open_main) = window::open(window::Settings {
            size: iced::Size::new(1100.0, 700.0),
            min_size: Some(iced::Size::new(600.0, 400.0)),
            decorations: false,
            transparent: true,
            resizable: true,
            icon: window_icon.clone(),
            ..Default::default()
        });

        let app = Self {
            db_path: db_path.clone(),
            main_window: main_window_id,
            focused_window: main_window_id,
            cursor_window: main_window_id,
            other_windows: std::collections::HashMap::new(),
            window_icon,
            vault_state,
            gui_scale,
            zoom_toast: None,
            vault_key: None,
            password_input: String::new(),
            confirm_password_input: String::new(),
            auth_error: None,
            folders: Vec::new(),
            notes: Vec::new(),
            selected_note: None,
            all_count: 0,
            fav_count: 0,
            folder_counts: Vec::new(),
            active_view: ActiveView::AllNotes,
            search_query: String::new(),
            context_menu: None,
            context_menu_pos: (0.0, 0.0),
            color_submenu_for: None,
            color_submenu_is_folder: false,
            move_submenu_for: None,
            new_note_submenu_for: None,
            editor_submenu: None,
            toolbar_move_open: false,
            window_size: (1100.0, 700.0),
            resizing: None,
            last_notes_loaded: false,
            window_controls_hovered: false,
            hovered_item: None,
            last_note_per_view: std::collections::HashMap::new(),
            is_maximized: false,
            cursor_pos: (0.0, 0.0),
            dialog_anim: 0.0,
            ctx_menu_anim: 0.0,
            page_anim: 1.0,
            rename_pending: 0,
            renaming_note: None,
            rename_buffer: String::new(),
            renaming_folder: None,
            folder_rename_buffer: String::new(),
            expanded_folders: std::collections::HashSet::new(),
            sort_mode: saved_sort_mode,
            sort_menu_open: false,
            show_sidebar: true,
            multi_selected: std::collections::HashSet::new(),
            multi_selected_folders: std::collections::HashSet::new(),
            last_clicked_note: None,
            ctrl_held: false,
            shift_held: false,
            alt_held: false,
            dragging: None,
            potential_drag: None,
            drag_start_pos: (0.0, 0.0),
            subfolders: Vec::new(),
            subfolder_notes: Vec::new(),
            editor_title: String::new(),
            editor_content: text_editor::Content::new(),
            line_editor: crate::ui::md_widget::MdEditorState::from_body(""),
            editor_dirty: false,
            last_edit_time: None,
            editor_search_open: false,
            editor_search_query: String::new(),
            editor_search_index: 0,
            editor_search_case_sensitive: false,
            editor_preview: false,
            markdown_items: Vec::new(),
            file_transfers: Vec::new(),
            password_data: PasswordData::default(),
            password_notes_content: text_editor::Content::new(),
            show_password: false,
            show_password_gen: false,
            password_gen_options: PasswordGenOptions::default(),
            copied_field: None,
            create_dialog_title: String::new(),
            create_dialog_type: NoteType::Text,
            create_dialog_color: FolderColor::Green,
            color_hue: 140.0,
            color_sat: 70.0,
            color_lit: 50.0,
            text_color_selection: None,
            create_dialog_folder: None,
            show_graph: false,
            show_settings: false,
            setting_framerate,
            setting_auto_save,
            setting_auto_save_delay,
            setting_font_size,
            setting_grid_size,
            setting_line_numbers,
            canvas_editor: CanvasEditor::new(),
            canvas_color_editing: None,
            loading_tick: 0,
            active_dialog: None,
            note_password_input: String::new(),
            note_password_confirm: String::new(),
            session_decrypted: std::collections::HashMap::new(),
            clipboard_notes: Vec::new(),
            clipboard_folders: Vec::new(),
            encrypting: false,
            folder_name_input: String::new(),
            folder_color_input: FolderColor::Blue,
            folder_parent_id: None,

            show_change_password: false,
            vault_old_password: String::new(),
            vault_new_password: String::new(),
            vault_new_password_confirm: String::new(),
        };

        (app, open_main.map(|_| Message::None))
    }

    pub fn title(&self, id: window::Id) -> String {
        if id == self.focused_window {
            String::from("Secure Notes")
        } else if let Some(win) = self.other_windows.get(&id) {
            let title = if win.editor_title.is_empty() { "Untitled" } else { &win.editor_title };
            format!("Secure Notes — {}", title)
        } else {
            String::from("Secure Notes")
        }
    }

    pub fn scale_factor(&self, _id: window::Id) -> f64 {
        self.gui_scale
    }

    pub fn theme(&self, _id: window::Id) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();

        if self.vault_state == VaultState::Loading {
            subs.push(iced::time::every(Duration::from_millis(300)).map(|_| Message::LoadingTick));
        }

        if self.setting_auto_save && self.editor_dirty && self.vault_state == VaultState::Unlocked {
            subs.push(iced::time::every(Duration::from_millis(500)).map(Message::AutoSaveTick));
        }

        if self.vault_state == VaultState::Unlocked {
            let ms = (1000 / self.setting_framerate.max(1)) as u64;
            subs.push(iced::time::every(Duration::from_millis(ms)).map(|_| Message::AnimationTick));
        }

        subs.push(iced::event::listen_with(|event, _status, wid| {
            match event {
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::CursorMoved(wid, position.x, position.y))
                }
                iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                    Some(Message::DragEnd(_status == iced::event::Status::Captured))
                }
                iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) => {
                    Some(Message::ModifiersChanged(mods.control(), mods.shift(), mods.alt()))
                }
                // ctrl+f even when a text input is focused
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: keyboard::Key::Character(ref c), modifiers, .. })
                    if modifiers.command() && (c.as_ref() == "f" || c.as_ref() == "F") && _status == iced::event::Status::Captured =>
                {
                    Some(Message::ToggleSearch)
                }
                _ => None,
            }
        }));

        subs.push(window::resize_events().map(|(wid, size)| Message::WindowResized(wid, size.width, size.height)));

        subs.push(window::close_requests().map(Message::WindowCloseRequested));

        subs.push(iced::event::listen_with(|event, _status, wid| {
            match event {
                iced::Event::Window(window::Event::Focused) => Some(Message::WindowFocused(wid)),
                iced::Event::Window(window::Event::Closed) => Some(Message::WindowClosed(wid)),
                iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::ImageDropped(wid, path)),
                _ => None,
            }
        }));

        if self.vault_state == VaultState::Unlocked {
            subs.push(keyboard::on_key_press(|key, modifiers| {
                let ctrl = modifiers.command();
                let shift = modifiers.shift();
                match key.as_ref() {
                    keyboard::Key::Character(c) if ctrl && shift => match c.as_ref() {
                        "N" | "n" => Some(Message::CreateQuickFolder(None)),
                        "W" | "w" => Some(Message::OpenNewWindow),
                        _ => None,
                    },
                    keyboard::Key::Character(c) if ctrl => match c.as_ref() {
                        "n" | "N" => Some(Message::CreateQuickNote(NoteType::Text)),
                        "s" | "S" => Some(Message::SaveNote),
                        "b" | "B" => Some(Message::ToggleSidebar),
                        "f" | "F" => Some(Message::ToggleSearch),
                        "c" | "C" => Some(Message::CopySelected),
                        "v" | "V" => Some(Message::PasteItems),
                        "+" | "=" => Some(Message::ZoomIn),
                        "-" | "_" => Some(Message::ZoomOut),
                        "0" => Some(Message::ZoomReset),
                        _ => None,
                    },
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        Some(Message::CancelRename) // also closes search via CancelRename handler
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowUp) if !ctrl => {
                        Some(Message::LineArrowUp)
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowDown) if !ctrl => {
                        Some(Message::LineArrowDown)
                    }
                    _ => None,
                }
            }));
        }

        Subscription::batch(subs)
    }



}
