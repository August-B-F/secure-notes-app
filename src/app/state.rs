use std::time::Instant;

use iced::widget::text_editor;
use uuid::Uuid;

use crate::models::*;
use crate::models::note::PasswordGenOptions;
use crate::ui::canvas_editor::CanvasEditor;

use super::*;

pub struct WindowState {
    pub notes: Vec<NotePreview>,
    pub subfolders: Vec<Folder>,
    pub subfolder_notes: Vec<(Uuid, Vec<NotePreview>)>,
    pub selected_note: Option<Note>,
    pub active_view: ActiveView,
    pub search_query: String,
    pub editor_title: String,
    pub editor_content: text_editor::Content,
    pub line_editor: crate::ui::md_widget::MdEditorState,
    pub editor_dirty: bool,
    pub last_edit_time: Option<Instant>,
    pub editor_search_open: bool,
    pub editor_search_query: String,
    pub editor_search_index: usize, // current match index (0-based)
    pub editor_search_case_sensitive: bool,
    pub editor_preview: bool,
    pub markdown_items: Vec<iced::widget::markdown::Item>,
    pub context_menu: Option<ContextMenu>,
    pub context_menu_pos: (f32, f32),
    pub color_submenu_for: Option<Uuid>,
    pub color_submenu_is_folder: bool,
    pub move_submenu_for: Option<Uuid>,
    pub new_note_submenu_for: Option<Uuid>,
    pub editor_submenu: Option<EditorSubmenu>,
    pub toolbar_move_open: bool,
    pub window_size: (f32, f32),
    pub resizing: Option<ResizeEdge>,
    pub last_notes_loaded: bool,
    pub window_controls_hovered: bool,
    pub hovered_item: Option<Uuid>,
    pub last_note_per_view: std::collections::HashMap<String, Uuid>,
    pub is_maximized: bool,
    pub cursor_pos: (f32, f32),
    pub dialog_anim: f32,
    pub ctx_menu_anim: f32,
    pub page_anim: f32,
    pub rename_pending: u8,
    pub renaming_note: Option<Uuid>,
    pub rename_buffer: String,
    pub renaming_folder: Option<Uuid>,
    pub folder_rename_buffer: String,
    pub expanded_folders: std::collections::HashSet<Uuid>,
    pub sort_menu_open: bool,
    pub multi_selected: std::collections::HashSet<Uuid>,
    pub multi_selected_folders: std::collections::HashSet<Uuid>,
    pub last_clicked_note: Option<Uuid>,
    pub ctrl_held: bool,
    pub shift_held: bool,
    pub alt_held: bool,
    pub dragging: Option<DragItem>,
    pub potential_drag: Option<DragItem>,
    pub drag_start_pos: (f32, f32),
    pub password_data: PasswordData,
    pub password_notes_content: text_editor::Content,
    pub show_password: bool,
    pub show_password_gen: bool,
    pub password_gen_options: PasswordGenOptions,
    pub copied_field: Option<String>,
    pub create_dialog_title: String,
    pub create_dialog_type: NoteType,
    pub create_dialog_color: FolderColor,
    pub color_hue: f32,
    pub color_sat: f32,
    pub color_lit: f32,
    pub text_color_selection: Option<((usize, usize), (usize, usize))>,
    pub create_dialog_folder: Option<Uuid>,
    pub show_graph: bool,
    pub show_settings: bool,
    pub canvas_editor: CanvasEditor,
    pub canvas_color_editing: Option<String>,
    pub active_dialog: Option<DialogKind>,
    pub note_password_input: String,
    pub note_password_confirm: String,
    pub encrypting: bool,
    pub folder_name_input: String,
    pub folder_color_input: FolderColor,
    pub folder_parent_id: Option<Uuid>,
}

impl WindowState {
    pub(super) fn new_default() -> Self {
        Self {
            notes: Vec::new(),
            subfolders: Vec::new(),
            subfolder_notes: Vec::new(),
            selected_note: None,
            active_view: ActiveView::AllNotes,
            search_query: String::new(),
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
            sort_menu_open: false,
            multi_selected: std::collections::HashSet::new(),
            multi_selected_folders: std::collections::HashSet::new(),
            last_clicked_note: None,
            ctrl_held: false,
            shift_held: false,
            alt_held: false,
            dragging: None,
            potential_drag: None,
            drag_start_pos: (0.0, 0.0),
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
            canvas_editor: CanvasEditor::new(),
            canvas_color_editing: None,
            active_dialog: None,
            note_password_input: String::new(),
            note_password_confirm: String::new(),
            encrypting: false,
            folder_name_input: String::new(),
            folder_color_input: FolderColor::Blue,
            folder_parent_id: None,
        }
    }
}

impl App {
    pub(super) fn snapshot_window_state(&mut self) -> WindowState {
        WindowState {
            notes: std::mem::take(&mut self.notes),
            subfolders: std::mem::take(&mut self.subfolders),
            subfolder_notes: std::mem::take(&mut self.subfolder_notes),
            selected_note: self.selected_note.take(),
            active_view: std::mem::replace(&mut self.active_view, ActiveView::AllNotes),
            search_query: std::mem::take(&mut self.search_query),
            editor_title: std::mem::take(&mut self.editor_title),
            editor_content: std::mem::replace(&mut self.editor_content, text_editor::Content::new()),
            line_editor: std::mem::replace(&mut self.line_editor, crate::ui::md_widget::MdEditorState::from_body("")),
            editor_dirty: std::mem::replace(&mut self.editor_dirty, false),
            last_edit_time: self.last_edit_time.take(),
            editor_search_open: std::mem::replace(&mut self.editor_search_open, false),
            editor_search_query: std::mem::take(&mut self.editor_search_query),
            editor_search_index: std::mem::replace(&mut self.editor_search_index, 0),
            editor_search_case_sensitive: std::mem::replace(&mut self.editor_search_case_sensitive, false),
            editor_preview: std::mem::replace(&mut self.editor_preview, false),
            markdown_items: std::mem::take(&mut self.markdown_items),
            context_menu: self.context_menu.take(),
            context_menu_pos: std::mem::replace(&mut self.context_menu_pos, (0.0, 0.0)),
            color_submenu_for: self.color_submenu_for.take(),
            color_submenu_is_folder: std::mem::replace(&mut self.color_submenu_is_folder, false),
            move_submenu_for: self.move_submenu_for.take(),
            new_note_submenu_for: self.new_note_submenu_for.take(),
            editor_submenu: self.editor_submenu.take(),
            toolbar_move_open: std::mem::replace(&mut self.toolbar_move_open, false),
            window_size: std::mem::replace(&mut self.window_size, (1100.0, 700.0)),
            resizing: self.resizing.take(),
            last_notes_loaded: std::mem::replace(&mut self.last_notes_loaded, false),
            window_controls_hovered: std::mem::replace(&mut self.window_controls_hovered, false),
            hovered_item: self.hovered_item.take(),
            last_note_per_view: std::mem::take(&mut self.last_note_per_view),
            is_maximized: std::mem::replace(&mut self.is_maximized, false),
            cursor_pos: std::mem::replace(&mut self.cursor_pos, (0.0, 0.0)),
            dialog_anim: std::mem::replace(&mut self.dialog_anim, 0.0),
            ctx_menu_anim: std::mem::replace(&mut self.ctx_menu_anim, 0.0),
            page_anim: std::mem::replace(&mut self.page_anim, 1.0),
            rename_pending: std::mem::replace(&mut self.rename_pending, 0),
            renaming_note: self.renaming_note.take(),
            rename_buffer: std::mem::take(&mut self.rename_buffer),
            renaming_folder: self.renaming_folder.take(),
            folder_rename_buffer: std::mem::take(&mut self.folder_rename_buffer),
            expanded_folders: std::mem::take(&mut self.expanded_folders),
            sort_menu_open: std::mem::replace(&mut self.sort_menu_open, false),
            multi_selected: std::mem::take(&mut self.multi_selected),
            multi_selected_folders: std::mem::take(&mut self.multi_selected_folders),
            last_clicked_note: self.last_clicked_note.take(),
            ctrl_held: std::mem::replace(&mut self.ctrl_held, false),
            shift_held: std::mem::replace(&mut self.shift_held, false),
            alt_held: std::mem::replace(&mut self.alt_held, false),
            dragging: self.dragging.take(),
            potential_drag: self.potential_drag.take(),
            drag_start_pos: std::mem::replace(&mut self.drag_start_pos, (0.0, 0.0)),
            password_data: std::mem::replace(&mut self.password_data, PasswordData::default()),
            password_notes_content: std::mem::replace(&mut self.password_notes_content, text_editor::Content::new()),
            show_password: std::mem::replace(&mut self.show_password, false),
            show_password_gen: std::mem::replace(&mut self.show_password_gen, false),
            password_gen_options: std::mem::replace(&mut self.password_gen_options, PasswordGenOptions::default()),
            copied_field: self.copied_field.take(),
            create_dialog_title: std::mem::take(&mut self.create_dialog_title),
            create_dialog_type: std::mem::replace(&mut self.create_dialog_type, NoteType::Text),
            create_dialog_color: std::mem::replace(&mut self.create_dialog_color, FolderColor::Green),
            color_hue: std::mem::replace(&mut self.color_hue, 140.0),
            color_sat: std::mem::replace(&mut self.color_sat, 70.0),
            color_lit: std::mem::replace(&mut self.color_lit, 50.0),
            text_color_selection: self.text_color_selection.take(),
            create_dialog_folder: self.create_dialog_folder.take(),
            show_graph: std::mem::replace(&mut self.show_graph, false),
            show_settings: std::mem::replace(&mut self.show_settings, false),
            canvas_editor: std::mem::replace(&mut self.canvas_editor, CanvasEditor::new()),
            canvas_color_editing: self.canvas_color_editing.take(),
            active_dialog: self.active_dialog.take(),
            note_password_input: std::mem::take(&mut self.note_password_input),
            note_password_confirm: std::mem::take(&mut self.note_password_confirm),
            encrypting: std::mem::replace(&mut self.encrypting, false),
            folder_name_input: std::mem::take(&mut self.folder_name_input),
            folder_color_input: std::mem::replace(&mut self.folder_color_input, FolderColor::Blue),
            folder_parent_id: self.folder_parent_id.take(),
        }
    }

    pub(super) fn restore_window_state(&mut self, ws: WindowState) {
        self.notes = ws.notes;
        self.subfolders = ws.subfolders;
        self.subfolder_notes = ws.subfolder_notes;
        self.selected_note = ws.selected_note;
        self.active_view = ws.active_view;
        self.search_query = ws.search_query;
        self.editor_title = ws.editor_title;
        self.editor_content = ws.editor_content;
        self.line_editor = ws.line_editor;
        self.editor_dirty = ws.editor_dirty;
        self.last_edit_time = ws.last_edit_time;
        self.editor_search_open = ws.editor_search_open;
        self.editor_search_query = ws.editor_search_query;
        self.editor_search_index = ws.editor_search_index;
        self.editor_search_case_sensitive = ws.editor_search_case_sensitive;
        self.editor_preview = ws.editor_preview;
        self.markdown_items = ws.markdown_items;
        self.context_menu = ws.context_menu;
        self.context_menu_pos = ws.context_menu_pos;
        self.color_submenu_for = ws.color_submenu_for;
        self.color_submenu_is_folder = ws.color_submenu_is_folder;
        self.move_submenu_for = ws.move_submenu_for;
        self.new_note_submenu_for = ws.new_note_submenu_for;
        self.editor_submenu = ws.editor_submenu;
        self.toolbar_move_open = ws.toolbar_move_open;
        self.window_size = ws.window_size;
        self.resizing = ws.resizing;
        self.last_notes_loaded = ws.last_notes_loaded;
        self.window_controls_hovered = ws.window_controls_hovered;
        self.hovered_item = ws.hovered_item;
        self.last_note_per_view = ws.last_note_per_view;
        self.is_maximized = ws.is_maximized;
        self.cursor_pos = ws.cursor_pos;
        self.dialog_anim = ws.dialog_anim;
        self.ctx_menu_anim = ws.ctx_menu_anim;
        self.page_anim = ws.page_anim;
        self.rename_pending = ws.rename_pending;
        self.renaming_note = ws.renaming_note;
        self.rename_buffer = ws.rename_buffer;
        self.renaming_folder = ws.renaming_folder;
        self.folder_rename_buffer = ws.folder_rename_buffer;
        self.expanded_folders = ws.expanded_folders;
        self.sort_menu_open = ws.sort_menu_open;
        self.multi_selected = ws.multi_selected;
        self.multi_selected_folders = ws.multi_selected_folders;
        self.last_clicked_note = ws.last_clicked_note;
        self.ctrl_held = ws.ctrl_held;
        self.shift_held = ws.shift_held;
        self.alt_held = ws.alt_held;
        self.dragging = ws.dragging;
        self.potential_drag = ws.potential_drag;
        self.drag_start_pos = ws.drag_start_pos;
        self.password_data = ws.password_data;
        self.password_notes_content = ws.password_notes_content;
        self.show_password = ws.show_password;
        self.show_password_gen = ws.show_password_gen;
        self.password_gen_options = ws.password_gen_options;
        self.copied_field = ws.copied_field;
        self.create_dialog_title = ws.create_dialog_title;
        self.create_dialog_type = ws.create_dialog_type;
        self.create_dialog_color = ws.create_dialog_color;
        self.color_hue = ws.color_hue;
        self.color_sat = ws.color_sat;
        self.color_lit = ws.color_lit;
        self.text_color_selection = ws.text_color_selection;
        self.create_dialog_folder = ws.create_dialog_folder;
        self.show_graph = ws.show_graph;
        self.show_settings = ws.show_settings;
        self.canvas_editor = ws.canvas_editor;
        self.canvas_color_editing = ws.canvas_color_editing;
        self.active_dialog = ws.active_dialog;
        self.note_password_input = ws.note_password_input;
        self.note_password_confirm = ws.note_password_confirm;
        self.encrypting = ws.encrypting;
        self.folder_name_input = ws.folder_name_input;
        self.folder_color_input = ws.folder_color_input;
        self.folder_parent_id = ws.folder_parent_id;
    }
}
