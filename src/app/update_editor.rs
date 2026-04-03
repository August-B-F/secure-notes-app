use super::*;

impl App {
    pub(super) fn handle_editor_message(&mut self, message: Message) -> Task<Message> {
        match message {
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
                            use crate::ui::md_widget::UndoEditKind;
                            state.push_undo_grouped(UndoEditKind::Typing);
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
                                    let (_cl, cc) = state.cursor;
                                    let byte_col = crate::ui::md_widget::char_to_byte(&line, cc);
                                    let leading_len = line.len() - trimmed.len();
                                    let prefix_byte_end = leading_len + prefix.len();
                                    let at_line_start = byte_col <= prefix_byte_end;

                                    if at_line_start {
                                        // Cursor is at or before the prefix: insert blank line above, keep current line intact
                                        state.lines.insert(_cl, String::new());
                                        state.cursor = (_cl + 1, cc);
                                    } else {
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
                                        let rest = state.lines[new_line].clone();
                                        state.lines[new_line] = format!("{}{}", new_prefix, rest);
                                        state.cursor.1 = new_prefix.chars().count();
                                    }
                                    // Renumber subsequent list items
                                    renumber_list(state, state.cursor.0);
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
                            renumber_list(state, state.cursor.0);
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
                            use crate::ui::md_widget::UndoEditKind;
                            state.push_undo_grouped(UndoEditKind::Delete);
                            state.backspace();
                            renumber_list(state, state.cursor.0);
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
                            use crate::ui::md_widget::UndoEditKind;
                            state.push_undo_grouped(UndoEditKind::Delete);
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
                            } else if let Some((start, end)) = state.selection_ordered() {
                                // Multi-line indent: add 2 spaces to each selected line
                                state.push_undo();
                                for li in start.0..=end.0.min(state.lines.len() - 1) {
                                    state.lines[li] = format!("  {}", &state.lines[li]);
                                }
                                state.cursor.1 += 2;
                                state.selection = Some((
                                    (start.0, start.1 + 2),
                                    (end.0, end.1 + 2),
                                ));
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            } else {
                                state.push_undo();
                                state.lines[line] = format!("  {}", &state.lines[line]);
                                state.cursor.1 += 2;
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            }
                        }
                    }
                    MdAction::Unindent => {
                        if let Some((start, end)) = state.selection_ordered() {
                            // Multi-line unindent: remove up to 2 leading spaces from each selected line
                            state.push_undo();
                            let mut any_changed = false;
                            for li in start.0..=end.0.min(state.lines.len() - 1) {
                                if state.lines[li].starts_with("  ") {
                                    state.lines[li] = state.lines[li][2..].to_string();
                                    any_changed = true;
                                } else if state.lines[li].starts_with(' ') {
                                    state.lines[li] = state.lines[li][1..].to_string();
                                    any_changed = true;
                                }
                            }
                            if any_changed {
                                state.cursor.1 = state.cursor.1.saturating_sub(2);
                                let new_start_col = start.1.saturating_sub(2);
                                let new_end_col = end.1.saturating_sub(2);
                                state.selection = Some((
                                    (start.0, new_start_col),
                                    (end.0, new_end_col),
                                ));
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            }
                        } else {
                            let (line, _) = state.cursor;
                            if line < state.lines.len() && state.lines[line].starts_with("  ") {
                                state.push_undo();
                                state.lines[line] = state.lines[line][2..].to_string();
                                state.cursor.1 = state.cursor.1.saturating_sub(2);
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            } else if line < state.lines.len() && state.lines[line].starts_with(' ') {
                                state.push_undo();
                                state.lines[line] = state.lines[line][1..].to_string();
                                state.cursor.1 = state.cursor.1.saturating_sub(1);
                                self.editor_dirty = true;
                                self.last_edit_time = Some(Instant::now());
                            }
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
                    MdAction::OpenLink(url) => {
                        let open_url = if url.contains("://") {
                            url
                        } else if url.contains('.') {
                            format!("https://{}", url)
                        } else {
                            // Not a valid URL — search for it
                            format!("https://www.google.com/search?q={}", url.replace(' ', "+"))
                        };
                        let _ = open::that(&open_url);
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
            Message::FormatLink => {
                self.active_editor_mut().push_undo();
                let editor = self.active_editor_mut();
                let sel_text = editor.selected_text();
                let looks_like_url = sel_text.as_ref().map_or(false, |t| {
                    t.contains('.') || t.contains("://")
                });
                if let Some(ref text) = sel_text {
                    editor.delete_selection();
                    if looks_like_url {
                        // Selected text is a URL — use it as both display text and link target
                        editor.insert_text(&format!("[{}]({})", text, text));
                    } else {
                        // Selected text is display text — put cursor in URL position
                        editor.insert_text(&format!("[{}](url)", text));
                    }
                } else {
                    editor.insert_text("[link text](url)");
                }
                if !looks_like_url {
                    // Select the "url" placeholder so user can immediately type the real URL
                    let editor = self.active_editor_mut();
                    let (line, col) = editor.cursor;
                    let url_end = col.saturating_sub(1);
                    let url_start = url_end.saturating_sub(3);
                    editor.cursor = (line, url_end);
                    editor.selection = Some(((line, url_start), (line, url_end)));
                }
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }
            Message::FormatRemoveLink => {
                self.active_editor_mut().push_undo();
                let editor = self.active_editor_mut();
                let (cl, _) = editor.cursor;
                if cl < editor.lines.len() {
                    let line = editor.lines[cl].clone();
                    // Replace all [text](url) with just text
                    let mut result = String::new();
                    let chars: Vec<char> = line.chars().collect();
                    let mut i = 0;
                    while i < chars.len() {
                        if chars[i] == '[' {
                            if let Some(bracket_end) = chars[i + 1..].iter().position(|c| *c == ']').map(|p| i + 1 + p) {
                                if bracket_end + 1 < chars.len() && chars[bracket_end + 1] == '(' {
                                    if let Some(paren_end) = chars[bracket_end + 2..].iter().position(|c| *c == ')').map(|p| bracket_end + 2 + p) {
                                        let link_text: String = chars[i + 1..bracket_end].iter().collect();
                                        result.push_str(&link_text);
                                        i = paren_end + 1;
                                        continue;
                                    }
                                }
                            }
                        }
                        result.push(chars[i]);
                        i += 1;
                    }
                    editor.lines[cl] = result;
                    self.editor_dirty = true;
                    self.last_edit_time = Some(Instant::now());
                }
                Task::none()
            }
            Message::FormatAlignLeft => { self.set_line_alignment(""); Task::none() }
            Message::FormatAlignCenter => { self.set_line_alignment("{center}"); Task::none() }
            Message::FormatAlignRight => { self.set_line_alignment("{right}"); Task::none() }
            Message::OpenEditorSubmenu(sub) => {
                if matches!(sub, EditorSubmenu::TextColor) {
                    self.text_color_selection = self.line_editor.selection_ordered();
                }
                self.editor_submenu = Some(sub);
                Task::none()
            }
            Message::OpenTextColorPicker => {
                self.text_color_selection = self.line_editor.selection_ordered();
                self.active_dialog = Some(DialogKind::TextColor);
                Task::none()
            }
            Message::ApplyTextColor => {
                let h = self.color_hue;
                let s = self.color_sat;
                let l = self.color_lit;
                let color_code = format!("{:.0},{:.0},{:.0}", h, s, l);
                self.active_dialog = None;
                return self.update(Message::FormatTextColor(color_code));
            }
            Message::FormatTextColor(color_code) => {
                self.line_editor.push_undo();
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
            Message::FormatRemove => {
                let editor = self.active_editor_mut();
                editor.push_undo();
                if let Some((start, end)) = editor.selection_ordered() {
                    for li in start.0..=end.0.min(editor.lines.len() - 1) {
                        let line = &editor.lines[li];
                        let stripped = strip_all_formatting(line);
                        editor.lines[li] = stripped;
                    }
                    // Remove color ranges for affected lines
                    let colors = &mut editor.colors;
                    colors.retain(|c| c.line < start.0 || c.line > end.0);
                    editor.selection = None;
                } else {
                    // No selection: strip current line
                    let li = editor.cursor.0;
                    if li < editor.lines.len() {
                        let stripped = strip_all_formatting(&editor.lines[li]);
                        editor.lines[li] = stripped;
                        editor.colors.retain(|c| c.line != li);
                    }
                }
                self.editor_dirty = true;
                self.last_edit_time = Some(Instant::now());
                Task::none()
            }

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

            _ => Task::none()
        }
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
        self.active_editor_mut().push_undo();
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
        self.active_editor_mut().push_undo();
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

    /// Set line alignment tag. `tag` is "" for left, "{center}" or "{right}".
    fn set_line_alignment(&mut self, tag: &str) {
        self.active_editor_mut().push_undo();
        let editor = self.active_editor_mut();
        let (cl, _) = editor.cursor;
        if cl < editor.lines.len() {
            let line = &editor.lines[cl];
            // Strip any existing alignment tag
            let stripped = line.trim_start_matches("{center}").trim_start_matches("{right}");
            if tag.is_empty() {
                editor.lines[cl] = stripped.to_string();
            } else {
                editor.lines[cl] = format!("{}{}", tag, stripped);
            }
            self.editor_dirty = true;
            self.last_edit_time = Some(Instant::now());
        }
    }

    /// Insert text at cursor position.
    fn insert_at_cursor(&mut self, text: &str) {
        self.line_editor.push_undo();
        self.line_editor.insert_text(text);
        self.editor_dirty = true;
        self.last_edit_time = Some(Instant::now());
    }

}
