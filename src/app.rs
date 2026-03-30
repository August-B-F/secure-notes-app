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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultState {
    Setup,
    Login,
    Loading, // Shown during key derivation
    Unlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActiveView {
    AllNotes,
    Favorites,
    Folder(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenu {
    Tag(Uuid),
    NoteItem(Uuid),
    NoteColor(Uuid),
    TagsEmpty,
    NotesEmpty,
    EditorFormat,
    TableCell(usize), // line index of the table row
    ImageMenu(usize), // line index of the image
    FileMenu(usize),  // line index of the file attachment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Modified,
    Created,
    NameAZ,
    NameZA,
    Type,
}

impl SortMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modified => "modified",
            Self::Created => "created",
            Self::NameAZ => "name_az",
            Self::NameZA => "name_za",
            Self::Type => "type",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "modified" => Self::Modified,
            "created" => Self::Created,
            "name_az" => Self::NameAZ,
            "name_za" => Self::NameZA,
            "type" => Self::Type,
            _ => Self::Modified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Right, Bottom, Left, Top,
    TopLeft, TopRight, BottomLeft, BottomRight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragItem {
    Note(Uuid),
    Folder(Uuid),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DialogKind {
    CreateNote,
    EncryptNote(Uuid),
    DecryptNote(Uuid),
    CreateFolder,
    RenameFolder(Uuid),
    DeleteNote(Uuid),
    DeleteFolder(Uuid),
    MoveFolderPicker(Uuid),
    NoteColor(Uuid),
    ChangePassword(Uuid),
    DeleteMultiConfirm,
    ChangeVaultPassword,
    TextColor,
}

pub struct WindowState {
    pub notes: Vec<NotePreview>,
    pub subfolders: Vec<Folder>,
    pub subfolder_notes: Vec<(Uuid, Vec<NotePreview>)>,
    pub selected_note: Option<Note>,
    pub active_view: ActiveView,
    pub search_query: String,
    pub editor_title: String,
    pub editor_content: text_editor::Content,
    pub line_editor: crate::ui::line_editor::LineEditorState,
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
    fn new_default() -> Self {
        Self {
            notes: Vec::new(),
            subfolders: Vec::new(),
            subfolder_notes: Vec::new(),
            selected_note: None,
            active_view: ActiveView::AllNotes,
            search_query: String::new(),
            editor_title: String::new(),
            editor_content: text_editor::Content::new(),
            line_editor: crate::ui::line_editor::LineEditorState::from_body(""),
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    PasswordInputChanged(String),
    ConfirmPasswordInputChanged(String),
    SubmitSetup,
    SubmitLogin,
    SetupDone(Result<[u8; 32], String>),
    LoginDone(Result<[u8; 32], String>), // Ok(vault_key) or Err(message)

    LockVault,

    OpenChangeVaultPasswordDialog,
    VaultOldPasswordChanged(String),
    VaultNewPasswordChanged(String),
    VaultNewPasswordConfirmChanged(String),
    SubmitChangeVaultPassword,
    ChangeVaultPasswordDone(Result<(), String>),

    WindowClose,
    WindowMinimize,
    WindowMaximize,
    WindowDrag,
    WindowControlsHover(bool),
    WindowResizeStart(ResizeEdge),
    WindowResized(window::Id, f32, f32),
    HoverItem(Option<Uuid>),

    SelectView(ActiveView),
    SelectNote(Uuid),
    SearchQueryChanged(String),

    ToggleContextMenu(ContextMenu),
    CloseContextMenu,

    CreateNote,
    CreateEncryptedNote,
    CreateQuickNote(NoteType),
    CreateNoteInFolder(NoteType, Uuid),
    OpenCreateNoteDialog,
    SubmitCreateNote,
    CreateDialogTitleChanged(String),
    CreateDialogTypeChanged(NoteType),
    CreateDialogColorChanged(FolderColor),
    CreateDialogFolderChanged(Option<Uuid>),
    DeleteNote(Uuid),
    ToggleFavorite(Uuid),
    TogglePin(Uuid),
    SetNoteColor(Uuid, FolderColor),
    SetFolderColor(Uuid, FolderColor),
    ToggleFolderFavorite(Uuid),
    OpenNoteColorDialog(Uuid),
    ApplyNoteColor(Uuid),
    FocusTitle,
    RenameNote(Uuid),
    RenameNoteChanged(String),
    RenameNoteSubmit,
    CancelRename,
    ToggleFolderSelect(Uuid),
    DeleteMultiSelected,
    OpenDeleteMultiDialog,
    MoveMultiSelectedToFolder(Option<Uuid>),
    CreateQuickFolder(Option<Uuid>),  // parent_id
    RenameFolderInline(Uuid),
    RenameFolderChanged(String),
    RenameFolderSubmit,
    ToggleColorSubmenu(Uuid),
    ToggleFolderColorSubmenu(Uuid),
    OpenColorSubmenu(Uuid),
    OpenFolderColorSubmenu(Uuid),
    OpenMoveSubmenu(Uuid),
    OpenNewNoteSubmenu(Uuid),
    ToggleNewNoteSubmenu(Uuid),
    ApplyFolderColor(Uuid),
    ColorPickerPreset(FolderColor),
    ColorPickerHue(f32),
    ColorPickerSat(f32),
    ColorPickerLit(f32),
    ColorPickerSVChanged(f32, f32),
    MoveNoteToFolder(Uuid, Option<Uuid>),

    EditorTitleChanged(String),
    EditorContentAction(text_editor::Action),
    SaveNote,
    AutoSaveTick(Instant),

    FormatBold,
    FormatItalic,
    FormatHeading,
    FormatList,
    FormatCheckbox,
    FormatCode,
    FormatDivider,
    FormatQuote,
    FormatTextColor(String),
    OpenTextColorPicker,
    ApplyTextColor,

    LineClicked(usize),
    LineRightClicked(usize),
    LineEditorAction(usize, text_editor::Action),
    LineInputChanged(usize, String),
    LineInputSubmit(usize),
    LineBlur,
    LineArrowUp,
    LineArrowDown,
    FocusActiveLine,
    MdEdit(crate::ui::md_widget::MdAction),

    ToggleSearch,
    ToggleSearchCaseSensitive,
    SearchQueryEditorChanged(String),
    SearchNext,
    SearchPrev,
    ToggleMarkdownPreview,
    ToggleSidebar,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    CloseSubmenus,

    PasswordWebsiteChanged(String),
    PasswordUsernameChanged(String),
    PasswordEmailChanged(String),
    PasswordValueChanged(String),
    PasswordNotesChanged(String),
    PasswordNotesAction(text_editor::Action),
    TogglePasswordVisibility,
    GeneratePassword,
    PasswordGenLength(u32),
    PasswordGenToggleUpper,
    PasswordGenToggleLower,
    PasswordGenToggleNumbers,
    PasswordGenToggleSymbols,
    TogglePasswordGenPanel,
    AddCustomField,
    RemoveCustomField(usize),
    CustomFieldLabelChanged(usize, String),
    CustomFieldValueChanged(usize, String),
    ToggleCustomFieldHidden(usize),
    CopyField(String, String),   // (field_name, value) — copy value, show feedback on field_name
    CopiedFeedbackClear,
    ClearCopiedBlockFeedback,

    SetSortMode(SortMode),
    ToggleSortMenu,
    ModifiersChanged(bool, bool, bool), // ctrl, shift, alt

    ToggleExpandFolder(Uuid),
    DragStart(DragItem),
    DragEnd,
    DragPotential(DragItem),  // mouse down — might become drag
    DropOnFolder(DragItem, Option<Uuid>),   // move/reparent
    ReorderDrop(DragItem, Uuid),            // finalize reorder in main panel
    ReorderPreview(Uuid, Uuid),             // dragged_folder, hovered_folder — live preview
    ReorderToEnd(Uuid),                     // move folder to end of list
    ReorderMainFolder(Uuid, i32), // folder_id, new sort_order

    CreateFolder,
    RenameFolder(Uuid),
    DeleteFolder(Uuid),
    FolderNameInputChanged(String),
    FolderColorSelected(FolderColor),

    OpenCreateFolderDialog,
    OpenCreateSubfolderDialog(Uuid),
    OpenRenameFolderDialog(Uuid),
    OpenDeleteNoteDialog(Uuid),
    OpenDeleteFolderDialog(Uuid),
    OpenEncryptDialog(Uuid),
    OpenDecryptDialog(Uuid),
    OpenMoveFolderPicker(Uuid),
    CloseDialog,

    NotePasswordInputChanged(String),
    NotePasswordConfirmChanged(String),
    SubmitEncrypt(Uuid),
    SubmitDecrypt(Uuid),
    EncryptionDone(Result<(), String>),
    DecryptionDone(Result<(Uuid, String, Vec<u8>), String>),
    RemoveEncryption(Uuid),
    ChangeEncryptionPassword(Uuid),
    SubmitChangePassword(Uuid),
    LockNote,

    AnimationTick,
    CursorMoved(window::Id, f32, f32),

    CanvasAddNode(f32, f32),
    CanvasAddNodeCenter,
    CanvasMoveNode(String, f32, f32),
    CanvasSelect(Option<String>),
    CanvasDeleteSelected,
    CanvasAddEdge(String, canvas_editor::CardSide, String, canvas_editor::CardSide),
    CanvasCardEdit(String, crate::ui::md_widget::MdAction),
    CanvasCardFocus(String),
    CanvasCardUnfocus,
    CanvasMultiSelect(Vec<String>),
    CanvasResizeNode(String, f32, f32, f32, f32),
    CanvasSetNodeBgColor(String, String),
    CanvasOpenColorPicker(String),
    CanvasApplyColor,
    CanvasRecenter,
    CanvasUndo,
    CanvasRedo,
    CanvasFitView,
    CanvasMoveNodeGroup(Vec<(String, f32, f32)>),
    CanvasSelectEdge(Option<String>),
    CanvasReverseEdge(String),
    CanvasDeleteEdge(String),
    CanvasCloseCtxMenu,
    CanvasPan(f32, f32),
    CanvasZoom(f32, f32, f32),
    CanvasViewportSize(f32, f32),
    CanvasHover(Option<String>),
    CanvasShowCtxMenu(f32, f32, canvas_editor::CanvasCtxTarget),

    ToggleGraphView,
    ShowSettings,

    SetFramerate(u32),
    SetAutoSaveDelay(u32),
    SetEditorFontSize(u32),
    SetCanvasGridSize(u32),
    ToggleAutoSave,
    ToggleLineNumbers,

    Refresh,
    DataLoaded(Vec<Folder>, Vec<NotePreview>, usize, usize, Vec<(Uuid, usize)>, Vec<Folder>, Vec<(Uuid, Vec<NotePreview>)>),
    NoteLoaded(Option<Note>),

    LoadingTick,
    None,

    CopySelected,
    PasteItems,
    ImageDropped(window::Id, std::path::PathBuf),
    CopyImage(usize), // copy image at line index to clipboard
    InsertImageData(Vec<u8>, String), // (bytes, mime)
    ImagesLoaded(Vec<(String, Vec<u8>, String)>), // (id, bytes, format) loaded from DB
    NoteMigrated(String, Vec<(String, Vec<u8>, String)>), // (cleaned_body, loaded_images)
    PasteDone,

    FileDropped(std::path::PathBuf),
    FileSaved(String, String, usize), // (file_id, filename, size) — after async encrypt+store
    FileExport(String, String),       // (file_id, filename) — quick save to Downloads
    FileExportAs(String, String),     // (file_id, filename) — save with file picker
    FileExportSelected,               // export all selected file notes
    FileExported(Option<String>),     // transfer_id to remove from toasts
    FileDelete(String),               // file_id — remove attachment
    FileDeleted(String),              // file_id — confirmed deleted
    FileProgress(f32, String),        // (0.0-1.0, label) progress update

    OpenNewWindow,
    WindowFocused(window::Id),
    WindowCloseRequested(window::Id),
    WindowClosed(window::Id),
}

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
    line_editor: crate::ui::line_editor::LineEditorState,
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
            include_bytes!("../assets/logo.png"),
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
            line_editor: crate::ui::line_editor::LineEditorState::from_body(""),
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
                    Some(Message::DragEnd)
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let had_dialog = self.active_dialog.is_some();

        // passthrough: these messages should not close context menus
        match &message {
            Message::ToggleContextMenu(_) | Message::CloseContextMenu
            | Message::AnimationTick | Message::LoadingTick
            | Message::AutoSaveTick(_) | Message::CursorMoved(_, _, _)
            | Message::CanvasCloseCtxMenu | Message::SetNoteColor(_, _) | Message::SetFolderColor(_, _)
            | Message::ToggleColorSubmenu(_) | Message::ToggleFolderColorSubmenu(_) | Message::OpenMoveFolderPicker(_) | Message::ColorPickerHue(_)
            | Message::OpenColorSubmenu(_) | Message::OpenFolderColorSubmenu(_) | Message::OpenMoveSubmenu(_) | Message::OpenNewNoteSubmenu(_) | Message::ToggleNewNoteSubmenu(_) | Message::CloseSubmenus
            | Message::ColorPickerSat(_) | Message::ColorPickerLit(_)
            | Message::ColorPickerSVChanged(_, _) | Message::ColorPickerPreset(_)
            | Message::RenameNoteChanged(_) | Message::RenameNoteSubmit
            | Message::RenameFolderChanged(_) | Message::RenameFolderSubmit
            | Message::CancelRename | Message::NotePasswordInputChanged(_)
            | Message::NotePasswordConfirmChanged(_)
            | Message::DragStart(_) | Message::DragEnd | Message::DropOnFolder(..)
            | Message::DragPotential(_) | Message::ReorderDrop(..)
            | Message::ReorderPreview(..) | Message::ReorderToEnd(_)
            | Message::SetSortMode(_) | Message::ToggleSortMenu
            | Message::ModifiersChanged(_, _, _) | Message::ToggleFolderSelect(_)
            | Message::WindowControlsHover(_) | Message::HoverItem(_)
            | Message::WindowResizeStart(_) | Message::WindowResized(_, _, _)
            | Message::WindowCloseRequested(_) | Message::WindowFocused(_) | Message::WindowClosed(_)
            | Message::CopyField(_, _) | Message::CopiedFeedbackClear | Message::ClearCopiedBlockFeedback
            | Message::ImageDropped(_, _) | Message::InsertImageData(_, _) | Message::ImagesLoaded(_) | Message::NoteMigrated(_, _)
            | Message::FileDropped(_) | Message::FileSaved(_, _, _) | Message::FileExported(_) | Message::FileDeleted(_) | Message::FileProgress(_, _)
            | Message::CopySelected | Message::PasteItems | Message::PasteDone
            | Message::LineClicked(_) | Message::LineRightClicked(_) | Message::LineEditorAction(_, _)
            | Message::LineInputChanged(_, _) | Message::LineInputSubmit(_) | Message::LineBlur
            | Message::LineArrowUp | Message::LineArrowDown | Message::FocusActiveLine
            | Message::MdEdit(_)
            | Message::None | Message::ZoomIn | Message::ZoomOut | Message::ZoomReset
            | Message::CanvasCardEdit(_, _) | Message::CanvasCardFocus(_) | Message::CanvasCardUnfocus
            | Message::CanvasPan(_, _) | Message::CanvasZoom(_, _, _) | Message::CanvasViewportSize(_, _) | Message::CanvasHover(_)
            | Message::CanvasOpenColorPicker(_) | Message::CanvasShowCtxMenu(_, _, _) | Message::CanvasCloseCtxMenu
            | Message::CanvasMoveNode(_, _, _) | Message::CanvasMoveNodeGroup(_) | Message::CanvasResizeNode(_, _, _, _, _)
            | Message::CanvasSelect(_) | Message::CanvasMultiSelect(_) | Message::CanvasSelectEdge(_)
            | Message::CanvasAddEdge(_, _, _, _) | Message::CanvasDeleteSelected | Message::CanvasAddNode(_, _) | Message::CanvasAddNodeCenter
            | Message::CanvasUndo | Message::CanvasRedo => {}
            _ => {
                self.context_menu = None; self.color_submenu_for = None; self.move_submenu_for = None; self.new_note_submenu_for = None; self.toolbar_move_open = false; self.potential_drag = None; self.hovered_item = None;
                // auto-submit active rename on unrelated actions
                let should_submit = match &message {
                    Message::SelectNote(id) => self.renaming_note != Some(*id),
                    Message::Refresh | Message::DataLoaded(..) | Message::NoteLoaded(_) => false,
                    Message::CreateQuickNote(_) | Message::CreateNoteInFolder(_, _)
                    | Message::CreateQuickFolder(_) | Message::CreateNote | Message::CreateEncryptedNote => false,
                    _ => true,
                };
                if should_submit && self.rename_pending == 0 && (self.renaming_note.is_some() || self.renaming_folder.is_some()) {
                    let submit_task = if self.renaming_note.is_some() {
                        self.update(Message::RenameNoteSubmit)
                    } else {
                        self.update(Message::RenameFolderSubmit)
                    };
                    let main_task = self.update(message);
                    return Task::batch([submit_task, main_task]);
                }
            }
        }

        let result = match message {
            #[allow(unreachable_code)]
            Message::WindowClose => {
                if self.other_windows.is_empty() {
                    let _ = self.maybe_save();
                    std::process::exit(0);
                    Task::none() // unreachable but satisfies type checker
                } else {
                    let closing = self.focused_window;
                    let next_id = *self.other_windows.keys().next().unwrap();
                    let loaded = self.other_windows.remove(&next_id).unwrap();
                    self.restore_window_state(loaded);
                    self.focused_window = next_id;
                    window::close(closing)
                }
            }
            Message::WindowMinimize => {
                window::minimize(self.focused_window, true)
            }
            Message::WindowMaximize => {
                self.is_maximized = !self.is_maximized;
                window::toggle_maximize(self.focused_window)
            }
            Message::WindowControlsHover(hovered) => {
                if self.cursor_window == self.focused_window {
                    self.window_controls_hovered = hovered;
                }
                Task::none()
            }
            Message::WindowResizeStart(edge) => {
                self.resizing = Some(edge);
                Task::none()
            }
            Message::WindowResized(wid, w, h) => {
                if wid == self.focused_window {
                    self.window_size = (w, h);
                } else if let Some(win) = self.other_windows.get_mut(&wid) {
                    win.window_size = (w, h);
                }
                Task::none()
            }
            Message::HoverItem(id) => {
                if self.cursor_window == self.focused_window {
                    self.hovered_item = id;
                }
                Task::none()
            }
            Message::WindowDrag => {
                window::drag(self.focused_window)
            }

            Message::LockVault => {
                let _ = self.maybe_save();
                self.vault_state = VaultState::Login;
                self.vault_key.take().map(|mut k| { k.iter_mut().for_each(|b| *b = 0); });
                self.password_input.clear();
                self.password_input = String::new(); // drop old allocation
                self.selected_note = None;
                self.editor_content = text_editor::Content::new();
                self.line_editor.image_cache.clear();
                self.line_editor.image_sizes.clear();
                self.show_graph = false;
                self.show_settings = false;
                self.session_decrypted.clear();
                Task::none()
            }

            Message::OpenChangeVaultPasswordDialog => {
                self.show_change_password = !self.show_change_password;
                if self.show_change_password {
                    self.vault_old_password.clear();
                    self.vault_new_password.clear();
                    self.vault_new_password_confirm.clear();
                    self.auth_error = None;
                }
                Task::none()
            }
            Message::VaultOldPasswordChanged(pw) => { self.vault_old_password = pw; self.auth_error = None; Task::none() }
            Message::VaultNewPasswordChanged(pw) => { self.vault_new_password = pw; self.auth_error = None; Task::none() }
            Message::VaultNewPasswordConfirmChanged(pw) => { self.vault_new_password_confirm = pw; self.auth_error = None; Task::none() }
            Message::SubmitChangeVaultPassword => {
                if self.vault_old_password.is_empty() { self.auth_error = Some("Enter your current password".into()); return Task::none(); }
                if self.vault_new_password.is_empty() { self.auth_error = Some("New password cannot be empty".into()); return Task::none(); }
                if self.vault_new_password != self.vault_new_password_confirm { self.auth_error = Some("New passwords do not match".into()); return Task::none(); }
                let old_pw = self.vault_old_password.clone();
                let new_pw = self.vault_new_password.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let salt = db::get_vault_salt(&conn).ok_or("No salt found")?;
                    let (nonce, ciphertext) = db::get_vault_verify(&conn).ok_or("No verification data")?;
                    let old_key = crypto::key_derivation::derive_key(old_pw.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let plaintext = crypto::encryption::decrypt(&old_key.key_bytes, &nonce, &ciphertext).map_err(|_| "Current password is incorrect".to_string())?;
                    if plaintext != b"notes-app-verify" { return Err("Current password is incorrect".to_string()); }
                    let new_salt = crypto::key_derivation::generate_salt();
                    let new_key = crypto::key_derivation::derive_key(new_pw.as_bytes(), &new_salt).map_err(|e| e.to_string())?;
                    let (new_ct, new_nonce) = crypto::encryption::encrypt(&new_key.key_bytes, b"notes-app-verify").map_err(|e| e.to_string())?;
                    db::set_vault_password(&conn, &new_salt, &new_nonce, &new_ct).map_err(|e| e.to_string())?;
                    Ok(())
                }, Message::ChangeVaultPasswordDone)
            }
            Message::ChangeVaultPasswordDone(result) => {
                match result {
                    Ok(()) => {
                        self.vault_old_password.clear();
                        self.vault_new_password.clear();
                        self.vault_new_password_confirm.clear();
                        self.auth_error = None;
                        self.show_change_password = false;
                    }
                    Err(e) => { self.auth_error = Some(e); }
                }
                Task::none()
            }

            Message::PasswordInputChanged(pw) => { self.password_input = pw; self.auth_error = None; Task::none() }
            Message::ConfirmPasswordInputChanged(pw) => { self.confirm_password_input = pw; self.auth_error = None; Task::none() }
            Message::SubmitSetup => {
                if self.password_input.is_empty() { self.auth_error = Some("Password cannot be empty".into()); return Task::none(); }
                if self.password_input != self.confirm_password_input { self.auth_error = Some("Passwords do not match".into()); return Task::none(); }
                self.vault_state = VaultState::Loading;
                self.loading_tick = 0;
                let password = self.password_input.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let salt = crypto::key_derivation::generate_salt();
                    let key = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let (ciphertext, nonce) = crypto::encryption::encrypt(&key.key_bytes, b"notes-app-verify").map_err(|e| e.to_string())?;
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    db::set_vault_password(&conn, &salt, &nonce, &ciphertext).map_err(|e| e.to_string())?;
                    Ok(key.key_bytes)
                }, Message::SetupDone)
            }
            Message::SetupDone(result) => {
                match result {
                    Ok(key) => { self.vault_state = VaultState::Loading; self.vault_key = Some(key); self.password_input = String::new(); self.confirm_password_input = String::new(); self.auth_error = None; return self.refresh_data(); }
                    Err(e) => { self.vault_state = VaultState::Setup; self.auth_error = Some(e); }
                }
                Task::none()
            }
            Message::SubmitLogin => {
                if self.password_input.is_empty() { self.auth_error = Some("Password cannot be empty".into()); return Task::none(); }
                self.vault_state = VaultState::Loading;
                self.loading_tick = 0;
                let password = self.password_input.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let salt = db::get_vault_salt(&conn).ok_or("No salt found")?;
                    let (nonce, ciphertext) = db::get_vault_verify(&conn).ok_or("No verification data")?;
                    let key = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let plaintext = crypto::encryption::decrypt(&key.key_bytes, &nonce, &ciphertext).map_err(|_| "Wrong password".to_string())?;
                    if plaintext == b"notes-app-verify" { Ok(key.key_bytes) } else { Err("Wrong password".to_string()) }
                }, Message::LoginDone)
            }
            Message::LoginDone(result) => {
                match result {
                    Ok(key) => { self.vault_state = VaultState::Loading; self.vault_key = Some(key); self.password_input = String::new(); self.auth_error = None; return self.refresh_data(); }
                    Err(e) => { self.vault_state = VaultState::Login; self.auth_error = Some(e); }
                }
                Task::none()
            }

            Message::SelectView(view) => {
                if self.renaming_note.is_some() { let _ = self.update(Message::RenameNoteSubmit); }
                if self.renaming_folder.is_some() { let _ = self.update(Message::RenameFolderSubmit); }
                let save_task = self.maybe_save();
                if let Some(ref n) = self.selected_note {
                    let key = Self::view_key(&self.active_view);
                    self.last_note_per_view.insert(key.clone(), n.id);
                    self.save_last_note(&self.active_view.clone(), n.id);
                }
                let restore_id = self.last_note_per_view.get(&Self::view_key(&view)).copied();
                self.active_view = view;
                self.show_graph = false;
                self.show_settings = false;
                self.multi_selected.clear();
                self.multi_selected_folders.clear();
                // sync load avoids flicker during view switch
                self.refresh_data_sync();
                if let Some(note_id) = restore_id {
                    if let Ok(conn) = db::open_connection(&self.db_path) {
                        if let Ok(Some(note)) = db::notes::get_note(&conn, note_id) {
                            self.editor_title = note.title.clone();
                            if !note.is_encrypted {
                                match note.note_type {
                                    NoteType::Password => {
                                        self.password_data = PasswordData::from_json(&note.body);
                                        self.password_notes_content = text_editor::Content::with_text(&self.password_data.notes);
                                        self.show_password = false;
                                    }
                                    NoteType::Canvas => { self.canvas_editor.load(&note.body); }
                                    _ => {}
                                }
                            }
                            self.editor_content = if !note.is_encrypted && note.note_type == NoteType::Text {
                                text_editor::Content::with_text(&note.body)
                            } else {
                                text_editor::Content::new()
                            };
                            if !note.is_encrypted && note.note_type == NoteType::Text {
                                if body_needs_image_migration(&note.body) {
                                    let body = note.body.clone();
                                    let db_path = self.db_path.clone();
                                    let note_id = note.id;
                                    let Some(vk) = self.vault_key else { return Task::none() };
                                    self.editor_dirty = false;
                                    self.selected_note = Some(note);
                                    return Task::batch([save_task, Task::perform(async move {
                                        let (cleaned, to_migrate) = migrate_body_images(&body);
                                        let mut loaded = Vec::new();
                                        if let Ok(conn) = db::open_connection(&db_path) {
                                            for (id, fmt, b64) in &to_migrate {
                                                if let Ok(bytes) = BASE64.decode(b64) {
                                                    let _ = db::save_image_encrypted(&conn, id, &bytes, fmt, &vk);
                                                    loaded.push((id.clone(), bytes, fmt.clone()));
                                                }
                                            }
                                            let _ = conn.execute("UPDATE notes SET body = ? WHERE id = ?", rusqlite::params![cleaned, note_id.to_string()]);
                                        }
                                        (cleaned, loaded)
                                    }, |(body, imgs)| Message::NoteMigrated(body, imgs))]);
                                } else {
                                    self.line_editor = crate::ui::line_editor::LineEditorState::from_body(&note.body);
                                    let (img_ids, _) = self.line_editor.collect_images();
                                    if !img_ids.is_empty() {
                                        let db_path = self.db_path.clone();
                                        let Some(vk) = self.vault_key else { return Task::none() };
                                        let load_task = Task::perform(async move {
                                            let mut r = Vec::new();
                                            if let Ok(conn) = db::open_connection(&db_path) {
                                                for id in &img_ids { if let Some((d, f)) = db::load_image_encrypted(&conn, id, &vk) { r.push((id.clone(), d, f)); } }
                                            }
                                            r
                                        }, Message::ImagesLoaded);
                                        self.editor_dirty = false;
                                        self.selected_note = Some(note);
                                        return Task::batch([save_task, load_task]);
                                    }
                                }
                            }
                            self.editor_dirty = false;
                            self.selected_note = Some(note);
                        }
                    }
                }
                save_task
            }
            Message::SelectNote(id) => {
                if let Some(ren_id) = self.renaming_note {
                    if ren_id != id {
                        let task = self.update(Message::RenameNoteSubmit);
                        let select_task = self.update(Message::SelectNote(id));
                        return Task::batch([task, select_task]);
                    }
                }
                if self.renaming_folder.is_some() {
                    let task = self.update(Message::RenameFolderSubmit);
                    let select_task = self.update(Message::SelectNote(id));
                    return Task::batch([task, select_task]);
                }

                if self.alt_held {
                    return self.update(Message::OpenNewWindow);
                }

                if self.ctrl_held {
                    if self.multi_selected.contains(&id) {
                        self.multi_selected.remove(&id);
                    } else {
                        self.multi_selected.insert(id);
                    }
                    self.last_clicked_note = Some(id);
                    return Task::none();
                } else if self.shift_held {
                    let visible = self.visible_item_ids();
                    if let Some(last) = self.last_clicked_note {
                        let pos_a = visible.iter().position(|(x, _)| *x == last);
                        let pos_b = visible.iter().position(|(x, _)| *x == id);
                        if let (Some(a), Some(b)) = (pos_a, pos_b) {
                            let (start, end) = if a < b { (a, b) } else { (b, a) };
                            for i in start..=end {
                                let (item_id, is_folder) = visible[i];
                                if is_folder {
                                    self.multi_selected_folders.insert(item_id);
                                } else {
                                    self.multi_selected.insert(item_id);
                                }
                            }
                        }
                    }
                    self.last_clicked_note = Some(id);
                    return Task::none();
                } else {
                    self.multi_selected.clear();
                    self.multi_selected_folders.clear();
                    self.last_clicked_note = Some(id);
                }

                self.show_graph = false;
                self.show_settings = false;
                self.page_anim = 0.0;
                let key = Self::view_key(&self.active_view);
                self.last_note_per_view.insert(key.clone(), id);
                self.save_last_note(&self.active_view.clone(), id);
                let save_task = self.maybe_save();
                let db_path = self.db_path.clone();
                let load_task = Task::perform(
                    async move { let conn = db::open_connection(&db_path).ok()?; db::notes::get_note(&conn, id).ok()? },
                    Message::NoteLoaded,
                );
                Task::batch([save_task, load_task])
            }
            Message::SearchQueryChanged(query) => { self.search_query = query; self.line_editor.focused = false; self.refresh_data() }

            Message::ToggleContextMenu(menu) => {
                if self.context_menu.as_ref() == Some(&menu) {
                    self.context_menu = None;
                    self.color_submenu_for = None;
                    self.move_submenu_for = None;
                } else {
                    if !matches!(&menu, ContextMenu::NoteColor(_)) {
                        self.context_menu_pos = self.cursor_pos;
                    }
                    self.ctx_menu_anim = 0.0;
                    self.color_submenu_for = None;
                    self.move_submenu_for = None;
                    self.context_menu = Some(menu);
                }
                Task::none()
            }
            Message::CloseContextMenu => {
                self.context_menu = None;
                self.color_submenu_for = None;
                self.move_submenu_for = None;
                Task::none()
            }

            Message::OpenCreateNoteDialog => {
                self.create_dialog_title.clear();
                self.create_dialog_type = NoteType::Text;
                self.create_dialog_color = FolderColor::Green;
                self.create_dialog_folder = match &self.active_view {
                    ActiveView::Folder(id) => Some(*id),
                    _ => None,
                };
                self.active_dialog = Some(DialogKind::CreateNote);
                Task::none()
            }
            Message::CreateDialogTitleChanged(t) => { self.create_dialog_title = t; Task::none() }
            Message::CreateDialogTypeChanged(t) => { self.create_dialog_type = t; Task::none() }
            Message::CreateDialogColorChanged(c) => { self.create_dialog_color = c; Task::none() }
            Message::CreateDialogFolderChanged(f) => { self.create_dialog_folder = f; Task::none() }
            Message::SubmitCreateNote => {
                self.active_dialog = None;
                let save_task = self.maybe_save();
                let mut note = Note::new(self.create_dialog_folder, self.create_dialog_color, self.create_dialog_type);
                note.title = self.create_dialog_title.clone();
                let note_id = note.id;
                let db_path = self.db_path.clone();
                let create_task = Task::perform(
                    async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::insert_note(&conn, &note); } },
                    move |_| Message::SelectNote(note_id),
                );
                Task::batch([save_task, create_task])
            }

            Message::CreateNoteInFolder(note_type, folder_id) => {
                let save_task = self.maybe_save();
                let color = self.folders.iter().find(|f| f.id == folder_id).map(|f| f.color).unwrap_or(FolderColor::Green);
                let note = Note::new(Some(folder_id), color, note_type);
                let note_id = note.id;
                self.renaming_note = Some(note_id);
                self.rename_buffer = String::new();
                self.rename_pending = 10;
                self.expanded_folders.insert(folder_id); // auto-expand the target folder
                let db_path = self.db_path.clone();
                let create_task = Task::perform(
                    async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::insert_note(&conn, &note); } },
                    move |_| Message::SelectNote(note_id),
                );
                Task::batch([save_task, create_task])
            }
            Message::CreateQuickNote(note_type) => {
                let save_task = self.maybe_save();
                let folder_id = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
                let color = folder_id.and_then(|fid| self.folders.iter().find(|f| f.id == fid)).map(|f| f.color).unwrap_or(FolderColor::Green);
                let mut note = Note::new(folder_id, color, note_type);
                if matches!(self.active_view, ActiveView::Favorites) { note.is_favorite = true; }
                let note_id = note.id;
                self.renaming_note = Some(note_id);
                self.rename_buffer = String::new();
                self.rename_pending = 10;
                let db_path = self.db_path.clone();
                let create_task = Task::perform(
                    async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::insert_note(&conn, &note); } },
                    move |_| Message::SelectNote(note_id),
                );
                Task::batch([save_task, create_task])
            }

            Message::CreateNote | Message::CreateEncryptedNote => {
                let save_task = self.maybe_save();
                let folder_id = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
                let color = folder_id.and_then(|fid| self.folders.iter().find(|f| f.id == fid)).map(|f| f.color).unwrap_or(FolderColor::Green);
                let is_encrypted = matches!(message, Message::CreateEncryptedNote);
                let mut note = Note::new(folder_id, color, NoteType::Text);
                if is_encrypted { note.is_encrypted = true; }
                let note_id = note.id;
                self.renaming_note = Some(note_id);
                self.rename_buffer = String::new();
                self.rename_pending = 10;
                let db_path = self.db_path.clone();
                let create_task = Task::perform(
                    async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::insert_note(&conn, &note); } },
                    move |_| Message::SelectNote(note_id),
                );
                Task::batch([save_task, create_task])
            }

            Message::DeleteNote(id) => {
                self.active_dialog = None;
                let file_id = self.selected_note.as_ref()
                    .filter(|n| n.id == id && n.note_type == NoteType::File)
                    .and_then(|n| crate::ui::file_viewer::parse_file_body(&n.body))
                    .map(|(fid, _, _)| fid);
                if self.selected_note.as_ref().map_or(false, |n| n.id == id) {
                    self.selected_note = None;
                    self.editor_title.clear();
                    self.editor_content = text_editor::Content::new();
                    self.editor_dirty = false;
                }
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        if let Some(ref fid) = file_id {
                            let _ = db::delete_file(&conn, fid);
                        }
                        let _ = db::notes::delete_note(&conn, id);
                    }
                }, |_| Message::Refresh)
            }
            Message::ToggleFavorite(id) => {
                if let Some(ref mut n) = self.selected_note { if n.id == id { n.is_favorite = !n.is_favorite; } }
                let db_path = self.db_path.clone();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::toggle_favorite(&conn, id); } }, |_| Message::Refresh)
            }
            Message::TogglePin(id) => {
                if let Some(ref mut n) = self.selected_note { if n.id == id { n.is_pinned = !n.is_pinned; } }
                let db_path = self.db_path.clone();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::toggle_pin(&conn, id); } }, |_| Message::Refresh)
            }
            Message::OpenNoteColorDialog(id) => {
                if let Some(preview) = self.notes.iter().find(|n| n.id == id) {
                    let c = preview.color.to_iced_color();
                    let max = c.r.max(c.g).max(c.b);
                    let min = c.r.min(c.g).min(c.b);
                    let d = max - min;
                    let v = max;
                    let s = if max > 0.0 { d / max } else { 0.0 };
                    let h = if d < 0.001 { 0.0 } else if (max - c.r).abs() < 0.001 {
                        60.0 * (((c.g - c.b) / d) % 6.0)
                    } else if (max - c.g).abs() < 0.001 {
                        60.0 * ((c.b - c.r) / d + 2.0)
                    } else {
                        60.0 * ((c.r - c.g) / d + 4.0)
                    };
                    self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                    self.color_sat = s * 100.0;
                    self.color_lit = v * 100.0;
                }
                self.active_dialog = Some(DialogKind::NoteColor(id));
                Task::none()
            }
            Message::ApplyNoteColor(id) => {
                self.active_dialog = None;
                let c = crate::ui::color_picker::hsv_to_rgb(self.color_hue, self.color_sat / 100.0, self.color_lit / 100.0);
                let best = FolderColor::PALETTE.iter().copied().min_by(|a, b| {
                    let ac = a.to_iced_color();
                    let bc = b.to_iced_color();
                    let da = (ac.r-c.r).powi(2) + (ac.g-c.g).powi(2) + (ac.b-c.b).powi(2);
                    let db = (bc.r-c.r).powi(2) + (bc.g-c.g).powi(2) + (bc.b-c.b).powi(2);
                    da.partial_cmp(&db).unwrap()
                }).unwrap_or(FolderColor::Green);
                if let Some(ref mut n) = self.selected_note { if n.id == id { n.color = best; } }
                let db_path = self.db_path.clone();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::update_note_color(&conn, id, best); } }, |_| Message::Refresh)
            }
            Message::FocusTitle => {
                iced::widget::text_input::focus(iced::widget::text_input::Id::new("note_title"))
            }
            Message::OpenColorSubmenu(id) => {
                self.color_submenu_for = Some(id);
                self.color_submenu_is_folder = false;
                self.move_submenu_for = None;
                self.new_note_submenu_for = None;
                Task::none()
            }
            Message::ToggleColorSubmenu(id) => {
                self.color_submenu_for = if self.color_submenu_for == Some(id) { None } else { Some(id) };
                self.color_submenu_is_folder = false;
                self.move_submenu_for = None;
                Task::none()
            }
            Message::OpenFolderColorSubmenu(id) => {
                self.color_submenu_for = Some(id);
                self.color_submenu_is_folder = true;
                self.move_submenu_for = None;
                self.new_note_submenu_for = None;
                if let Some(f) = self.folders.iter().chain(self.subfolders.iter()).find(|f| f.id == id) {
                    let c = f.color.to_iced_color();
                    let max = c.r.max(c.g).max(c.b);
                    let min = c.r.min(c.g).min(c.b);
                    let d = max - min;
                    let h = if d == 0.0 { 0.0 } else if max == c.r { 60.0 * (((c.g - c.b) / d) % 6.0) } else if max == c.g { 60.0 * (((c.b - c.r) / d) + 2.0) } else { 60.0 * (((c.r - c.g) / d) + 4.0) };
                    let s = if max == 0.0 { 0.0 } else { d / max };
                    self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                    self.color_sat = s * 100.0;
                    self.color_lit = max * 100.0;
                }
                Task::none()
            }
            Message::OpenMoveSubmenu(id) => {
                self.move_submenu_for = Some(id);
                self.color_submenu_for = None;
                self.new_note_submenu_for = None;
                Task::none()
            }
            Message::ToggleFolderColorSubmenu(id) => {
                self.color_submenu_for = if self.color_submenu_for == Some(id) { None } else { Some(id) };
                self.color_submenu_is_folder = true;
                self.new_note_submenu_for = None;
                if let Some(f) = self.folders.iter().chain(self.subfolders.iter()).find(|f| f.id == id) {
                    let c = f.color.to_iced_color();
                    let max = c.r.max(c.g).max(c.b);
                    let min = c.r.min(c.g).min(c.b);
                    let d = max - min;
                    let h = if d == 0.0 { 0.0 } else if max == c.r { 60.0 * (((c.g - c.b) / d) % 6.0) } else if max == c.g { 60.0 * (((c.b - c.r) / d) + 2.0) } else { 60.0 * (((c.r - c.g) / d) + 4.0) };
                    let s = if max == 0.0 { 0.0 } else { d / max };
                    self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                    self.color_sat = s * 100.0;
                    self.color_lit = max * 100.0;
                }
                Task::none()
            }
            Message::OpenNewNoteSubmenu(id) => {
                self.new_note_submenu_for = Some(id);
                self.color_submenu_for = None;
                self.move_submenu_for = None;
                Task::none()
            }
            Message::ToggleNewNoteSubmenu(id) => {
                self.new_note_submenu_for = if self.new_note_submenu_for == Some(id) { None } else { Some(id) };
                self.color_submenu_for = None;
                self.move_submenu_for = None;
                Task::none()
            }
            Message::ApplyFolderColor(id) => {
                self.active_dialog = None;
                self.color_submenu_for = None;
                self.context_menu = None;
                let c = crate::ui::color_picker::hsv_to_rgb(self.color_hue, self.color_sat / 100.0, self.color_lit / 100.0);
                let best = FolderColor::PALETTE.iter().copied().min_by(|a, b| {
                    let ac = a.to_iced_color(); let bc = b.to_iced_color();
                    let da = (ac.r-c.r).powi(2) + (ac.g-c.g).powi(2) + (ac.b-c.b).powi(2);
                    let db = (bc.r-c.r).powi(2) + (bc.g-c.g).powi(2) + (bc.b-c.b).powi(2);
                    da.partial_cmp(&db).unwrap()
                }).unwrap_or(FolderColor::Blue);
                return self.update(Message::SetFolderColor(id, best));
            }
            Message::RenameNote(id) => {
                let title = self.notes.iter().find(|n| n.id == id).map(|n| n.title.clone()).unwrap_or_default();
                self.rename_buffer = if title.is_empty() { String::new() } else { title };
                self.renaming_note = Some(id);
                self.rename_pending = 10;
                self.line_editor.focused = false; // unfocus editor to prevent double input
                Task::none()
            }
            Message::RenameNoteChanged(new_name) => {
                self.rename_buffer = new_name;
                Task::none()
            }
            Message::ToggleFolderSelect(id) => {
                if self.renaming_note.is_some() {
                    let task = self.update(Message::RenameNoteSubmit);
                    let folder_task = self.update(Message::ToggleFolderSelect(id));
                    return Task::batch([task, folder_task]);
                }
                if self.renaming_folder.is_some() && self.renaming_folder != Some(id) {
                    let task = self.update(Message::RenameFolderSubmit);
                    let folder_task = self.update(Message::ToggleFolderSelect(id));
                    return Task::batch([task, folder_task]);
                }
                if self.ctrl_held {
                    if self.multi_selected_folders.contains(&id) {
                        self.multi_selected_folders.remove(&id);
                    } else {
                        self.multi_selected_folders.insert(id);
                    }
                    self.last_clicked_note = Some(id);
                    return Task::none();
                } else if self.shift_held {
                    let visible = self.visible_item_ids();
                    if let Some(last) = self.last_clicked_note {
                        let pos_a = visible.iter().position(|(x, _)| *x == last);
                        let pos_b = visible.iter().position(|(x, _)| *x == id);
                        if let (Some(a), Some(b)) = (pos_a, pos_b) {
                            let (start, end) = if a < b { (a, b) } else { (b, a) };
                            for i in start..=end {
                                let (item_id, is_folder) = visible[i];
                                if is_folder {
                                    self.multi_selected_folders.insert(item_id);
                                } else {
                                    self.multi_selected.insert(item_id);
                                }
                            }
                        }
                    }
                    self.last_clicked_note = Some(id);
                    return Task::none();
                } else {
                    self.multi_selected_folders.clear();
                    self.multi_selected.clear();
                    self.last_clicked_note = Some(id);
                    if self.expanded_folders.contains(&id) {
                        self.expanded_folders.remove(&id);
                    } else {
                        self.expanded_folders.insert(id);
                    }
                }
                Task::none()
            }
            Message::OpenDeleteMultiDialog => {
                if self.multi_selected.is_empty() && self.multi_selected_folders.is_empty() { return Task::none(); }
                // skip confirmation dialog for empty items
                let all_notes_empty = self.multi_selected.iter().all(|id| {
                    self.notes.iter().find(|n| n.id == *id).map_or(true, |n| n.title.is_empty() && n.snippet.is_empty())
                });
                let all_folders_empty = self.multi_selected_folders.iter().all(|id| {
                    let note_count = self.folder_counts.iter().find(|(fid, _)| fid == id).map(|(_, c)| *c).unwrap_or(0);
                    let subfolder_count = self.subfolders.iter().filter(|f| f.parent_id == Some(*id)).count();
                    note_count == 0 && subfolder_count == 0
                });
                if all_notes_empty && all_folders_empty {
                    return self.update(Message::DeleteMultiSelected);
                }
                self.active_dialog = Some(DialogKind::DeleteMultiConfirm);
                Task::none()
            }
            Message::DeleteMultiSelected => {
                self.active_dialog = None;
                if self.multi_selected.is_empty() && self.multi_selected_folders.is_empty() { return Task::none(); }
                let ids: Vec<Uuid> = self.multi_selected.drain().collect();
                let db_path = self.db_path.clone();
                if let Some(ref n) = self.selected_note {
                    if ids.contains(&n.id) {
                        self.selected_note = None;
                        self.editor_content = text_editor::Content::new();
                        self.editor_dirty = false;
                    }
                }
                let folder_ids: Vec<Uuid> = self.multi_selected_folders.drain().collect();
                return Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        for id in &ids {
                            let _ = db::notes::delete_note(&conn, *id);
                        }
                        for id in &folder_ids {
                            let _ = db::folders::delete_folder(&conn, *id);
                        }
                    }
                }, |_| Message::Refresh);
            }
            Message::MoveMultiSelectedToFolder(folder_id) => {
                if self.multi_selected.is_empty() && self.multi_selected_folders.is_empty() { return Task::none(); }
                let ids: Vec<Uuid> = self.multi_selected.drain().collect();
                let folder_ids: Vec<Uuid> = self.multi_selected_folders.drain().collect();
                let db_path = self.db_path.clone();
                return Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        for id in &ids {
                            let _ = db::notes::move_to_folder(&conn, *id, folder_id);
                        }
                        for id in &folder_ids {
                            if Some(*id) != folder_id {
                                let _ = db::folders::reparent_folder(&conn, *id, folder_id);
                            }
                        }
                    }
                }, |_| Message::Refresh);
            }
            Message::CreateQuickFolder(parent_id) => {
                let parent_id = parent_id.or_else(|| match &self.active_view {
                    ActiveView::Folder(id) => Some(*id),
                    _ => None,
                });
                if parent_id.is_none() { return Task::none(); }
                let color = parent_id.and_then(|pid| self.folders.iter().find(|f| f.id == pid)).map(|f| f.color).unwrap_or(FolderColor::Blue);
                let mut folder = Folder::new(String::from("New folder"), color, parent_id);
                let folder_id = folder.id;
                self.renaming_folder = Some(folder_id);
                self.folder_rename_buffer = String::new();
                self.rename_pending = 10;
                if let Some(pid) = parent_id {
                    self.expanded_folders.insert(pid);
                }
                let db_path = self.db_path.clone();
                return Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) { let _ = db::folders::insert_folder(&conn, &folder); }
                }, |_| Message::Refresh);
            }
            Message::RenameFolderInline(id) => {
                let name = self.folders.iter().find(|f| f.id == id).map(|f| f.name.clone()).unwrap_or_default();
                self.folder_rename_buffer = name;
                self.renaming_folder = Some(id);
                self.rename_pending = 10;
                self.line_editor.focused = false;
                Task::none()
            }
            Message::RenameFolderChanged(name) => {
                self.folder_rename_buffer = name;
                Task::none()
            }
            Message::RenameFolderSubmit => {
                if let Some(id) = self.renaming_folder.take() {
                    let new_name = self.folder_rename_buffer.clone();
                    self.folder_rename_buffer.clear();
                    if new_name.trim().is_empty() { return Task::none(); }
                    // update in-memory to avoid flash before db write
                    for f in &mut self.folders { if f.id == id { f.name = new_name.clone(); } }
                    for f in &mut self.subfolders { if f.id == id { f.name = new_name.clone(); } }
                    let db_path = self.db_path.clone();
                    return Task::perform(async move {
                        if let Ok(conn) = db::open_connection(&db_path) {
                            if let Ok(folders) = db::folders::list_folders(&conn) {
                                if let Some(mut f) = folders.into_iter().find(|f| f.id == id) {
                                    f.name = new_name;
                                    let _ = db::folders::update_folder(&conn, &f);
                                }
                            }
                        }
                    }, |_| Message::Refresh);
                }
                Task::none()
            }
            Message::CancelRename => {
                if self.renaming_note.is_some() {
                    self.renaming_note = None;
                    self.rename_buffer.clear();
                } else if self.renaming_folder.is_some() {
                    self.renaming_folder = None;
                    self.folder_rename_buffer.clear();
                } else if self.active_dialog.is_some() {
                    self.active_dialog = None;
                    self.note_password_input.clear();
                } else if self.editor_search_open {
                    self.editor_search_open = false;
                    self.line_editor.search_matches.clear();
                    self.line_editor.current_match = 0;
                    self.editor_search_index = 0;
                } else if self.context_menu.is_some() {
                    self.context_menu = None;
                    self.color_submenu_for = None;
                } else if self.line_editor.focused {
                    self.line_editor.focused = false;
                }
                Task::none()
            }
            Message::RenameNoteSubmit => {
                if let Some(id) = self.renaming_note.take() {
                    let new_title = if self.rename_buffer.is_empty() {
                        self.notes.iter().find(|n| n.id == id).map(|n| n.title.clone())
                            .or_else(|| self.selected_note.as_ref().filter(|n| n.id == id).map(|n| n.title.clone()))
                            .unwrap_or_default()
                    } else {
                        self.rename_buffer.clone()
                    };
                    self.rename_buffer.clear();
                    if let Some(ref mut n) = self.selected_note { if n.id == id { n.title = new_title.clone(); } }
                    for preview in &mut self.notes { if preview.id == id { preview.title = new_title.clone(); break; } }
                    for (_, sns) in &mut self.subfolder_notes { for p in sns { if p.id == id { p.title = new_title.clone(); } } }
                    self.editor_title = new_title.clone();
                    if let Ok(conn) = db::open_connection(&self.db_path) {
                        let _ = db::notes::rename_note(&conn, id, &new_title);
                    }
                    return self.refresh_data();
                }
                Task::none()
            }
            Message::SetNoteColor(id, color) => {
                if let Some(ref mut n) = self.selected_note { if n.id == id { n.color = color; } }
                let db_path = self.db_path.clone();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::update_note_color(&conn, id, color); } }, |_| Message::Refresh)
            }
            Message::SetFolderColor(id, color) => {
                for f in &mut self.folders { if f.id == id { f.color = color; } }
                for f in &mut self.subfolders { if f.id == id { f.color = color; } }
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        if let Ok(folders) = db::folders::list_folders(&conn) {
                            if let Some(mut f) = folders.into_iter().find(|f| f.id == id) {
                                f.color = color;
                                let _ = db::folders::update_folder(&conn, &f);
                            }
                        }
                    }
                }, |_| Message::Refresh)
            }
            Message::ToggleFolderFavorite(id) => {
                for f in &mut self.folders { if f.id == id { f.is_favorite = !f.is_favorite; } }
                for f in &mut self.subfolders { if f.id == id { f.is_favorite = !f.is_favorite; } }
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) { let _ = db::folders::toggle_folder_favorite(&conn, id); }
                }, |_| Message::Refresh)
            }
            Message::ColorPickerPreset(color) => {
                self.create_dialog_color = color;
                let c = color.to_iced_color();
                let max = c.r.max(c.g).max(c.b);
                let min = c.r.min(c.g).min(c.b);
                let d = max - min;
                let v = max;
                let s = if max > 0.0 { d / max } else { 0.0 };
                let h = if d < 0.001 { 0.0 } else if (max - c.r).abs() < 0.001 {
                    60.0 * (((c.g - c.b) / d) % 6.0)
                } else if (max - c.g).abs() < 0.001 {
                    60.0 * ((c.b - c.r) / d + 2.0)
                } else {
                    60.0 * ((c.r - c.g) / d + 4.0)
                };
                self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                self.color_sat = s * 100.0;
                self.color_lit = v * 100.0;
                self.auto_apply_color()
            }
            Message::ColorPickerHue(h) => { self.color_hue = h; self.auto_apply_color() }
            Message::ColorPickerSat(s) => { self.color_sat = s; self.auto_apply_color() }
            Message::ColorPickerLit(l) => { self.color_lit = l; self.auto_apply_color() }
            Message::ColorPickerSVChanged(s, v) => { self.color_sat = s; self.color_lit = v; self.auto_apply_color() }
            Message::MoveNoteToFolder(item_id, folder_id) => {
                self.active_dialog = None;
                self.toolbar_move_open = false;
                self.context_menu = None;
                self.move_submenu_for = None;
                let is_folder = self.folders.iter().chain(self.subfolders.iter()).any(|f| f.id == item_id);
                if is_folder {
                    if Some(item_id) == folder_id { return Task::none(); }
                    let db_path = self.db_path.clone();
                    Task::perform(async move {
                        if let Ok(conn) = db::open_connection(&db_path) { let _ = db::folders::reparent_folder(&conn, item_id, folder_id); }
                    }, |_| Message::Refresh)
                } else {
                    if let Some(ref mut n) = self.selected_note { if n.id == item_id { n.folder_id = folder_id; } }
                    let db_path = self.db_path.clone();
                    Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::move_to_folder(&conn, item_id, folder_id); } }, |_| Message::Refresh)
                }
            }

            Message::EditorTitleChanged(title) => {
                self.editor_title = title.clone();
                self.editor_dirty = true;
                self.line_editor.focused = false;
                self.last_edit_time = Some(Instant::now());
                if let Some(ref mut n) = self.selected_note { n.title = title.clone(); }
                if let Some(ref n) = self.selected_note {
                    for preview in &mut self.notes {
                        if preview.id == n.id { preview.title = title; break; }
                    }
                }
                self.sync_editor_to_other_windows();
                Task::none()
            }
            Message::EditorContentAction(action) => {
                let is_edit = action.is_edit();
                self.editor_content.perform(action);
                if is_edit {
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    self.sync_editor_to_other_windows();
                }
                Task::none()
            }
            Message::LineClicked(_) | Message::LineRightClicked(_) | Message::FocusActiveLine
            | Message::LineInputChanged(_, _) | Message::LineInputSubmit(_)
            | Message::LineArrowUp | Message::LineArrowDown => {
                Task::none()
            }
            Message::LineEditorAction(_i, _action) => {
                Task::none()
            }
            Message::LineBlur => { Task::none() }

            Message::MdEdit(action) => {
                if self.renaming_note.is_some() && matches!(action, crate::ui::md_widget::MdAction::Click(_, _) | crate::ui::md_widget::MdAction::Focus) {
                    let _ = self.update(Message::RenameNoteSubmit);
                }
                use crate::ui::md_widget::{MdAction, MdMotion};
                let state = &mut self.line_editor;
                match action {
                    MdAction::Click(x, y) => {
                        let (line, col) = hit_test_position(state, x, y, self.setting_font_size as f32);
                        state.cursor = (line, col);
                        state.selection = None;
                        state.focused = true;
                        state.focus_instant = Some(Instant::now());
                        state.is_dragging = true;
                        state.last_click = Some((Instant::now(), Point::new(x, y)));
                        state.click_count = 1;
                    }
                    MdAction::DoubleClick(x, y) => {
                        let (line, col) = hit_test_position(state, x, y, self.setting_font_size as f32);
                        let word_bounds = find_word_bounds(&state.lines[line.min(state.lines.len() - 1)], col);
                        state.selection = Some(((line, word_bounds.0), (line, word_bounds.1)));
                        state.cursor = (line, word_bounds.1);
                        state.click_count = 2;
                    }
                    MdAction::TripleClick(x, y) => {
                        let (line, _) = hit_test_position(state, x, y, self.setting_font_size as f32);
                        let line_len = state.lines[line.min(state.lines.len() - 1)].chars().count();
                        state.selection = Some(((line, 0), (line, line_len)));
                        state.cursor = (line, line_len);
                        state.click_count = 3;
                    }
                    MdAction::ShiftClick(x, y) => {
                        let (line, col) = hit_test_position(state, x, y, self.setting_font_size as f32);
                        if state.selection.is_none() {
                            state.selection = Some((state.cursor, (line, col)));
                        } else {
                            state.selection.as_mut().unwrap().1 = (line, col);
                        }
                        state.cursor = (line, col);
                        state.focused = true;
                        state.focus_instant = Some(Instant::now());
                    }
                    MdAction::DragTo(x, y) => {
                        let (line, col) = hit_test_position(state, x, y, self.setting_font_size as f32);
                        if state.selection.is_none() {
                            state.selection = Some((state.cursor, (line, col)));
                        } else {
                            state.selection.as_mut().unwrap().1 = (line, col);
                        }
                        state.cursor = (line, col);
                    }
                    MdAction::Release => { state.is_dragging = false; }
                    MdAction::Undo => {
                        state.undo();
                        self.editor_dirty = true;
                    }
                    MdAction::Redo => {
                        state.redo();
                        self.editor_dirty = true;
                    }
                    MdAction::Insert(c) => {
                        if c == '\x08' {
                            // \x08 sentinel means escape was pressed in slash menu
                            state.slash_menu_open = false;
                            state.slash_filter.clear();
                            state.slash_selected = 0;
                        } else {
                            state.push_undo();
                            state.insert_char(c);
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                            let (line_idx, _) = state.cursor;
                            if line_idx < state.lines.len() {
                                let line = state.lines[line_idx].trim_start().to_string();
                                if line.starts_with('/') && !line.contains(' ') {
                                    state.slash_menu_open = true;
                                    state.slash_filter = line[1..].to_string();
                                    state.slash_selected = 0;
                                } else {
                                    state.slash_menu_open = false;
                                    state.slash_filter.clear();
                                }
                            }
                        }
                    }
                    MdAction::Paste(text) => {
                        if !text.is_empty() {
                            state.push_undo();
                            state.insert_text(&text);
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        } else {
                            // text empty, check for image in clipboard
                            if let Ok(mut clip) = arboard::Clipboard::new() {
                                if let Ok(img) = clip.get_image() {
                                    let mut w = img.width;
                                    let mut h = img.height;
                                    let mut pixels: Vec<u8> = img.bytes.into_owned();

                                    let max_dim = 800usize;
                                    if w > max_dim || h > max_dim {
                                        let scale = max_dim as f32 / w.max(h) as f32;
                                        let nw = ((w as f32 * scale) as usize).max(1);
                                        let nh = ((h as f32 * scale) as usize).max(1);
                                        let mut resized = vec![0u8; nw * nh * 4];
                                        for ny in 0..nh {
                                            for nx in 0..nw {
                                                let ox = ((nx as f32 / scale) as usize).min(w - 1);
                                                let oy = ((ny as f32 / scale) as usize).min(h - 1);
                                                let si = (oy * w + ox) * 4;
                                                let di = (ny * nw + nx) * 4;
                                                if si + 4 <= pixels.len() && di + 4 <= resized.len() {
                                                    resized[di..di+4].copy_from_slice(&pixels[si..si+4]);
                                                }
                                            }
                                        }
                                        w = nw; h = nh; pixels = resized;
                                    }

                                    let id = format!("img:{}", Uuid::new_v4());
                                    state.push_undo();
                                    let (cl, _) = state.cursor;
                                    state.lines.insert(cl + 1, format!("![]({})", id));
                                    state.cursor = (cl + 2, 0);
                                    self.editor_dirty = true;
                                    self.last_edit_time = Some(Instant::now());
                                    state.image_cache.insert(id.clone(), iced::widget::image::Handle::from_rgba(w as u32, h as u32, pixels.clone()));
                                    let max_w = (state.text_area_width - 20.0).max(100.0);
                                    let scale_fit = if (w as f32) > max_w { max_w / w as f32 } else { 1.0 };
                                    state.image_sizes.insert(id.clone(), (w as f32 * scale_fit, h as f32 * scale_fit));
                                    let db_path = self.db_path.clone();
                                    let Some(vk) = self.vault_key else { return Task::none() };
                                    let fmt = format!("rgba:{}:{}", w, h);
                                    let id2 = id.clone();
                                    return Task::perform(async move {
                                        if let Ok(conn) = db::open_connection(&db_path) {
                                            let _ = db::save_image_encrypted(&conn, &id2, &pixels, &fmt, &vk);
                                        }
                                    }, |_| Message::None);
                                }
                            }
                        }
                    }
                    MdAction::Enter => {
                        let (cl, _) = state.cursor;
                        // don't split image references on enter
                        let is_img = cl < state.lines.len() && {
                            let t = state.lines[cl].trim_start();
                            t.starts_with("![") && t.contains("](img:") && t.ends_with(')')
                        };
                        if is_img {
                            state.push_undo();
                            state.lines.insert(cl + 1, String::new());
                            state.cursor = (cl + 1, 0);
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        } else {
                            let is_table = cl < state.lines.len() && {
                                let t = state.lines[cl].trim_start();
                                t.starts_with('|') && t.ends_with('|')
                            };
                            if is_table {
                            let is_last = if cl + 1 >= state.lines.len() { true } else {
                                let nt = state.lines[cl + 1].trim_start();
                                !(nt.starts_with('|') && nt.ends_with('|'))
                            };
                            if is_last {
                                // last row: exit table
                                state.push_undo();
                                state.lines.insert(cl + 1, String::new());
                                state.cursor = (cl + 1, 0);
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            } else {
                                let mut next = cl + 1;
                                if next < state.lines.len() {
                                    let nt = state.lines[next].trim_start();
                                    if nt.contains('-') && nt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') {
                                        next += 1;
                                    }
                                }
                                if next < state.lines.len() {
                                    state.cursor = (next, crate::ui::md_widget::cell_to_raw_col(&state.lines[next], 0, 0));
                                }
                            }
                            } else {
                                let line = if cl < state.lines.len() { state.lines[cl].clone() } else { String::new() };
                                let trimmed = line.trim_start();
                                let leading: String = line.chars().take(line.len() - trimmed.len()).collect();

                                let (prefix, has_content) = if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                                    ("- [ ] ", trimmed.len() > 6)
                                } else if trimmed.starts_with("- [ ] ") {
                                    ("- [ ] ", trimmed.len() > 6)
                                } else if trimmed.starts_with("- ") {
                                    ("- ", trimmed.len() > 2)
                                } else if trimmed.len() > 2 && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                                    if let Some(dot_pos) = trimmed.find(". ") {
                                        let num_str = &trimmed[..dot_pos];
                                        if num_str.chars().all(|c| c.is_ascii_digit()) {
                                            let next_num = num_str.parse::<u32>().unwrap_or(0) + 1;
                                            let has = trimmed.len() > dot_pos + 2;
                                            let _ = next_num;
                                            (&trimmed[..dot_pos + 2], has)
                                        } else { ("", false) }
                                    } else { ("", false) }
                                } else {
                                    ("", false)
                                };

                                state.push_undo();
                                if !prefix.is_empty() && has_content {
                                    let new_prefix = if trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                                        if let Some(dot_pos) = trimmed.find(". ") {
                                            let num: u32 = trimmed[..dot_pos].parse().unwrap_or(0);
                                            format!("{}{}. ", leading, num + 1)
                                        } else {
                                            format!("{}{}", leading, prefix)
                                        }
                                    } else {
                                        format!("{}{}", leading, prefix)
                                    };
                                    state.insert_newline();
                                    let new_line = state.cursor.0;
                                    state.lines[new_line] = new_prefix.clone();
                                    state.cursor.1 = new_prefix.chars().count();
                                } else if !prefix.is_empty() && !has_content {
                                    // empty list item: clear prefix instead of continuing
                                    state.lines[cl] = String::new();
                                    state.cursor = (cl, 0);
                                } else {
                                    state.insert_newline();
                                }
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            }
                        } // end else (not image)
                    }
                    MdAction::Backspace => {
                        if state.selection.is_some() {
                            state.push_undo();
                            state.delete_selection();
                            state.lines.retain(|l| {
                                let t = l.trim();
                                // remove broken image refs left over from selection delete
                                !t.starts_with("![") || (t.contains("](") && t.ends_with(')'))
                            });
                            if state.lines.is_empty() { state.lines.push(String::new()); }
                            if state.cursor.0 >= state.lines.len() { state.cursor.0 = state.lines.len() - 1; }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        } else {
                            let (line_idx, col) = state.cursor;
                        // image line: delete whole line
                        let is_image_line = line_idx < state.lines.len() && {
                            let t = state.lines[line_idx].trim_start();
                            t.starts_with("![") && t.contains("](img:") && t.ends_with(')')
                        };
                        if is_image_line {
                            state.push_undo();
                            state.lines.remove(line_idx);
                            if state.lines.is_empty() { state.lines.push(String::new()); }
                            if state.cursor.0 >= state.lines.len() { state.cursor.0 = state.lines.len() - 1; }
                            state.cursor.1 = 0;
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        } else {
                        let is_table = line_idx < state.lines.len() && {
                            let t = state.lines[line_idx].trim_start();
                            t.starts_with('|') && t.ends_with('|')
                        };
                        if is_table {
                            use crate::ui::md_widget::{cursor_to_cell, cell_to_raw_col, parse_table_cells};

                            if let Some((sel_start, sel_end)) = state.selection_ordered() {
                                state.push_undo();
                                let start_line = sel_start.0;
                                let end_line = sel_end.0;
                                for li in start_line..=end_line.min(state.lines.len() - 1) {
                                    let lt = state.lines[li].trim_start();
                                    if !(lt.starts_with('|') && lt.ends_with('|')) { continue; }
                                    if lt.contains('-') && lt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { continue; }

                                    let parsed = parse_table_cells(&state.lines[li]);
                                    let sc = if li == start_line { sel_start.1 } else { 0 };
                                    let ec = if li == end_line { sel_end.1 } else { state.lines[li].chars().count() };
                                    let (s_cell, s_in) = cursor_to_cell(&state.lines[li], sc);
                                    let (e_cell, e_in) = cursor_to_cell(&state.lines[li], ec);

                                    // reverse order to keep byte offsets valid
                                    for ci in (s_cell..=e_cell.min(parsed.len().saturating_sub(1))).rev() {
                                        if let Some((cs, _ce, cell_text)) = parsed.get(ci) {
                                            let c_start = if ci == s_cell { s_in } else { 0 };
                                            let c_end = if ci == e_cell { e_in } else { cell_text.chars().count() };
                                            if c_start < c_end {
                                                let abs_start = cs + c_start;
                                                let abs_end = cs + c_end;
                                                let bs = char_to_byte_static(&state.lines[li], abs_start);
                                                let be = char_to_byte_static(&state.lines[li], abs_end);
                                                state.lines[li].replace_range(bs..be, "");
                                            }
                                        }
                                    }
                                }
                                let new_col = cell_to_raw_col(&state.lines[start_line],
                                    cursor_to_cell(&state.lines[start_line], sel_start.1).0,
                                    cursor_to_cell(&state.lines[start_line], sel_start.1).1);
                                state.cursor = (start_line, new_col);
                                state.selection = None;
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            } else {
                                let (cell_idx, col_in_cell) = cursor_to_cell(&state.lines[line_idx], col);
                                if col_in_cell > 0 {
                                    state.push_undo();
                                    let parsed = parse_table_cells(&state.lines[line_idx]);
                                    if let Some((cs, _ce, _cell_text)) = parsed.get(cell_idx) {
                                        let byte_start = char_to_byte_static(&state.lines[line_idx], cs + col_in_cell - 1);
                                        let byte_end = char_to_byte_static(&state.lines[line_idx], cs + col_in_cell);
                                        state.lines[line_idx].replace_range(byte_start..byte_end, "");
                                        state.cursor.1 = cell_to_raw_col(&state.lines[line_idx], cell_idx, col_in_cell - 1);
                                        self.editor_dirty = true;
                                        self.last_edit_time = Some(Instant::now());
                                    }
                                } else if cell_idx > 0 {
                                    let parsed = parse_table_cells(&state.lines[line_idx]);
                                    if let Some((_, _, prev_text)) = parsed.get(cell_idx - 1) {
                                        state.cursor.1 = cell_to_raw_col(&state.lines[line_idx], cell_idx - 1, prev_text.chars().count());
                                    }
                                }
                            }
                        } else {
                            state.push_undo();
                            state.backspace();
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                            let (line_idx, _) = state.cursor;
                            if line_idx < state.lines.len() {
                                let line = state.lines[line_idx].trim_start().to_string();
                                if line.starts_with('/') && !line.contains(' ') {
                                    state.slash_menu_open = true;
                                    state.slash_filter = line[1..].to_string();
                                    state.slash_selected = 0;
                                } else {
                                    state.slash_menu_open = false;
                                    state.slash_filter.clear();
                                }
                            }
                        }
                    }
                    } // end else (not image)
                    } // end else (no selection)
                    MdAction::Delete => {
                        let (line_idx, col) = state.cursor;
                        let is_table = line_idx < state.lines.len() && {
                            let t = state.lines[line_idx].trim_start();
                            t.starts_with('|') && t.ends_with('|')
                        };
                        if is_table {
                            // table: constrain delete to cell boundaries
                            use crate::ui::md_widget::{cursor_to_cell, cell_to_raw_col, parse_table_cells};
                            let (cell_idx, col_in_cell) = cursor_to_cell(&state.lines[line_idx], col);
                            let parsed = parse_table_cells(&state.lines[line_idx]);
                            if let Some((_cs, _ce, cell_text)) = parsed.get(cell_idx) {
                                if col_in_cell < cell_text.chars().count() {
                                    state.push_undo();
                                    let abs_col = cell_to_raw_col(&state.lines[line_idx], cell_idx, col_in_cell);
                                    let byte_start = char_to_byte_static(&state.lines[line_idx], abs_col);
                                    let byte_end = char_to_byte_static(&state.lines[line_idx], abs_col + 1);
                                    state.lines[line_idx].replace_range(byte_start..byte_end, "");
                                    self.editor_dirty = true;
                                    self.last_edit_time = Some(Instant::now());
                                }
                            }
                        } else {
                            state.push_undo();
                            state.delete();
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::Indent => {
                        let (line, _) = state.cursor;
                        if line < state.lines.len() {
                            let lt = state.lines[line].trim_start();
                            if lt.starts_with('|') && lt.ends_with('|') && !(lt.contains('-') && lt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')) {
                                use crate::ui::md_widget::{cursor_to_cell, cell_to_raw_col, parse_table_cells};
                                let parsed = parse_table_cells(&state.lines[line]);
                                let (cell_idx, _) = cursor_to_cell(&state.lines[line], state.cursor.1);
                                if cell_idx + 1 < parsed.len() {
                                    state.cursor.1 = cell_to_raw_col(&state.lines[line], cell_idx + 1, 0);
                                } else if line + 1 < state.lines.len() {
                                    let mut next = line + 1;
                                    if next < state.lines.len() {
                                        let nt = state.lines[next].trim_start();
                                        if nt.contains('-') && nt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { next += 1; }
                                    }
                                    if next < state.lines.len() && state.lines[next].trim_start().starts_with('|') {
                                        state.cursor = (next, cell_to_raw_col(&state.lines[next], 0, 0));
                                    }
                                }
                                state.selection = None;
                            } else {
                                state.lines[line] = format!("  {}", &state.lines[line]);
                                state.cursor.1 += 2;
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            }
                        }
                    }
                    MdAction::Unindent => {
                        let (line, _) = state.cursor;
                        if line < state.lines.len() && state.lines[line].starts_with("  ") {
                            state.lines[line] = state.lines[line][2..].to_string();
                            state.cursor.1 = state.cursor.1.saturating_sub(2);
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::Move(motion) => {
                        state.selection = None;
                        let (cl, cc) = state.cursor;
                        let is_tbl = cl < state.lines.len() && {
                            let t = state.lines[cl].trim_start();
                            t.starts_with('|') && t.ends_with('|') && !(t.contains('-') && t.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' '))
                        };
                        if is_tbl {
                            use crate::ui::md_widget::{cursor_to_cell, cell_to_raw_col, parse_table_cells};
                            let parsed = parse_table_cells(&state.lines[cl]);
                            let (cell_idx, col_in) = cursor_to_cell(&state.lines[cl], cc);
                            let cell_text = parsed.get(cell_idx).map(|(_, _, t)| t.clone()).unwrap_or_default();
                            let cell_len = cell_text.chars().count();
                            match motion {
                                MdMotion::Left => {
                                    if col_in > 0 {
                                        state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx, col_in - 1);
                                    } else if cell_idx > 0 {
                                        let prev_len = parsed.get(cell_idx - 1).map(|(_, _, t)| t.chars().count()).unwrap_or(0);
                                        state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx - 1, prev_len);
                                    }
                                }
                                MdMotion::Right => {
                                    if col_in < cell_len {
                                        state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx, col_in + 1);
                                    } else if cell_idx + 1 < parsed.len() {
                                        state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx + 1, 0);
                                    }
                                }
                                MdMotion::Up => {
                                    let mut prev = cl.saturating_sub(1);
                                    if prev > 0 && prev < state.lines.len() {
                                        let pt = state.lines[prev].trim_start();
                                        if pt.contains('-') && pt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { prev = prev.saturating_sub(1); }
                                    }
                                    if prev < state.lines.len() && state.lines[prev].trim_start().starts_with('|') {
                                        let pp = parse_table_cells(&state.lines[prev]);
                                        let ci = cell_idx.min(pp.len().saturating_sub(1));
                                        let ct = pp.get(ci).map(|(_, _, t)| t.chars().count()).unwrap_or(0);
                                        state.cursor = (prev, cell_to_raw_col(&state.lines[prev], ci, col_in.min(ct)));
                                    } else {
                                        apply_motion(state, motion);
                                    }
                                }
                                MdMotion::Down => {
                                    let mut next = cl + 1;
                                    if next < state.lines.len() {
                                        let nt = state.lines[next].trim_start();
                                        if nt.contains('-') && nt.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { next += 1; }
                                    }
                                    if next < state.lines.len() && state.lines[next].trim_start().starts_with('|') {
                                        let pp = parse_table_cells(&state.lines[next]);
                                        let ci = cell_idx.min(pp.len().saturating_sub(1));
                                        let ct = pp.get(ci).map(|(_, _, t)| t.chars().count()).unwrap_or(0);
                                        state.cursor = (next, cell_to_raw_col(&state.lines[next], ci, col_in.min(ct)));
                                    } else if next < state.lines.len() {
                                        state.cursor = (next, 0);
                                    }
                                }
                                MdMotion::Home => {
                                    state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx, 0);
                                }
                                MdMotion::End => {
                                    state.cursor.1 = cell_to_raw_col(&state.lines[cl], cell_idx, cell_len);
                                }
                                _ => { apply_motion(state, motion); }
                            }
                        } else {
                            apply_motion(state, motion);
                            // skip table separator rows
                            let (cl2, _) = state.cursor;
                            if cl2 < state.lines.len() {
                                let t = state.lines[cl2].trim_start();
                                if t.contains('-') && t.starts_with('|') && t.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') {
                                    match motion {
                                        MdMotion::Down | MdMotion::Right => {
                                            if cl2 + 1 < state.lines.len() {
                                                state.cursor = (cl2 + 1, crate::ui::md_widget::cell_to_raw_col(&state.lines[cl2 + 1], 0, 0));
                                            }
                                        }
                                        MdMotion::Up | MdMotion::Left => {
                                            if cl2 > 0 {
                                                state.cursor = (cl2 - 1, crate::ui::md_widget::cell_to_raw_col(&state.lines[cl2 - 1], 0, 0));
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    MdAction::Select(motion) => {
                        if state.selection.is_none() {
                            state.selection = Some((state.cursor, state.cursor));
                        }
                        apply_motion(state, motion);
                        state.selection.as_mut().unwrap().1 = state.cursor;
                    }
                    MdAction::SelectAll => { state.select_all(); }
                    MdAction::Copy => {
                        let (cl, _) = state.cursor;
                        if cl < state.lines.len() {
                            let t = state.lines[cl].trim_start().to_string();
                            if t.starts_with("![") && t.contains("](img:") && t.ends_with(')') {
                                return self.update(Message::CopyImage(cl));
                            }
                        }
                    }
                    MdAction::Cut => {
                        state.push_undo();
                        state.delete_selection();
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                    MdAction::ScrollTo(pos) => {
                        state.scroll_offset = pos.max(0.0);
                        state.scroll_velocity = 0.0;
                    }
                    MdAction::Scroll(delta) => {
                        let avail_w = state.text_area_width;
                        let total_h = state.content_height(self.setting_font_size as f32, avail_w);
                        let viewport_h = state.text_area_width * 0.7;
                        let max_scroll = (total_h - viewport_h).max(0.0);
                        // split into immediate + momentum for smooth feel
                        state.scroll_offset = (state.scroll_offset - delta * 0.7).max(0.0).min(max_scroll);
                        state.scroll_velocity += delta * 0.3;
                        state.scroll_velocity = state.scroll_velocity.clamp(-80.0, 80.0);
                    }
                    MdAction::ToggleCheckbox(line_idx) => {
                        if line_idx < state.lines.len() {
                            let l = &state.lines[line_idx];
                            let trimmed = l.trim_start();
                            let prefix_len = l.len() - trimmed.len();
                            let prefix = &l[..prefix_len];
                            if trimmed.starts_with("- [ ] ") {
                                state.lines[line_idx] = format!("{}- [x] {}", prefix, &trimmed[6..]);
                            } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                                state.lines[line_idx] = format!("{}- [ ] {}", prefix, &trimmed[6..]);
                            }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::RightClick => {
                        let (cl, _) = state.cursor;
                        let is_table_line = cl < state.lines.len() && {
                            let t = state.lines[cl].trim_start();
                            t.starts_with('|') && t.ends_with('|')
                        };
                        let is_image = cl < state.lines.len() && {
                            let t = state.lines[cl].trim_start();
                            t.starts_with("![") && t.contains("](img:") && t.ends_with(')')
                        };
                        let is_file = cl < state.lines.len() && {
                            let t = state.lines[cl].trim_start();
                            t.starts_with("[file:") && t.ends_with(']')
                        };
                        if is_file {
                            self.context_menu = Some(ContextMenu::FileMenu(cl));
                        } else if is_image {
                            self.context_menu = Some(ContextMenu::ImageMenu(cl));
                        } else if is_table_line {
                            self.context_menu = Some(ContextMenu::TableCell(cl));
                        } else {
                            self.context_menu = Some(ContextMenu::EditorFormat);
                        }
                        self.context_menu_pos = self.cursor_pos;
                    }
                    MdAction::SlashClickSelect(clicked_fi) => {
                        state.slash_selected = clicked_fi;
                        use crate::ui::md_widget::{filter_slash_commands as fc2, SLASH_COMMANDS as SC2};
                        let filtered = fc2(&state.slash_filter);
                        if !filtered.is_empty() && clicked_fi < filtered.len() {
                            let idx = filtered[clicked_fi];
                            let cmd = &SC2[idx];
                            let (line_idx, _) = state.cursor;
                            match cmd.name {
                                "code" => {
                                    state.lines[line_idx] = "```".to_string();
                                    state.lines.insert(line_idx + 1, String::new());
                                    state.lines.insert(line_idx + 2, "```".to_string());
                                    state.cursor = (line_idx + 1, 0);
                                }
                                "table" => {
                                    state.lines[line_idx] = "|            |            |            |".to_string();
                                    state.lines.insert(line_idx + 1, "|------------|------------|------------|".to_string());
                                    state.lines.insert(line_idx + 2, "|            |            |            |".to_string());
                                    state.lines.insert(line_idx + 3, "|            |            |            |".to_string());
                                    state.lines.insert(line_idx + 4, "|            |            |            |".to_string());
                                    state.cursor = (line_idx, crate::ui::md_widget::cell_to_raw_col(&state.lines[line_idx], 0, 0));
                                }
                                "password" => {
                                    state.lines[line_idx] = "%%pass".to_string();
                                    state.lines.insert(line_idx + 1, String::new());
                                    state.lines.insert(line_idx + 2, "%%pass".to_string());
                                    state.cursor = (line_idx + 1, 0);
                                }
                                _ => {}
                            }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                        state.slash_menu_open = false;
                        state.slash_filter.clear();
                        state.slash_selected = 0;
                    }
                    MdAction::SlashSelect => {
                        use crate::ui::md_widget::{filter_slash_commands, SLASH_COMMANDS};
                        let filtered = filter_slash_commands(&state.slash_filter);
                        if !filtered.is_empty() {
                            let idx = filtered[state.slash_selected.min(filtered.len() - 1)];
                            let cmd = &SLASH_COMMANDS[idx];
                            let (line_idx, _) = state.cursor;
                            match cmd.name {
                                "code" => {
                                    state.lines[line_idx] = "```".to_string();
                                    state.lines.insert(line_idx + 1, String::new());
                                    state.lines.insert(line_idx + 2, "```".to_string());
                                    state.cursor = (line_idx + 1, 0);
                                }
                                "table" => {
                                    state.lines[line_idx] = "|            |            |            |".to_string();
                                    state.lines.insert(line_idx + 1, "|------------|------------|------------|".to_string());
                                    state.lines.insert(line_idx + 2, "|            |            |            |".to_string());
                                    state.lines.insert(line_idx + 3, "|            |            |            |".to_string());
                                    state.lines.insert(line_idx + 4, "|            |            |            |".to_string());
                                    state.cursor = (line_idx, crate::ui::md_widget::cell_to_raw_col(&state.lines[line_idx], 0, 0));
                                }
                                "password" => {
                                    state.lines[line_idx] = "%%pass".to_string();
                                    state.lines.insert(line_idx + 1, String::new());
                                    state.lines.insert(line_idx + 2, "%%pass".to_string());
                                    state.cursor = (line_idx + 1, 0);
                                }
                                _ => {}
                            }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                        state.slash_menu_open = false;
                        state.slash_filter.clear();
                        state.slash_selected = 0;
                    }
                    MdAction::SlashArrow(down) => {
                        let filtered = crate::ui::md_widget::filter_slash_commands(&state.slash_filter);
                        if !filtered.is_empty() {
                            if down {
                                state.slash_selected = (state.slash_selected + 1) % filtered.len();
                            } else {
                                state.slash_selected = if state.slash_selected == 0 { filtered.len() - 1 } else { state.slash_selected - 1 };
                            }
                        }
                    }
                    MdAction::TableAddRow(line_idx) => {
                        if line_idx < state.lines.len() {
                            let existing = &state.lines[line_idx];
                            let col_count = existing.matches('|').count().saturating_sub(1);
                            let new_row = if col_count > 0 {
                                let mut r = "|".to_string();
                                for _ in 0..col_count {
                                    r.push_str("          |");
                                }
                                r
                            } else {
                                "|          |          |          |".to_string()
                            };
                            state.lines.insert(line_idx + 1, new_row);
                            state.cursor = (line_idx + 1, 2);
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::TableAddCol(line_idx) => {
                        let mut start = line_idx;
                        while start > 0 {
                            let prev = state.lines[start - 1].trim_start().to_string();
                            if prev.starts_with('|') && prev.ends_with('|') { start -= 1; } else { break; }
                        }
                        let mut end = line_idx;
                        while end + 1 < state.lines.len() {
                            let next = state.lines[end + 1].trim_start().to_string();
                            if next.starts_with('|') && next.ends_with('|') { end += 1; } else { break; }
                        }
                        for li in start..=end {
                            let l = &state.lines[li];
                            let is_sep = l.trim_start().contains('-') && l.trim_start().chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                            if is_sep {
                                state.lines[li] = format!("{}----------|", l);
                            } else {
                                state.lines[li] = format!("{}          |", l);
                            }
                        }
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                    MdAction::TableDeleteRow(line_idx) => {
                        if line_idx < state.lines.len() && state.lines.len() > 1 {
                            state.lines.remove(line_idx);
                            if state.cursor.0 >= state.lines.len() {
                                state.cursor.0 = state.lines.len() - 1;
                            }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::TableDeleteCol(line_idx) => {
                        use crate::ui::md_widget::cursor_to_cell;
                        let (cell_idx, _) = cursor_to_cell(&state.lines[line_idx], state.cursor.1);
                        let mut start = line_idx;
                        while start > 0 { let p = state.lines[start-1].trim_start();
                            if p.starts_with('|') && p.ends_with('|') { start -= 1; } else { break; } }
                        let mut end = line_idx;
                        while end + 1 < state.lines.len() { let n = state.lines[end+1].trim_start();
                            if n.starts_with('|') && n.ends_with('|') { end += 1; } else { break; } }
                        state.push_undo();
                        for li in start..=end {
                            let cells_raw: Vec<&str> = state.lines[li].trim_start()[1..state.lines[li].trim_start().len()-1].split('|').collect();
                            if cell_idx < cells_raw.len() && cells_raw.len() > 1 {
                                let mut new_cells: Vec<&str> = cells_raw.clone();
                                new_cells.remove(cell_idx);
                                state.lines[li] = format!("|{}|", new_cells.join("|"));
                            }
                        }
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                    MdAction::TableDelete(line_idx) => {
                        let mut start = line_idx;
                        while start > 0 { let p = state.lines[start-1].trim_start();
                            if p.starts_with('|') && p.ends_with('|') { start -= 1; } else { break; } }
                        let mut end = line_idx;
                        while end + 1 < state.lines.len() { let n = state.lines[end+1].trim_start();
                            if n.starts_with('|') && n.ends_with('|') { end += 1; } else { break; } }
                        state.push_undo();
                        for _ in start..=end {
                            if start < state.lines.len() { state.lines.remove(start); }
                        }
                        if state.lines.is_empty() { state.lines.push(String::new()); }
                        state.cursor = (start.min(state.lines.len()-1), 0);
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                    MdAction::ImageResizeStart(line_idx, mx, my, cur_w, cur_h) => {
                        let t = state.lines.get(line_idx).map(|l| l.trim_start().to_string()).unwrap_or_default();
                        if let (Some(a), Some(b)) = (t.find("]("), t.rfind(')')) {
                            let img_id = t[a + 2..b].to_string();
                            state.image_sizes.insert(img_id, (cur_w, cur_h));
                        }
                        state.image_resizing = Some((line_idx, mx, my));
                    }
                    MdAction::ImageResizeDrag(mx, my) => {
                        if let Some((line_idx, start_x, _start_y)) = state.image_resizing {
                            let dx = mx - start_x;
                            let t = state.lines.get(line_idx).map(|l| l.trim_start().to_string()).unwrap_or_default();
                            if let (Some(a), Some(b)) = (t.find("]("), t.rfind(')')) {
                                let img_id = t[a + 2..b].to_string();
                                if let Some(_handle) = state.image_cache.get(&img_id) {
                                    let (cw, ch) = state.image_sizes.get(&img_id).copied().unwrap_or_else(|| {
                                        (state.text_area_width * 0.6, state.text_area_width * 0.4)
                                    });
                                    let new_w = (cw + dx).max(80.0).min(state.text_area_width - 20.0);
                                    let aspect = if cw > 0.0 { ch / cw } else { 0.75 };
                                    let new_h = new_w * aspect;
                                    state.image_sizes.insert(img_id, (new_w, new_h));
                                    state.image_resizing = Some((line_idx, mx, my));
                                }
                            }
                        }
                    }
                    MdAction::ImageResizeEnd => {
                        state.image_resizing = None;
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                    MdAction::ImageDelete(line_idx) => {
                        if line_idx < state.lines.len() {
                            state.push_undo();
                            state.lines.remove(line_idx);
                            if state.lines.is_empty() { state.lines.push(String::new()); }
                            if state.cursor.0 >= state.lines.len() { state.cursor.0 = state.lines.len() - 1; }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                    }
                    MdAction::ImageResize(_, _, _) => {} // handled via drag
                    MdAction::FileExport(file_id, filename) => {
                        self.context_menu = None;
                        return self.update(Message::FileExport(file_id, filename));
                    }
                    MdAction::FileDelete(file_id) => {
                        self.context_menu = None;
                        return self.update(Message::FileDelete(file_id));
                    }
                    MdAction::CodeLangMenuOpen(line_idx) => {
                        state.code_lang_menu = Some(line_idx);
                    }
                    MdAction::CodeLangSelect(line_idx, lang) => {
                        if line_idx < state.lines.len() {
                            state.push_undo();
                            if lang.is_empty() {
                                state.lines[line_idx] = "```".to_string();
                            } else {
                                state.lines[line_idx] = format!("```{}", lang);
                            }
                            self.editor_dirty = true;
                            self.last_edit_time = Some(Instant::now());
                        }
                        state.code_lang_menu = None;
                    }
                    MdAction::TogglePasswordVisible(block_start) => {
                        if state.password_visible.contains(&block_start) {
                            state.password_visible.remove(&block_start);
                        } else {
                            state.password_visible.insert(block_start);
                        }
                    }
                    MdAction::CopyPasswordBlock(block_start) => {
                        let mut content = String::new();
                        let mut in_block = false;
                        for (li, l) in state.lines.iter().enumerate() {
                            if li == block_start && l.trim_start() == "%%pass" {
                                in_block = true;
                                continue;
                            }
                            if in_block {
                                if l.trim_start() == "%%pass" { break; }
                                if !content.is_empty() { content.push('\n'); }
                                content.push_str(l);
                            }
                        }
                        if !content.is_empty() {
                            state.copied_block_line = Some(block_start);
                            let task = iced::clipboard::write(content);
                            return Task::batch([
                                task,
                                Task::perform(
                                    async { tokio::time::sleep(std::time::Duration::from_secs(2)).await; },
                                    |_| Message::ClearCopiedBlockFeedback,
                                ),
                            ]);
                        }
                    }
                    MdAction::CopyCodeBlock(start_line) => {
                        let mut content = String::new();
                        let mut in_block = false;
                        for (li, l) in state.lines.iter().enumerate() {
                            if li == start_line && l.trim_start().starts_with("```") {
                                in_block = true;
                                continue; // skip opening fence
                            }
                            if in_block {
                                if l.trim_start().starts_with("```") {
                                    break; // closing fence
                                }
                                if !content.is_empty() { content.push('\n'); }
                                content.push_str(l);
                            }
                        }
                        if !content.is_empty() {
                            state.copied_block_line = Some(start_line);
                            let task = iced::clipboard::write(content);
                            return Task::batch([
                                task,
                                Task::perform(
                                    async { tokio::time::sleep(std::time::Duration::from_secs(2)).await; },
                                    |_| Message::ClearCopiedBlockFeedback,
                                ),
                            ]);
                        }
                    }
                    MdAction::Focus => { state.focused = true; state.focus_instant = Some(Instant::now()); }
                    MdAction::Unfocus => { state.focused = false; state.slash_menu_open = false; }
                    MdAction::WindowFocus(f) => { state.is_window_focused = f; }
                    MdAction::Tick(now, width) => {
                        state.now = now;
                        state.text_area_width = width;
                        if state.scroll_velocity.abs() > 0.5 {
                            let total_h = state.content_height(self.setting_font_size as f32, width);
                            let viewport_h = width * 0.7;
                            let max_scroll = (total_h - viewport_h).max(0.0);
                            state.scroll_offset = (state.scroll_offset - state.scroll_velocity).max(0.0).min(max_scroll);
                            state.scroll_velocity *= 0.82; // friction — decelerates smoothly
                        } else {
                            state.scroll_velocity = 0.0;
                        }
                    }
                }
                if self.editor_dirty {
                    if let Some(ref sel) = self.selected_note {
                        let nid = sel.id;
                        let body = self.line_editor.to_body();
                        let snippet = generate_snippet(&body);
                        let now = chrono::Utc::now();
                        for p in &mut self.notes {
                            if p.id == nid { p.snippet = snippet.clone(); p.modified_at = now; break; }
                        }
                        for (_, sns) in &mut self.subfolder_notes {
                            for p in sns { if p.id == nid { p.snippet = snippet.clone(); p.modified_at = now; break; } }
                        }
                        self.apply_sort();
                    }
                }
                Task::none()
            }

            Message::SaveNote => self.save_current_note(),
            Message::AutoSaveTick(_now) => {
                if let Some(last_edit) = self.last_edit_time {
                    if last_edit.elapsed() >= Duration::from_secs(self.setting_auto_save_delay as u64) && self.editor_dirty {
                        return self.save_current_note();
                    }
                }
                Task::none()
            }

            Message::FormatBold => { self.insert_markers("**"); Task::none() }
            Message::FormatItalic => { self.insert_markers("*"); Task::none() }
            Message::FormatHeading => { self.toggle_line_prefix("## "); Task::none() }
            Message::FormatList => { self.toggle_line_prefix("- "); Task::none() }
            Message::FormatCheckbox => { self.toggle_line_prefix("- [ ] "); Task::none() }
            Message::FormatCode => { self.insert_markers("`"); Task::none() }
            Message::FormatDivider => { self.insert_at_cursor("\n---\n"); Task::none() }
            Message::OpenTextColorPicker => {
                self.active_dialog = Some(DialogKind::TextColor);
                Task::none()
            }
            Message::ApplyTextColor => {
                let h = self.color_hue;
                let s = self.color_sat;
                let l = self.color_lit;
                let color_code = format!("{:.0},{:.0},{:.0}", h, s * 100.0, l * 100.0);
                self.active_dialog = None;
                return self.update(Message::FormatTextColor(color_code));
            }
            Message::FormatTextColor(color_code) => {
                use crate::ui::md_widget::ColorRange;
                if let Some((start, end)) = self.line_editor.selection_ordered() {
                    if start.0 == end.0 {
                        self.line_editor.colors.retain(|c| !(c.line == start.0 && c.start_col == start.1 && c.end_col == end.1));
                        self.line_editor.colors.push(ColorRange {
                            line: start.0,
                            start_col: start.1,
                            end_col: end.1,
                            color: color_code,
                        });
                    } else {
                        for li in start.0..=end.0 {
                            let sc = if li == start.0 { start.1 } else { 0 };
                            let ec = if li == end.0 { end.1 } else { self.line_editor.lines[li].chars().count() };
                            self.line_editor.colors.retain(|c| !(c.line == li && c.start_col == sc && c.end_col == ec));
                            self.line_editor.colors.push(ColorRange {
                                line: li, start_col: sc, end_col: ec, color: color_code.clone(),
                            });
                        }
                    }
                    self.line_editor.selection = None;
                }
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::FormatQuote => { self.toggle_line_prefix("> "); Task::none() }

            Message::ToggleSearchCaseSensitive => {
                self.editor_search_case_sensitive = !self.editor_search_case_sensitive;
                self.update_search_matches();
                Task::none()
            }
            Message::ToggleSearch => {
                self.editor_search_open = !self.editor_search_open;
                if !self.editor_search_open {
                    self.line_editor.search_matches.clear();
                    self.line_editor.current_match = 0;
                    self.editor_search_index = 0;
                }
                if self.editor_search_open {
                    if !self.editor_search_query.is_empty() {
                        self.update_search_matches();
                    }
                    return text_input::focus(text_input::Id::new("editor_search"));
                }
                Task::none()
            }
            Message::SearchQueryEditorChanged(q) => {
                self.editor_search_query = q;
                self.editor_search_index = 0;
                self.line_editor.current_match = 0;
                self.update_search_matches();
                if !self.line_editor.search_matches.is_empty() {
                    let sm = self.line_editor.search_matches[0].clone();
                    self.jump_to_search_match(sm.line, sm.start_col);
                }
                Task::none()
            }
            Message::SearchNext => {
                let count = self.line_editor.search_matches.len();
                if count == 0 { return Task::none(); }
                let next = if self.editor_search_index + 1 >= count { 0 } else { self.editor_search_index + 1 };
                self.editor_search_index = next;
                self.line_editor.current_match = next;
                let sm = self.line_editor.search_matches[next].clone();
                self.jump_to_search_match(sm.line, sm.start_col);
                Task::none()
            }
            Message::SearchPrev => {
                let count = self.line_editor.search_matches.len();
                if count == 0 { return Task::none(); }
                let prev = if self.editor_search_index == 0 { count - 1 } else { self.editor_search_index - 1 };
                self.editor_search_index = prev;
                self.line_editor.current_match = prev;
                let sm = self.line_editor.search_matches[prev].clone();
                self.jump_to_search_match(sm.line, sm.start_col);
                Task::none()
            }
            Message::ToggleSidebar => {
                self.show_sidebar = !self.show_sidebar;
                Task::none()
            }
            Message::ZoomIn => {
                self.gui_scale = (self.gui_scale + 0.1).min(2.0);
                self.zoom_toast = Some(Instant::now());
                self.save_setting("gui_scale", &format!("{:.1}", self.gui_scale))
            }
            Message::ZoomOut => {
                self.gui_scale = (self.gui_scale - 0.1).max(0.5);
                self.zoom_toast = Some(Instant::now());
                self.save_setting("gui_scale", &format!("{:.1}", self.gui_scale))
            }
            Message::ZoomReset => {
                self.gui_scale = 1.0;
                self.zoom_toast = Some(Instant::now());
                self.save_setting("gui_scale", "1.0")
            }
            Message::CloseSubmenus => {
                self.color_submenu_for = None;
                self.move_submenu_for = None;
                self.new_note_submenu_for = None;
                Task::none()
            }
            Message::ToggleMarkdownPreview => {
                self.editor_preview = !self.editor_preview;
                if self.editor_preview {
                    self.line_editor.sync_active_to_lines();
                    self.markdown_items = iced::widget::markdown::parse(&self.line_editor.to_body()).collect();
                }
                Task::none()
            }

            Message::PasswordWebsiteChanged(v) => { self.password_data.website = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); self.sync_editor_to_other_windows(); Task::none() }
            Message::PasswordUsernameChanged(v) => { self.password_data.username = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); self.sync_editor_to_other_windows(); Task::none() }
            Message::PasswordEmailChanged(v) => { self.password_data.email = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); self.sync_editor_to_other_windows(); Task::none() }
            Message::PasswordValueChanged(v) => { self.password_data.password = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); self.sync_editor_to_other_windows(); Task::none() }
            Message::PasswordNotesChanged(v) => { self.password_data.notes = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); self.sync_editor_to_other_windows(); Task::none() }
            Message::PasswordNotesAction(action) => {
                let is_edit = action.is_edit();
                self.password_notes_content.perform(action);
                if is_edit {
                    self.password_data.notes = self.password_notes_content.text();
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    self.sync_editor_to_other_windows();
                }
                Task::none()
            }
            Message::TogglePasswordVisibility => { self.show_password = !self.show_password; Task::none() }
            Message::TogglePasswordGenPanel => { self.show_password_gen = !self.show_password_gen; Task::none() }
            Message::PasswordGenLength(len) => { self.password_gen_options.length = len; Task::none() }
            Message::PasswordGenToggleUpper => { self.password_gen_options.uppercase = !self.password_gen_options.uppercase; Task::none() }
            Message::PasswordGenToggleLower => { self.password_gen_options.lowercase = !self.password_gen_options.lowercase; Task::none() }
            Message::PasswordGenToggleNumbers => { self.password_gen_options.numbers = !self.password_gen_options.numbers; Task::none() }
            Message::PasswordGenToggleSymbols => { self.password_gen_options.symbols = !self.password_gen_options.symbols; Task::none() }
            Message::GeneratePassword => {
                self.password_data.password = self.password_gen_options.generate();
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                self.sync_editor_to_other_windows();
                Task::none()
            }
            Message::AddCustomField => {
                self.password_data.custom_fields.push(CustomField { label: String::new(), value: String::new(), hidden: false });
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::RemoveCustomField(idx) => {
                if idx < self.password_data.custom_fields.len() {
                    self.password_data.custom_fields.remove(idx);
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    self.sync_editor_to_other_windows();
                }
                Task::none()
            }
            Message::CustomFieldLabelChanged(idx, v) => {
                if let Some(f) = self.password_data.custom_fields.get_mut(idx) { f.label = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); }
                Task::none()
            }
            Message::CustomFieldValueChanged(idx, v) => {
                if let Some(f) = self.password_data.custom_fields.get_mut(idx) { f.value = v; self.editor_dirty = true; self.last_edit_time = Some(Instant::now()); }
                Task::none()
            }
            Message::ToggleCustomFieldHidden(idx) => {
                if let Some(f) = self.password_data.custom_fields.get_mut(idx) { f.hidden = !f.hidden; }
                Task::none()
            }
            Message::CopyField(field_name, value) => {
                self.copied_field = Some(field_name);
                let clear_task = Task::perform(
                    async { tokio::time::sleep(Duration::from_secs(2)).await },
                    |_| Message::CopiedFeedbackClear,
                );
                Task::batch([iced::clipboard::write(value), clear_task])
            }
            Message::CopiedFeedbackClear => {
                self.copied_field = None;
                Task::none()
            }
            Message::ClearCopiedBlockFeedback => {
                self.line_editor.copied_block_line = None;
                Task::none()
            }

            Message::NoteMigrated(cleaned_body, loaded_images) => {
                self.line_editor = crate::ui::line_editor::LineEditorState::from_body(&cleaned_body);
                let tw = self.line_editor.text_area_width;
                for (id, bytes, fmt) in loaded_images {
                    if fmt.starts_with("rgba:") {
                        let parts: Vec<&str> = fmt.splitn(3, ':').collect();
                        if let (Ok(w), Ok(h)) = (parts.get(1).unwrap_or(&"0").parse::<u32>(), parts.get(2).unwrap_or(&"0").parse::<u32>()) {
                            self.line_editor.image_cache.insert(id.clone(), iced::widget::image::Handle::from_rgba(w, h, bytes));
                            let max_w = (tw - 16.0).max(100.0);
                            let scale = (max_w / w as f32).min(400.0 / h as f32).min(1.0);
                            self.line_editor.image_sizes.insert(id, (w as f32 * scale, h as f32 * scale));
                        }
                    } else {
                        self.line_editor.image_cache.insert(id.clone(), iced::widget::image::Handle::from_bytes(bytes));
                        self.line_editor.image_sizes.insert(id, ((tw - 16.0).min(400.0), (tw - 16.0).min(400.0) * 0.66));
                    }
                }
                Task::none()
            }
            Message::ImagesLoaded(loaded) => {
                let tw = self.line_editor.text_area_width;
                for (id, bytes, fmt) in loaded {
                    if !self.line_editor.image_cache.contains_key(&id) {
                        if fmt.starts_with("rgba:") {
                            let parts: Vec<&str> = fmt.splitn(3, ':').collect();
                            if let (Ok(w), Ok(h)) = (parts.get(1).unwrap_or(&"0").parse::<u32>(), parts.get(2).unwrap_or(&"0").parse::<u32>()) {
                                self.line_editor.image_cache.insert(id.clone(), iced::widget::image::Handle::from_rgba(w, h, bytes));
                                                if !self.line_editor.image_sizes.contains_key(&id) {
                                    let max_w = (tw - 16.0).max(100.0);
                                    let scale = (max_w / w as f32).min(400.0 / h as f32).min(1.0);
                                    self.line_editor.image_sizes.insert(id, (w as f32 * scale, h as f32 * scale));
                                }
                            }
                        } else {
                            self.line_editor.image_cache.insert(id.clone(), iced::widget::image::Handle::from_bytes(bytes));
                            if !self.line_editor.image_sizes.contains_key(&id) {
                                self.line_editor.image_sizes.insert(id, ((tw - 16.0).min(400.0), (tw - 16.0).min(400.0) * 0.66));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CopyImage(line_idx) => {
                if line_idx < self.line_editor.lines.len() {
                    let t = self.line_editor.lines[line_idx].trim_start().to_string();
                    if let (Some(a), Some(b)) = (t.find("]("), t.rfind(')')) {
                        let img_id = t[a + 2..b].to_string();
                        if let Some(&(_w, _h)) = self.line_editor.image_sizes.get(&img_id) {
                            let db_path = self.db_path.clone();
                            let Some(vk) = self.vault_key else { return Task::none() };
                            return Task::perform(async move {
                                if let Ok(conn) = db::open_connection(&db_path) {
                                    if let Some((data, fmt)) = db::load_image_encrypted(&conn, &img_id, &vk) {
                                        if let Ok(mut clip) = arboard::Clipboard::new() {
                                            if fmt.starts_with("rgba:") {
                                                let parts: Vec<&str> = fmt.splitn(3, ':').collect();
                                                if let (Ok(iw), Ok(ih)) = (parts.get(1).unwrap_or(&"0").parse::<usize>(), parts.get(2).unwrap_or(&"0").parse::<usize>()) {
                                                    let _ = clip.set_image(arboard::ImageData { width: iw, height: ih, bytes: std::borrow::Cow::Owned(data) });
                                                }
                                            }
                                        }
                                    }
                                }
                            }, |_| Message::None);
                        }
                    }
                }
                Task::none()
            }
            Message::ImageDropped(_wid, path) => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico"].contains(&ext.as_str()) {
                    if let Ok(data) = std::fs::read(&path) {
                        let mime = match ext.as_str() {
                            "png" => "image/png",
                            "jpg" | "jpeg" => "image/jpeg",
                            "gif" => "image/gif",
                            "bmp" => "image/bmp",
                            "webp" => "image/webp",
                            _ => "image/png",
                        };
                        return self.update(Message::InsertImageData(data, mime.to_string()));
                    }
                } else if path.is_file() {
                    return self.update(Message::FileDropped(path));
                }
                Task::none()
            }
            Message::InsertImageData(data, mime) => {
                if self.selected_note.is_some() {
                    let id = format!("img:{}", Uuid::new_v4());
                    self.line_editor.push_undo();
                    let (cl, _) = self.line_editor.cursor;
                    self.line_editor.lines.insert(cl + 1, format!("![]({})", id));
                    self.line_editor.cursor = (cl + 1, 0);
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    self.line_editor.image_cache.insert(id.clone(), iced::widget::image::Handle::from_bytes(data.clone()));
                    let db_path = self.db_path.clone();
                    let Some(vk) = self.vault_key else { return Task::none() };
                    return Task::perform(async move {
                        if let Ok(conn) = db::open_connection(&db_path) {
                            let _ = db::save_image_encrypted(&conn, &id, &data, &mime, &vk);
                        }
                    }, |_| Message::None);
                }
                Task::none()
            }

            Message::CopySelected => {
                self.clipboard_notes.clear();
                self.clipboard_folders.clear();
                if !self.multi_selected.is_empty() || !self.multi_selected_folders.is_empty() {
                    self.clipboard_notes = self.multi_selected.iter().copied().collect();
                    self.clipboard_folders = self.multi_selected_folders.iter().copied().collect();
                } else if let Some(ref note) = self.selected_note {
                    self.clipboard_notes.push(note.id);
                }
                Task::none()
            }
            Message::PasteItems => {
                if self.clipboard_notes.is_empty() && self.clipboard_folders.is_empty() {
                    return Task::none();
                }
                let note_ids = self.clipboard_notes.clone();
                let folder_ids = self.clipboard_folders.clone();
                let target_folder = match &self.active_view {
                    ActiveView::Folder(id) => Some(*id),
                    _ => None,
                };
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = match db::open_connection(&db_path) { Ok(c) => c, Err(_) => return };
                    for nid in &note_ids {
                        if let Ok(Some(note)) = db::notes::get_note(&conn, *nid) {
                            let mut dup = note.clone();
                            dup.id = Uuid::new_v4();
                            dup.title = format!("{} (copy)", dup.title);
                            dup.folder_id = target_folder.or(note.folder_id);
                            dup.created_at = chrono::Utc::now();
                            dup.modified_at = chrono::Utc::now();
                            let _ = db::notes::insert_note(&conn, &dup);
                        }
                    }
                    // shallow copy — folder only, not contents
                    for fid in &folder_ids {
                        if let Some(folder) = db::folders::list_folders(&conn).ok()
                            .and_then(|fs| fs.into_iter().find(|f| f.id == *fid))
                        {
                            let dup = Folder {
                                id: Uuid::new_v4(),
                                name: format!("{} (copy)", folder.name),
                                parent_id: target_folder.or(folder.parent_id),
                                color: folder.color,
                                sort_order: folder.sort_order,
                                collapsed: false,
                                is_favorite: false,
                            };
                            let _ = db::folders::insert_folder(&conn, &dup);
                        }
                    }
                }, |_| Message::PasteDone)
            }
            Message::PasteDone => {
                self.refresh_data()
            }

            Message::FileDropped(path) => {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
                let file_id = Uuid::new_v4().to_string()[..8].to_string();
                let note_id = Uuid::new_v4();
                let folder_id = match &self.active_view {
                    ActiveView::Folder(id) => Some(*id),
                    _ => None,
                };
                let fid = file_id.clone();
                let fname = filename.clone();
                let fid_fallback = file_id.clone();
                let fname_fallback = filename.clone();

                let progress = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
                self.file_transfers.push((file_id.clone(), truncate_filename(&filename, 24), progress.clone()));

                let db_path = self.db_path.clone();
                let Some(vk) = self.vault_key else { return Task::none() };
                let nid = note_id.to_string();
                let fid2 = file_id.clone();
                let fname2 = filename.clone();
                let folder_str = folder_id.map(|f| f.to_string());
                Task::perform(async move {
                    let conn = match db::open_connection(&db_path) {
                        Ok(c) => c,
                        Err(e) => { eprintln!("File import: DB open failed: {e}"); return None; }
                    };
                    let size = match db::save_file_chunked(&conn, &fid, &nid, &fname, &path, &vk, &progress) {
                        Ok(s) => s,
                        Err(e) => { eprintln!("File import: save_file_chunked failed: {e}"); return None; }
                    };
                    let size_str = format_file_size(size);
                    let body = format!("[file:{}:{}:{}]", fid, fname, size_str);
                    if let Err(e) = conn.execute(
                        "INSERT INTO notes (id, folder_id, title, body, note_type) VALUES (?, ?, ?, ?, 'File')",
                        rusqlite::params![nid, folder_str, fname, body],
                    ) {
                        eprintln!("File import: note insert failed: {e}");
                        return None;
                    }
                    Some((fid2, fname2, size))
                }, move |result| {
                    if let Some((fid, fname, size)) = result {
                        Message::FileSaved(fid, fname, size)
                    } else {
                        Message::FileSaved(fid_fallback.clone(), fname_fallback.clone(), 0)
                    }
                })
            }
            Message::FileSaved(file_id, _filename, _size) => {
                self.file_transfers.retain(|(id, _, _)| id != &file_id);
                self.refresh_data()
            }
            Message::FileExport(file_id, filename) => {
                let transfer_id = format!("export-{}", file_id);
                {let p = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)); self.file_transfers.push((transfer_id.clone(), truncate_filename(&filename, 24), p));};
                let db_path = self.db_path.clone();
                let Some(vk) = self.vault_key else { return Task::none() };
                let fname = filename.clone();
                let fid = file_id.clone();
                let tid = transfer_id;
                Task::perform(async move {
                    if let Some(dest) = rfd_save_dialog(&fname).await {
                        if let Ok(conn) = db::open_connection(&db_path) {
                            let _ = db::export_file_chunked(&conn, &fid, &dest, &vk);
                        }
                    }
                    Some(tid)
                }, |result| Message::FileExported(result))
            }
            Message::FileExportAs(file_id, filename) => {
                let transfer_id = format!("export-{}", file_id);
                {let p = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)); self.file_transfers.push((transfer_id.clone(), truncate_filename(&filename, 24), p));};
                let db_path = self.db_path.clone();
                let Some(vk) = self.vault_key else { return Task::none() };
                let fid = file_id.clone();
                let fname = filename.clone();
                let tid = transfer_id;
                Task::perform(async move {
                    let dialog = rfd::AsyncFileDialog::new()
                        .set_file_name(&fname)
                        .set_title("Save file as");
                    if let Some(handle) = dialog.save_file().await {
                        let dest = handle.path().to_path_buf();
                        if let Ok(conn) = db::open_connection(&db_path) {
                            let _ = db::export_file_chunked(&conn, &fid, &dest, &vk);
                        }
                    }
                    Some(tid)
                }, |result| Message::FileExported(result))
            }
            Message::FileExportSelected => {
                let mut tasks = Vec::new();
                let selected: Vec<Uuid> = self.multi_selected.iter().copied().collect();
                for nid in selected {
                    let preview = self.notes.iter().find(|n| n.id == nid);
                    if preview.map_or(false, |p| p.note_type == NoteType::File) {
                        let db_path = self.db_path.clone();
                        let Some(vk) = self.vault_key else { return Task::none() };
                        let tid = format!("export-{}", nid);
                        let title = preview.map(|p| p.title.clone()).unwrap_or_default();
                        {let p = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)); self.file_transfers.push((tid.clone(), truncate_filename(&title, 24), p));};
                        tasks.push(Task::perform(async move {
                            if let Ok(conn) = db::open_connection(&db_path) {
                                if let Ok(Some(note)) = db::notes::get_note(&conn, nid) {
                                    if let Some((fid, fname, _)) = crate::ui::file_viewer::parse_file_body(&note.body) {
                                        if let Some(dest) = rfd_save_dialog(&fname).await {
                                            if let Some(data) = db::load_file_encrypted(&conn, &fid, &vk) {
                                                let _ = std::fs::write(&dest, &data);
                                            }
                                        }
                                    }
                                }
                            }
                            Some(tid)
                        }, |result| Message::FileExported(result)));
                    }
                }
                if tasks.is_empty() { return Task::none(); }
                Task::batch(tasks)
            }
            Message::FileExported(result) => {
                if let Some(tid) = result {
                    self.file_transfers.retain(|(id, _, _)| id != &tid);
                }
                Task::none()
            }
            Message::FileDelete(_file_id) => {
                if let Some(ref note) = self.selected_note {
                    self.active_dialog = Some(DialogKind::DeleteNote(note.id));
                }
                Task::none()
            }
            Message::FileDeleted(_file_id) => {
                Task::none()
            }
            Message::FileProgress(_progress, _label) => {
                Task::none()
            }

            Message::DragPotential(item) => {
                self.potential_drag = Some(item);
                self.drag_start_pos = self.cursor_pos;
                Task::none()
            }
            Message::DragStart(item) => {
                self.potential_drag = None;
                self.drag_start_pos = self.cursor_pos;
                self.dragging = Some(item);
                self.hovered_item = None;
                Task::none()
            }
            Message::DragEnd => {
                if self.resizing.is_some() {
                    self.resizing = None;
                    return Task::none();
                }
                if self.dragging.is_none() && self.potential_drag.is_none() {
                    if self.rename_pending == 0 {
                        if self.renaming_note.is_some() {
                            return self.update(Message::RenameNoteSubmit);
                        }
                        if self.renaming_folder.is_some() {
                            return self.update(Message::RenameFolderSubmit);
                        }
                    }
                    return Task::none();
                }
                // persist reorder if dragging root folders
                if let Some(DragItem::Folder(fid)) = &self.dragging {
                    let src_is_root = self.folders.iter().any(|f| f.id == *fid && f.parent_id.is_none());
                    if src_is_root {
                        let root_ids: Vec<(i32, String)> = self.folders.iter()
                            .filter(|f| f.parent_id.is_none())
                            .map(|f| (f.sort_order, f.id.to_string()))
                            .collect();
                        let db_path = self.db_path.clone();
                        self.potential_drag = None;
                        self.dragging = None;
                        return Task::perform(async move {
                            if let Ok(conn) = db::open_connection(&db_path) {
                                for (order, id) in &root_ids {
                                    conn.execute("UPDATE folders SET sort_order = ? WHERE id = ?",
                                        rusqlite::params![order, id]).ok();
                                }
                            }
                        }, |_| Message::Refresh);
                    }
                }
                self.potential_drag = None;
                self.dragging = None;
                Task::none()
            }
            Message::DropOnFolder(item, target_folder_id) => {
                self.dragging = None;
                self.potential_drag = None;
                match item {
                    DragItem::Note(note_id) => {
                        let note_ids: Vec<Uuid> = if self.multi_selected.contains(&note_id) && (self.multi_selected.len() + self.multi_selected_folders.len()) > 1 {
                            self.multi_selected.drain().collect()
                        } else {
                            vec![note_id]
                        };
                        let folder_ids: Vec<Uuid> = if note_ids.len() > 1 || !self.multi_selected_folders.is_empty() {
                            self.multi_selected_folders.drain().collect()
                        } else {
                            vec![]
                        };
                        let db_path = self.db_path.clone();
                        Task::perform(async move {
                            if let Ok(conn) = db::open_connection(&db_path) {
                                for id in &note_ids {
                                    let _ = db::notes::move_to_folder(&conn, *id, target_folder_id);
                                }
                                for id in &folder_ids {
                                    if Some(*id) != target_folder_id {
                                        let _ = db::folders::reparent_folder(&conn, *id, target_folder_id);
                                    }
                                }
                            }
                        }, |_| Message::Refresh)
                    }
                    DragItem::Folder(folder_id) => {
                        if Some(folder_id) == target_folder_id { return Task::none(); }
                        let folder_ids: Vec<Uuid> = if self.multi_selected_folders.contains(&folder_id) && (self.multi_selected.len() + self.multi_selected_folders.len()) > 1 {
                            self.multi_selected_folders.drain().collect()
                        } else {
                            vec![folder_id]
                        };
                        let note_ids: Vec<Uuid> = if folder_ids.len() > 1 || !self.multi_selected.is_empty() {
                            self.multi_selected.drain().collect()
                        } else {
                            vec![]
                        };
                        let db_path = self.db_path.clone();
                        Task::perform(async move {
                            if let Ok(conn) = db::open_connection(&db_path) {
                                for id in &folder_ids {
                                    if Some(*id) != target_folder_id {
                                        let _ = db::folders::reparent_folder(&conn, *id, target_folder_id);
                                    }
                                }
                                for id in &note_ids {
                                    let _ = db::notes::move_to_folder(&conn, *id, target_folder_id);
                                }
                            }
                        }, |_| Message::Refresh)
                    }
                }
            }
            Message::ReorderPreview(dragged_id, hovered_id) => {
                if dragged_id == hovered_id { return Task::none(); }
                let root_ids: Vec<Uuid> = self.folders.iter()
                    .filter(|f| f.parent_id.is_none())
                    .map(|f| f.id)
                    .collect();
                let mut new_order: Vec<Uuid> = root_ids.iter().filter(|id| **id != dragged_id).copied().collect();
                if let Some(pos) = new_order.iter().position(|id| *id == hovered_id) {
                    new_order.insert(pos, dragged_id);
                } else {
                    new_order.push(dragged_id);
                }
                for (i, id) in new_order.iter().enumerate() {
                    if let Some(f) = self.folders.iter_mut().find(|f| f.id == *id) {
                        f.sort_order = i as i32;
                    }
                }
                self.folders.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
                Task::none()
            }
            Message::ReorderToEnd(folder_id) => {
                let mut root_ids: Vec<Uuid> = self.folders.iter()
                    .filter(|f| f.parent_id.is_none())
                    .map(|f| f.id)
                    .collect();
                root_ids.retain(|id| *id != folder_id);
                root_ids.push(folder_id);
                for (i, id) in root_ids.iter().enumerate() {
                    if let Some(f) = self.folders.iter_mut().find(|f| f.id == *id) {
                        f.sort_order = i as i32;
                    }
                }
                self.folders.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
                Task::none()
            }
            Message::ReorderDrop(item, target_id) => {
                self.dragging = None;
                self.potential_drag = None;
                if let DragItem::Folder(folder_id) = item {
                    if folder_id == target_id { return Task::none(); }
                    let root_ids: Vec<Uuid> = self.folders.iter()
                        .filter(|f| f.parent_id.is_none())
                        .map(|f| f.id)
                        .collect();
                    let mut new_order: Vec<Uuid> = root_ids.iter().filter(|id| **id != folder_id).copied().collect();
                    if let Some(pos) = new_order.iter().position(|id| *id == target_id) {
                        new_order.insert(pos, folder_id);
                    } else {
                        new_order.push(folder_id);
                    }
                    let db_path = self.db_path.clone();
                    for (i, id) in new_order.iter().enumerate() {
                        if let Some(f) = self.folders.iter_mut().find(|f| f.id == *id) {
                            f.sort_order = i as i32;
                        }
                    }
                    self.folders.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
                    return Task::perform(async move {
                        if let Ok(conn) = db::open_connection(&db_path) {
                            for (i, id) in new_order.iter().enumerate() {
                                conn.execute("UPDATE folders SET sort_order = ? WHERE id = ?",
                                    rusqlite::params![i as i32, id.to_string()]).ok();
                            }
                        }
                    }, |_| Message::Refresh);
                }
                Task::none()
            }
            Message::ReorderMainFolder(folder_id, new_order) => {
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        conn.execute("UPDATE folders SET sort_order = ? WHERE id = ?", rusqlite::params![new_order, folder_id.to_string()]).ok();
                    }
                }, |_| Message::Refresh)
            }
            Message::ModifiersChanged(ctrl, shift, alt) => {
                self.ctrl_held = ctrl;
                self.shift_held = shift;
                self.alt_held = alt;
                Task::none()
            }
            Message::SetSortMode(mode) => {
                self.sort_mode = mode;
                self.sort_menu_open = false;
                let db_path = self.db_path.clone();
                let mode_str = mode.as_str().to_string();
                let _ = std::thread::spawn(move || {
                    if let Ok(conn) = db::open_connection(&db_path) { let _ = db::set_setting(&conn, "sort_mode", &mode_str); }
                });
                self.apply_sort();
                Task::none()
            }
            Message::ToggleSortMenu => {
                self.sort_menu_open = !self.sort_menu_open;
                Task::none()
            }
            Message::ToggleExpandFolder(id) => {
                if self.expanded_folders.contains(&id) {
                    self.expanded_folders.remove(&id);
                } else {
                    self.expanded_folders.insert(id);
                }
                self.multi_selected_folders.clear();
                self.multi_selected.clear();
                Task::none()
            }

            Message::CreateFolder => {
                self.active_dialog = None;
                let name = self.folder_name_input.trim().to_string();
                if name.is_empty() { return Task::none(); }
                let color = self.folder_color_input;
                let folder = Folder::new(name, color, self.folder_parent_id);
                let db_path = self.db_path.clone();
                self.folder_name_input.clear();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::folders::insert_folder(&conn, &folder); } }, |_| Message::Refresh)
            }
            Message::RenameFolder(id) => {
                self.active_dialog = None;
                let name = self.folder_name_input.trim().to_string();
                if name.is_empty() { return Task::none(); }
                let color = self.folder_color_input;
                let db_path = self.db_path.clone();
                self.folder_name_input.clear();
                Task::perform(async move {
                    if let Ok(conn) = db::open_connection(&db_path) {
                        if let Ok(folders) = db::folders::list_folders(&conn) {
                            if let Some(mut f) = folders.into_iter().find(|f| f.id == id) { f.name = name; f.color = color; let _ = db::folders::update_folder(&conn, &f); }
                        }
                    }
                }, |_| Message::Refresh)
            }
            Message::DeleteFolder(id) => {
                self.active_dialog = None;
                if matches!(self.active_view, ActiveView::Folder(fid) if fid == id) { self.active_view = ActiveView::AllNotes; }
                let db_path = self.db_path.clone();
                Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::folders::delete_folder(&conn, id); } }, |_| Message::Refresh)
            }
            Message::FolderNameInputChanged(name) => { self.folder_name_input = name; Task::none() }
            Message::FolderColorSelected(color) => { self.folder_color_input = color; Task::none() }

            Message::OpenCreateFolderDialog => { self.folder_name_input.clear(); self.folder_color_input = FolderColor::Blue; self.folder_parent_id = None; self.active_dialog = Some(DialogKind::CreateFolder); Task::none() }
            Message::OpenCreateSubfolderDialog(parent_id) => {
                self.folder_name_input.clear(); self.folder_color_input = FolderColor::Blue;
                self.folder_parent_id = Some(parent_id);
                self.active_dialog = Some(DialogKind::CreateFolder); Task::none()
            }
            Message::OpenRenameFolderDialog(id) => {
                if let Some(f) = self.folders.iter().find(|f| f.id == id) { self.folder_name_input = f.name.clone(); self.folder_color_input = f.color; }
                self.active_dialog = Some(DialogKind::RenameFolder(id)); Task::none()
            }
            Message::OpenDeleteNoteDialog(id) => {
                // skip confirmation for empty notes
                let is_empty = if self.selected_note.as_ref().map_or(false, |n| n.id == id) {
                    let n = self.selected_note.as_ref().unwrap();
                    n.title.is_empty() && n.body.trim().is_empty()
                } else {
                    self.notes.iter().find(|n| n.id == id).map_or(false, |n| {
                        n.title.is_empty() && n.snippet.is_empty()
                    })
                };
                if is_empty {
                    return self.update(Message::DeleteNote(id));
                }
                self.active_dialog = Some(DialogKind::DeleteNote(id)); Task::none()
            }
            Message::OpenDeleteFolderDialog(id) => {
                // skip confirmation for empty folders
                let note_count = self.folder_counts.iter().find(|(fid, _)| *fid == id).map(|(_, c)| *c).unwrap_or(0);
                let subfolder_count = self.subfolders.iter().filter(|f| f.parent_id == Some(id)).count();
                if note_count == 0 && subfolder_count == 0 {
                    return self.update(Message::DeleteFolder(id));
                }
                self.active_dialog = Some(DialogKind::DeleteFolder(id)); Task::none()
            }
            Message::OpenEncryptDialog(id) => { self.note_password_input.clear(); self.note_password_confirm.clear(); self.auth_error = None; self.active_dialog = Some(DialogKind::EncryptNote(id)); Task::none() }
            Message::OpenDecryptDialog(id) => { self.note_password_input.clear(); self.auth_error = None; self.active_dialog = Some(DialogKind::DecryptNote(id)); Task::none() }
            Message::OpenMoveFolderPicker(id) => {
                if self.context_menu.is_some() {
                    self.move_submenu_for = if self.move_submenu_for == Some(id) { None } else { Some(id) };
                    self.color_submenu_for = None;
                } else {
                    self.toolbar_move_open = !self.toolbar_move_open;
                }
                Task::none()
            }
            Message::CloseDialog => { self.active_dialog = None; self.note_password_input.clear(); self.context_menu = None; Task::none() }

            Message::NotePasswordInputChanged(pw) => { self.note_password_input = pw; self.auth_error = None; Task::none() }
            Message::NotePasswordConfirmChanged(s) => { self.note_password_confirm = s; Task::none() }
            Message::LockNote => {
                if let Some(ref n) = self.selected_note {
                    self.session_decrypted.remove(&n.id);
                    let id = n.id;
                    let db_path = self.db_path.clone();
                    return Task::perform(async move { let conn = db::open_connection(&db_path).ok()?; db::notes::get_note(&conn, id).ok()? }, Message::NoteLoaded);
                }
                Task::none()
            }
            Message::ChangeEncryptionPassword(id) => {
                self.note_password_input.clear();
                self.note_password_confirm.clear();
                self.auth_error = None;
                self.active_dialog = Some(DialogKind::ChangePassword(id));
                Task::none()
            }
            Message::SubmitChangePassword(note_id) => {
                if self.note_password_input != self.note_password_confirm {
                    self.auth_error = Some("Passwords don't match".into());
                    return Task::none();
                }
                if self.note_password_input.len() < 4 {
                    self.auth_error = Some("Password too short (min 4)".into());
                    return Task::none();
                }
                self.active_dialog = None;
                self.encrypting = true;
                let new_password = self.note_password_input.clone().into_bytes();
                self.note_password_input.clear();
                self.note_password_confirm.clear();
                self.session_decrypted.insert(note_id, new_password.clone());
                let plaintext = if let Some(ref n) = self.selected_note {
                    if n.note_type == NoteType::Password { self.password_data.to_json() }
                    else if n.note_type == NoteType::Canvas { self.canvas_editor.data.to_json() }
                    else { self.line_editor.to_body() }
                } else { String::new() };
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let salt = crypto::key_derivation::generate_salt();
                    let derived = crypto::key_derivation::derive_key(&new_password, &salt).map_err(|e| e.to_string())?;
                    let (ciphertext, nonce) = crypto::encryption::encrypt(&derived.key_bytes, plaintext.as_bytes()).map_err(|e| e.to_string())?;
                    db::notes::update_note_encryption(&conn, note_id, &BASE64.encode(&ciphertext), true, Some(&BASE64.encode(nonce)), Some(&BASE64.encode(salt))).map_err(|e| e.to_string())?;
                    Ok(())
                }, |r: Result<(), String>| match r { Ok(()) => Message::EncryptionDone(Ok(())), Err(e) => Message::EncryptionDone(Err(e)) })
            }
            Message::RemoveEncryption(note_id) => {
                if let Some(ref n) = self.selected_note {
                    if n.id == note_id {
                        let body = self.line_editor.to_body();
                        let db_path = self.db_path.clone();
                        self.session_decrypted.remove(&note_id);
                        return Task::perform(async move {
                            let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                            db::notes::update_note_encryption(&conn, note_id, &body, false, None, None).map_err(|e| e.to_string())?;
                            Ok(())
                        }, |r: Result<(), String>| match r { Ok(()) => Message::Refresh, Err(e) => Message::EncryptionDone(Err(e)) });
                    }
                }
                Task::none()
            }
            Message::SubmitEncrypt(note_id) => {
                if self.note_password_input != self.note_password_confirm {
                    self.auth_error = Some("Passwords don't match".into());
                    return Task::none();
                }
                if self.note_password_input.len() < 4 {
                    self.auth_error = Some("Password too short (min 4)".into());
                    return Task::none();
                }
                self.active_dialog = None;
                self.encrypting = true;
                let password = self.note_password_input.clone();
                self.note_password_input.clear();
                self.note_password_confirm.clear();
                let db_path = self.db_path.clone();
                let body_text = if self.selected_note.as_ref().map_or(false, |n| n.id == note_id) { self.line_editor.to_body() } else { String::new() };
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let body = if body_text.is_empty() { db::notes::get_note(&conn, note_id).map_err(|e| e.to_string())?.ok_or("Note not found")?.body } else { body_text };
                    let salt = crypto::key_derivation::generate_salt();
                    let derived = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let (ciphertext, nonce) = crypto::encryption::encrypt(&derived.key_bytes, body.as_bytes()).map_err(|e| e.to_string())?;
                    db::notes::update_note_encryption(&conn, note_id, &BASE64.encode(&ciphertext), true, Some(&BASE64.encode(nonce)), Some(&BASE64.encode(salt))).map_err(|e| e.to_string())?;
                    Ok(())
                }, |r: Result<(), String>| match r { Ok(()) => Message::EncryptionDone(Ok(())), Err(e) => Message::EncryptionDone(Err(e)) })
            }
            Message::SubmitDecrypt(note_id) => {
                self.active_dialog = None;
                self.encrypting = true;
                let password = self.note_password_input.clone();
                self.note_password_input.clear();
                let db_path = self.db_path.clone();
                // session-only: decrypt in memory, never write plaintext to db
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let note = db::notes::get_note(&conn, note_id).map_err(|e| e.to_string())?.ok_or("Note not found")?;
                    let (nonce_b64, salt_b64) = db::notes::get_encryption_meta(&conn, note_id).map_err(|e| e.to_string())?;
                    let nonce_bytes = BASE64.decode(&nonce_b64.ok_or("Missing nonce")?).map_err(|e| e.to_string())?;
                    let salt_bytes = BASE64.decode(&salt_b64.ok_or("Missing salt")?).map_err(|e| e.to_string())?;
                    let mut salt = [0u8; 16]; salt.copy_from_slice(&salt_bytes);
                    let mut nonce = [0u8; 12]; nonce.copy_from_slice(&nonce_bytes);
                    let derived = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let ciphertext = BASE64.decode(&note.body).map_err(|e| e.to_string())?;
                    let plaintext = crypto::encryption::decrypt(&derived.key_bytes, &nonce, &ciphertext).map_err(|e| e.to_string())?;
                    let body = String::from_utf8(plaintext).map_err(|e| e.to_string())?;
                    Ok((note_id, body, password.into_bytes()))
                }, |r: Result<(Uuid, String, Vec<u8>), String>| match r { Ok(v) => Message::DecryptionDone(Ok(v)), Err(e) => Message::DecryptionDone(Err(e)) })
            }
            Message::EncryptionDone(result) => {
                self.encrypting = false;
                if let Err(e) = result { self.auth_error = Some(e); }
                if let Some(ref n) = self.selected_note {
                    let id = n.id;
                    let db_path = self.db_path.clone();
                    return Task::batch([self.refresh_data(), Task::perform(async move { let conn = db::open_connection(&db_path).ok()?; db::notes::get_note(&conn, id).ok()? }, Message::NoteLoaded)]);
                }
                self.refresh_data()
            }
            Message::DecryptionDone(result) => {
                self.encrypting = false;
                match result {
                    Ok((id, body, pw_bytes)) => {
                        self.session_decrypted.insert(id, pw_bytes);
                        if let Some(ref mut n) = self.selected_note {
                            if n.id == id {
                                n.body = body.clone();
                                match n.note_type {
                                    NoteType::Password => {
                                        self.password_data = PasswordData::from_json(&body);
                                        self.password_notes_content = text_editor::Content::with_text(&self.password_data.notes);
                                        self.show_password = false;
                                    }
                                    NoteType::Canvas => {
                                        self.canvas_editor.load(&body);
                                    }
                                    NoteType::Text => {
                                        self.editor_content = text_editor::Content::with_text(&body);
                                    }
                                    NoteType::File => {} // file body is just metadata
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.auth_error = Some(e);
                    }
                }
                self.refresh_data()
            }

            Message::CursorMoved(wid, x, y) => {
                let x: f32 = x;
                let y: f32 = y;
                self.cursor_window = wid;
                if wid == self.focused_window {
                    self.cursor_pos = (x, y);
                    if let Some(edge) = self.resizing {
                        if self.is_maximized { return Task::none(); }
                        let (ww, wh) = self.window_size;
                        let min_w: f32 = 600.0;
                        let min_h: f32 = 400.0;
                        let (new_w, new_h) = match edge {
                            ResizeEdge::Right => (x.max(min_w), wh),
                            ResizeEdge::Bottom => (ww, y.max(min_h)),
                            ResizeEdge::BottomRight => (x.max(min_w), y.max(min_h)),
                            _ => (ww, wh),
                        };
                        if (new_w - ww).abs() > 1.0 || (new_h - wh).abs() > 1.0 {
                            self.window_size = (new_w, new_h);
                            return window::resize(wid, iced::Size::new(new_w, new_h));
                        }
                        return Task::none();
                    }
                    if let Some(ref item) = self.potential_drag {
                        let (sx, sy) = self.drag_start_pos;
                        let dist = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
                        if dist > 8.0 {
                            self.dragging = Some(item.clone());
                            self.potential_drag = None;
                        }
                    }
                }
                Task::none()
            }

            Message::AnimationTick => {
                self.canvas_editor.tick_animations();
                if let Some(t) = self.zoom_toast {
                    if t.elapsed() > Duration::from_millis(1200) { self.zoom_toast = None; }
                }
                if self.hovered_item.is_some() && self.dragging.is_some() {
                    self.hovered_item = None;
                }
                self.page_anim = 1.0;
                self.dialog_anim = if self.active_dialog.is_some() { 1.0 } else { 0.0 };
                if self.context_menu.is_some() && self.ctx_menu_anim < 1.0 {
                    self.ctx_menu_anim = 1.0;
                } else if self.context_menu.is_none() {
                    self.ctx_menu_anim = 0.0;
                }
                // retry focus until the rename input element exists
                if self.rename_pending > 0 {
                    self.rename_pending -= 1;
                    let id_str = if self.renaming_note.is_some() {
                        Some("inline_rename")
                    } else if self.renaming_folder.is_some() {
                        Some("inline_folder_rename")
                    } else {
                        self.rename_pending = 0;
                        None
                    };
                    if let Some(id_str) = id_str {
                        let id = iced::widget::text_input::Id::new(id_str);
                        return Task::batch([
                            iced::widget::text_input::focus(id.clone()),
                            iced::widget::text_input::select_all(id),
                        ]);
                    }
                }
                Task::none()
            }

            Message::CanvasAddNodeCenter => {
                self.canvas_editor.push_undo();
                let (cx, cy) = self.canvas_editor.viewport_center();
                let node = CanvasNode::new(cx - 80.0, cy - 24.0);
                self.canvas_editor.data.nodes.push(node);
                self.canvas_editor.sync_editors();
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::CanvasAddNode(x, y) => {
                self.canvas_editor.push_undo();
                self.canvas_editor.ctx_menu_info = None;
                let node = CanvasNode::new(x - 70.0, y - 22.0);
                self.canvas_editor.data.nodes.push(node);
                self.canvas_editor.sync_editors();
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::CanvasMoveNode(id, x, y) => {
                if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == id) {
                    node.x = x;
                    node.y = y;
                }

                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::CanvasSelect(id) => {
                self.canvas_editor.selected = id.into_iter().collect();
                self.canvas_editor.selected_edges.clear();

                Task::none()
            }
            Message::CanvasDeleteSelected => {
                self.canvas_editor.push_undo();
                self.canvas_editor.ctx_menu_info = None;
                let mut changed = false;
                if !self.canvas_editor.selected.is_empty() {
                    let sel = self.canvas_editor.selected.clone();
                    self.canvas_editor.data.nodes.retain(|n| !sel.contains(&n.id));
                    self.canvas_editor.data.edges.retain(|e| !sel.contains(&e.from) && !sel.contains(&e.to));
                    self.canvas_editor.selected.clear();
                    changed = true;
                }
                if !self.canvas_editor.selected_edges.is_empty() {
                    let sel_e = self.canvas_editor.selected_edges.clone();
                    self.canvas_editor.data.edges.retain(|e| !sel_e.contains(&e.id));
                    self.canvas_editor.selected_edges.clear();
                    changed = true;
                }
                if changed {
                    self.canvas_editor.sync_editors();
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                }
                Task::none()
            }
            Message::CanvasAddEdge(from, from_side, to, to_side) => {
                self.canvas_editor.push_undo();
                if !self.canvas_editor.data.edges.iter().any(|e| e.from == from && e.from_side == from_side && e.to == to && e.to_side == to_side) {
                    self.canvas_editor.data.edges.push(canvas_editor::CanvasEdge { id: uuid::Uuid::new_v4().to_string()[..8].to_string(), from, to, from_side, to_side });
    
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                }
                Task::none()
            }
            Message::CanvasCardEdit(card_id, action) => {
                // temporarily swap card editor into line_editor to reuse MdEdit handler
                if let Some(mut card_ed) = self.canvas_editor.card_editors.remove(&card_id) {
                    std::mem::swap(&mut self.line_editor, &mut card_ed);
                    let task = self.update(Message::MdEdit(action));
                    std::mem::swap(&mut self.line_editor, &mut card_ed);
                    self.canvas_editor.card_editors.insert(card_id.clone(), card_ed);
                    if let Some(editor) = self.canvas_editor.card_editors.get(&card_id) {
                        let body = editor.to_body();
                        if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == card_id) {
                            node.label = body;
                            let (_min_w, min_h) = node.min_size_for_label();
                            node.h = min_h; // auto-size both ways
                        }
                    }
                    task
                } else {
                    Task::none()
                }
            }
            Message::CanvasCardFocus(card_id) => {
                self.canvas_editor.ctx_menu_info = None;
                // snapshot before editing so canvas ctrl+z can revert text changes
                self.canvas_editor.push_undo();
                if let Some(ref old_id) = self.canvas_editor.focused_card {
                    if let Some(editor) = self.canvas_editor.card_editors.get_mut(old_id) {
                        editor.focused = false;
                    }
                }
                if let Some(editor) = self.canvas_editor.card_editors.get_mut(&card_id) {
                    editor.focused = true;
                    editor.focus_instant = Some(Instant::now());
                }
                self.canvas_editor.focused_card = Some(card_id);
                Task::none()
            }
            Message::CanvasCardUnfocus => {
                if let Some(ref old_id) = self.canvas_editor.focused_card {
                    // sync final text to node label before unfocusing
                    if let Some(editor) = self.canvas_editor.card_editors.get(old_id) {
                        let body = editor.to_body();
                        let oid = old_id.clone();
                        if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == oid) {
                            node.label = body;
                            let (_mw, mh) = node.min_size_for_label();
                            node.h = mh;
                        }
                    }
                    if let Some(editor) = self.canvas_editor.card_editors.get_mut(old_id) {
                        editor.focused = false;
                    }
                }
                self.canvas_editor.focused_card = None;
                Task::none()
            }
            Message::CanvasUndo => {
                self.canvas_editor.undo();
                self.editor_dirty = true;
                Task::none()
            }
            Message::CanvasRedo => {
                self.canvas_editor.redo();
                self.editor_dirty = true;
                Task::none()
            }
            Message::CanvasRecenter => {
                self.canvas_editor.recenter();
                Task::none()
            }
            Message::CanvasFitView => {
                self.canvas_editor.fit_view();
                Task::none()
            }
            Message::CanvasMoveNodeGroup(moves) => {
                for (id, x, y) in moves {
                    if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == id) {
                        node.x = x; node.y = y;
                    }
                }

                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::CanvasSelectEdge(id) => {
                self.canvas_editor.selected_edges = id.into_iter().collect();
                self.canvas_editor.selected.clear();

                Task::none()
            }
            Message::CanvasReverseEdge(id) => {
                if let Some(edge) = self.canvas_editor.data.edges.iter_mut().find(|e| e.id == id) {
                    std::mem::swap(&mut edge.from, &mut edge.to);
                    std::mem::swap(&mut edge.from_side, &mut edge.to_side);
    
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                }
                Task::none()
            }
            Message::CanvasCloseCtxMenu => {
                self.canvas_editor.ctx_menu_info = None;
                self.canvas_color_editing = None;
                Task::none()
            }
            Message::CanvasDeleteEdge(id) => {
                self.canvas_editor.data.edges.retain(|e| e.id != id);
                self.canvas_editor.selected_edges.clear();

                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::CanvasOpenColorPicker(id) => {
                if let Some(node) = self.canvas_editor.data.nodes.iter().find(|n| n.id == id) {
                    let c = node.parse_bg_color();
                    let max = c.r.max(c.g).max(c.b);
                    let min = c.r.min(c.g).min(c.b);
                    let d = max - min;
                    let v = max;
                    let s = if max > 0.0 { d / max } else { 0.0 };
                    let h = if d < 0.001 { 0.0 } else if (max - c.r).abs() < 0.001 {
                        60.0 * (((c.g - c.b) / d) % 6.0)
                    } else if (max - c.g).abs() < 0.001 {
                        60.0 * ((c.b - c.r) / d + 2.0)
                    } else {
                        60.0 * ((c.r - c.g) / d + 4.0)
                    };
                    self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                    self.color_sat = s * 100.0;
                    self.color_lit = v * 100.0;
                }
                self.canvas_color_editing = Some(id);
                Task::none()
            }
            Message::CanvasApplyColor => {
                self.canvas_editor.push_undo();
                if let Some(ref id) = self.canvas_color_editing {
                    let c = crate::ui::color_picker::hsv_to_rgb(self.color_hue, self.color_sat / 100.0, self.color_lit / 100.0);
                    let hex = format!("#{:02X}{:02X}{:02X}", (c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
                    if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == *id) {
                        node.bg_color = Some(hex);
        
                        self.editor_dirty = true;
                        self.last_edit_time = Some(Instant::now());
                    }
                }
                self.canvas_color_editing = None;
                Task::none()
            }
            Message::CanvasSetNodeBgColor(id, color) => {
                if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == id) {
                    node.bg_color = Some(color);
    
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                }
                Task::none()
            }
            Message::CanvasMultiSelect(ids) => {
                self.canvas_editor.selected = ids;

                Task::none()
            }
            Message::CanvasResizeNode(id, x, y, w, h) => {
                if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == id) {
                    node.x = x; node.y = y; node.w = w; node.h = h;
                    node.user_min_h = h;
                }

                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }

            Message::CanvasPan(dx, dy) => {
                self.canvas_editor.pan.0 += dx;
                self.canvas_editor.pan.1 += dy;
                Task::none()
            }
            Message::CanvasZoom(dy, mx, my) => {
                let old = self.canvas_editor.zoom;
                self.canvas_editor.zoom = (old * (1.0 + dy * 0.1)).clamp(0.3, 5.0);
                let f = self.canvas_editor.zoom / old;
                self.canvas_editor.pan.0 = mx - (mx - self.canvas_editor.pan.0) * f;
                self.canvas_editor.pan.1 = my - (my - self.canvas_editor.pan.1) * f;
                if self.canvas_editor.zoom < 0.55 && self.canvas_editor.focused_card.is_some() {
                    let _ = self.update(Message::CanvasCardUnfocus);
                }
                Task::none()
            }
            Message::CanvasViewportSize(w, h) => {
                self.canvas_editor.viewport_size = (w, h);
                Task::none()
            }
            Message::CanvasHover(id) => {
                self.canvas_editor.last_hovered = id;
                Task::none()
            }
            Message::CanvasShowCtxMenu(x, y, target) => {
                self.canvas_editor.ctx_menu_info = Some(crate::ui::canvas_editor::CanvasCtxMenu {
                    pos: (x, y),
                    target,
                });
                Task::none()
            }

            Message::ToggleGraphView => {
                self.show_graph = !self.show_graph;
                self.show_settings = false;
                Task::none()
            }
            Message::ShowSettings => {
                self.show_settings = !self.show_settings;
                self.show_graph = false;
                Task::none()
            }

            Message::SetFramerate(fps) => {
                self.setting_framerate = fps;
                self.save_setting("framerate", &fps.to_string())
            }
            Message::SetAutoSaveDelay(secs) => {
                self.setting_auto_save_delay = secs;
                self.save_setting("auto_save_delay", &secs.to_string())
            }
            Message::SetEditorFontSize(sz) => {
                self.setting_font_size = sz;
                self.save_setting("font_size", &sz.to_string())
            }
            Message::SetCanvasGridSize(sz) => {
                self.setting_grid_size = sz;
                self.save_setting("grid_size", &sz.to_string())
            }
            Message::ToggleAutoSave => {
                self.setting_auto_save = !self.setting_auto_save;
                self.save_setting("auto_save", if self.setting_auto_save { "true" } else { "false" })
            }
            Message::ToggleLineNumbers => {
                self.setting_line_numbers = !self.setting_line_numbers;
                self.save_setting("line_numbers", if self.setting_line_numbers { "true" } else { "false" })
            }

            Message::Refresh => self.refresh_data(),
            Message::DataLoaded(folders, notes, all_count, fav_count, folder_counts, subs, sub_notes) => {
                if self.vault_state == VaultState::Loading {
                    self.vault_state = VaultState::Unlocked;
                }
                self.folders = folders;
                self.notes = notes;
                self.all_count = all_count;
                self.fav_count = fav_count;
                self.folder_counts = folder_counts;
                self.subfolders = subs;
                self.subfolder_notes = sub_notes;
                self.apply_sort();
                if let Some(ref sel) = self.selected_note {
                    let sel_id = sel.id;
                    let in_notes = self.notes.iter().any(|n| n.id == sel_id);
                    let in_subs = self.subfolder_notes.iter().any(|(_, ns)| ns.iter().any(|n| n.id == sel_id));
                    if !in_notes && !in_subs {
                        self.selected_note = None;
                        self.editor_title.clear();
                        self.editor_content = text_editor::Content::new();
                        self.editor_dirty = false;
                    }
                }
                if !self.last_notes_loaded {
                    self.last_notes_loaded = true;
                    if let Ok(conn) = db::open_connection(&self.db_path) {
                        for view_key in ["all", "fav"].iter().map(|s| s.to_string()).chain(
                            self.folders.iter().map(|f| f.id.to_string())
                        ) {
                            let setting_key = format!("last_note_{}", view_key);
                            if let Some(val) = db::get_setting(&conn, &setting_key) {
                                if let Ok(id) = Uuid::parse_str(&val) {
                                    self.last_note_per_view.insert(view_key, id);
                                }
                            }
                        }
                    }
                    if let Some(note_id) = self.last_note_per_view.get(&Self::view_key(&self.active_view)).copied() {
                        let db_path = self.db_path.clone();
                        return Task::perform(
                            async move { let conn = db::open_connection(&db_path).ok()?; db::notes::get_note(&conn, note_id).ok()? },
                            Message::NoteLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::NoteLoaded(note_opt) => {
                if let Some(note) = note_opt {
                    self.line_editor.image_cache.clear();
                    self.line_editor.image_sizes.clear();
                    self.editor_title = note.title.clone();
                    if !note.is_encrypted {
                        match note.note_type {
                            NoteType::Password => {
                                self.password_data = PasswordData::from_json(&note.body);
                                self.password_notes_content = text_editor::Content::with_text(&self.password_data.notes);
                                self.show_password = false;
                            }
                            NoteType::Canvas => {
                                self.canvas_editor.load(&note.body);
                            }
                            _ => {}
                        }
                    }
                    self.editor_content = if note.is_encrypted {
                        text_editor::Content::new()
                    } else if note.note_type == NoteType::Text {
                        text_editor::Content::with_text(&note.body)
                    } else {
                        text_editor::Content::new()
                    };
                    if !note.is_encrypted && note.note_type == NoteType::Text {
                        if body_needs_image_migration(&note.body) {
                            // migrate legacy inline base64 images to encrypted storage
                            let body = note.body.clone();
                            let db_path = self.db_path.clone();
                            let note_id = note.id;
                            let Some(vk) = self.vault_key else { return Task::none() };
                            self.selected_note = Some(note);
                            return Task::perform(async move {
                                let (cleaned, to_migrate) = migrate_body_images(&body);
                                let mut loaded = Vec::new();
                                if let Ok(conn) = db::open_connection(&db_path) {
                                    for (id, fmt, b64) in &to_migrate {
                                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                                            let _ = db::save_image_encrypted(&conn, id, &bytes, fmt, &vk);
                                            loaded.push((id.clone(), bytes, fmt.clone()));
                                        }
                                    }
                                    let _ = conn.execute("UPDATE notes SET body = ? WHERE id = ?", rusqlite::params![cleaned, note_id.to_string()]);
                                }
                                (cleaned, loaded)
                            }, |(body, imgs)| Message::NoteMigrated(body, imgs));
                        } else {
                            self.line_editor = crate::ui::line_editor::LineEditorState::from_body(&note.body);
                            let (img_ids, _) = self.line_editor.collect_images();
                            if !img_ids.is_empty() {
                                let db_path = self.db_path.clone();
                                let Some(vk) = self.vault_key else { return Task::none() };
                                let load_task = Task::perform(async move {
                                    let mut results = Vec::new();
                                    if let Ok(conn) = db::open_connection(&db_path) {
                                        for id in &img_ids { if let Some((data, fmt)) = db::load_image_encrypted(&conn, id, &vk) { results.push((id.clone(), data, fmt)); } }
                                    }
                                    results
                                }, Message::ImagesLoaded);
                                self.selected_note = Some(note);
                                return load_task;
                            }
                        }
                    }
                    self.editor_dirty = false;
                    self.last_edit_time = None;
                    self.selected_note = Some(note);
                    self.page_anim = 0.0;
                }
                self.refresh_data()
            }
            Message::LoadingTick => { self.loading_tick += 1; Task::none() }
            Message::None => Task::none(),

            Message::OpenNewWindow => {
                let (wid, open_task) = window::open(window::Settings {
                    size: iced::Size::new(1100.0, 700.0),
                    min_size: Some(iced::Size::new(600.0, 400.0)),
                    decorations: false,
                    transparent: true,
                    resizable: true,
                    icon: self.window_icon.clone(),
                    ..Default::default()
                });
                let mut new_state = WindowState::new_default();
                new_state.active_view = ActiveView::AllNotes;
                self.other_windows.insert(wid, new_state);
                let db_path = self.db_path.clone();
                let load_task = Task::perform(async move {
                    let conn = match db::open_connection(&db_path) { Ok(c) => c, Err(_) => return (Vec::new(), Vec::new(), Vec::new(), Vec::new()) };
                    let folders = db::folders::list_folders(&conn).unwrap_or_default();
                    let notes = db::notes::list_previews(&conn, None, None).unwrap_or_default();
                    (folders, notes, Vec::<Folder>::new(), Vec::<(Uuid, Vec<NotePreview>)>::new())
                }, move |(_, _notes, _subs, _sub_notes)| {
                    Message::Refresh
                });
                Task::batch([open_task.map(|_| Message::None), load_task])
            }
            Message::WindowFocused(wid) => {
                self.switch_focus(wid)
            }
            #[allow(unreachable_code)]
            Message::WindowCloseRequested(_wid) => {
                let _ = self.maybe_save();
                std::process::exit(0);
                Task::none()
            }
            #[allow(unreachable_code)]
            Message::WindowClosed(wid) => {
                self.other_windows.remove(&wid);
                if wid == self.focused_window {
                    if let Some(&next_id) = self.other_windows.keys().next() {
                        let loaded = self.other_windows.remove(&next_id).unwrap();
                        self.restore_window_state(loaded);
                        self.focused_window = next_id;
                        Task::none()
                    } else {
                        std::process::exit(0);
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
        };

        if !had_dialog && self.active_dialog.is_some() {
            self.dialog_anim = 0.0;
        }

        result
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        if id != self.focused_window {
            return if let Some(win) = self.other_windows.get(&id) {
                self.view_stored_window(id, win)
            } else {
                Space::new(Length::Fill, Length::Fill).into()
            };
        }

        let content = match self.vault_state {
            VaultState::Setup => dialog::password_dialog::view_setup(&self.password_input, &self.confirm_password_input, self.auth_error.as_deref(), self.window_controls_hovered, self.is_maximized),
            VaultState::Login => dialog::password_dialog::view_login(&self.password_input, self.auth_error.as_deref(), self.window_controls_hovered, self.is_maximized, self.show_password),
            VaultState::Loading => self.view_loading(),
            VaultState::Unlocked => self.view_main(),
        };
        // 1px padding makes rounded corners visible against transparent bg
        let window_style = if self.is_maximized { theme::window_container_maximized as fn(&iced::Theme) -> container::Style } else { theme::window_container };
        let pad = if self.is_maximized { 0 } else { 1 };
        let main = container(
            container(content)
                .style(window_style)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            ..Default::default()
        })
        .padding(pad)
        .width(Length::Fill)
        .height(Length::Fill);

        if self.is_maximized {
            return main.into();
        }

        let (ww, wh) = self.window_size;
        let b: u16 = 5;
        let ww_u = (ww as u16).saturating_sub(b);
        let wh_u = (wh as u16).saturating_sub(b);

        let right_handle: Element<Message> = column![
            Space::with_height(0),
            row![
                Space::with_width(ww_u),
                mouse_area(Space::new(b, Length::Fill))
                    .on_press(Message::WindowResizeStart(ResizeEdge::Right))
                    .interaction(iced::mouse::Interaction::ResizingHorizontally),
            ].height(Length::Fill),
        ].into();

        let bottom_handle: Element<Message> = column![
            Space::with_height(wh_u),
            mouse_area(Space::new(Length::Fill, b))
                .on_press(Message::WindowResizeStart(ResizeEdge::Bottom))
                .interaction(iced::mouse::Interaction::ResizingVertically),
        ].into();

        let cb: u16 = 12;
        let corner_handle: Element<Message> = column![
            Space::with_height((wh as u16).saturating_sub(cb)),
            row![
                Space::with_width((ww as u16).saturating_sub(cb)),
                mouse_area(Space::new(cb, cb))
                    .on_press(Message::WindowResizeStart(ResizeEdge::BottomRight))
                    .interaction(iced::mouse::Interaction::ResizingDiagonallyDown),
            ],
        ].into();

        stack![main, right_handle, bottom_handle, corner_handle].into()
    }

    fn view_loading(&self) -> Element<'_, Message> {
        use iced::alignment::Horizontal;
        let dots = match self.loading_tick % 4 {
            0 => "   ",
            1 => ".  ",
            2 => ".. ",
            _ => "...",
        };

        let title_bar = mouse_area(
            container(
                row![
                    iced::widget::image(iced::widget::image::Handle::from_path("assets/logo.png")).width(16).height(16),
                    Space::with_width(Length::Fill),
                    container(Space::new(16, 16)).padding([4, 6]),
                    Space::with_width(8),
                    self.window_controls(),
                ].spacing(8).align_y(iced::Alignment::Center).padding([8, 10]),
            )
            .style({
                let maximized = self.is_maximized;
                move |_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border { radius: if maximized { 0.0.into() } else { iced::border::top(10.0) }, ..Default::default() },
                    ..Default::default()
                }
            })
            .width(Length::Fill),
        ).on_press(Message::WindowDrag);

        let content = column![
            Space::with_height(Length::Fill),
            text(format!("Unlocking{dots}")).size(16).style(|_t| theme::primary_text()).align_x(Horizontal::Center),
            text("Deriving encryption key").size(12).style(|_t| theme::secondary_text()).align_x(Horizontal::Center),
            Space::with_height(Length::Fill),
        ]
        .spacing(4)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

        column![
            title_bar,
            container(content)
                .style({
                    let maximized = self.is_maximized;
                    move |_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::BG_PRIMARY)),
                        border: iced::Border { radius: if maximized { 0.0.into() } else { iced::border::bottom(10.0) }, ..Default::default() },
                        ..Default::default()
                    }
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ].into()
    }

    fn view_main(&self) -> Element<'_, Message> {
        let lock_btn = button(svg(crate::ui::icons::lock_closed()).width(16).height(16))
            .on_press(Message::LockVault)
            .style(theme::icon_button)
            .padding([4, 6]);

        let title_bar = mouse_area(
            container(
                row![
                    iced::widget::image(iced::widget::image::Handle::from_path("assets/logo.png")).width(16).height(16),
                    Space::with_width(Length::Fill),
                    lock_btn,
                    Space::with_width(8),
                    self.window_controls(),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .padding([8, 10]),
            )
            .style({
                let maximized = self.is_maximized;
                move |_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border {
                        radius: if maximized { 0.0.into() } else { iced::border::top(10.0) },
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .width(Length::Fill),
        )
        .on_press(Message::WindowDrag)
        .on_enter(Message::HoverItem(None));

        let tags = tags_panel::view(
            &self.active_view,
            &self.folders,
            self.all_count,
            self.fav_count,
            &self.folder_counts,
            &self.context_menu,
            &self.dragging,
            self.renaming_folder,
            &self.folder_rename_buffer,
            self.hovered_item,
            self.is_maximized,
        );

        let is_folder_view = matches!(self.active_view, ActiveView::Folder(_));
        let active_folder_id = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
        let notes = notes_list::view(
            &self.notes,
            self.selected_note.as_ref().map(|n| n.id),
            &self.search_query,
            &self.context_menu,
            self.renaming_note,
            &self.rename_buffer,
            is_folder_view,
            &self.subfolders,
            &self.subfolder_notes,
            &self.expanded_folders,
            &self.dragging,
            active_folder_id,
            self.sort_mode,
            self.sort_menu_open,
            &self.multi_selected,
            self.renaming_folder,
            &self.folder_rename_buffer,
            self.ctrl_held,
            self.shift_held,
            &self.multi_selected_folders,
            self.hovered_item,
        );

        let right_panel: Element<Message> = {
            let panel = if self.show_settings {
                settings_view::view(self)
            } else if let Some(ref note) = self.selected_note {
                editor::view(note, &self.editor_title, &self.editor_content, self.editor_dirty, &self.password_data, &self.password_notes_content, self.show_password, self.show_password_gen, &self.password_gen_options, self.copied_field.as_deref(), &self.canvas_editor, &self.canvas_color_editing, self.color_hue, self.color_sat, self.color_lit, self.session_decrypted.contains_key(&note.id), self.encrypting, &self.note_password_input, self.auth_error.as_deref(), self.is_maximized, self.toolbar_move_open, &self.folders, self.setting_font_size, self.editor_search_open, &self.editor_search_query, self.editor_search_index, self.editor_search_case_sensitive, &self.line_editor)
            } else {
                empty_state::view(self.is_maximized)
            };
            mouse_area(panel).on_enter(Message::HoverItem(None)).into()
        };

        let right_panel: Element<Message> = if !self.file_transfers.is_empty() {
            use iced::widget::{stack, container, column, row, text, Space};
            use std::sync::atomic::Ordering;
            let bar_w = 200.0;
            let count = self.file_transfers.len();

            let toast_content: Element<Message> = if count <= 3 {
                let mut toasts = column![].spacing(4);
                for (_id, label, progress_arc) in &self.file_transfers {
                    let pct = progress_arc.load(Ordering::Relaxed) as f32 / 1000.0;
                    let fill_w = (bar_w * pct).max(2.0);
                    let pct_text = format!("{}%", (pct * 100.0) as u32);
                    let toast = container(
                        column![
                            row![
                                text(label.clone()).size(11).style(|_t| theme::primary_text()),
                                Space::with_width(iced::Length::Fill),
                                text(pct_text).size(10).style(|_t| theme::secondary_text()),
                            ].align_y(iced::Alignment::Center),
                            stack![
                                container(Space::new(bar_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(theme::BG_TERTIARY)),
                                    border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                                }),
                                container(Space::new(fill_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(iced::Color::from_rgb(0.18, 0.55, 0.31))),
                                    border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                                }),
                            ]
                        ].spacing(5)
                    )
                    .style(|_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                        border: iced::Border { radius: 8.0.into(), ..Default::default() }, ..Default::default()
                    })
                    .padding([10, 14]).width(230);
                    toasts = toasts.push(toast);
                }
                toasts.into()
            } else {
                let total_pct: f32 = self.file_transfers.iter()
                    .map(|(_, _, p)| p.load(Ordering::Relaxed) as f32 / 1000.0)
                    .sum::<f32>() / count as f32;
                let fill_w = (bar_w * total_pct).max(2.0);
                let done = self.file_transfers.iter().filter(|(_, _, p)| p.load(Ordering::Relaxed) >= 1000).count();
                let label = format!("Processing {} files ({}/{})", count, done, count);

                container(
                    column![
                        row![
                            text(label).size(11).style(|_t| theme::primary_text()),
                            Space::with_width(iced::Length::Fill),
                            text(format!("{}%", (total_pct * 100.0) as u32)).size(10).style(|_t| theme::secondary_text()),
                        ].align_y(iced::Alignment::Center),
                        stack![
                            container(Space::new(bar_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(theme::BG_TERTIARY)),
                                border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                            }),
                            container(Space::new(fill_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(iced::Color::from_rgb(0.18, 0.55, 0.31))),
                                border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                            }),
                        ]
                    ].spacing(5)
                )
                .style(|_t: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() }, ..Default::default()
                })
                .padding([10, 14]).width(230).into()
            };

            let overlay = container(
                container(toast_content).padding([8, 8])
            )
            .width(iced::Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top);

            stack![right_panel, overlay].into()
        } else {
            right_panel
        };

        let content_row: Element<Message> = if self.show_sidebar {
            row![tags, notes, right_panel].into()
        } else {
            row![right_panel].into()
        };
        let mut main_layout: Element<Message> = column![
            title_bar,
            content_row,
        ].into();

        if let Some(ref drag_item) = self.dragging {
            let (label, ghost_color) = match drag_item {
                DragItem::Note(id) => {
                    let n = self.notes.iter().find(|n| n.id == *id);
                    let multi = self.multi_selected.len() + self.multi_selected_folders.len();
                    let name = if multi > 1 && self.multi_selected.contains(id) {
                        format!("{} items", multi)
                    } else {
                        n.map(|n| if n.title.is_empty() { "Untitled".to_string() } else { n.title.clone() }).unwrap_or_default()
                    };
                    let color = n.map(|n| n.color.to_iced_color()).unwrap_or(iced::Color::from_rgb(0.3, 0.3, 0.35));
                    (name, color)
                }
                DragItem::Folder(id) => {
                    let f = self.folders.iter().chain(self.subfolders.iter()).find(|f| f.id == *id);
                    let multi = self.multi_selected.len() + self.multi_selected_folders.len();
                    let name = if multi > 1 && self.multi_selected_folders.contains(id) {
                        format!("{} items", multi)
                    } else {
                        f.map(|f| f.name.clone()).unwrap_or_default()
                    };
                    let color = f.map(|f| f.color.to_iced_color()).unwrap_or(iced::Color::from_rgb(0.3, 0.3, 0.35));
                    (name, color)
                }
            };
            let (cx, cy) = self.cursor_pos;
            let ghost = container(
                text(label).size(11).style(|_t| iced::widget::text::Style { color: Some(iced::Color::WHITE) })
            )
            .style(move |_t: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(ghost_color.r, ghost_color.g, ghost_color.b, 0.85))),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([3, 8]);

            let ghost_positioned: Element<Message> = column![
                Space::with_height(cy.max(0.0) as u16),
                row![Space::with_width(cx.max(0.0) as u16 + 10), container(ghost).width(Length::Shrink)].width(Length::Shrink),
            ].into();

            main_layout = stack![main_layout, ghost_positioned].into();
        }

        let mut result: Element<Message> = if let Some(ref dialog_kind) = self.active_dialog {
            let dialog_view = self.view_dialog(dialog_kind);
            let da = self.dialog_anim;
            let blocking_overlay: Element<Message> = mouse_area(
                container(
                    container(Space::new(Length::Fill, Length::Fill))
                        .style(move |_t: &iced::Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35 * da))),
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .height(Length::Fill)
                ).width(Length::Fill).height(Length::Fill)
            )
            .on_press(Message::CloseDialog)
            .on_right_press(Message::CloseDialog)
            .on_enter(Message::HoverItem(None))
            .into();
            stack![main_layout, stack![blocking_overlay, dialog_view]].into()
        } else if let Some(ref ctx) = self.context_menu {
            let ctx_overlay = self.view_context_menu(ctx);
            stack![main_layout, ctx_overlay].into()
        } else {
            main_layout
        };

        if let Some(toast_time) = self.zoom_toast {
            if toast_time.elapsed() < Duration::from_millis(1200) {
                let opacity = if toast_time.elapsed() > Duration::from_millis(800) {
                    1.0 - (toast_time.elapsed().as_millis() as f32 - 800.0) / 400.0
                } else { 1.0 };
                let zoom_label = format!("{}%", (self.gui_scale * 100.0).round() as u32);
                let toast_pill = container(
                    text(zoom_label).size(11).style(move |_t| iced::widget::text::Style {
                        color: Some(iced::Color::from_rgba(
                            0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0, opacity
                        )),
                    })
                )
                .style(move |_t: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0.9 * opacity
                    ))),
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding([5, 10]);
                let toast: Element<Message> = column![
                    Space::with_height(42),
                    row![Space::with_width(Length::Fill), toast_pill, Space::with_width(12)],
                ].into();
                result = stack![result, toast].into();
            }
        }
        result
    }

    /// Render a context menu as a floating overlay.
    /// Simple floating context menu — small card, no clutter.
    fn view_context_menu(&self, ctx: &ContextMenu) -> Element<'_, Message> {
        use crate::ui::icons;

        let menu_content: Element<Message> = match ctx {
            ContextMenu::Tag(folder_id) => {
                let fid = *folder_id;
                let multi_count = self.multi_selected.len() + self.multi_selected_folders.len();
                if multi_count > 1 && self.multi_selected_folders.contains(&fid) {
                    let label = format!("{} items selected", multi_count);
                    container(column![
                        text(label).size(11).style(|_t| theme::secondary_text()).width(Length::Fill).align_x(iced::alignment::Horizontal::Center),
                        ctx_btn(icons::move_folder_icon(), "Move  >", Message::OpenMoveFolderPicker(fid)),
                        ctx_btn_danger(icons::trash_danger(), "Delete all", Message::OpenDeleteMultiDialog),
                    ].spacing(1).padding(4))
                    .style(theme::context_menu_container)
                    .max_width(170)
                    .into()
                } else {
                    let is_fav = self.folders.iter().chain(self.subfolders.iter()).find(|f| f.id == fid).map_or(false, |f| f.is_favorite);
                    let fav_icon = if is_fav { icons::star_filled() } else { icons::star_outline() };
                    let fav_label = if is_fav { "Unfavorite" } else { "Favorite" };
                    container(column![
                        ctx_btn_hover(icons::plus_icon(), "New  \u{203A}", Message::OpenNewNoteSubmenu(fid), Message::ToggleNewNoteSubmenu(fid)),
                        ctx_btn(fav_icon, fav_label, Message::ToggleFolderFavorite(fid)),
                        ctx_btn(icons::pencil_icon(), "Rename", Message::RenameFolderInline(fid)),
                        ctx_btn_hover(icons::palette_icon(), "Color  \u{203A}", Message::OpenFolderColorSubmenu(fid), Message::ToggleFolderColorSubmenu(fid)),
                        ctx_btn_hover(icons::move_folder_icon(), "Move  \u{203A}", Message::OpenMoveSubmenu(fid), Message::OpenMoveFolderPicker(fid)),
                        ctx_btn_danger(icons::trash_danger(), "Delete", Message::OpenDeleteFolderDialog(fid)),
                    ].spacing(1).padding(4))
                    .style(theme::context_menu_container)
                    .max_width(170)
                    .into()
                }
            }
            ContextMenu::NoteItem(note_id) => {
                let nid = *note_id;
                let multi_count = self.multi_selected.len() + self.multi_selected_folders.len();

                if multi_count > 1 && (self.multi_selected.contains(&nid) || self.multi_selected_folders.contains(&nid)) {
                    let label = format!("{} items selected", multi_count);
                    let has_files = self.multi_selected.iter().any(|id| {
                        self.notes.iter().any(|n| n.id == *id && n.note_type == NoteType::File)
                    });
                    let mut items = column![
                        text(label).size(11).style(|_t| theme::secondary_text()).width(Length::Fill).align_x(iced::alignment::Horizontal::Center),
                    ].spacing(1);
                    if has_files {
                        items = items.push(ctx_btn(icons::save_icon(), "Save files", Message::FileExportSelected));
                    }
                    items = items.push(ctx_btn(icons::move_folder_icon(), "Move  >", Message::OpenMoveFolderPicker(nid)));
                    items = items.push(ctx_btn_danger(icons::trash_danger(), "Delete all", Message::OpenDeleteMultiDialog));
                    container(items.padding(4))
                    .style(theme::context_menu_container)
                    .max_width(170)
                    .into()
                } else {

                let preview = self.notes.iter().find(|n| n.id == nid);
                let is_encrypted = preview.map_or(false, |p| p.is_encrypted);
                let is_session_dec = self.session_decrypted.contains_key(&nid);
                let is_file_note = preview.map_or(false, |p| p.note_type == NoteType::File);

                let mut items = column![
                    ctx_btn(icons::pencil_icon(), "Rename", Message::RenameNote(nid)),
                    ctx_btn_hover(icons::palette_icon(), "Color  \u{203A}", Message::OpenColorSubmenu(nid), Message::ToggleColorSubmenu(nid)),
                    ctx_btn_hover(icons::move_folder_icon(), "Move  \u{203A}", Message::OpenMoveSubmenu(nid), Message::OpenMoveFolderPicker(nid)),
                ].spacing(1);

                if is_file_note {
                    let file_info = self.selected_note.as_ref()
                        .filter(|n| n.id == nid)
                        .and_then(|n| crate::ui::file_viewer::parse_file_body(&n.body));
                    if let Some((fid, fname, _size)) = file_info {
                        items = items.push(ctx_btn(icons::save_icon(), "Save", Message::FileExport(fid.clone(), fname.clone())));
                        items = items.push(ctx_btn(icons::save_icon(), "Save As", Message::FileExportAs(fid, fname)));
                    } else {
                        let title = preview.map(|p| p.title.clone()).unwrap_or_default();
                        items = items.push(ctx_btn(icons::save_icon(), "Save", Message::SelectNote(nid)));
                    }
                }

                if is_encrypted && is_session_dec {
                    items = items.push(ctx_btn(icons::lock_closed(), "Lock", Message::LockNote));
                } else if is_encrypted {
                    items = items.push(ctx_btn(icons::lock_active(), "Unlock", Message::SelectNote(nid)));
                } else if !is_file_note {
                    items = items.push(ctx_btn(icons::lock_closed(), "Encrypt", Message::OpenEncryptDialog(nid)));
                }

                items = items.push(ctx_btn_danger(icons::trash_danger(), "Delete", Message::OpenDeleteNoteDialog(nid)));

                container(items.padding(4))
                .style(theme::context_menu_container)
                .max_width(190)
                .into()
                } // end else
            }
            ContextMenu::TagsEmpty => {
                container(column![
                    ctx_btn(icons::note_text_icon(theme::TEXT_SECONDARY), "New text note", Message::CreateQuickNote(NoteType::Text)),
                    ctx_btn(icons::note_password_icon(theme::TEXT_SECONDARY), "New password", Message::CreateQuickNote(NoteType::Password)),
                    ctx_btn(icons::note_canvas_icon(theme::TEXT_SECONDARY), "New canvas", Message::CreateQuickNote(NoteType::Canvas)),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(170)
                .into()
            }
            ContextMenu::NotesEmpty => {
                let parent = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
                let mut items = column![
                    ctx_btn(icons::note_text_icon(theme::TEXT_SECONDARY), "New text note", Message::CreateQuickNote(NoteType::Text)),
                    ctx_btn(icons::note_password_icon(theme::TEXT_SECONDARY), "New password", Message::CreateQuickNote(NoteType::Password)),
                    ctx_btn(icons::note_canvas_icon(theme::TEXT_SECONDARY), "New canvas", Message::CreateQuickNote(NoteType::Canvas)),
                ].spacing(1);
                if let Some(pid) = parent {
                    items = items.push(ctx_btn(icons::folder_icon(), "New folder", Message::CreateQuickFolder(Some(pid))));
                }
                container(items.padding(4))
                .style(theme::context_menu_container)
                .max_width(170)
                .into()
            }
            ContextMenu::NoteColor(_) => {
                container(Space::new(0, 0)).into()
            }
            ContextMenu::EditorFormat => {
                container(column![
                    ctx_btn(icons::fmt_bold(), "Bold", Message::FormatBold),
                    ctx_btn(icons::fmt_heading(), "Heading", Message::FormatHeading),
                    ctx_btn(icons::fmt_list(), "List", Message::FormatList),
                    ctx_btn(icons::fmt_checkbox(), "Checkbox", Message::FormatCheckbox),
                    ctx_btn(icons::fmt_code(), "Code", Message::FormatCode),
                    ctx_btn(icons::fmt_divider(), "Divider", Message::FormatDivider),
                    ctx_btn(icons::palette_icon(), "Text color", Message::OpenTextColorPicker),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(200)
                .into()
            }
            ContextMenu::FileMenu(line_idx) => {
                let li = *line_idx;
                let (file_id, filename) = if li < self.line_editor.lines.len() {
                    let t = self.line_editor.lines[li].trim();
                    let inner = &t[1..t.len().saturating_sub(1)]; // strip []
                    let after = &inner[5..]; // skip "file:"
                    if let Some(c1) = after.find(':') {
                        let fid = after[..c1].to_string(); // just the UUID
                        let rest = &after[c1+1..];
                        if let Some(c2) = rest.rfind(':') {
                            (fid, rest[..c2].to_string())
                        } else { (fid, rest.to_string()) }
                    } else { (String::new(), String::new()) }
                } else { (String::new(), String::new()) };
                container(column![
                    ctx_btn(icons::save_icon(), "Export file", Message::MdEdit(crate::ui::md_widget::MdAction::FileExport(file_id.clone(), filename))),
                    ctx_btn(icons::trash_danger(), "Delete attachment", Message::MdEdit(crate::ui::md_widget::MdAction::FileDelete(file_id))),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(200)
                .into()
            }
            ContextMenu::ImageMenu(line_idx) => {
                let li = *line_idx;
                container(column![
                    ctx_btn(icons::copy_icon(), "Copy image", Message::CopyImage(li)),
                    ctx_btn(icons::trash_muted(), "Delete image", Message::MdEdit(crate::ui::md_widget::MdAction::ImageDelete(li))),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(200)
                .into()
            }
            ContextMenu::TableCell(line_idx) => {
                let li = *line_idx;
                container(column![
                    ctx_btn(icons::plus_icon(), "Add row", Message::MdEdit(crate::ui::md_widget::MdAction::TableAddRow(li))),
                    ctx_btn(icons::plus_icon(), "Add column", Message::MdEdit(crate::ui::md_widget::MdAction::TableAddCol(li))),
                    ctx_btn(icons::trash_muted(), "Delete row", Message::MdEdit(crate::ui::md_widget::MdAction::TableDeleteRow(li))),
                    ctx_btn(icons::trash_muted(), "Delete column", Message::MdEdit(crate::ui::md_widget::MdAction::TableDeleteCol(li))),
                    ctx_btn(icons::trash_danger(), "Delete table", Message::MdEdit(crate::ui::md_widget::MdAction::TableDelete(li))),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(200)
                .into()
            }
        };

        let backdrop: Element<Message> = {
            mouse_area(container(Space::new(Length::Fill, Length::Fill)))
                .on_press(Message::CloseContextMenu)
                .on_right_press(Message::CloseContextMenu)
                .on_enter(Message::HoverItem(None))
                .into()
        };

        let item_count = match ctx {
            ContextMenu::Tag(_) => 6,
            ContextMenu::NoteItem(_) => 7, // may have extra file buttons
            ContextMenu::NoteColor(_) => 7,
            ContextMenu::TagsEmpty => 3,
            ContextMenu::NotesEmpty => 4,
            ContextMenu::EditorFormat => 7,
            ContextMenu::ImageMenu(_) => 2,
            ContextMenu::FileMenu(_) => 2,
            ContextMenu::TableCell(_) => 5,
        };
        let menu_h = (item_count as f32) * 32.0 + 12.0;
        let menu_w = 170.0;

        let (cx, cy) = self.context_menu_pos;
        let (screen_w, screen_h) = (self.window_size.0 / self.gui_scale as f32, self.window_size.1 / self.gui_scale as f32);
        let final_x = if cx + menu_w > screen_w { (cx - menu_w).max(0.0) } else { cx };
        let final_y = if cy + menu_h > screen_h { (cy - menu_h).max(0.0) } else { cy };
        let mx = final_x.max(0.0) as u16;
        let my = final_y.max(0.0) as u16;

        let side_panel: Option<Element<Message>> = if let Some(folder_id) = self.new_note_submenu_for {
            let mut new_items = column![].spacing(1);
            new_items = new_items.push(
                button(row![svg(icons::note_text_icon(theme::TEXT_SECONDARY)).width(14).height(14), text("Text note").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                .on_press(Message::CreateNoteInFolder(NoteType::Text, folder_id)).style(theme::context_menu_button).padding([5, 10]).width(Length::Fill)
            );
            new_items = new_items.push(
                button(row![svg(icons::note_password_icon(theme::TEXT_SECONDARY)).width(14).height(14), text("Password").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                .on_press(Message::CreateNoteInFolder(NoteType::Password, folder_id)).style(theme::context_menu_button).padding([5, 10]).width(Length::Fill)
            );
            new_items = new_items.push(
                button(row![svg(icons::note_canvas_icon(theme::TEXT_SECONDARY)).width(14).height(14), text("Canvas").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                .on_press(Message::CreateNoteInFolder(NoteType::Canvas, folder_id)).style(theme::context_menu_button).padding([5, 10]).width(Length::Fill)
            );
            new_items = new_items.push(
                button(row![svg(icons::folder_icon()).width(14).height(14), text("Subfolder").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                .on_press(Message::CreateQuickFolder(Some(folder_id))).style(theme::context_menu_button).padding([5, 10]).width(Length::Fill)
            );
            Some(container(new_items.padding(4))
            .style(theme::context_menu_container)
            .width(160)
            .into())
        } else if let Some(_color_nid) = self.color_submenu_for {
            use crate::ui::color_picker;
            let picker = color_picker::view(self.color_hue, self.color_sat, self.color_lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);
            Some(container(
                column![picker].spacing(2).padding(12)
            )
            .style(theme::context_menu_container)
            .width(240)
            .into())
        } else if let Some(move_nid) = self.move_submenu_for {
            let is_multi = self.multi_selected.len() + self.multi_selected_folders.len() > 1;
            let current_folder = self.selected_note.as_ref().and_then(|n| n.folder_id);
            let mut move_items = column![].spacing(1);

            let none_selected = current_folder.is_none();
            let none_bg = if none_selected { theme::BG_TERTIARY } else { theme::TRANSPARENT };
            move_items = move_items.push(
                button(
                    row![svg(icons::folder_icon()).width(14).height(14), text("No Folder").size(12).style(|_t| theme::primary_text())]
                        .spacing(8).align_y(iced::Alignment::Center)
                )
                .on_press(if is_multi { Message::MoveMultiSelectedToFolder(None) } else { Message::MoveNoteToFolder(move_nid, None) })
                .style(move |_t: &iced::Theme, status: button::Status| {
                    let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => none_bg };
                    button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                })
                .padding([6, 10]).width(Length::Fill)
            );

            for f in &self.folders {
                if f.parent_id.is_some() { continue; }
                let fid = f.id;
                let is_current = current_folder == Some(fid);
                let folder_color = f.color.to_iced_color();
                let current_bg = if is_current { theme::BG_TERTIARY } else { theme::TRANSPARENT };

                move_items = move_items.push(
                    button(
                        row![
                            svg(icons::folder_colored(folder_color)).width(14).height(14),
                            text(&f.name).size(12).style(|_t| theme::primary_text()),
                        ].spacing(8).align_y(iced::Alignment::Center)
                    )
                    .on_press(if is_multi { Message::MoveMultiSelectedToFolder(Some(fid)) } else { Message::MoveNoteToFolder(move_nid, Some(fid)) })
                    .style(move |_t: &iced::Theme, status: button::Status| {
                        let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => current_bg };
                        button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                    })
                    .padding([6, 10]).width(Length::Fill)
                );
            }

            Some(container(
                iced::widget::scrollable(move_items.padding(4))
                    .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
                    .style(theme::dark_scrollable)
            )
            .style(theme::context_menu_container)
            .width(180)
            .max_height(300)
            .into())
        } else {
            None
        };

        let align = if self.move_submenu_for.is_some() { iced::Alignment::End } else { iced::Alignment::Start };
        let menu_row: Element<Message> = if let Some(panel) = side_panel {
            row![
                container(menu_content).width(Length::Shrink),
                Space::with_width(4),
                panel,
            ].width(Length::Shrink).align_y(align).into()
        } else {
            container(menu_content).width(Length::Shrink).into()
        };

        let positioned_menu: Element<Message> = column![
            Space::with_height(my),
            row![
                Space::with_width(mx),
                mouse_area(menu_row).on_press(Message::None).on_right_press(Message::None),
            ].width(Length::Shrink),
        ].into();

        stack![backdrop, positioned_menu].into()
    }

    fn view_dialog(&self, kind: &DialogKind) -> Element<'_, Message> {
        match kind {
            DialogKind::CreateNote => create_dialog::view(
                &self.create_dialog_title,
                self.create_dialog_type,
                self.create_dialog_color,
                self.create_dialog_folder,
                &self.folders,
                self.color_hue,
                self.color_sat,
                self.color_lit,
            ),
            DialogKind::EncryptNote(id) => dialog::password_dialog::view_encrypt(*id, &self.note_password_input, &self.note_password_confirm, self.auth_error.as_deref()),
            DialogKind::DecryptNote(id) => dialog::password_dialog::view_decrypt(*id, &self.note_password_input, self.auth_error.as_deref()),
            DialogKind::CreateFolder => dialog::folder_dialog::view("New Folder", &self.folder_name_input, self.folder_color_input, Message::CreateFolder, self.color_hue, self.color_sat, self.color_lit),
            DialogKind::RenameFolder(id) => dialog::folder_dialog::view("Rename Folder", &self.folder_name_input, self.folder_color_input, Message::RenameFolder(*id), self.color_hue, self.color_sat, self.color_lit),
            DialogKind::DeleteNote(id) => dialog::confirm_dialog::view("Delete Note", "Are you sure? This cannot be undone.", Message::DeleteNote(*id)),
            DialogKind::DeleteFolder(id) => dialog::confirm_dialog::view("Delete Folder", "Notes will be moved to All Notes.", Message::DeleteFolder(*id)),
            DialogKind::MoveFolderPicker(note_id) => self.view_move_folder_picker(*note_id),
            DialogKind::NoteColor(note_id) => self.view_note_color_picker(*note_id),
            DialogKind::ChangePassword(id) => dialog::password_dialog::view_change_password(*id, &self.note_password_input, &self.note_password_confirm, self.auth_error.as_deref()),
            DialogKind::DeleteMultiConfirm => {
                let count = self.multi_selected.len() + self.multi_selected_folders.len();
                dialog::confirm_dialog::view("Delete Items", &format!("Delete {} selected items? This cannot be undone.", count), Message::DeleteMultiSelected)
            }
            DialogKind::ChangeVaultPassword => {
                Space::new(0, 0).into()
            }
            DialogKind::TextColor => {
                use iced::alignment::Horizontal;
                let picker = crate::ui::color_picker::view(self.color_hue, self.color_sat, self.color_lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);
                let card = container(
                    column![
                        text("Text Color").size(16).style(|_t| theme::primary_text()),
                        Space::with_height(8),
                        picker,
                        Space::with_height(8),
                        row![
                            button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
                                .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
                            button(text("Apply").size(13).align_x(Horizontal::Center).width(Length::Fill))
                                .on_press(Message::ApplyTextColor).style(theme::submit_button).padding([8, 16]).width(Length::Fill),
                        ].spacing(8),
                    ].spacing(4).padding(20),
                ).style(theme::dialog_card).width(320);
                container(card).style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill).into()
            }
        }
    }

    fn view_note_color_picker(&self, note_id: Uuid) -> Element<'_, Message> {
        use iced::alignment::Horizontal;
        use iced::widget::{button, container, text, Space};
        use crate::ui::color_picker;

        let picker = color_picker::view(self.color_hue, self.color_sat, self.color_lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);

        let card = container(
            column![
                text("Note Color").size(18).style(|_t| theme::primary_text()),
                Space::with_height(10),
                picker,
                Space::with_height(10),
                row![
                    button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
                        .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
                    button(text("Apply").size(13).align_x(Horizontal::Center).width(Length::Fill))
                        .on_press(Message::ApplyNoteColor(note_id)).style(theme::submit_button).padding([8, 16]).width(Length::Fill),
                ].spacing(8),
            ].spacing(4).max_width(320).padding(20),
        ).style(theme::dialog_card);

        container(container(card).center_x(Length::Fill).center_y(Length::Fill))
            .style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).into()
    }

    fn view_move_folder_picker(&self, note_id: Uuid) -> Element<'_, Message> {
        use iced::widget::{button, container, text, Space};

        let is_multi = self.multi_selected.len() + self.multi_selected_folders.len() > 1;
        let current_folder = self.selected_note.as_ref().and_then(|n| n.folder_id);

        let mut items = iced::widget::column![].spacing(1);

        let none_selected = current_folder.is_none();
        let none_bg = if none_selected { theme::BG_TERTIARY } else { theme::TRANSPARENT };
        items = items.push(
            button(
                row![
                    svg(icons::folder_icon()).width(14).height(14),
                    text("No Folder").size(12).style(|_t| theme::primary_text()),
                ].spacing(8).align_y(iced::Alignment::Center)
            )
            .on_press(if is_multi { Message::MoveMultiSelectedToFolder(None) } else { Message::MoveNoteToFolder(note_id, None) })
            .style(move |_t: &iced::Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => theme::BG_HOVER,
                    _ => none_bg,
                };
                button::Style {
                    background: Some(iced::Background::Color(bg)),
                    border: iced::Border { radius: 6.0.into(), ..Default::default() },
                    text_color: theme::TEXT_PRIMARY,
                    ..Default::default()
                }
            })
            .padding([7, 10])
            .width(Length::Fill)
        );

        items = items.push(
            container(Space::new(Length::Fill, 1))
                .style(|_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
                    ..Default::default()
                })
                .padding([3, 8])
        );

        for f in &self.folders {
            if f.parent_id.is_some() { continue; } // Only show root folders
            let fid = f.id;
            let is_current = current_folder == Some(fid);
            let folder_color = f.color.to_iced_color();
            let current_bg = if is_current { theme::BG_TERTIARY } else { theme::TRANSPARENT };

            items = items.push(
                button(
                    row![
                        svg(icons::folder_colored(folder_color)).width(14).height(14),
                        text(&f.name).size(12).style(|_t| theme::primary_text()),
                        iced::widget::horizontal_space(),
                        {
                            let marker: Element<Message> = if is_current {
                                svg(icons::pin_filled()).width(10).height(10).into()
                            } else {
                                Space::new(0, 0).into()
                            };
                            marker
                        },
                    ].spacing(8).align_y(iced::Alignment::Center)
                )
                .on_press(if is_multi { Message::MoveMultiSelectedToFolder(Some(fid)) } else { Message::MoveNoteToFolder(note_id, Some(fid)) })
                .style(move |_t: &iced::Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => theme::BG_HOVER,
                        _ => current_bg,
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg)),
                        border: iced::Border { radius: 6.0.into(), ..Default::default() },
                        text_color: theme::TEXT_PRIMARY,
                        ..Default::default()
                    }
                })
                .padding([7, 10])
                .width(Length::Fill)
            );
        }

        let dropdown = container(
            iced::widget::scrollable(items.padding(4))
                .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
                .style(theme::dark_scrollable)
        )
        .style(|_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SECONDARY)),
            border: iced::Border {
                radius: 10.0.into(),
                width: 1.0,
                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            },
            ..Default::default()
        })
        .width(240)
        .max_height(350);

        let backdrop: Element<Message> = mouse_area(
            container(Space::new(Length::Fill, Length::Fill)),
        )
        .on_press(Message::CloseDialog)
        .into();

        let positioned = container(dropdown)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        stack![backdrop, positioned].into()
    }

    fn window_controls(&self) -> Element<'_, Message> {
        let h = self.window_controls_hovered;
        let min_content: Element<Message> = if h {
            svg(icons::win_minimize()).width(8).height(8).into()
        } else {
            Space::new(12, 12).into()
        };
        let max_content: Element<Message> = if h {
            svg(icons::win_maximize()).width(8).height(8).into()
        } else {
            Space::new(12, 12).into()
        };
        let close_content: Element<Message> = if h {
            svg(icons::win_close()).width(8).height(8).into()
        } else {
            Space::new(12, 12).into()
        };
        let p = if h { 2 } else { 0 };
        let controls = row![
            button(min_content).on_press(Message::WindowMinimize)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0xE5, 0xD5, 0x4D), false)).padding(p),
            button(max_content).on_press(Message::WindowMaximize)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0x4D, 0xC8, 0x6A), false)).padding(p),
            button(close_content).on_press(Message::WindowClose)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0xE5, 0x4D, 0x4D), false)).padding(p),
        ].spacing(8).align_y(iced::Alignment::Center);

        mouse_area(controls)
            .on_enter(Message::WindowControlsHover(true))
            .on_exit(Message::WindowControlsHover(false))
            .into()
    }


    /// Move cursor to a search match at (line, col) and scroll to it.
    fn jump_to_search_match(&mut self, line: usize, col: usize) {
        self.line_editor.cursor = (line, col);
        self.line_editor.selection = None;
        self.line_editor.focused = true;

        let fs = self.setting_font_size as f32;
        let mut y = 0.0f32;
        for li in 0..line {
            if li < self.line_editor.lines.len() {
                y += crate::ui::md_widget::line_height(&self.line_editor.lines[li], fs);
            }
        }
        let match_h = if line < self.line_editor.lines.len() {
            crate::ui::md_widget::line_height(&self.line_editor.lines[line], fs)
        } else { fs * 1.3 };
        if y < self.line_editor.scroll_offset {
            self.line_editor.scroll_offset = y;
        }
        let viewport_h = 400.0;
        if y + match_h > self.line_editor.scroll_offset + viewport_h {
            self.line_editor.scroll_offset = y + match_h - viewport_h + 20.0;
        }
    }

    /// Compute all search match positions and store on the line_editor state for highlighting.
    fn update_search_matches(&mut self) {
        use crate::ui::md_widget::SearchMatch;
        self.line_editor.search_matches.clear();
        if self.editor_search_query.is_empty() { return; }
        let query = &self.editor_search_query;
        let q_chars = query.chars().count();
        let case_sensitive = self.editor_search_case_sensitive;
        for (li, line) in self.line_editor.lines.iter().enumerate() {
            let (search_line, search_query) = if case_sensitive {
                (line.clone(), query.clone())
            } else {
                (line.to_lowercase(), query.to_lowercase())
            };
            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_query) {
                let byte_pos = start + pos;
                let char_start = line[..byte_pos].chars().count();
                self.line_editor.search_matches.push(SearchMatch {
                    line: li,
                    start_col: char_start,
                    end_col: char_start + q_chars,
                });
                start = byte_pos + search_query.len();
            }
        }
    }


    /// Toggle inline markers (bold, code) — applies per-line when multi-line selected.
    fn active_editor_mut(&mut self) -> &mut crate::ui::md_widget::MdEditorState {
        if let Some(ref focused_id) = self.canvas_editor.focused_card {
            if let Some(editor) = self.canvas_editor.card_editors.get_mut(focused_id) {
                return editor;
            }
        }
        &mut self.line_editor
    }

    fn insert_markers(&mut self, marker: &str) {
        let editor = self.active_editor_mut();
        if let Some((start, end)) = editor.selection_ordered() {
            if start.0 != end.0 {
                for li in start.0..=end.0.min(editor.lines.len() - 1) {
                    let line = &editor.lines[li];
                    let trimmed = line.trim_start();
                    let leading = &line[..line.len() - trimmed.len()];
                    if trimmed.starts_with(marker) && trimmed.ends_with(marker) && trimmed.len() >= marker.len() * 2 {
                        editor.lines[li] = format!("{}{}", leading, &trimmed[marker.len()..trimmed.len() - marker.len()]);
                    } else {
                        editor.lines[li] = format!("{}{}{}{}", leading, marker, trimmed, marker);
                    }
                }
                editor.selection = None;
            } else {
                if let Some(sel_text) = editor.selected_text() {
                    editor.delete_selection();
                    let text = if sel_text.starts_with(marker) && sel_text.ends_with(marker) && sel_text.len() >= marker.len() * 2 {
                        sel_text[marker.len()..sel_text.len() - marker.len()].to_string()
                    } else {
                        format!("{}{}{}", marker, sel_text, marker)
                    };
                    editor.insert_text(&text);
                }
            }
        } else {
            let double = format!("{}{}", marker, marker);
            let editor = self.active_editor_mut();
            editor.insert_text(&double);
            for _ in 0..marker.chars().count() { editor.move_left(); }
        }
        self.editor_dirty = true;
        self.last_edit_time = Some(Instant::now());
        if let Some(ref focused_id) = self.canvas_editor.focused_card {
            if let Some(editor) = self.canvas_editor.card_editors.get(&focused_id.clone()) {
                let body = editor.to_body();
                let fid = focused_id.clone();
                if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == fid) {
                    node.label = body;
                }
            }
        }
    }

    /// Toggle a line-prefix marker — applies to all selected lines.
    fn toggle_line_prefix(&mut self, prefix: &str) {
        let prefix_chars = prefix.chars().count();
        let editor = self.active_editor_mut();
        let (start_line, end_line) = if let Some((start, end)) = editor.selection_ordered() {
            (start.0, end.0)
        } else {
            (editor.cursor.0, editor.cursor.0)
        };

        let mut added = true;
        for li in start_line..=end_line.min(editor.lines.len() - 1) {
            if editor.lines[li].starts_with(prefix) {
                editor.lines[li] = editor.lines[li][prefix.len()..].to_string();
                added = false;
            } else {
                editor.lines[li] = format!("{}{}", prefix, &editor.lines[li]);
            }
        }
        editor.selection = None;
        let (cl, cc) = editor.cursor;
        if cl >= start_line && cl <= end_line {
            if added {
                editor.cursor.1 = cc + prefix_chars;
            } else {
                editor.cursor.1 = cc.saturating_sub(prefix_chars);
            }
        }
        self.editor_dirty = true;
        self.last_edit_time = Some(Instant::now());
    }

    /// Insert text at cursor position.
    fn insert_at_cursor(&mut self, text: &str) {
        self.line_editor.insert_text(text);
        self.editor_dirty = true;
        self.last_edit_time = Some(Instant::now());
    }

    fn save_last_note(&self, view: &ActiveView, note_id: Uuid) {
        let db_path = self.db_path.clone();
        let key = format!("last_note_{}", Self::view_key(view));
        let val = note_id.to_string();
        let _ = std::thread::spawn(move || {
            if let Ok(conn) = db::open_connection(&db_path) { let _ = db::set_setting(&conn, &key, &val); }
        });
    }

    fn view_key(view: &ActiveView) -> String {
        match view {
            ActiveView::AllNotes => "all".to_string(),
            ActiveView::Favorites => "fav".to_string(),
            ActiveView::Folder(id) => id.to_string(),
        }
    }

    fn apply_sort(&mut self) {
        let mode = self.sort_mode;
        let sort_notes = |notes: &mut Vec<NotePreview>| {
            notes.sort_by(|a, b| {
                match b.is_pinned.cmp(&a.is_pinned) {
                    std::cmp::Ordering::Equal => match mode {
                        SortMode::Modified => b.modified_at.cmp(&a.modified_at),
                        SortMode::Created => a.modified_at.cmp(&b.modified_at),
                        SortMode::NameAZ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                        SortMode::NameZA => b.title.to_lowercase().cmp(&a.title.to_lowercase()),
                        SortMode::Type => format!("{:?}", a.note_type).cmp(&format!("{:?}", b.note_type)),
                    },
                    other => other,
                }
            });
        };
        sort_notes(&mut self.notes);
        for (_, notes) in &mut self.subfolder_notes {
            sort_notes(notes);
        }
    }

    fn maybe_save(&mut self) -> Task<Message> {
        if self.editor_dirty { self.save_current_note() } else { Task::none() }
    }

    fn save_current_note(&mut self) -> Task<Message> {
        let Some(ref mut note) = self.selected_note else { return Task::none(); };

        note.title = self.editor_title.clone();

        // re-encrypt session-decrypted notes before saving
        if note.is_encrypted && self.session_decrypted.contains_key(&note.id) {
            let plaintext = if note.note_type == NoteType::Password {
                self.password_data.to_json()
            } else if note.note_type == NoteType::Canvas {
                self.canvas_editor.sync_labels();
                self.canvas_editor.data.to_json()
            } else {
                self.line_editor.sync_active_to_lines();
                self.line_editor.to_body()
            };
            self.editor_dirty = false;
            self.last_edit_time = None;
            let note_id = note.id;
            let title = note.title.clone();
            let password = self.session_decrypted.get(&note.id).cloned().unwrap_or_default();
            let db_path = self.db_path.clone();
            return Task::perform(async move {
                let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                let _ = db::notes::rename_note(&conn, note_id, &title);
                let salt = crypto::key_derivation::generate_salt();
                let derived = crypto::key_derivation::derive_key(&password, &salt).map_err(|e| e.to_string())?;
                let (ciphertext, nonce) = crypto::encryption::encrypt(&derived.key_bytes, plaintext.as_bytes()).map_err(|e| e.to_string())?;
                db::notes::update_note_encryption(&conn, note_id, &BASE64.encode(&ciphertext), true, Some(&BASE64.encode(nonce)), Some(&BASE64.encode(salt))).map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            }, |_: Result<(), String>| Message::None);
        }

        if note.is_encrypted { self.editor_dirty = false; return Task::none(); }

        if note.note_type == NoteType::Password {
            note.body = self.password_data.to_json();
        } else if note.note_type == NoteType::Canvas {
            note.body = self.canvas_editor.data.to_json();
        } else {
            self.line_editor.sync_active_to_lines();
            note.body = self.line_editor.to_body();
        }
        self.editor_dirty = false;
        self.last_edit_time = None;

        let note_clone = note.clone();
        let db_path = self.db_path.clone();
        // skip refresh to avoid reloading/re-parsing images from db
        Task::perform(async move { if let Ok(conn) = db::open_connection(&db_path) { let _ = db::notes::update_note(&conn, &note_clone); } }, |_| Message::None)
    }


    fn snapshot_window_state(&mut self) -> WindowState {
        WindowState {
            notes: std::mem::take(&mut self.notes),
            subfolders: std::mem::take(&mut self.subfolders),
            subfolder_notes: std::mem::take(&mut self.subfolder_notes),
            selected_note: self.selected_note.take(),
            active_view: std::mem::replace(&mut self.active_view, ActiveView::AllNotes),
            search_query: std::mem::take(&mut self.search_query),
            editor_title: std::mem::take(&mut self.editor_title),
            editor_content: std::mem::replace(&mut self.editor_content, text_editor::Content::new()),
            line_editor: std::mem::replace(&mut self.line_editor, crate::ui::line_editor::LineEditorState::from_body("")),
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

    fn restore_window_state(&mut self, ws: WindowState) {
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

    /// Propagate editor content to other windows that have the same note open.
    fn sync_editor_to_other_windows(&mut self) {
        let Some(ref note) = self.selected_note else { return };
        let note_id = note.id;
        let note_type = note.note_type;
        let body = match note_type {
            NoteType::Text => self.line_editor.to_body(),
            NoteType::Password => self.password_data.to_json(),
            _ => return,
        };
        let title = self.editor_title.clone();
        for win in self.other_windows.values_mut() {
            if win.selected_note.as_ref().map(|n| n.id) == Some(note_id) {
                win.editor_title = title.clone();
                if note_type == NoteType::Text {
                    win.editor_content = text_editor::Content::with_text(&body);
                } else if note_type == NoteType::Password {
                    win.password_data = PasswordData::from_json(&body);
                }
                if let Some(ref mut n) = win.selected_note {
                    n.title = title.clone();
                }
                for preview in &mut win.notes {
                    if preview.id == note_id { preview.title = title.clone(); }
                }
            }
        }
    }

    fn switch_focus(&mut self, new_id: window::Id) -> Task<Message> {
        if new_id == self.focused_window { return Task::none(); }
        if !self.other_windows.contains_key(&new_id) { return Task::none(); }
        let save_task = self.maybe_save();
        self.context_menu = None;
        self.hovered_item = None;
        self.color_submenu_for = None;
        self.move_submenu_for = None;
        self.dragging = None;
        self.potential_drag = None;
        let saved = self.snapshot_window_state();
        self.other_windows.insert(self.focused_window, saved);
        if let Some(loaded) = self.other_windows.remove(&new_id) {
            self.restore_window_state(loaded);
        }
        self.focused_window = new_id;
        Task::batch([save_task, self.refresh_data()])
    }

    /// Render a full 3-panel view for a non-focused window using stored state.
    fn view_stored_window<'a>(&'a self, _wid: window::Id, win: &'a WindowState) -> Element<'a, Message> {
        let content: Element<Message> = match self.vault_state {
            VaultState::Setup => dialog::password_dialog::view_setup(&self.password_input, &self.confirm_password_input, self.auth_error.as_deref(), win.window_controls_hovered, win.is_maximized),
            VaultState::Login => dialog::password_dialog::view_login(&self.password_input, self.auth_error.as_deref(), win.window_controls_hovered, win.is_maximized, self.show_password),
            VaultState::Loading => self.view_loading(),
            VaultState::Unlocked => {
                let lock_btn = button(svg(crate::ui::icons::lock_closed()).width(16).height(16))
                    .on_press(Message::LockVault)
                    .style(theme::icon_button)
                    .padding([4, 6]);

                let title_bar = mouse_area(
                    container(
                        row![
                            iced::widget::image(iced::widget::image::Handle::from_path("assets/logo.png")).width(16).height(16),
                            Space::with_width(Length::Fill),
                            lock_btn,
                            Space::with_width(8),
                            self.window_controls(),
                        ].spacing(8).align_y(iced::Alignment::Center).padding([8, 10]),
                    )
                    .style({
                        let maximized = win.is_maximized;
                        move |_t: &iced::Theme| container::Style {
                            background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                            border: iced::Border { radius: if maximized { 0.0.into() } else { iced::border::top(10.0) }, ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .width(Length::Fill),
                )
                .on_press(Message::WindowDrag)
                .on_enter(Message::HoverItem(None));

                let tags = tags_panel::view(
                    &win.active_view, &self.folders, self.all_count, self.fav_count,
                    &self.folder_counts, &win.context_menu, &win.dragging,
                    win.renaming_folder, &win.folder_rename_buffer, None, win.is_maximized,
                );

                let is_folder_view = matches!(win.active_view, ActiveView::Folder(_));
                let active_folder_id = match &win.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
                let notes = notes_list::view(
                    &win.notes, win.selected_note.as_ref().map(|n| n.id), &win.search_query,
                    &win.context_menu, win.renaming_note, &win.rename_buffer,
                    is_folder_view, &win.subfolders, &win.subfolder_notes,
                    &win.expanded_folders, &win.dragging, active_folder_id,
                    self.sort_mode, win.sort_menu_open, &win.multi_selected,
                    win.renaming_folder, &win.folder_rename_buffer,
                    win.ctrl_held, win.shift_held, &win.multi_selected_folders, None,
                );

                let right_panel: Element<Message> = if win.show_settings {
                    settings_view::view(self)
                } else if let Some(ref note) = win.selected_note {
                    editor::view(note, &win.editor_title, &win.editor_content, win.editor_dirty, &win.password_data, &win.password_notes_content, win.show_password, win.show_password_gen, &win.password_gen_options, win.copied_field.as_deref(), &win.canvas_editor, &win.canvas_color_editing, win.color_hue, win.color_sat, win.color_lit, self.session_decrypted.contains_key(&note.id), win.encrypting, &win.note_password_input, self.auth_error.as_deref(), win.is_maximized, win.toolbar_move_open, &self.folders, self.setting_font_size, win.editor_search_open, &win.editor_search_query, win.editor_search_index, win.editor_search_case_sensitive, &win.line_editor)
                } else {
                    empty_state::view(win.is_maximized)
                };

                let content_row: Element<Message> = if self.show_sidebar {
                    row![tags, notes, right_panel].into()
                } else {
                    row![right_panel].into()
                };
                let main_layout: Element<Message> = column![
                    title_bar,
                    content_row,
                ].into();

                if let Some(ref dialog_kind) = win.active_dialog {
                    let dialog_view = self.view_dialog(dialog_kind);
                    stack![main_layout, dialog_view].into()
                } else {
                    main_layout
                }
            }
        };

        let window_style = if win.is_maximized { theme::window_container_maximized as fn(&iced::Theme) -> container::Style } else { theme::window_container };
        let pad = if win.is_maximized { 0 } else { 1 };
        let main = container(
            container(content).style(window_style).width(Length::Fill).height(Length::Fill)
        )
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            ..Default::default()
        })
        .padding(pad)
        .width(Length::Fill)
        .height(Length::Fill);

        main.into()
    }

    /// Build a flat list of all visible item IDs in display order (folders + notes),
    /// matching the tree view rendering order for shift-click range selection.
    fn visible_item_ids(&self) -> Vec<(Uuid, bool)> {
        let active_parent = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
        let mut items = Vec::new();
        self.collect_visible_items(&mut items, active_parent);
        for n in &self.notes {
            items.push((n.id, false));
        }
        items
    }

    fn collect_visible_items(&self, items: &mut Vec<(Uuid, bool)>, parent_id: Option<Uuid>) {
        let children: Vec<&Folder> = self.subfolders.iter().filter(|f| f.parent_id == parent_id).collect();
        for sf in children {
            items.push((sf.id, true));
            if self.expanded_folders.contains(&sf.id) {
                if let Some((_, notes)) = self.subfolder_notes.iter().find(|(id, _)| *id == sf.id) {
                    for n in notes {
                        items.push((n.id, false));
                    }
                }
                self.collect_visible_items(items, Some(sf.id));
            }
        }
    }

    /// Load data synchronously for instant view transitions (no intermediate frames).
    fn refresh_data_sync(&mut self) {
        let conn = match db::open_connection(&self.db_path) { Ok(c) => c, Err(_) => return };
        let search = if self.search_query.is_empty() { None } else { Some(self.search_query.as_str()) };
        self.folders = db::folders::list_folders(&conn).unwrap_or_default();
        self.all_count = db::folders::count_all_notes(&conn).unwrap_or(0);
        self.fav_count = db::folders::count_favorites(&conn).unwrap_or(0);
        self.folder_counts = self.folders.iter().filter_map(|f| db::folders::count_notes_in_folder(&conn, f.id).ok().map(|c| (f.id, c))).collect();
        self.notes = match &self.active_view {
            ActiveView::AllNotes => db::notes::list_previews(&conn, None, search).unwrap_or_default(),
            ActiveView::Favorites => db::notes::list_favorites(&conn).unwrap_or_default(),
            ActiveView::Folder(id) => db::notes::list_previews(&conn, Some(*id), search).unwrap_or_default(),
        };
        let (subs, sub_notes) = match &self.active_view {
            ActiveView::Folder(id) => {
                let all_folders = &self.folders;
                let mut all_subs = Vec::new();
                let mut queue = vec![*id];
                while let Some(parent) = queue.pop() {
                    for f in all_folders.iter().filter(|f| f.parent_id == Some(parent)) {
                        all_subs.push(f.clone());
                        queue.push(f.id);
                    }
                }
                let sn: Vec<(Uuid, Vec<NotePreview>)> = all_subs.iter().map(|f| {
                    (f.id, db::notes::list_previews(&conn, Some(f.id), search).unwrap_or_default())
                }).collect();
                (all_subs, sn)
            }
            ActiveView::Favorites => {
                let all_folders = db::folders::list_folders(&conn).unwrap_or_default();
                let fav_root: Vec<Folder> = all_folders.iter().filter(|f| f.is_favorite).cloned().collect();
                let mut all_subs = Vec::new();
                for fav in &fav_root {
                    all_subs.push(fav.clone());
                    let mut queue = vec![fav.id];
                    while let Some(parent) = queue.pop() {
                        for f in all_folders.iter().filter(|f| f.parent_id == Some(parent)) {
                            all_subs.push(f.clone());
                            queue.push(f.id);
                        }
                    }
                }
                let sn: Vec<(Uuid, Vec<NotePreview>)> = all_subs.iter().map(|f| {
                    (f.id, db::notes::list_previews(&conn, Some(f.id), search).unwrap_or_default())
                }).collect();
                (all_subs, sn)
            }
            _ => (Vec::new(), Vec::new()),
        };
        self.subfolders = subs;
        self.subfolder_notes = sub_notes;
        self.apply_sort();
        if let Some(ref sel) = self.selected_note {
            let sel_id = sel.id;
            let in_notes = self.notes.iter().any(|n| n.id == sel_id);
            let in_subs = self.subfolder_notes.iter().any(|(_, ns)| ns.iter().any(|n| n.id == sel_id));
            if !in_notes && !in_subs {
                self.selected_note = None;
                self.editor_title.clear();
                self.editor_content = text_editor::Content::new();
                self.editor_dirty = false;
            }
        }
    }

    fn save_setting(&self, key: &str, value: &str) -> Task<Message> {
        let db_path = self.db_path.clone();
        let key = key.to_string();
        let value = value.to_string();
        Task::perform(async move {
            if let Ok(conn) = db::open_connection(&db_path) {
                let _ = db::set_setting(&conn, &key, &value);
            }
        }, |_| Message::None)
    }

    fn refresh_data(&self) -> Task<Message> {
        let db_path = self.db_path.clone();
        let view = self.active_view.clone();
        let search = if self.search_query.is_empty() { None } else { Some(self.search_query.clone()) };

        Task::perform(async move {
            let conn = match db::open_connection(&db_path) { Ok(c) => c, Err(_) => return (Vec::new(), Vec::new(), 0, 0, Vec::new(), Vec::new(), Vec::new()) };
            let folders = db::folders::list_folders(&conn).unwrap_or_default();
            let all_count = db::folders::count_all_notes(&conn).unwrap_or(0);
            let fav_count = db::folders::count_favorites(&conn).unwrap_or(0);
            let folder_counts: Vec<(Uuid, usize)> = folders.iter().filter_map(|f| db::folders::count_notes_in_folder(&conn, f.id).ok().map(|c| (f.id, c))).collect();
            let notes = match &view {
                ActiveView::AllNotes => db::notes::list_previews(&conn, None, search.as_deref()).unwrap_or_default(),
                ActiveView::Favorites => db::notes::list_favorites(&conn).unwrap_or_default(),
                ActiveView::Folder(id) => db::notes::list_previews(&conn, Some(*id), search.as_deref()).unwrap_or_default(),
            };
            let (subs, sub_notes) = match &view {
                ActiveView::Folder(id) => {
                    let all_folders = db::folders::list_folders(&conn).unwrap_or_default();
                    let mut all_subs = Vec::new();
                    let mut queue = vec![*id];
                    while let Some(parent) = queue.pop() {
                        for f in all_folders.iter().filter(|f| f.parent_id == Some(parent)) {
                            all_subs.push(f.clone());
                            queue.push(f.id);
                        }
                    }
                    let sn: Vec<(Uuid, Vec<NotePreview>)> = all_subs.iter().map(|f| {
                        let n = db::notes::list_previews(&conn, Some(f.id), search.as_deref()).unwrap_or_default();
                        (f.id, n)
                    }).collect();
                    (all_subs, sn)
                }
                ActiveView::Favorites => {
                    let all_folders = db::folders::list_folders(&conn).unwrap_or_default();
                    let fav_root_folders: Vec<Folder> = all_folders.iter().filter(|f| f.is_favorite).cloned().collect();
                    let mut all_subs = Vec::new();
                    for fav in &fav_root_folders {
                        all_subs.push(fav.clone());
                        let mut queue = vec![fav.id];
                        while let Some(parent) = queue.pop() {
                            for f in all_folders.iter().filter(|f| f.parent_id == Some(parent)) {
                                all_subs.push(f.clone());
                                queue.push(f.id);
                            }
                        }
                    }
                    let sn: Vec<(Uuid, Vec<NotePreview>)> = all_subs.iter().map(|f| {
                        let n = db::notes::list_previews(&conn, Some(f.id), search.as_deref()).unwrap_or_default();
                        (f.id, n)
                    }).collect();
                    (all_subs, sn)
                }
                _ => (Vec::new(), Vec::new()),
            };
            (folders, notes, all_count, fav_count, folder_counts, subs, sub_notes)
        }, |(folders, notes, all_count, fav_count, folder_counts, subs, sub_notes)| Message::DataLoaded(folders, notes, all_count, fav_count, folder_counts, subs, sub_notes))
    }
}

fn ctx_btn_hover<'a>(icon: iced::widget::svg::Handle, label: &str, hover_msg: Message, click_msg: Message) -> Element<'a, Message> {
    mouse_area(
        button(
            row![svg(icon).width(14).height(14), text(label.to_owned()).size(12)]
                .spacing(8).align_y(iced::Alignment::Center),
        )
        .on_press(click_msg)
        .style(theme::context_menu_button)
        .padding([5, 10])
        .width(Length::Fill)
    )
    .on_enter(hover_msg)
    .into()
}

/// Context menu button that closes any open submenus on hover
fn ctx_btn<'a>(icon: iced::widget::svg::Handle, label: &str, msg: Message) -> Element<'a, Message> {
    mouse_area(
        button(
            row![svg(icon).width(14).height(14), text(label.to_owned()).size(12)]
                .spacing(8).align_y(iced::Alignment::Center),
        )
        .on_press(msg)
        .style(theme::context_menu_button)
        .padding([5, 10])
        .width(Length::Fill)
    )
    .on_enter(Message::CloseSubmenus)
    .into()
}

fn ctx_btn_danger<'a>(icon: iced::widget::svg::Handle, label: &str, msg: Message) -> Element<'a, Message> {
    mouse_area(
        button(
            row![svg(icon).width(14).height(14), text(label.to_owned()).size(12).style(|_t| theme::danger_text())]
                .spacing(8).align_y(iced::Alignment::Center),
        )
        .on_press(msg)
        .style(theme::context_menu_danger_button)
        .padding([5, 10])
        .width(Length::Fill)
    )
    .on_enter(Message::CloseSubmenus)
    .into()
}



/// Check if a note body has inline image data that needs migration (>1KB lines with image patterns)
fn body_needs_image_migration(body: &str) -> bool {
    body.contains("![") && (body.contains("](data:") || body.contains("](rgba:"))
}

/// Clean a body by replacing inline image data with img:UUID refs.
/// Returns (cleaned_body, Vec<(id, format, base64_string)>)
fn migrate_body_images(body: &str) -> (String, Vec<(String, String, String)>) {
    let mut lines: Vec<String> = body.split('\n').map(String::from).collect();
    let mut to_migrate = Vec::new();

    for li in 0..lines.len() {
        let line = &lines[li];
        if line.len() < 100 { continue; }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("![") { continue; }
        if let (Some(a), Some(b)) = (trimmed.find("]("), trimmed.rfind(')')) {
            let src = &trimmed[a + 2..b];
            let alt = &trimmed[2..a];
            let id = format!("img:{}", Uuid::new_v4());
            if src.starts_with("data:") {
                if let Some(comma) = src.find(',') {
                    let mime = src[5..src.find(';').unwrap_or(comma)].to_string();
                    let b64 = src[comma + 1..].to_string();
                    to_migrate.push((id.clone(), mime, b64));
                    lines[li] = format!("![{}]({})", alt, id);
                }
            } else if src.starts_with("rgba:") {
                let parts: Vec<&str> = src.splitn(4, ':').collect();
                if parts.len() == 4 {
                    let fmt = format!("rgba:{}:{}", parts[1], parts[2]);
                    let b64 = parts[3].to_string();
                    to_migrate.push((id.clone(), fmt, b64));
                    lines[li] = format!("![{}]({})", alt, id);
                }
            }
        }
    }
    (lines.join("\n"), to_migrate)
}

impl App {
    fn auto_apply_color(&mut self) -> Task<Message> {
        if let Some(ref card_id) = self.canvas_color_editing {
            let c = crate::ui::color_picker::hsv_to_rgb(self.color_hue, self.color_sat / 100.0, self.color_lit / 100.0);
            let hex = format!("#{:02X}{:02X}{:02X}", (c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
            let cid = card_id.clone();
            if let Some(node) = self.canvas_editor.data.nodes.iter_mut().find(|n| n.id == cid) {
                node.bg_color = Some(hex);
            }
            self.editor_dirty = true;
            self.last_edit_time = Some(std::time::Instant::now());
            return Task::none();
        }
        if let Some(id) = self.color_submenu_for {
            let c = crate::ui::color_picker::hsv_to_rgb(self.color_hue, self.color_sat / 100.0, self.color_lit / 100.0);
            let best = FolderColor::PALETTE.iter().copied().min_by(|a, b| {
                let ac = a.to_iced_color(); let bc = b.to_iced_color();
                let da = (ac.r-c.r).powi(2) + (ac.g-c.g).powi(2) + (ac.b-c.b).powi(2);
                let db = (bc.r-c.r).powi(2) + (bc.g-c.g).powi(2) + (bc.b-c.b).powi(2);
                da.partial_cmp(&db).unwrap()
            }).unwrap_or(FolderColor::Green);
            if self.color_submenu_is_folder {
                for f in &mut self.folders { if f.id == id { f.color = best; } }
                for f in &mut self.subfolders { if f.id == id { f.color = best; } }
            } else {
                if let Some(ref mut n) = self.selected_note { if n.id == id { n.color = best; } }
                for p in &mut self.notes { if p.id == id { p.color = best; } }
            }
            let db_path = self.db_path.clone();
            let is_folder = self.color_submenu_is_folder;
            return Task::perform(async move {
                if let Ok(conn) = db::open_connection(&db_path) {
                    if is_folder {
                        if let Ok(folders) = db::folders::list_folders(&conn) {
                            if let Some(mut f) = folders.into_iter().find(|f| f.id == id) {
                                f.color = best;
                                let _ = db::folders::update_folder(&conn, &f);
                            }
                        }
                    } else {
                        let _ = db::notes::update_note_color(&conn, id, best);
                    }
                }
            }, |_| Message::None);
        }
        Task::none()
    }
}

fn generate_snippet(body: &str) -> String {
    let mut clean = String::new();
    let mut in_pass = false;
    let mut in_code = false;
    for line in body.lines() {
        let t = line.trim();
        if t == "%%pass" { in_pass = !in_pass; continue; }
        if in_pass { continue; }
        if t.starts_with("```") { in_code = !in_code; continue; }
        if t.starts_with("![") && (t.contains("](img:") || t.contains("](data:") || t.contains("](rgba:")) && t.ends_with(')') {
            if !clean.is_empty() { clean.push(' '); }
            clean.push_str("Image");
            continue;
        }
        if t.starts_with('|') && t.contains('-') && t.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') { continue; }
        if t.starts_with('|') && t.ends_with('|') {
            let cells: Vec<&str> = t[1..t.len()-1].split('|').map(|c| c.trim()).filter(|c| !c.is_empty()).collect();
            if !cells.is_empty() { if !clean.is_empty() { clean.push(' '); } clean.push_str(&cells.join(" | ")); continue; }
        }
        let stripped = t.trim_start_matches("# ").trim_start_matches("## ").trim_start_matches("### ")
            .trim_start_matches("#### ").trim_start_matches("> ").trim_start_matches("- [ ] ")
            .trim_start_matches("- [x] ").trim_start_matches("- [X] ").trim_start_matches("- ")
            .replace("**", "").replace("``", "").replace("`", "");
        let result = stripped.trim();
        if !result.is_empty() {
            if !clean.is_empty() { clean.push(' '); }
            clean.push_str(result);
        }
        if clean.len() >= 60 { break; }
    }
    clean.chars().take(60).collect::<String>().trim().to_string()
}

fn char_to_byte_static(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

fn hit_test_position(state: &crate::ui::md_widget::MdEditorState, x: f32, y: f32, font_size: f32) -> (usize, usize) {
    use iced::advanced::text::{Paragraph as ParagraphTrait, self};
    use iced::Font;
    type Para = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;

    let avail_w = state.text_area_width;
    let mut cumulative_y: f32 = 0.0;
    let mut target_line = 0;
    for (i, line) in state.lines.iter().enumerate() {
        let lh = crate::ui::md_widget::wrapped_line_height(line, font_size, avail_w, &state.image_sizes);
        if y < cumulative_y + lh { target_line = i; break; }
        cumulative_y += lh;
        target_line = i;
    }
    target_line = target_line.min(state.lines.len().saturating_sub(1));

    let trimmed = state.lines[target_line].trim_start();
    let (lfs, lfont) = if trimmed.starts_with("# ") { (font_size * 1.8, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }) }
    else if trimmed.starts_with("## ") { (font_size * 1.5, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }) }
    else if trimmed.starts_with("### ") { (font_size * 1.3, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }) }
    else { (font_size, Font::DEFAULT) };

    let mut in_pass = false;
    let mut pass_fence_count = 0;
    let mut in_code = false;
    let mut code_fence_count = 0;
    for li in 0..=target_line {
        let lt = state.lines[li].trim_start();
        if lt == "%%pass" { pass_fence_count += 1; }
        if lt.starts_with("```") { code_fence_count += 1; }
    }
    if pass_fence_count > 0 && pass_fence_count % 2 == 1 && trimmed != "%%pass" { in_pass = true; }
    if pass_fence_count >= 2 && pass_fence_count % 2 == 0 && trimmed != "%%pass" {
        let mut pf_open = false;
        for li in 0..=target_line {
            if state.lines[li].trim_start() == "%%pass" { pf_open = !pf_open; }
            if li == target_line && pf_open && state.lines[li].trim_start() != "%%pass" { in_pass = true; }
        }
    }
    if code_fence_count % 2 == 1 && !trimmed.starts_with("```") { in_code = true; }
    {
        let mut cf_open = false;
        for li in 0..=target_line {
            if state.lines[li].trim_start().starts_with("```") { cf_open = !cf_open; }
            if li == target_line && cf_open { in_code = true; }
        }
    }

    let is_table = trimmed.starts_with('|') && trimmed.ends_with('|')
        && !(trimmed.contains('-') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' '));
    if is_table {
        use crate::ui::md_widget::{parse_table_cells, cell_to_raw_col};
        let parsed = parse_table_cells(&state.lines[target_line]);
        let cell_count = parsed.len().max(1);
        let cell_w = state.text_area_width / cell_count as f32;
        let clicked_cell = (x / cell_w) as usize;
        let clicked_cell = clicked_cell.min(cell_count.saturating_sub(1));
        let cell_x = clicked_cell as f32 * cell_w + 8.0;
        let x_in_cell = (x - cell_x).max(0.0);

        let cell_text = parsed.get(clicked_cell).map(|(_, _, t)| t.clone()).unwrap_or_default();
        let max_cc = cell_text.chars().count();
        let mut best_cc = 0;
        let mut best_d = x_in_cell.abs();
        for cc in 1..=max_cc {
            let t: String = cell_text.chars().take(cc).collect();
            let para = Para::with_text(iced::advanced::Text {
                content: &t, bounds: iced::Size::new(f32::MAX, f32::MAX),
                size: iced::Pixels(font_size * 0.85), line_height: text::LineHeight::Relative(1.3),
                font: Font::DEFAULT, horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
            });
            let w = para.min_width();
            let d = (x_in_cell - w).abs();
            if d < best_d { best_d = d; best_cc = cc; }
            if w > x_in_cell { break; }
        }
        let raw_col = cell_to_raw_col(&state.lines[target_line], clicked_cell, best_cc);
        return (target_line, raw_col);
    }

    let (hit_fs, hit_font, x_offset) = if in_pass {
        (font_size, Font::DEFAULT, 10.0)
    } else if in_code {
        (font_size * 0.9, Font::MONOSPACE, 6.0)
    } else {
        (lfs, lfont, 0.0)
    };

    let line_text = &state.lines[target_line];
    let display_text: String = if in_pass {
        "\u{2022}".repeat(line_text.chars().count())
    } else {
        line_text.clone()
    };

    let wrap_mode = if in_code || in_pass { text::Wrapping::None } else { text::Wrapping::WordOrGlyph };
    let para_bounds = if in_code || in_pass { f32::MAX } else { avail_w };
    let para = Para::with_text(iced::advanced::Text {
        content: &display_text,
        bounds: iced::Size::new(para_bounds, f32::MAX),
        size: iced::Pixels(hit_fs),
        line_height: text::LineHeight::Relative(1.3),
        font: hit_font,
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: wrap_mode,
    });

    let local_y = y - cumulative_y;
    let local_x = (x - x_offset).max(0.0);
    let col = para.hit_test(iced::Point::new(local_x, local_y))
        .map(|hit| hit.cursor())
        .unwrap_or(line_text.chars().count());

    (target_line, col)
}

fn apply_motion(state: &mut crate::ui::md_widget::MdEditorState, motion: crate::ui::md_widget::MdMotion) {
    use crate::ui::md_widget::MdMotion;
    match motion {
        MdMotion::Left => state.move_left(),
        MdMotion::Right => state.move_right(),
        MdMotion::Up => state.move_up(),
        MdMotion::Down => state.move_down(),
        MdMotion::Home => state.move_home(),
        MdMotion::End => state.move_end(),
        MdMotion::DocStart => state.move_doc_start(),
        MdMotion::DocEnd => state.move_doc_end(),
        MdMotion::WordLeft => { state.move_left(); while state.cursor.1 > 0 { let line = &state.lines[state.cursor.0]; let bp = line.char_indices().nth(state.cursor.1).map(|(i,_)|i).unwrap_or(line.len()); if bp > 0 && line.as_bytes().get(bp-1).map_or(false,|b| *b==b' ') { break; } state.move_left(); } }
        MdMotion::WordRight => { state.move_right(); let max = state.lines[state.cursor.0].chars().count(); while state.cursor.1 < max { let line = &state.lines[state.cursor.0]; let bp = line.char_indices().nth(state.cursor.1).map(|(i,_)|i).unwrap_or(line.len()); if line.as_bytes().get(bp).map_or(false,|b| *b==b' ') { break; } state.move_right(); } }
    }
}

fn find_word_bounds(line: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    let col = col.min(chars.len());
    if col >= chars.len() { return (col, col); }

    let is_space = chars[col].is_whitespace();
    let mut start = col;
    let mut end = col;

    if is_space {
        while start > 0 && chars[start - 1].is_whitespace() { start -= 1; }
        while end < chars.len() && chars[end].is_whitespace() { end += 1; }
    } else {
        while start > 0 && !chars[start - 1].is_whitespace() { start -= 1; }
        while end < chars.len() && !chars[end].is_whitespace() { end += 1; }
    }
    (start, end)
}

/// Process an MdAction on an MdEditorState — shared between MdEdit and CanvasCardEdit handlers.
fn handle_md_action(state: &mut crate::ui::md_widget::MdEditorState, action: crate::ui::md_widget::MdAction, font_size: f32) {
    use crate::ui::md_widget::{MdAction, MdMotion};
    match action {
        MdAction::Click(x, y) => {
            let (line, col) = hit_test_position(state, x, y, font_size);
            state.cursor = (line, col);
            state.selection = None;
            state.focused = true;
            state.focus_instant = Some(std::time::Instant::now());
            state.is_dragging = true;
            state.last_click = Some((std::time::Instant::now(), iced::Point::new(x, y)));
            state.click_count = 1;
        }
        MdAction::DoubleClick(x, y) => {
            let (line, col) = hit_test_position(state, x, y, font_size);
            let wb = find_word_bounds(&state.lines[line.min(state.lines.len() - 1)], col);
            state.selection = Some(((line, wb.0), (line, wb.1)));
            state.cursor = (line, wb.1);
            state.click_count = 2;
        }
        MdAction::TripleClick(x, y) => {
            let (line, _) = hit_test_position(state, x, y, font_size);
            let line_len = state.lines[line.min(state.lines.len() - 1)].chars().count();
            state.selection = Some(((line, 0), (line, line_len)));
            state.cursor = (line, line_len);
            state.click_count = 3;
        }
        MdAction::ShiftClick(x, y) => {
            let (line, col) = hit_test_position(state, x, y, font_size);
            let anchor = state.selection.map(|(s, _)| s).unwrap_or(state.cursor);
            state.selection = Some((anchor, (line, col)));
            state.cursor = (line, col);
        }
        MdAction::DragTo(x, y) => {
            if state.is_dragging {
                let (line, col) = hit_test_position(state, x, y, font_size);
                let anchor = state.selection.map(|(s, _)| s).unwrap_or(state.cursor);
                state.selection = Some((anchor, (line, col)));
                state.cursor = (line, col);
            }
        }
        MdAction::Release => { state.is_dragging = false; }
        MdAction::Insert(c) => { state.push_undo(); state.insert_char(c); }
        MdAction::Paste(text) => { state.push_undo(); state.insert_text(&text); }
        MdAction::Enter => { state.push_undo(); state.insert_newline(); }
        MdAction::Backspace => { state.push_undo(); state.backspace(); }
        MdAction::Delete => { state.push_undo(); state.delete(); }
        MdAction::Undo => { state.undo(); }
        MdAction::Redo => { state.redo(); }
        MdAction::SelectAll => { state.select_all(); }
        MdAction::Copy => { /* clipboard handled by md_widget internally */ }
        MdAction::Cut => { state.push_undo(); state.delete_selection(); }
        MdAction::Move(motion) => {
            match motion {
                MdMotion::Left => state.move_left(),
                MdMotion::Right => state.move_right(),
                MdMotion::Up => state.move_up(),
                MdMotion::Down => state.move_down(),
                MdMotion::Home => state.move_home(),
                MdMotion::End => state.move_end(),
                MdMotion::DocStart => state.move_doc_start(),
                MdMotion::DocEnd => state.move_doc_end(),
                MdMotion::WordLeft => state.move_left(),
                MdMotion::WordRight => state.move_right(),
            }
        }
        MdAction::Select(motion) => {
            let anchor = state.selection.map(|(s, _)| s).unwrap_or(state.cursor);
            match motion {
                MdMotion::Left => state.move_left(),
                MdMotion::Right => state.move_right(),
                MdMotion::Up => state.move_up(),
                MdMotion::Down => state.move_down(),
                MdMotion::Home => state.move_home(),
                MdMotion::End => state.move_end(),
                MdMotion::DocStart => state.move_doc_start(),
                MdMotion::DocEnd => state.move_doc_end(),
                MdMotion::WordLeft => state.move_left(),
                MdMotion::WordRight => state.move_right(),
            }
            state.selection = Some((anchor, state.cursor));
        }
        MdAction::Focus => { state.focused = true; state.focus_instant = Some(std::time::Instant::now()); }
        MdAction::Unfocus => { state.focused = false; }
        MdAction::Tick(now, w) => {
            state.now = now; state.text_area_width = w;
            if state.scroll_velocity.abs() > 0.5 {
                state.scroll_offset = (state.scroll_offset - state.scroll_velocity).max(0.0);
                state.scroll_velocity *= 0.82;
            } else { state.scroll_velocity = 0.0; }
        }
        MdAction::WindowFocus(f) => { state.is_window_focused = f; }
        MdAction::ToggleCheckbox(line) => {
            if line < state.lines.len() {
                let l = state.lines[line].clone();
                if l.contains("- [x]") {
                    state.lines[line] = l.replacen("- [x]", "- [ ]", 1);
                } else if l.contains("- [ ]") {
                    state.lines[line] = l.replacen("- [ ]", "- [x]", 1);
                }
            }
        }
        MdAction::Indent => {
            let li = state.cursor.0;
            if li < state.lines.len() {
                state.push_undo();
                state.lines[li] = format!("    {}", state.lines[li]);
                state.cursor.1 += 4;
            }
        }
        MdAction::Unindent => {
            let li = state.cursor.0;
            if li < state.lines.len() && state.lines[li].starts_with("    ") {
                state.push_undo();
                state.lines[li] = state.lines[li][4..].to_string();
                state.cursor.1 = state.cursor.1.saturating_sub(4);
            }
        }
        MdAction::ScrollTo(pos) => { state.scroll_offset = pos.max(0.0); state.scroll_velocity = 0.0; }
        MdAction::Scroll(dy) => {
            state.scroll_offset = (state.scroll_offset - dy * 0.7).max(0.0);
            state.scroll_velocity += dy * 0.3;
            state.scroll_velocity = state.scroll_velocity.clamp(-80.0, 80.0);
        }
        MdAction::RightClick => {}
        MdAction::FileExport(_, _) | MdAction::FileDelete(_) => {} // handled by MdEdit handler
        _ => {} // Slash menu, code blocks, images, etc.
    }
}

fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.chars().count() <= max_len { return name.to_string(); }
    let ext_start = name.rfind('.').unwrap_or(name.len());
    let ext = &name[ext_start..]; // e.g. ".exe"
    let stem_max = max_len.saturating_sub(ext.len() + 3); // room for "..."
    if stem_max == 0 { return format!("...{}", ext); }
    let stem: String = name.chars().take(stem_max).collect();
    format!("{}...{}", stem, ext)
}

fn format_file_size(bytes: usize) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

/// Simple file save — writes to Downloads folder with the given filename.
/// Returns the path if successful.
async fn rfd_save_dialog(filename: &str) -> Option<std::path::PathBuf> {
    // strip path components to prevent directory traversal
    let safe_name = std::path::Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let downloads = dirs::download_dir().or_else(dirs::desktop_dir).unwrap_or_else(|| std::path::PathBuf::from("."));
    let mut dest = downloads.join(safe_name);
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string();
    let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    let mut i = 1;
    while dest.exists() {
        dest = downloads.join(if ext.is_empty() {
            format!("{} ({})", stem, i)
        } else {
            format!("{} ({}).{}", stem, i, ext)
        });
        i += 1;
    }
    Some(dest)
}

#[allow(dead_code)]
fn color_dot_btn(color: iced::Color, msg: Message) -> Element<'static, Message> {
    use iced::widget::button;
    button(
        iced::widget::container(iced::widget::Space::new(12, 12))
            .style(move |_t: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(color)),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            })
    )
    .on_press(msg)
    .style(|_t: &iced::Theme, _s| iced::widget::button::Style {
        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        ..Default::default()
    })
    .padding(2)
    .into()
}
