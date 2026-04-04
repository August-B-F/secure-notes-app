use super::*;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let had_dialog = self.active_dialog.is_some();

        // Close sort menu when user clicks outside it (on editor, sidebar, notes, etc.)
        if self.sort_menu_open {
            match &message {
                Message::SelectNote(_) | Message::SelectView(_)
                | Message::LineClicked(_) | Message::LineRightClicked(_)
                | Message::MdEdit(_) | Message::LineInputChanged(_, _)
                | Message::EditorTitleChanged(_) | Message::EditorContentAction(_)
                | Message::ToggleContextMenu(_) | Message::CloseContextMenu
                | Message::SearchQueryChanged(_)
                | Message::FormatBold | Message::FormatItalic | Message::FormatHeading
                | Message::FormatList | Message::FormatCheckbox | Message::FormatCode
                | Message::WindowDrag
                | Message::CanvasSelect(_) | Message::CanvasPan(_, _)
                => { self.sort_menu_open = false; }
                _ => {}
            }
        }

        // passthrough: these messages should not close context menus
        match &message {
            Message::ToggleContextMenu(_) | Message::CloseContextMenu
            | Message::AnimationTick | Message::LoadingTick
            | Message::AutoSaveTick(_) | Message::CursorMoved(_, _, _)
            | Message::CanvasCloseCtxMenu | Message::SetNoteColor(_, _) | Message::SetFolderColor(_, _)
            | Message::ToggleColorSubmenu(_) | Message::ToggleFolderColorSubmenu(_) | Message::OpenMoveFolderPicker(_) | Message::ColorPickerHue(_)
            | Message::OpenColorSubmenu(_) | Message::OpenFolderColorSubmenu(_) | Message::OpenMoveSubmenu(_) | Message::OpenNewNoteSubmenu(_) | Message::ToggleNewNoteSubmenu(_) | Message::OpenEditorSubmenu(_) | Message::CloseSubmenus
            | Message::ColorPickerSat(_) | Message::ColorPickerLit(_)
            | Message::ColorPickerSVChanged(_, _) | Message::ColorPickerPreset(_) | Message::ColorPickerHexInput(_)
            | Message::RenameNoteChanged(_) | Message::RenameNoteSubmit
            | Message::RenameFolderChanged(_) | Message::RenameFolderSubmit
            | Message::CancelRename | Message::NotePasswordInputChanged(_)
            | Message::NotePasswordConfirmChanged(_)
            | Message::DragStart(_) | Message::DragEnd(_) | Message::DropOnFolder(..)
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
                self.context_menu = None; self.color_submenu_for = None; self.move_submenu_for = None; self.new_note_submenu_for = None; self.editor_submenu = None; self.toolbar_move_open = false; self.potential_drag = None; self.hovered_item = None; self.sort_menu_open = false;
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
            Message::PasswordInputChanged(_) | Message::ConfirmPasswordInputChanged(_)
            | Message::SubmitSetup | Message::SetupDone(_) | Message::SubmitLogin | Message::LoginDone(_)
            | Message::LockVault
            | Message::OpenChangeVaultPasswordDialog | Message::VaultOldPasswordChanged(_)
            | Message::VaultNewPasswordChanged(_) | Message::VaultNewPasswordConfirmChanged(_)
            | Message::SubmitChangeVaultPassword | Message::ChangeVaultPasswordDone(_) => self.handle_vault_message(message),

            Message::WindowClose | Message::WindowMinimize | Message::WindowMaximize
            | Message::WindowDrag | Message::WindowControlsHover(_)
            | Message::WindowResizeStart(_) | Message::WindowResized(_, _, _) | Message::HoverItem(_)
            | Message::ToggleContextMenu(_) | Message::CloseContextMenu
            | Message::DragPotential(_) | Message::DragStart(_) | Message::DragEnd(_)
            | Message::AnimationTick | Message::CursorMoved(_, _, _) | Message::LoadingTick
            | Message::ModifiersChanged(_, _, _) | Message::SetSortMode(_) | Message::ToggleSortMenu | Message::ToggleExpandFolder(_)
            | Message::SetFramerate(_) | Message::SetAutoSaveDelay(_) | Message::SetEditorFontSize(_)
            | Message::SetCanvasGridSize(_) | Message::ToggleAutoSave | Message::ToggleLineNumbers
            | Message::OpenNewWindow | Message::WindowFocused(_) | Message::WindowCloseRequested(_) | Message::WindowClosed(_)
            | Message::None => self.handle_ui_message(message),

            Message::EditorTitleChanged(_) | Message::EditorContentAction(_)
            | Message::LineClicked(_) | Message::LineRightClicked(_) | Message::FocusActiveLine
            | Message::LineInputChanged(_, _) | Message::LineInputSubmit(_)
            | Message::LineArrowUp | Message::LineArrowDown
            | Message::LineEditorAction(_, _) | Message::LineBlur
            | Message::MdEdit(_)
            | Message::SaveNote | Message::AutoSaveTick(_)
            | Message::FormatBold | Message::FormatItalic | Message::FormatHeading
            | Message::FormatList | Message::FormatCheckbox | Message::FormatCode
            | Message::FormatDivider | Message::FormatLink | Message::FormatRemoveLink
            | Message::FormatAlignLeft | Message::FormatAlignCenter | Message::FormatAlignRight
            | Message::OpenEditorSubmenu(_)
            | Message::OpenTextColorPicker | Message::ApplyTextColor | Message::FormatTextColor(_)
            | Message::FormatQuote | Message::FormatRemove
            | Message::ToggleSearch | Message::ToggleSearchCaseSensitive
            | Message::SearchQueryEditorChanged(_) | Message::SearchNext | Message::SearchPrev
            | Message::ToggleMarkdownPreview
            | Message::PasswordWebsiteChanged(_) | Message::PasswordUsernameChanged(_)
            | Message::PasswordEmailChanged(_) | Message::PasswordValueChanged(_)
            | Message::PasswordNotesChanged(_) | Message::PasswordNotesAction(_)
            | Message::TogglePasswordVisibility | Message::TogglePasswordGenPanel
            | Message::PasswordGenLength(_) | Message::PasswordGenToggleUpper
            | Message::PasswordGenToggleLower | Message::PasswordGenToggleNumbers
            | Message::PasswordGenToggleSymbols | Message::GeneratePassword
            | Message::AddCustomField | Message::RemoveCustomField(_)
            | Message::CustomFieldLabelChanged(_, _) | Message::CustomFieldValueChanged(_, _)
            | Message::ToggleCustomFieldHidden(_)
            | Message::CopyField(_, _) | Message::CopiedFeedbackClear
            | Message::ClearCopiedBlockFeedback
            | Message::ToggleSidebar | Message::ZoomIn | Message::ZoomOut | Message::ZoomReset
            | Message::CloseSubmenus
            | Message::CanvasAddNodeCenter | Message::CanvasAddNode(_, _)
            | Message::CanvasMoveNode(_, _, _) | Message::CanvasMoveNodeGroup(_)
            | Message::CanvasSelect(_) | Message::CanvasMultiSelect(_)
            | Message::CanvasDeleteSelected | Message::CanvasAddEdge(_, _, _, _)
            | Message::CanvasCardEdit(_, _) | Message::CanvasCardFocus(_) | Message::CanvasCardUnfocus
            | Message::CanvasUndo | Message::CanvasRedo
            | Message::CanvasRecenter | Message::CanvasFitView
            | Message::CanvasSelectEdge(_) | Message::CanvasReverseEdge(_)
            | Message::CanvasCloseCtxMenu | Message::CanvasDeleteEdge(_)
            | Message::CanvasOpenColorPicker(_) | Message::CanvasApplyColor
            | Message::CanvasSetNodeBgColor(_, _)
            | Message::CanvasResizeNode(_, _, _, _, _)
            | Message::CanvasPan(_, _) | Message::CanvasZoom(_, _, _)
            | Message::CanvasViewportSize(_, _) | Message::CanvasHover(_)
            | Message::CanvasShowCtxMenu(_, _, _)
            | Message::ToggleGraphView | Message::ShowSettings => self.handle_editor_message(message),

            _ => self.handle_data_message(message),
        };

        if !had_dialog && self.active_dialog.is_some() {
            self.dialog_anim = 0.0;
        }

        result
    }
}
