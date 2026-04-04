use super::*;

impl App {
    pub(super) fn handle_data_message(&mut self, message: Message) -> Task<Message> {
        match message {
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
                                    self.line_editor = crate::ui::md_widget::MdEditorState::from_body(&note.body);
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
                // Ignore note selection if we're in the middle of a text drag-select in the editor
                if self.line_editor.is_dragging {
                    return Task::none();
                }
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
                if let Some(note) = self.notes.iter().find(|n| n.id == id) {
                    let c = note.color.to_iced_color();
                    let max = c.r.max(c.g).max(c.b);
                    let min = c.r.min(c.g).min(c.b);
                    let d = max - min;
                    let h = if d < 0.001 { 0.0 } else if (max - c.r).abs() < 0.001 {
                        60.0 * (((c.g - c.b) / d) % 6.0)
                    } else if (max - c.g).abs() < 0.001 {
                        60.0 * ((c.b - c.r) / d + 2.0)
                    } else {
                        60.0 * ((c.r - c.g) / d + 4.0)
                    };
                    let s = if max > 0.0 { d / max } else { 0.0 };
                    self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                    self.color_sat = s * 100.0;
                    self.color_lit = max * 100.0;
                }
                Task::none()
            }
            Message::ToggleColorSubmenu(id) => {
                let opening = self.color_submenu_for != Some(id);
                self.color_submenu_for = if opening { Some(id) } else { None };
                self.color_submenu_is_folder = false;
                self.move_submenu_for = None;
                if opening {
                    if let Some(note) = self.notes.iter().find(|n| n.id == id) {
                        let c = note.color.to_iced_color();
                        let max = c.r.max(c.g).max(c.b);
                        let min = c.r.min(c.g).min(c.b);
                        let d = max - min;
                        let h = if d < 0.001 { 0.0 } else if (max - c.r).abs() < 0.001 {
                            60.0 * (((c.g - c.b) / d) % 6.0)
                        } else if (max - c.g).abs() < 0.001 {
                            60.0 * ((c.b - c.r) / d + 2.0)
                        } else {
                            60.0 * ((c.r - c.g) / d + 4.0)
                        };
                        let s = if max > 0.0 { d / max } else { 0.0 };
                        self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                        self.color_sat = s * 100.0;
                        self.color_lit = max * 100.0;
                    }
                }
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
            Message::ColorPickerHexInput(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        let rf = r as f32 / 255.0;
                        let gf = g as f32 / 255.0;
                        let bf = b as f32 / 255.0;
                        let max = rf.max(gf).max(bf);
                        let min = rf.min(gf).min(bf);
                        let d = max - min;
                        let h = if d < 0.001 { 0.0 } else if (max - rf).abs() < 0.001 {
                            60.0 * (((gf - bf) / d) % 6.0)
                        } else if (max - gf).abs() < 0.001 {
                            60.0 * ((bf - rf) / d + 2.0)
                        } else {
                            60.0 * ((rf - gf) / d + 4.0)
                        };
                        let s = if max > 0.0 { d / max } else { 0.0 };
                        self.color_hue = if h < 0.0 { h + 360.0 } else { h };
                        self.color_sat = s * 100.0;
                        self.color_lit = max * 100.0;
                        return self.auto_apply_color();
                    }
                }
                Task::none()
            }
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

            Message::NoteMigrated(cleaned_body, loaded_images) => {
                self.line_editor = crate::ui::md_widget::MdEditorState::from_body(&cleaned_body);
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
                            self.line_editor = crate::ui::md_widget::MdEditorState::from_body(&note.body);
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

            _ => Task::none()
        }
    }
}
