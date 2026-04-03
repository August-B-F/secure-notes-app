use super::*;

pub(super) fn ctx_btn_hover<'a>(icon: iced::widget::svg::Handle, label: &str, hover_msg: Message, click_msg: Message) -> Element<'a, Message> {
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
pub(super) fn ctx_btn<'a>(icon: iced::widget::svg::Handle, label: &str, msg: Message) -> Element<'a, Message> {
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

pub(super) fn ctx_btn_danger<'a>(icon: iced::widget::svg::Handle, label: &str, msg: Message) -> Element<'a, Message> {
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
pub(super) fn body_needs_image_migration(body: &str) -> bool {
    body.contains("![") && (body.contains("](data:") || body.contains("](rgba:"))
}

/// Clean a body by replacing inline image data with img:UUID refs.
/// Returns (cleaned_body, Vec<(id, format, base64_string)>)
pub(super) fn migrate_body_images(body: &str) -> (String, Vec<(String, String, String)>) {
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
    pub(super) fn auto_apply_color(&mut self) -> Task<Message> {
        // Auto-apply text color when text color picker is open
        let is_text_color = matches!(self.editor_submenu, Some(EditorSubmenu::TextColor))
            || matches!(self.active_dialog, Some(DialogKind::TextColor));
        if is_text_color {
            if let Some((start, end)) = self.text_color_selection {
                let h = self.color_hue;
                let s = self.color_sat;
                let l = self.color_lit;
                let color_code = format!("{:.0},{:.0},{:.0}", h, s, l);
                use crate::ui::md_widget::ColorRange;
                if start.0 == end.0 {
                    self.line_editor.colors.retain(|c| !(c.line == start.0 && c.start_col == start.1 && c.end_col == end.1));
                    self.line_editor.colors.push(ColorRange {
                        line: start.0, start_col: start.1, end_col: end.1, color: color_code,
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
                self.editor_dirty = true;
                self.last_edit_time = Some(std::time::Instant::now());
            }
            return Task::none();
        }
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

/// Strip all inline formatting from a line: bold, italic, code, color tags, links.
pub(super) fn strip_all_formatting(line: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        // Color tags: {c:...}text{/c} -> text
        if i + 3 < len && chars[i] == '{' && chars[i + 1] == 'c' && chars[i + 2] == ':' {
            if let Some(tag_end) = chars[i + 3..].iter().position(|c| *c == '}').map(|p| i + 3 + p) {
                let remaining: String = chars[tag_end + 1..].iter().collect();
                if let Some(close_pos) = remaining.find("{/c}") {
                    let content: String = chars[tag_end + 1..tag_end + 1 + close_pos].iter().collect();
                    result.push_str(&content);
                    i = tag_end + 1 + close_pos + 4;
                    continue;
                }
            }
        }
        // Bold: **text** -> text
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = chars[i + 2..].windows(2).position(|w| w[0] == '*' && w[1] == '*').map(|p| i + 2 + p) {
                let content: String = chars[i + 2..end].iter().collect();
                result.push_str(&content);
                i = end + 2;
                continue;
            }
        }
        // Italic: *text* -> text (single asterisk, not preceded by another *)
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') && (i + 1 >= len || chars[i + 1] != '*') {
            if let Some(end) = chars[i + 1..].iter().position(|c| *c == '*').map(|p| i + 1 + p) {
                if end + 1 >= len || chars[end + 1] != '*' {
                    let content: String = chars[i + 1..end].iter().collect();
                    result.push_str(&content);
                    i = end + 1;
                    continue;
                }
            }
        }
        // Code: `text` -> text
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|c| *c == '`').map(|p| i + 1 + p) {
                let content: String = chars[i + 1..end].iter().collect();
                result.push_str(&content);
                i = end + 1;
                continue;
            }
        }
        // Links: [text](url) -> text
        if chars[i] == '[' {
            if let Some(bracket_end) = chars[i + 1..].iter().position(|c| *c == ']').map(|p| i + 1 + p) {
                if bracket_end + 1 < len && chars[bracket_end + 1] == '(' {
                    if let Some(paren_end) = chars[bracket_end + 2..].iter().position(|c| *c == ')').map(|p| bracket_end + 2 + p) {
                        let content: String = chars[i + 1..bracket_end].iter().collect();
                        result.push_str(&content);
                        i = paren_end + 1;
                        continue;
                    }
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Re-number consecutive numbered list items starting from `from_line`.
/// Walks backward to find the first item in the run, then renumbers forward.
pub(super) fn renumber_list(state: &mut crate::ui::md_widget::MdEditorState, from_line: usize) {
    fn is_numbered(line: &str) -> Option<(String, usize)> {
        let trimmed = line.trim_start();
        let leading = &line[..line.len() - trimmed.len()];
        if let Some(dot_pos) = trimmed.find(". ") {
            let num_str = &trimmed[..dot_pos];
            if !num_str.is_empty() && num_str.chars().all(|c| c.is_ascii_digit()) {
                return Some((leading.to_string(), dot_pos));
            }
        }
        None
    }
    // Walk backward to find the start of the numbered list run
    let mut start = from_line;
    while start > 0 {
        if is_numbered(&state.lines[start - 1]).is_some() { start -= 1; } else { break; }
    }
    // Walk forward and renumber
    let mut num = 1u32;
    let mut i = start;
    while i < state.lines.len() {
        if let Some((leading, dot_pos)) = is_numbered(&state.lines[i]) {
            let trimmed = state.lines[i].trim_start();
            let rest = &trimmed[dot_pos..]; // ". content..."
            state.lines[i] = format!("{}{}{}", leading, num, rest);
            num += 1;
            i += 1;
        } else {
            break;
        }
    }
}

pub(super) fn generate_snippet(body: &str) -> String {
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

pub(super) fn char_to_byte_static(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

pub(super) fn hit_test_position(state: &crate::ui::md_widget::MdEditorState, x: f32, y: f32, font_size: f32) -> (usize, usize) {
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

    // Also detect if target is a fence line (%%pass or ```) inside a block
    let is_pass_fence = trimmed == "%%pass" && pass_fence_count > 0;
    let is_code_fence = trimmed.starts_with("```") && code_fence_count > 0;
    let (hit_fs, hit_font, x_offset) = if in_pass || is_pass_fence {
        (font_size, Font::DEFAULT, 6.0)
    } else if in_code || is_code_fence {
        (font_size * 0.9, Font::MONOSPACE, 6.0)
    } else {
        (lfs, lfont, 0.0)
    };

    let line_text = &state.lines[target_line];

    // When cursor is already on a heading line, the rendering splits into
    // marker (e.g. "# ") + content, with content wrapping in reduced width.
    // Match hit-test to this layout.
    let is_heading = trimmed.starts_with("# ") || trimmed.starts_with("## ")
        || trimmed.starts_with("### ") || trimmed.starts_with("#### ");
    let is_cursor_heading = is_heading && state.focused && target_line == state.cursor.0;
    if is_cursor_heading {
        let marker_len = if trimmed.starts_with("#### ") { 5 }
            else if trimmed.starts_with("### ") { 4 }
            else if trimmed.starts_with("## ") { 3 }
            else { 2 };
        let leading_len = line_text.len() - trimmed.len();
        let marker_char_len = leading_len + marker_len;
        let marker_text: String = line_text.chars().take(marker_char_len).collect();
        let marker_w = crate::ui::md_widget::measure_text_width(&marker_text, lfs, lfont);
        let local_y = y - cumulative_y;
        let local_x = x;
        if local_x < marker_w {
            // Click in marker area
            let para = Para::with_text(iced::advanced::Text {
                content: &marker_text,
                bounds: iced::Size::new(marker_w + 1.0, f32::MAX),
                size: iced::Pixels(lfs),
                line_height: text::LineHeight::Relative(1.3),
                font: lfont,
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            });
            let col = para.hit_test(iced::Point::new(local_x, local_y))
                .map(|h| h.cursor()).unwrap_or(marker_char_len);
            return (target_line, col);
        } else {
            // Click in content area
            let content: String = line_text.chars().skip(marker_char_len).collect();
            let content_w = (avail_w - marker_w).max(10.0);
            let para = Para::with_text(iced::advanced::Text {
                content: &content,
                bounds: iced::Size::new(content_w, f32::MAX),
                size: iced::Pixels(lfs),
                line_height: text::LineHeight::Relative(1.3),
                font: lfont,
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::WordOrGlyph,
            });
            let col = para.hit_test(iced::Point::new(local_x - marker_w, local_y))
                .map(|h| h.cursor()).unwrap_or(content.chars().count());
            return (target_line, marker_char_len + col);
        }
    }

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

pub(super) fn apply_motion(state: &mut crate::ui::md_widget::MdEditorState, motion: crate::ui::md_widget::MdMotion) {
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

pub(super) fn find_word_bounds(line: &str, col: usize) -> (usize, usize) {
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

/// Process an MdAction on an MdEditorState -- shared between MdEdit and CanvasCardEdit handlers.
pub(super) fn handle_md_action(state: &mut crate::ui::md_widget::MdEditorState, action: crate::ui::md_widget::MdAction, font_size: f32) {
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
        MdAction::OpenLink(url) => {
            let _ = open::that(&url);
        }
        MdAction::FileExport(_, _) | MdAction::FileDelete(_) => {} // handled by MdEdit handler
        _ => {} // Slash menu, code blocks, images, etc.
    }
}

pub(super) fn truncate_filename(name: &str, max_len: usize) -> String {
    if name.chars().count() <= max_len { return name.to_string(); }
    let ext_start = name.rfind('.').unwrap_or(name.len());
    let ext = &name[ext_start..]; // e.g. ".exe"
    let stem_max = max_len.saturating_sub(ext.len() + 3); // room for "..."
    if stem_max == 0 { return format!("...{}", ext); }
    let stem: String = name.chars().take(stem_max).collect();
    format!("{}...{}", stem, ext)
}

pub(super) fn format_file_size(bytes: usize) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

/// Simple file save -- writes to Downloads folder with the given filename.
/// Returns the path if successful.
pub(super) async fn rfd_save_dialog(filename: &str) -> Option<std::path::PathBuf> {
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
pub(super) fn color_dot_btn(color: iced::Color, msg: Message) -> Element<'static, Message> {
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
