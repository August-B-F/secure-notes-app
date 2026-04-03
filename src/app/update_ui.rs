use super::*;

impl App {
    pub(super) fn handle_ui_message(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::DragEnd(captured) => {
                if self.resizing.is_some() {
                    self.resizing = None;
                    return Task::none();
                }
                if self.dragging.is_none() && self.potential_drag.is_none() {
                    if self.rename_pending == 0 && !captured {
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

            _ => Task::none()
        }
    }
}
