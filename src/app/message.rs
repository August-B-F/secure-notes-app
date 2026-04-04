use std::path::PathBuf;
use std::time::Instant;

use iced::widget::text_editor;
use iced::window;
use uuid::Uuid;

use crate::models::*;
use crate::models::note::NoteType;
use crate::ui::canvas_editor;

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
pub enum EditorSubmenu { Format, Paragraph, Insert, TextColor }

#[derive(Debug, Clone, PartialEq)]
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
    ColorPickerHexInput(String),
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
    FormatLink,
    FormatRemoveLink,
    FormatRemove,
    FormatAlignLeft,
    FormatAlignCenter,
    FormatAlignRight,
    OpenEditorSubmenu(EditorSubmenu),

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
    DragEnd(bool), // true if a widget captured the click
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
    ImageDropped(window::Id, PathBuf),
    CopyImage(usize), // copy image at line index to clipboard
    InsertImageData(Vec<u8>, String), // (bytes, mime)
    ImagesLoaded(Vec<(String, Vec<u8>, String)>), // (id, bytes, format) loaded from DB
    NoteMigrated(String, Vec<(String, Vec<u8>, String)>), // (cleaned_body, loaded_images)
    PasteDone,

    FileDropped(PathBuf),
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
