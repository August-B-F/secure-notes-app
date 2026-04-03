use super::*;

impl App {
    pub(super) fn save_last_note(&self, view: &ActiveView, note_id: Uuid) {
        let db_path = self.db_path.clone();
        let key = format!("last_note_{}", Self::view_key(view));
        let val = note_id.to_string();
        let _ = std::thread::spawn(move || {
            if let Ok(conn) = db::open_connection(&db_path) { let _ = db::set_setting(&conn, &key, &val); }
        });
    }

    pub(super) fn view_key(view: &ActiveView) -> String {
        match view {
            ActiveView::AllNotes => "all".to_string(),
            ActiveView::Favorites => "fav".to_string(),
            ActiveView::Folder(id) => id.to_string(),
        }
    }

    pub(super) fn apply_sort(&mut self) {
        let mode = self.sort_mode;
        let sort_notes = |notes: &mut Vec<NotePreview>| {
            notes.sort_by(|a, b| {
                match b.is_pinned.cmp(&a.is_pinned) {
                    std::cmp::Ordering::Equal => match mode {
                        SortMode::Modified => b.modified_at.cmp(&a.modified_at),
                        SortMode::Created => a.modified_at.cmp(&b.modified_at),
                        SortMode::NameAZ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                        SortMode::NameZA => b.title.to_lowercase().cmp(&a.title.to_lowercase()),
                        SortMode::Type => format!("{:?}", a.note_type).cmp(&format!("{:?}", b.note_type))
                            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
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

    pub(super) fn maybe_save(&mut self) -> Task<Message> {
        if self.editor_dirty { self.save_current_note() } else { Task::none() }
    }

    pub(super) fn save_current_note(&mut self) -> Task<Message> {
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


    /// Propagate editor content to other windows that have the same note open.
    pub(super) fn sync_editor_to_other_windows(&mut self) {
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

    pub(super) fn switch_focus(&mut self, new_id: window::Id) -> Task<Message> {
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

    /// matching the tree view rendering order for shift-click range selection.
    pub(super) fn visible_item_ids(&self) -> Vec<(Uuid, bool)> {
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
    pub(super) fn refresh_data_sync(&mut self) {
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

    pub(super) fn save_setting(&self, key: &str, value: &str) -> Task<Message> {
        let db_path = self.db_path.clone();
        let key = key.to_string();
        let value = value.to_string();
        Task::perform(async move {
            if let Ok(conn) = db::open_connection(&db_path) {
                let _ = db::set_setting(&conn, &key, &value);
            }
        }, |_| Message::None)
    }

    pub(super) fn refresh_data(&self) -> Task<Message> {
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
