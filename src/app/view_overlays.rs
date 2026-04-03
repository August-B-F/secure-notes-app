use super::*;
use iced::widget::column;

impl App {
    pub(super) fn view_context_menu(&self, ctx: &ContextMenu) -> Element<'_, Message> {
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
                    ctx_btn(icons::folder_icon(), "New folder", Message::CreateQuickFolder(None)),
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
                    ctx_btn_hover(icons::fmt_bold(), "Format  \u{203A}", Message::OpenEditorSubmenu(EditorSubmenu::Format), Message::OpenEditorSubmenu(EditorSubmenu::Format)),
                    ctx_btn_hover(icons::fmt_align_left(), "Paragraph  \u{203A}", Message::OpenEditorSubmenu(EditorSubmenu::Paragraph), Message::OpenEditorSubmenu(EditorSubmenu::Paragraph)),
                    ctx_btn_hover(icons::fmt_link(), "Insert  \u{203A}", Message::OpenEditorSubmenu(EditorSubmenu::Insert), Message::OpenEditorSubmenu(EditorSubmenu::Insert)),
                    ctx_btn_hover(icons::palette_icon(), "Text color  \u{203A}", Message::OpenEditorSubmenu(EditorSubmenu::TextColor), Message::OpenEditorSubmenu(EditorSubmenu::TextColor)),
                ].spacing(1).padding(4))
                .style(theme::context_menu_container)
                .max_width(180)
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
            ContextMenu::TagsEmpty => 4,
            ContextMenu::NotesEmpty => 4,
            ContextMenu::EditorFormat => 4,
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
        } else if let Some(ref sub) = self.editor_submenu {
            match sub {
                EditorSubmenu::TextColor => {
                    use crate::ui::color_picker;
                    let picker = color_picker::view(self.color_hue, self.color_sat, self.color_lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);
                    Some(container(
                        column![
                            picker,
                        ].spacing(8).padding(12)
                    )
                    .style(theme::context_menu_container)
                    .width(240)
                    .into())
                }
                _ => {
                    let items = match sub {
                        EditorSubmenu::Format => column![
                            ctx_btn(icons::fmt_bold(), "Bold", Message::FormatBold),
                            ctx_btn(icons::fmt_italic(), "Italic", Message::FormatItalic),
                            ctx_btn(icons::fmt_heading(), "Heading", Message::FormatHeading),
                            ctx_btn(icons::fmt_code(), "Code", Message::FormatCode),
                            ctx_btn(icons::fmt_divider(), "Divider", Message::FormatDivider),
                            ctx_btn(icons::fmt_clear(), "Clear formatting", Message::FormatRemove),
                        ].spacing(1),
                        EditorSubmenu::Paragraph => column![
                            ctx_btn(icons::fmt_list(), "Bullet list", Message::FormatList),
                            ctx_btn(icons::fmt_checkbox(), "Checkbox", Message::FormatCheckbox),
                            ctx_btn(icons::fmt_align_left(), "Align left", Message::FormatAlignLeft),
                            ctx_btn(icons::fmt_align_center(), "Align center", Message::FormatAlignCenter),
                            ctx_btn(icons::fmt_align_right(), "Align right", Message::FormatAlignRight),
                        ].spacing(1),
                        EditorSubmenu::Insert => column![
                            ctx_btn(icons::fmt_link(), "Link", Message::FormatLink),
                            ctx_btn(icons::fmt_unlink(), "Remove link", Message::FormatRemoveLink),
                            ctx_btn(icons::fmt_divider(), "Divider", Message::FormatDivider),
                        ].spacing(1),
                        EditorSubmenu::TextColor => unreachable!(),
                    };
                    Some(container(items.padding(4))
                        .style(theme::context_menu_container)
                        .width(170)
                        .into())
                }
            }
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

    pub(super) fn view_dialog(&self, kind: &DialogKind) -> Element<'_, Message> {
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
                        button(text("Close").size(13).align_x(Horizontal::Center).width(Length::Fill))
                            .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
                    ].spacing(4).padding(20),
                ).style(theme::dialog_card).width(320);
                container(card).style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill).into()
            }
        }
    }

    pub(super) fn view_note_color_picker(&self, note_id: Uuid) -> Element<'_, Message> {
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

    pub(super) fn view_move_folder_picker(&self, note_id: Uuid) -> Element<'_, Message> {
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

    pub(super) fn window_controls(&self) -> Element<'_, Message> {
        let controls = row![
            button(svg(icons::win_minimize()).width(16).height(16)
                .style(theme::svg_hover_color(iced::Color::from_rgb8(0xE5, 0xD5, 0x4D))))
                .on_press(Message::WindowMinimize)
                .style(theme::transparent_button)
                .padding([4, 6]),
            button(svg(icons::win_maximize()).width(16).height(16)
                .style(theme::svg_hover_color(iced::Color::from_rgb8(0x4D, 0xC8, 0x6A))))
                .on_press(Message::WindowMaximize)
                .style(theme::transparent_button)
                .padding([4, 6]),
            button(svg(icons::win_close()).width(16).height(16)
                .style(theme::svg_hover_color(iced::Color::from_rgb8(0xE5, 0x4D, 0x4D))))
                .on_press(Message::WindowClose)
                .style(theme::transparent_button)
                .padding([4, 6]),
        ].spacing(4).align_y(iced::Alignment::Center);

        controls.into()
    }

    pub(super) fn view_stored_window<'a>(&'a self, _wid: window::Id, win: &'a WindowState) -> Element<'a, Message> {
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
                            iced::widget::image(iced::widget::image::Handle::from_bytes(include_bytes!("../../assets/logo.png").to_vec())).width(16).height(16),
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
}
