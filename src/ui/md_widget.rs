//! Fully custom markdown editor widget.
//! Renders formatted markdown natively: headings are large, bold is bold,
//! markers are hidden. Cursor, selection, keyboard input all handled internally.

use iced::advanced::layout;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Paragraph as ParagraphTrait};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::{alignment, event, keyboard, mouse, window};
use iced::{
    Background, Border, Color, Element, Event, Font, Length, Padding, Pixels, Point, Rectangle,
    Size,
};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::ui::md_highlight::{highlight_inline, Highlight};


type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

/// A line in the editor, with both raw text and a cached formatted paragraph.
#[allow(dead_code)]
struct LineData {
    raw: String,
    /// Paragraph for rendering (with formatting, markers hidden)
    paragraph: Paragraph,
    /// The display text (markers stripped) — needed for hit testing offset mapping
    display_chars: Vec<DisplayChar>,
    /// Height of this line's paragraph
    height: f32,
}

/// Maps display character index → raw character index
#[derive(Clone)]
#[allow(dead_code)]
struct DisplayChar {
    raw_index: usize,
}


/// A color annotation for a range of text within a line.
#[derive(Clone, Debug)]
pub struct ColorRange {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub color: String, // HSL code like "120,80,50" or preset like "r"
}

/// A search match location: (line, start_col, end_col)
#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// A slash command definition
pub struct SlashCommand {
    pub name: &'static str,
    pub label: &'static str,
    #[allow(dead_code)]
    pub description: &'static str,
    pub icon: &'static str,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand { name: "code", label: "Code Block", description: "Fenced code block with copy button", icon: "</>" },
    SlashCommand { name: "table", label: "Table", description: "Markdown table with columns", icon: ":::" },
    SlashCommand { name: "password", label: "Password Field", description: "Hidden password field", icon: "***" },
];

/// Get filtered slash commands matching a query
pub fn filter_slash_commands(query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    SLASH_COMMANDS.iter().enumerate()
        .filter(|(_, cmd)| cmd.name.starts_with(&q) || cmd.label.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect()
}

/// Tracks the type of the last edit for undo grouping.
#[derive(Clone, Copy, PartialEq)]
pub enum UndoEditKind {
    Typing,     // consecutive character insertions
    Delete,     // backspace/delete
    Other,      // paste, indent, formatting, etc.
}

pub struct MdEditorState {
    pub lines: Vec<String>,
    pub cursor: (usize, usize), // (line, raw_col)
    pub selection: Option<((usize, usize), (usize, usize))>, // (start, end) in (line, raw_col)
    pub colors: Vec<ColorRange>, // visual-only color annotations
    pub search_matches: Vec<SearchMatch>, // highlighted search results
    pub current_match: usize, // index into search_matches for the active match
    pub slash_menu_open: bool,
    pub slash_filter: String, // text after '/' for filtering
    pub slash_selected: usize, // highlighted item index in filtered list
    pub image_cache: HashMap<String, iced::widget::image::Handle>, // id/path → handle
    pub image_sizes: HashMap<String, (f32, f32)>, // id → (width, height) custom sizes
    pub image_resizing: Option<(usize, f32, f32)>, // (line_idx, start_x, start_y) during drag
    pub copied_block_line: Option<usize>, // line index that was just copied (for checkmark feedback)
    pub password_visible: std::collections::HashSet<usize>, // line indices of password blocks with visible text
    pub undo_stack: VecDeque<(Vec<String>, (usize, usize))>, // (lines, cursor) snapshots
    pub redo_stack: VecDeque<(Vec<String>, (usize, usize))>,
    pub code_lang_menu: Option<usize>, // line index of code block showing language picker
    pub text_area_width: f32, // last known text area width for hit testing
    pub focused: bool,
    pub scroll_offset: f32,
    pub scroll_velocity: f32, // pixels/frame for smooth scrolling
    pub cached_content_height: Option<(f32, f32, f32)>, // (avail_w, font_size, height) — invalidated on edit
    pub focus_instant: Option<Instant>,
    pub now: Instant,
    pub last_click: Option<(Instant, Point)>,
    pub click_count: u8,
    pub is_dragging: bool,
    pub is_window_focused: bool,
    // Undo grouping state
    undo_last_kind: Option<UndoEditKind>,
    undo_last_time: Option<Instant>,
    undo_last_cursor: (usize, usize),
}

impl MdEditorState {
    /// No-op compatibility shims (formerly in line_editor.rs)
    pub fn sync_active_to_lines(&mut self) {}
    pub fn activate(&mut self, _index: usize) {}
    pub fn deactivate(&mut self) {}
    pub fn sync_to_lines(&mut self) {}

    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: (0, 0),
            selection: None,
            colors: Vec::new(),
            search_matches: Vec::new(),
            current_match: 0,
            slash_menu_open: false,
            slash_filter: String::new(),
            slash_selected: 0,
            image_cache: HashMap::new(),
            image_sizes: HashMap::new(),
            image_resizing: None,
            copied_block_line: None,
            password_visible: std::collections::HashSet::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            code_lang_menu: None,
            text_area_width: 600.0,
            focused: false,
            scroll_offset: 0.0,
            scroll_velocity: 0.0,
            cached_content_height: None,
            focus_instant: None,
            now: Instant::now(),
            last_click: None,
            click_count: 0,
            is_dragging: false,
            is_window_focused: true,
            undo_last_kind: None,
            undo_last_time: None,
            undo_last_cursor: (0, 0),
        }
    }

    pub fn from_body(body: &str) -> Self {
        let mut colors = Vec::new();
        let mut clean_lines = Vec::new();

        let raw_lines: Vec<&str> = if body.is_empty() { vec![""] } else { body.split('\n').collect() };
        for (li, raw_line) in raw_lines.iter().enumerate() {
            let (clean, line_colors) = strip_color_tags(raw_line, li);
            clean_lines.push(clean);
            colors.extend(line_colors);
        }

        Self { lines: clean_lines, colors, search_matches: Vec::new(), ..Self::new() }
    }

    /// Collect image references. Returns:
    /// - img:UUID ids to load from DB
    /// - old inline data as raw strings to migrate in background (id, format_prefix, raw_base64)
    /// Replaces old inline lines with short img:UUID refs immediately (no decoding).
    pub fn collect_images(&mut self) -> (Vec<String>, Vec<(String, String, String)>) {
        use uuid::Uuid;
        let mut ids = Vec::new();
        let mut to_migrate: Vec<(String, String, String)> = Vec::new(); // (id, format_info, base64_string)

        for li in 0..self.lines.len() {
            let line = &self.lines[li];
            if line.len() < 10 { continue; }
            let trimmed = line.trim_start();
            if !trimmed.starts_with("![") { continue; }

            if let (Some(a), Some(b)) = (trimmed.find("]("), trimmed.rfind(')')) {
                let src = &trimmed[a + 2..b];
                let alt = trimmed[2..a].to_string();

                if src.starts_with("img:") {
                    ids.push(src.to_string());
                } else if src.starts_with("data:") {
                    if let Some(comma) = src.find(',') {
                        let mime = src[5..src.find(';').unwrap_or(comma)].to_string();
                        let b64 = src[comma + 1..].to_string();
                        let id = format!("img:{}", Uuid::new_v4());
                        to_migrate.push((id.clone(), mime, b64));
                        self.lines[li] = format!("![{}]({})", alt, id);
                    }
                } else if src.starts_with("rgba:") {
                    let parts: Vec<&str> = src.splitn(4, ':').collect();
                    if parts.len() == 4 {
                        let fmt = format!("rgba:{}:{}", parts[1], parts[2]);
                        let b64 = parts[3].to_string();
                        let id = format!("img:{}", Uuid::new_v4());
                        to_migrate.push((id.clone(), fmt, b64));
                        self.lines[li] = format!("![{}]({})", alt, id);
                    }
                }
            }
        }
        (ids, to_migrate)
    }

    /// Fast body text (for search, sync) — keeps img:UUID references as-is
    pub fn to_body(&self) -> String {
        let mut result_lines: Vec<String> = self.lines.clone();
        for li in 0..result_lines.len() {
            let mut line_colors: Vec<&ColorRange> = self.colors.iter()
                .filter(|c| c.line == li).collect();
            line_colors.sort_by(|a, b| b.start_col.cmp(&a.start_col));
            for cr in line_colors {
                let line = &result_lines[li];
                let chars: Vec<char> = line.chars().collect();
                let start_byte = chars.iter().take(cr.start_col).map(|c| c.len_utf8()).sum::<usize>();
                let end_byte = chars.iter().take(cr.end_col).map(|c| c.len_utf8()).sum::<usize>();
                if end_byte <= line.len() && start_byte <= end_byte {
                    let before = &line[..start_byte];
                    let content = &line[start_byte..end_byte];
                    let after = &line[end_byte..];
                    result_lines[li] = format!("{}{{c:{}}}{}{{/c}}{}", before, cr.color, content, after);
                }
            }
        }
        result_lines.join("\n")
    }


    /// Get total content height, cached for performance.
    pub fn content_height(&mut self, fs: f32, avail_w: f32) -> f32 {
        if let Some((w, f, h)) = self.cached_content_height {
            if (w - avail_w).abs() < 1.0 && (f - fs).abs() < 0.1 {
                return h;
            }
        }
        let h: f32 = self.lines.iter()
            .map(|l| wrapped_line_height(l, fs, avail_w, &self.image_sizes))
            .sum();
        self.cached_content_height = Some((avail_w, fs, h));
        h
    }

    /// Save current state to undo stack before making changes
    /// Always push a new undo snapshot (for discrete operations like paste, indent, formatting).
    pub fn push_undo(&mut self) {
        self.undo_stack.push_back((self.lines.clone(), self.cursor));
        self.redo_stack.clear();
        if self.undo_stack.len() > 200 { self.undo_stack.pop_front(); }
        self.cached_content_height = None;
        self.undo_last_kind = None;
        self.undo_last_time = None;
    }

    /// Push undo with grouping: consecutive edits of the same kind within a short
    /// time window are batched into a single undo step (like VS Code).
    /// A new undo snapshot is created when:
    /// - The edit kind changes (typing → delete, etc.)
    /// - More than 1 second has passed since the last edit
    /// - The cursor jumped (not adjacent to previous position)
    pub fn push_undo_grouped(&mut self, kind: UndoEditKind) {
        let now = Instant::now();
        let should_group = self.undo_last_kind == Some(kind)
            && self.undo_last_time.map_or(false, |t| now.duration_since(t).as_millis() < 1000)
            && self.cursor.0 == self.undo_last_cursor.0
            && (self.cursor.1 as isize - self.undo_last_cursor.1 as isize).unsigned_abs() <= 2;

        if !should_group {
            self.undo_stack.push_back((self.lines.clone(), self.cursor));
            self.redo_stack.clear();
            if self.undo_stack.len() > 200 { self.undo_stack.pop_front(); }
            self.cached_content_height = None;
        }
        self.undo_last_kind = Some(kind);
        self.undo_last_time = Some(now);
        self.undo_last_cursor = self.cursor;
    }

    pub fn undo(&mut self) {
        if let Some((lines, cursor)) = self.undo_stack.pop_back() {
            self.redo_stack.push_back((self.lines.clone(), self.cursor));
            self.cached_content_height = None;
            self.lines = lines;
            self.cursor = cursor;
            self.selection = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some((lines, cursor)) = self.redo_stack.pop_back() {
            self.undo_stack.push_back((self.lines.clone(), self.cursor));
            self.cached_content_height = None;
            self.lines = lines;
            self.cursor = cursor;
            self.selection = None;
        }
    }

    pub fn set_body(&mut self, body: &str) {
        self.lines = if body.is_empty() {
            vec![String::new()]
        } else {
            body.split('\n').map(String::from).collect()
        };
        self.cursor = (0, 0);
        self.selection = None;
    }


    pub fn insert_char(&mut self, c: char) {
        self.delete_selection();
        let (line, col) = self.cursor;
        if line < self.lines.len() {
            let byte_pos = char_to_byte(&self.lines[line], col);
            self.lines[line].insert(byte_pos, c);
            // Shift color ranges on this line that start at or after insertion point
            for cr in &mut self.colors {
                if cr.line == line {
                    if cr.start_col >= col { cr.start_col += 1; }
                    if cr.end_col >= col { cr.end_col += 1; }
                }
            }
            self.cursor.1 = col + 1;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.delete_selection();
        for c in text.chars() {
            if c == '\n' {
                self.insert_newline();
            } else if !c.is_control() {
                self.insert_char(c);
            }
        }
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection();
        let (line, col) = self.cursor;
        if line < self.lines.len() {
            let byte_pos = char_to_byte(&self.lines[line], col);
            let rest = self.lines[line][byte_pos..].to_string();
            self.lines[line].truncate(byte_pos);
            self.lines.insert(line + 1, rest);
            // Update color ranges: shift subsequent lines down, split ranges at cursor
            let mut new_colors = Vec::new();
            self.colors.retain(|cr| {
                if cr.line == line && cr.end_col > col {
                    // This range spans the split point — split it
                    if cr.start_col < col {
                        // Part stays on current line (truncated), part goes to new line
                        new_colors.push(ColorRange {
                            line: line + 1,
                            start_col: 0,
                            end_col: cr.end_col - col,
                            color: cr.color.clone(),
                        });
                    } else {
                        // Entire range moves to new line
                        new_colors.push(ColorRange {
                            line: line + 1,
                            start_col: cr.start_col - col,
                            end_col: cr.end_col - col,
                            color: cr.color.clone(),
                        });
                        return false; // remove from current position
                    }
                }
                true
            });
            // Truncate ranges that were split (keep only the part before the split)
            for cr in &mut self.colors {
                if cr.line == line && cr.end_col > col {
                    cr.end_col = col;
                }
                // Shift all lines after the split down by 1
                if cr.line > line {
                    cr.line += 1;
                }
            }
            self.colors.extend(new_colors);
            self.cursor = (line + 1, 0);
        }
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() { return; }
        let (line, col) = self.cursor;
        if col > 0 {
            let byte_start = char_to_byte(&self.lines[line], col - 1);
            let byte_end = char_to_byte(&self.lines[line], col);
            self.lines[line].replace_range(byte_start..byte_end, "");
            // Shift color ranges on this line left by 1 for ranges after the deleted char
            for cr in &mut self.colors {
                if cr.line == line {
                    if cr.start_col >= col { cr.start_col -= 1; }
                    else if cr.start_col == col - 1 { /* range starts at deleted char */ }
                    if cr.end_col >= col { cr.end_col -= 1; }
                }
            }
            // Remove zero-width ranges
            self.colors.retain(|cr| cr.start_col < cr.end_col);
            self.cursor.1 = col - 1;
        } else if line > 0 {
            let prev_char_len = char_len(&self.lines[line - 1]);
            let current = self.lines.remove(line);
            self.lines[line - 1].push_str(&current);
            // Merge colors from deleted line into previous line, offset by prev line length
            for cr in &mut self.colors {
                if cr.line == line {
                    cr.line = line - 1;
                    cr.start_col += prev_char_len;
                    cr.end_col += prev_char_len;
                } else if cr.line > line {
                    cr.line -= 1;
                }
            }
            self.cursor = (line - 1, prev_char_len);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() { return; }
        let (line, col) = self.cursor;
        if line < self.lines.len() {
            let line_chars = char_len(&self.lines[line]);
            if col < line_chars {
                let byte_start = char_to_byte(&self.lines[line], col);
                let byte_end = char_to_byte(&self.lines[line], col + 1);
                self.lines[line].replace_range(byte_start..byte_end, "");
                // Shift color ranges left by 1 for ranges after the deleted char
                for cr in &mut self.colors {
                    if cr.line == line {
                        if cr.start_col > col { cr.start_col -= 1; }
                        if cr.end_col > col { cr.end_col -= 1; }
                    }
                }
                self.colors.retain(|cr| cr.start_col < cr.end_col);
            } else if line + 1 < self.lines.len() {
                let next_line_len = char_len(&self.lines[line]);
                let next = self.lines.remove(line + 1);
                self.lines[line].push_str(&next);
                // Merge colors from next line into current, offset by current line length
                for cr in &mut self.colors {
                    if cr.line == line + 1 {
                        cr.line = line;
                        cr.start_col += next_line_len;
                        cr.end_col += next_line_len;
                    } else if cr.line > line + 1 {
                        cr.line -= 1;
                    }
                }
            }
        }
    }

    pub fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection_ordered() else { return false; };
        if start == end { self.selection = None; return false; }

        let lines_removed = end.0 - start.0;
        if start.0 == end.0 {
            let deleted_chars = end.1 - start.1;
            let bs = char_to_byte(&self.lines[start.0], start.1);
            let be = char_to_byte(&self.lines[start.0], end.1);
            self.lines[start.0].replace_range(bs..be, "");
            // Adjust colors on the same line
            for cr in &mut self.colors {
                if cr.line == start.0 {
                    if cr.start_col >= end.1 {
                        cr.start_col -= deleted_chars;
                        cr.end_col -= deleted_chars;
                    } else if cr.start_col >= start.1 {
                        cr.start_col = start.1;
                        if cr.end_col >= end.1 {
                            cr.end_col -= deleted_chars;
                        } else {
                            cr.end_col = start.1;
                        }
                    } else if cr.end_col > start.1 {
                        cr.end_col = if cr.end_col >= end.1 { cr.end_col - deleted_chars } else { start.1 };
                    }
                }
            }
        } else {
            let bs = char_to_byte(&self.lines[start.0], start.1);
            let be = char_to_byte(&self.lines[end.0], end.1);
            let end_rest = self.lines[end.0][be..].to_string();
            self.lines[start.0].truncate(bs);
            self.lines[start.0].push_str(&end_rest);
            self.lines.drain(start.0 + 1..=end.0);
            // Remove colors for deleted lines, adjust surviving colors
            self.colors.retain(|cr| {
                if cr.line > start.0 && cr.line <= end.0 { return false; } // deleted line
                if cr.line == start.0 && cr.start_col >= start.1 { return false; } // in deleted part of first line
                true
            });
            // Merge colors from after deletion on end line → start line
            for cr in &mut self.colors {
                if cr.line == end.0 && cr.start_col >= end.1 {
                    // This range was on the end line after the selection — move to start line
                    cr.line = start.0;
                    cr.start_col = cr.start_col - end.1 + start.1;
                    cr.end_col = cr.end_col - end.1 + start.1;
                } else if cr.line > end.0 {
                    cr.line -= lines_removed;
                }
                // Truncate ranges that partially overlap the deletion on the first line
                if cr.line == start.0 && cr.end_col > start.1 && cr.start_col < start.1 {
                    cr.end_col = start.1;
                }
            }
        }
        self.colors.retain(|cr| cr.start_col < cr.end_col);
        self.cursor = start;
        self.selection = None;
        true
    }

    pub fn selection_ordered(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection.map(|(a, b)| {
            if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) { (a, b) } else { (b, a) }
        })
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_ordered()?;
        if start == end { return None; }
        let mut result = String::new();
        for i in start.0..=end.0 {
            let line = &self.lines[i];
            let from = if i == start.0 { char_to_byte(line, start.1) } else { 0 };
            let to = if i == end.0 { char_to_byte(line, end.1) } else { line.len() };
            result.push_str(&line[from..to]);
            if i < end.0 { result.push('\n'); }
        }
        Some(result)
    }


    pub fn move_left(&mut self) {
        let (line, col) = self.cursor;
        if col > 0 {
            self.cursor.1 = col - 1;
        } else if line > 0 {
            self.cursor = (line - 1, char_len(&self.lines[line - 1]));
        }
    }

    pub fn move_right(&mut self) {
        let (line, col) = self.cursor;
        if line < self.lines.len() && col < char_len(&self.lines[line]) {
            self.cursor.1 = col + 1;
        } else if line + 1 < self.lines.len() {
            self.cursor = (line + 1, 0);
        }
    }

    fn line_fs(&self) -> f32 {
        if self.cursor.0 >= self.lines.len() { return 15.0; }
        let trimmed = self.lines[self.cursor.0].trim_start();
        if trimmed.starts_with("# ") { 15.0 * 1.8 }
        else if trimmed.starts_with("## ") { 15.0 * 1.5 }
        else if trimmed.starts_with("### ") { 15.0 * 1.3 }
        else { 15.0 }
    }

    fn make_para(&self) -> Paragraph {
        let line = if self.cursor.0 < self.lines.len() { &self.lines[self.cursor.0] } else { "" };
        Paragraph::with_text(iced::advanced::Text {
            content: line, bounds: Size::new(self.text_area_width.max(10.0), f32::MAX),
            size: Pixels(self.line_fs()), line_height: iced::advanced::text::LineHeight::Relative(1.3),
            font: Font::DEFAULT, horizontal_alignment: iced::alignment::Horizontal::Left,
            vertical_alignment: iced::alignment::Vertical::Top,
            shaping: iced::advanced::text::Shaping::Advanced,
            wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
        })
    }

    pub fn move_up(&mut self) {
        let w = self.text_area_width;
        if w > 0.0 && self.cursor.0 < self.lines.len() {
            let lfs = self.line_fs();
            let (cx, cy) = wrapped_cursor_pos(&self.lines[self.cursor.0], self.cursor.1, lfs, Font::DEFAULT, w);
            let vlh = lfs * 1.3;
            if cy > vlh * 0.5 {
                // move within wrapped line
                let para = self.make_para();
                if let Some(hit) = para.hit_test(Point::new(cx, (cy - vlh).max(0.0) + vlh * 0.3)) {
                    self.cursor.1 = hit.cursor();
                    return;
                }
            }
            // move to previous logical line, land on last visual line at same x
            if self.cursor.0 > 0 {
                self.cursor.0 -= 1;
                let prev_lfs = self.line_fs();
                let prev = &self.lines[self.cursor.0];
                let num_vl = wrapped_visual_lines(prev, prev_lfs, Font::DEFAULT, w);
                let prev_para = Paragraph::with_text(iced::advanced::Text {
                    content: prev, bounds: Size::new(w, f32::MAX),
                    size: Pixels(prev_lfs), line_height: iced::advanced::text::LineHeight::Relative(1.3),
                    font: Font::DEFAULT, horizontal_alignment: iced::alignment::Horizontal::Left,
                    vertical_alignment: iced::alignment::Vertical::Top,
                    shaping: iced::advanced::text::Shaping::Advanced,
                    wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
                });
                let target_y = (num_vl.saturating_sub(1)) as f32 * prev_lfs * 1.3 + prev_lfs * 1.3 * 0.3;
                if let Some(hit) = prev_para.hit_test(Point::new(cx, target_y)) {
                    self.cursor.1 = hit.cursor();
                } else {
                    self.cursor.1 = char_len(prev);
                }
            }
            return;
        }
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.cursor.1 = self.cursor.1.min(char_len(&self.lines[self.cursor.0]));
        }
    }

    pub fn move_down(&mut self) {
        let w = self.text_area_width;
        if w > 0.0 && self.cursor.0 < self.lines.len() {
            let lfs = self.line_fs();
            let line = &self.lines[self.cursor.0];
            let num_vl = wrapped_visual_lines(line, lfs, Font::DEFAULT, w);
            let (cx, cy) = wrapped_cursor_pos(line, self.cursor.1, lfs, Font::DEFAULT, w);
            let vlh = lfs * 1.3;
            let cur_vl = (cy / vlh).round() as usize;

            if num_vl > 1 && cur_vl + 1 < num_vl {
                // move within wrapped line
                let para = self.make_para();
                if let Some(hit) = para.hit_test(Point::new(cx, (cur_vl + 1) as f32 * vlh + vlh * 0.3)) {
                    self.cursor.1 = hit.cursor();
                    return;
                }
            }
            // move to next logical line, preserving visual x position
            if self.cursor.0 + 1 < self.lines.len() {
                self.cursor.0 += 1;
                let next_lfs = self.line_fs();
                let next = &self.lines[self.cursor.0];
                let next_para = Paragraph::with_text(iced::advanced::Text {
                    content: next, bounds: Size::new(w, f32::MAX),
                    size: Pixels(next_lfs), line_height: iced::advanced::text::LineHeight::Relative(1.3),
                    font: Font::DEFAULT, horizontal_alignment: iced::alignment::Horizontal::Left,
                    vertical_alignment: iced::alignment::Vertical::Top,
                    shaping: iced::advanced::text::Shaping::Advanced,
                    wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
                });
                // land at same x on the first visual line of next paragraph
                if let Some(hit) = next_para.hit_test(Point::new(cx, next_lfs * 1.3 * 0.3)) {
                    self.cursor.1 = hit.cursor();
                } else {
                    self.cursor.1 = self.cursor.1.min(char_len(next));
                }
            }
            return;
        }
        if self.cursor.0 + 1 < self.lines.len() {
            self.cursor.0 += 1;
            self.cursor.1 = self.cursor.1.min(char_len(&self.lines[self.cursor.0]));
        }
    }

    pub fn move_home(&mut self) { self.cursor.1 = 0; }
    pub fn move_end(&mut self) {
        if self.cursor.0 < self.lines.len() {
            self.cursor.1 = char_len(&self.lines[self.cursor.0]);
        }
    }

    pub fn move_doc_start(&mut self) { self.cursor = (0, 0); }
    pub fn move_doc_end(&mut self) {
        let last = self.lines.len().saturating_sub(1);
        self.cursor = (last, char_len(&self.lines[last]));
    }

    pub fn select_all(&mut self) {
        self.selection = Some(((0, 0), self.cursor));
        self.move_doc_end();
        if let Some(ref mut sel) = self.selection {
            sel.1 = self.cursor;
        }
    }
}


pub struct MdEditorWidget<'a, Message> {
    state: &'a MdEditorState,
    on_edit: Box<dyn Fn(MdAction) -> Message + 'a>,
    font_size: f32,
    padding: Padding,
    scrollbar: bool,
}

#[derive(Debug, Clone)]
pub enum MdAction {
    Click(f32, f32),
    ShiftClick(f32, f32),
    DragTo(f32, f32),
    DoubleClick(f32, f32),
    TripleClick(f32, f32),
    Release,
    Insert(char),
    Paste(String),
    Enter,
    Backspace,
    Delete,
    Indent,
    Unindent,
    Move(MdMotion),
    Select(MdMotion),
    SelectAll,
    Copy,
    Cut,
    Undo,
    Redo,
    Scroll(f32),
    RightClick,
    ToggleCheckbox(usize), // line index
    Focus,
    Unfocus,
    WindowFocus(bool),
    Tick(Instant, f32), // (now, text_area_width)
    SlashSelect, // execute the currently selected slash command
    SlashClickSelect(usize), // click-select a specific item index in filtered list
    SlashArrow(bool), // true = down, false = up
    CopyCodeBlock(usize), // copy content of code block starting at line index
    CopyPasswordBlock(usize), // copy content of password block at line index
    TogglePasswordVisible(usize), // toggle show/hide for password block at line index
    TableAddRow(usize), // add a row after line index
    TableAddCol(usize), // add a column to table containing line index
    TableDeleteRow(usize), // delete row at line index
    TableDeleteCol(usize), // delete the column the cursor is in
    TableDelete(usize), // delete entire table
    ScrollTo(f32), // set scroll position directly (for scrollbar drag)
    ImageResize(usize, f32, f32), // (line, new_width, new_height)
    ImageResizeStart(usize, f32, f32, f32, f32), // (line, mouse_x, mouse_y, current_w, current_h)
    ImageResizeDrag(f32, f32), // (mouse_x, mouse_y) during drag
    ImageResizeEnd,
    ImageDelete(usize), // delete image at line
    CodeLangMenuOpen(usize), // open language picker for code block at line
    CodeLangSelect(usize, String), // set language for code block at line
    FileExport(String, String), // (file_id, filename) — export attached file
    FileDelete(String), // file_id — delete attached file
    OpenLink(String), // open URL in browser
}

#[derive(Debug, Clone, Copy)]
pub enum MdMotion {
    Left, Right, Up, Down, Home, End, DocStart, DocEnd,
    WordLeft, WordRight,
}

impl<'a, Message: 'a> MdEditorWidget<'a, Message> {
    pub fn new(state: &'a MdEditorState, on_edit: impl Fn(MdAction) -> Message + 'a) -> Self {
        Self {
            state,
            on_edit: Box::new(on_edit),
            font_size: 15.0,
            padding: Padding::new(12.0).left(16.0).right(16.0),
            scrollbar: true,
        }
    }

    pub fn size(mut self, s: f32) -> Self { self.font_size = s; self }
    pub fn no_scrollbar(mut self) -> Self { self.scrollbar = false; self }
    pub fn padding(mut self, p: impl Into<Padding>) -> Self { self.padding = p.into(); self }
}


struct WidgetState {
    dragging: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<Point>,
    click_count: u8,
    shift_held: bool,
    scrollbar_dragging: bool,
    scrollbar_drag_start_y: f32,    // mouse y when drag started
    scrollbar_drag_start_scroll: f32, // scroll_offset when drag started
}

impl<'a, Message: 'a> Widget<Message, iced::Theme, iced::Renderer> for MdEditorWidget<'a, Message> {
    fn tag(&self) -> tree::Tag { tree::Tag::of::<WidgetState>() }
    fn state(&self) -> tree::State { tree::State::new(WidgetState { dragging: false, last_click_time: None, last_click_pos: None, click_count: 0, shift_held: false, scrollbar_dragging: false, scrollbar_drag_start_y: 0.0, scrollbar_drag_start_scroll: 0.0 }) }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.width(Length::Fill).height(Length::Fill).max())
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::text::Renderer as TR;
        use iced::advanced::Renderer as QR;

        let bounds = layout.bounds();
        let text_bounds = bounds.shrink(self.padding);
        let fs = self.font_size;
        let cursor_line = self.state.cursor.0;
        let cursor_col = self.state.cursor.1;
        let scroll = self.state.scroll_offset;

        let n_lines = self.state.lines.len();
        let mut in_code_block = vec![false; n_lines];
        let mut code_block_start: Vec<Option<usize>> = vec![None; n_lines];
        let mut code_block_lang: Vec<String> = vec![String::new(); n_lines];
        {
            let mut i = 0;
            while i < n_lines {
                if self.state.lines[i].trim_start().starts_with("```") {
                    let start = i;
                    let lang = self.state.lines[i].trim_start().trim_start_matches('`').trim().to_lowercase();
                    let mut found_end = false;
                    for j in (start + 1)..n_lines {
                        if self.state.lines[j].trim_start().starts_with("```") {
                            for li in start..=j {
                                in_code_block[li] = true;
                                code_block_start[li] = Some(start);
                                code_block_lang[li] = lang.clone();
                            }
                            i = j + 1;
                            found_end = true;
                            break;
                        }
                    }
                    if !found_end { i += 1; } // Incomplete — skip, don't format
                } else {
                    i += 1;
                }
            }
        }

        let mut in_password_block = vec![false; n_lines];
        let mut password_block_range: Vec<Option<(usize, usize)>> = vec![None; n_lines];
        {
            let mut i = 0;
            while i < n_lines {
                if self.state.lines[i].trim_start() == "%%pass" {
                    let start = i;
                    let mut found_end = false;
                    for j in (start + 1)..n_lines {
                        if self.state.lines[j].trim_start() == "%%pass" {
                            for li in start..=j {
                                in_password_block[li] = true;
                                password_block_range[li] = Some((start, j));
                            }
                            i = j + 1;
                            found_end = true;
                            break;
                        }
                    }
                    if !found_end { i += 1; }
                } else {
                    i += 1;
                }
            }
        }

        let mut table_block_range: Vec<Option<(usize, usize)>> = vec![None; n_lines];
        {
            let mut i = 0;
            while i < n_lines {
                let t = self.state.lines[i].trim_start();
                if t.starts_with('|') && t.ends_with('|') {
                    let start = i;
                    let mut end = i;
                    while end + 1 < n_lines {
                        let nt = self.state.lines[end + 1].trim_start();
                        if nt.starts_with('|') && nt.ends_with('|') { end += 1; } else { break; }
                    }
                    for li in start..=end { table_block_range[li] = Some((start, end)); }
                    i = end + 1;
                } else {
                    i += 1;
                }
            }
        }

        let image_lines: Vec<bool> = self.state.lines.iter()
            .map(|l| {
                let t = l.trim_start();
                t.starts_with("![") && t.contains("](") && t.ends_with(')')
            })
            .collect();

        let text_color = Color::from_rgb(0.85, 0.85, 0.87);

        {
            let border_c = Color::from_rgb(0x32 as f32 / 255.0, 0x32 as f32 / 255.0, 0x32 as f32 / 255.0);
            let bg_prim = Color::from_rgb(0x1F as f32 / 255.0, 0x1F as f32 / 255.0, 0x1F as f32 / 255.0);
            let mut drawn_blocks = std::collections::HashSet::<usize>::new();
            let mut py = text_bounds.y - scroll;
            for (li, l) in self.state.lines.iter().enumerate() {
                let lh = wrapped_line_height(l, fs, text_bounds.width, &self.state.image_sizes);
                if let Some((start, end)) = password_block_range[li] {
                    if !drawn_blocks.contains(&start) {
                        drawn_blocks.insert(start);
                        let block_y = py;
                        let mut block_h = lh;
                        let mut bj = li + 1;
                        while bj <= end && bj < self.state.lines.len() {
                            block_h += wrapped_line_height(&self.state.lines[bj], fs, text_bounds.width, &self.state.image_sizes);
                            bj += 1;
                        }
                        if block_y + block_h > text_bounds.y && block_y < text_bounds.y + text_bounds.height {
                            QR::fill_quad(renderer, Quad {
                                bounds: Rectangle::new(Point::new(text_bounds.x, block_y), Size::new(text_bounds.width, block_h)),
                                border: Border { radius: 8.0.into(), width: 1.0, color: border_c },
                                ..Quad::default()
                            }, Background::Color(bg_prim));
                        }
                    }
                }
                py += lh;
            }
        }

        {
            let border_c = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
            let mut drawn = std::collections::HashSet::<usize>::new();
            let mut ty = text_bounds.y - scroll;
            for (li, l) in self.state.lines.iter().enumerate() {
                let lh = wrapped_line_height(l, fs, text_bounds.width, &self.state.image_sizes);
                if let Some((start, end)) = table_block_range[li] {
                    if !drawn.contains(&start) {
                        drawn.insert(start);
                        let block_y = ty;
                        let mut block_h = lh;
                        for bj in (li + 1)..=end.min(n_lines - 1) {
                            block_h += wrapped_line_height(&self.state.lines[bj], fs, text_bounds.width, &self.state.image_sizes);
                        }
                        if block_y + block_h > text_bounds.y && block_y < text_bounds.y + text_bounds.height {
                            QR::fill_quad(renderer, Quad {
                                bounds: Rectangle::new(Point::new(text_bounds.x, block_y), Size::new(text_bounds.width, block_h)),
                                border: Border { radius: 8.0.into(), width: 1.0, color: border_c },
                                ..Quad::default()
                            }, Background::Color(Color::TRANSPARENT));
                        }
                    }
                }
                ty += lh;
            }
        }

        let mut y = text_bounds.y - scroll;

        for (i, line) in self.state.lines.iter().enumerate() {
            let is_cursor_line = self.state.focused && i == cursor_line;
            let trimmed = line.trim_start();

            let (line_fs, line_font) = if trimmed.starts_with("# ") {
                (fs * 1.8, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT })
            } else if trimmed.starts_with("## ") {
                (fs * 1.5, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT })
            } else if trimmed.starts_with("### ") {
                (fs * 1.3, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT })
            } else if trimmed.starts_with("#### ") {
                (fs * 1.15, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT })
            } else {
                (fs, Font::DEFAULT)
            };

            let is_img_for_height = i < image_lines.len() && image_lines[i];
            let line_h = if is_img_for_height {
                let trimmed = line.trim_start();
                let img_h = if let (Some(ae), Some(pe)) = (trimmed.find("]("), trimmed.rfind(')')) {
                    let path = &trimmed[ae + 2..pe];
                    if let Some(&(_, ch)) = self.state.image_sizes.get(path) {
                        (ch + 8.0).max(line_fs * 1.3)
                    } else if let Some(handle) = self.state.image_cache.get(path) {
                        use iced::advanced::image::Renderer as IR;
                        let sz = IR::measure_image(renderer, handle);
                        let max_w = text_bounds.width - 16.0;
                        let scale = (max_w / sz.width as f32).min(400.0 / sz.height as f32).min(1.0);
                        (sz.height as f32 * scale + 8.0).max(line_fs * 1.3)
                    } else {
                        line_fs * 1.3
                    }
                } else { line_fs * 1.3 };
                img_h
            } else {
                let is_file = trimmed.starts_with("[file:") && trimmed.ends_with(']');
                if is_file { fs * 3.2 } else {
                    wrapped_line_height(line, fs, text_bounds.width, &self.state.image_sizes)
                }
            };

            let buf = 20.0;
            if y + line_h < text_bounds.y - buf { y += line_h; continue; }
            if y > text_bounds.y + text_bounds.height + buf { y += line_h; continue; }

            let mut actual_h = line_h;
            let draw_x = text_bounds.x;

            let is_selected = self.state.selection_ordered()
                .map(|(start, end)| i >= start.0 && i <= end.0 && start != end)
                .unwrap_or(false);

            let is_divider = trimmed == "---" || trimmed == "***" || trimmed == "___";
            if is_divider && !is_cursor_line && !is_selected {
                let rule_y = y + line_h / 2.0;
                QR::fill_quad(renderer, Quad {
                    bounds: Rectangle::new(Point::new(text_bounds.x, rule_y), Size::new(text_bounds.width, 1.0)),
                    ..Quad::default()
                }, Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.12)));
            }

            let is_file_line = trimmed.starts_with("[file:") && trimmed.ends_with(']');
            if is_file_line && !is_cursor_line {
                let inner = &trimmed[1..trimmed.len()-1]; // strip [ ]
                let after_prefix = &inner[5..]; // skip "file:"
                if let Some(colon1) = after_prefix.find(':') {
                    let file_id = &after_prefix[..colon1];
                    let rest = &after_prefix[colon1+1..];
                    if let Some(colon2) = rest.rfind(':') {
                        let filename = &rest[..colon2];
                        let size_str = &rest[colon2+1..];

                        let card_h = fs * 2.8;
                        let card_w = text_bounds.width.min(320.0);
                        let card_x = draw_x;
                        let card_y = y + 2.0;

                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(card_x, card_y), Size::new(card_w, card_h)),
                            border: Border { radius: (fs * 0.4).into(), width: 1.0, color: Color::from_rgba(1.0, 1.0, 1.0, 0.08) },
                            ..Quad::default()
                        }, Background::Color(Color::from_rgb(0.13, 0.13, 0.15)));

                        let icon_x = card_x + fs * 0.7;
                        let icon_y = card_y + card_h / 2.0 - fs * 0.5;
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(icon_x, icon_y), Size::new(fs * 0.8, fs * 1.0)),
                            border: Border { radius: (fs * 0.12).into(), ..Border::default() },
                            ..Quad::default()
                        }, Background::Color(Color::from_rgb(0.35, 0.35, 0.38)));
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(icon_x + fs * 0.5, icon_y), Size::new(fs * 0.3, fs * 0.3)),
                            border: Border { radius: (fs * 0.06).into(), ..Border::default() },
                            ..Quad::default()
                        }, Background::Color(Color::from_rgb(0.25, 0.25, 0.28)));

                        TR::fill_text(renderer, iced::advanced::Text {
                            content: filename.to_string(),
                            bounds: Size::new(card_w - fs * 3.0, fs * 1.3),
                            size: Pixels(fs * 0.85), line_height: text::LineHeight::Relative(1.3),
                            font: Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT },
                            horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
                        }, Point::new(card_x + fs * 2.0, card_y + fs * 0.35), Color::from_rgb(0.88, 0.88, 0.90), bounds);

                        TR::fill_text(renderer, iced::advanced::Text {
                            content: size_str.to_string(),
                            bounds: Size::new(card_w - fs * 3.0, fs * 1.0),
                            size: Pixels(fs * 0.7), line_height: text::LineHeight::Relative(1.3),
                            font: Font::DEFAULT,
                            horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Basic, wrapping: text::Wrapping::None,
                        }, Point::new(card_x + fs * 2.0, card_y + fs * 1.5), Color::from_rgb(0.55, 0.55, 0.58), bounds);
                    }
                }
                y += line_h;
                continue;
            }

            if !is_cursor_line && !is_selected && (trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") || trimmed.starts_with("- [ ] ")) {
                let checked = trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ");
                let leading_text = &line[..line.len() - trimmed.len()];
                let lead_w = measure_text_width(leading_text, fs, Font::DEFAULT);

                let box_size = fs * 0.85;
                let box_y = y + (line_h - box_size) / 2.0 + 2.0;
                let box_x = draw_x + lead_w + 1.0;

                if checked {
                    QR::fill_quad(renderer, Quad {
                        bounds: Rectangle::new(Point::new(box_x, box_y), Size::new(box_size, box_size)),
                        border: Border { radius: 3.0.into(), ..Border::default() },
                        ..Quad::default()
                    }, Background::Color(Color::from_rgb(0.22, 0.52, 0.32)));

                    let placeholder_w = {
                        let pp = Paragraph::with_text(iced::advanced::Text {
                            content: "    ", bounds: Size::new(f32::MAX, f32::MAX),
                            size: Pixels(fs), line_height: text::LineHeight::Relative(1.3),
                            font: Font::DEFAULT, horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
                        });
                        pp.min_width()
                    };
                    let text_content = &trimmed[6..];
                    let text_w = {
                        let tp = Paragraph::with_text(iced::advanced::Text {
                            content: text_content, bounds: Size::new(f32::MAX, f32::MAX),
                            size: Pixels(fs), line_height: text::LineHeight::Relative(1.3),
                            font: Font::DEFAULT, horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced, wrapping: text::Wrapping::None,
                        });
                        tp.min_width()
                    };
                    let strike_x = draw_x + lead_w + placeholder_w;
                    let strike_y = y + line_h / 2.0;
                    QR::fill_quad(renderer, Quad {
                        bounds: Rectangle::new(Point::new(strike_x, strike_y), Size::new(text_w, 1.0)),
                        ..Quad::default()
                    }, Background::Color(Color::from_rgb(0.45, 0.45, 0.47)));
                } else {
                    QR::fill_quad(renderer, Quad {
                        bounds: Rectangle::new(Point::new(box_x, box_y), Size::new(box_size, box_size)),
                        border: Border { radius: 3.0.into(), width: 1.5, color: Color::from_rgb(0.4, 0.4, 0.43) },
                        ..Quad::default()
                    }, Background::Color(Color::TRANSPARENT));
                }
            }

            let is_table_line = trimmed.starts_with('|') && trimmed.ends_with('|');
            let is_img_line = i < image_lines.len() && image_lines[i];
            if is_selected && !is_img_line {
                if let Some((start, end)) = self.state.selection_ordered() {
                    let sel_start_col = if i == start.0 { start.1 } else { 0 };
                    let sel_end_col = if i == end.0 { end.1 } else { char_len(line) };

                    if is_table_line && !is_separator_line(trimmed) {
                        let parsed = parse_table_cells(line);
                        let cell_count = parsed.len().max(1);
                        let cell_w = text_bounds.width / cell_count as f32;
                        let (start_cell, start_in) = cursor_to_cell(line, sel_start_col);
                        let (end_cell, end_in) = cursor_to_cell(line, sel_end_col);
                        for ci in start_cell..=end_cell.min(cell_count - 1) {
                            let cx = draw_x + ci as f32 * cell_w;
                            let cell_text = parsed.get(ci).map(|(_, _, t)| t.clone()).unwrap_or_default();
                            let cs = if ci == start_cell { start_in } else { 0 };
                            let ce = if ci == end_cell { end_in } else { cell_text.chars().count() };
                            let st: String = cell_text.chars().take(cs).collect();
                            let et: String = cell_text.chars().take(ce).collect();
                            let sx = measure_text_width(&st, fs * 0.85, Font::DEFAULT);
                            let ex = measure_text_width(&et, fs * 0.85, Font::DEFAULT);
                            let sel_rect = Rectangle::new(
                                Point::new(cx + 8.0 + sx, y + 1.0),
                                Size::new((ex - sx).max(2.0), actual_h - 2.0),
                            );
                            if let Some(clipped) = bounds.intersection(&sel_rect) {
                                QR::fill_quad(renderer, Quad {
                                    bounds: clipped, border: Border { radius: 2.0.into(), ..Border::default() },
                                    ..Quad::default()
                                }, Background::Color(Color::from_rgba(0.7, 0.75, 0.85, 0.15)));
                            }
                        }
                    } else if !is_table_line {
                        let (s_x, s_y) = wrapped_cursor_pos(line, sel_start_col, line_fs, line_font, text_bounds.width);
                        let (e_x, e_y) = wrapped_cursor_pos(line, sel_end_col, line_fs, line_font, text_bounds.width);
                        let vlh = line_fs * 1.3;
                        let sel_color = Color::from_rgba(0.7, 0.75, 0.85, 0.15);

                        if (s_y - e_y).abs() < 1.0 {
                            let sel_rect = Rectangle::new(
                                Point::new(draw_x + s_x, y + s_y + 1.0),
                                Size::new((e_x - s_x).max(2.0), vlh - 2.0),
                            );
                            if let Some(clipped) = bounds.intersection(&sel_rect) {
                                QR::fill_quad(renderer, Quad { bounds: clipped, border: Border { radius: 2.0.into(), ..Border::default() }, ..Quad::default() }, Background::Color(sel_color));
                            }
                        } else {
                            let start_vl = (s_y / vlh) as usize;
                            let end_vl = (e_y / vlh) as usize;
                            for vl in start_vl..=end_vl {
                                let vy = vl as f32 * vlh;
                                let left = if vl == start_vl { s_x } else { 0.0 };
                                let right = if vl == end_vl { e_x } else { text_bounds.width };
                                let sel_rect = Rectangle::new(
                                    Point::new(draw_x + left, y + vy + 1.0),
                                    Size::new((right - left).max(2.0), vlh - 2.0),
                                );
                                if let Some(clipped) = bounds.intersection(&sel_rect) {
                                    QR::fill_quad(renderer, Quad { bounds: clipped, border: Border { radius: 2.0.into(), ..Border::default() }, ..Quad::default() }, Background::Color(sel_color));
                                }
                            }
                        }
                    }
                }
            }

            for (mi, sm) in self.state.search_matches.iter().enumerate() {
                if sm.line != i { continue; }
                let sm_rect = if is_table_line && !is_separator_line(trimmed) {
                    let parsed = parse_table_cells(line);
                    let cell_count = parsed.len().max(1);
                    let cell_w = text_bounds.width / cell_count as f32;
                    let (s_cell, s_in) = cursor_to_cell(line, sm.start_col);
                    let (e_cell, e_in) = cursor_to_cell(line, sm.end_col);
                    let cx = draw_x + s_cell.min(cell_count - 1) as f32 * cell_w + 8.0;
                    let s_text = parsed.get(s_cell).map(|(_, _, t)| t.clone()).unwrap_or_default();
                    let st: String = s_text.chars().take(s_in).collect();
                    let et: String = s_text.chars().take(if s_cell == e_cell { e_in } else { s_text.chars().count() }).collect();
                    let sx = measure_text_width(&st, fs * 0.85, Font::DEFAULT);
                    let ex = measure_text_width(&et, fs * 0.85, Font::DEFAULT);
                    Rectangle::new(Point::new(cx + sx, y + 1.0), Size::new((ex - sx).max(2.0), actual_h - 2.0))
                } else {
                    let sm_start_text: String = line.chars().take(sm.start_col).collect();
                    let sm_end_text: String = line.chars().take(sm.end_col).collect();
                    let sm_sx = measure_text_width(&sm_start_text, line_fs, line_font);
                    let sm_ex = measure_text_width(&sm_end_text, line_fs, line_font);
                    Rectangle::new(Point::new(draw_x + sm_sx, y + 1.0), Size::new(sm_ex - sm_sx, actual_h - 2.0))
                };
                let is_current = mi == self.state.current_match;
                let color = if is_current {
                    Color::from_rgba(0.3, 0.8, 0.3, 0.35) // brighter lime for current
                } else {
                    Color::from_rgba(0.3, 0.7, 0.3, 0.18) // faint lime for others
                };
                if let Some(clipped) = bounds.intersection(&sm_rect) {
                    QR::fill_quad(renderer, Quad {
                        bounds: clipped,
                        border: Border { radius: 2.0.into(), ..Border::default() },
                        ..Quad::default()
                    }, Background::Color(color));
                }
            }

            let line_colors: Vec<&ColorRange> = self.state.colors.iter().filter(|c| c.line == i).collect();

            let is_code_block = i < in_code_block.len() && in_code_block[i];
            let is_password_block = i < in_password_block.len() && in_password_block[i];
            let is_image_line = i < image_lines.len() && image_lines[i];

            fn draw_icon(renderer: &mut iced::Renderer, handle: iced::widget::svg::Handle, x: f32, y: f32, size: f32, color: Option<Color>) {
                use iced::advanced::svg::Renderer as SR;
                let svg = iced::advanced::svg::Svg {
                    handle,
                    color,
                    rotation: iced::Radians(0.0),
                    opacity: 1.0,
                };
                SR::draw_svg(renderer, svg, Rectangle::new(Point::new(x, y), Size::new(size, size)));
            }

            if is_code_block {
                let is_opening_fence = line.trim_start().starts_with("```") && code_block_start[i] == Some(i);
                let is_closing_fence = line.trim_start().starts_with("```") && code_block_start[i] != Some(i);
                let is_fence = is_opening_fence || is_closing_fence;
                let code_bg_rect = Rectangle::new(Point::new(text_bounds.x, y), Size::new(text_bounds.width, actual_h));
                if let Some(clipped) = text_bounds.intersection(&code_bg_rect) {
                    QR::fill_quad(renderer, Quad {
                        bounds: clipped,
                        border: Border { radius: if is_fence { 4.0 } else { 0.0 }.into(), ..Border::default() },
                        ..Quad::default()
                    }, Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.2)));
                }

                if is_opening_fence {
                    let lang = &code_block_lang[i];
                    let icon_size = 14.0;
                    let margin = 8.0;
                    let muted = Color::from_rgb(0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0);

                    let is_copied = self.state.copied_block_line == Some(i);
                    let copy_x = text_bounds.x + text_bounds.width - icon_size - margin;
                    let btn_y = y + actual_h - icon_size / 2.0 + 1.0;
                    let handle = if is_copied { crate::ui::icons::copy_check() } else { crate::ui::icons::copy_icon() };
                    draw_icon(renderer, handle, copy_x, btn_y, icon_size, None);

                    let lang_label = if lang.is_empty() { "plain" } else { lang.as_str() };
                    let lang_text = format!("{} \u{25BE}", lang_label);
                    let lang_w = measure_text_width(&lang_text, fs * 0.7, Font::MONOSPACE);
                    let lang_x = copy_x - lang_w - 12.0;
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: lang_text,
                        bounds: Size::new(lang_w + 4.0, actual_h),
                        size: Pixels(fs * 0.7),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::MONOSPACE,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::None,
                    }, Point::new(lang_x, y + actual_h - icon_size / 2.0 + 1.0), muted, bounds);
                }
            }

            if is_password_block {
                let is_pass_fence = line.trim_start() == "%%pass";
                let block_range = password_block_range[i];
                let block_start = block_range.map(|r| r.0).unwrap_or(i);
                let is_visible = self.state.password_visible.contains(&block_start);
                let is_copied = self.state.copied_block_line == Some(block_start);

                let is_first_content = !is_pass_fence && block_range.map(|r| r.0 + 1 == i).unwrap_or(false);
                if is_first_content {
                    let icon_size = 14.0;
                    let btn_y = y + (actual_h - icon_size) / 2.0;
                    let copy_x = draw_x + text_bounds.width - icon_size * 2.0 - 18.0;
                    draw_icon(renderer,
                        if is_copied { crate::ui::icons::copy_check() } else { crate::ui::icons::copy_icon() },
                        copy_x, btn_y, icon_size, None);
                    let eye_x = draw_x + text_bounds.width - icon_size - 8.0;
                    draw_icon(renderer,
                        if is_visible { crate::ui::icons::eye_closed() } else { crate::ui::icons::eye_open() },
                        eye_x, btn_y, icon_size, None);
                }

                let pass_x = draw_x + 6.0;
                if is_pass_fence {
                    if is_cursor_line {
                        TR::fill_text(renderer, iced::advanced::Text {
                            content: line.to_string(),
                            bounds: Size::new(text_bounds.width, actual_h),
                            size: Pixels(fs),
                            line_height: text::LineHeight::Relative(1.3),
                            font: Font::DEFAULT,
                            horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced,
                            wrapping: text::Wrapping::None,
                        }, Point::new(pass_x, y),
                        Color::from_rgb(0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0), bounds);
                    }
                } else if is_cursor_line {
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: line.to_string(),
                        bounds: Size::new(text_bounds.width - 60.0, actual_h + fs * 1.3),
                        size: Pixels(fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::DEFAULT,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(pass_x, y), text_color, bounds);
                } else if is_visible {
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: line.to_string(),
                        bounds: Size::new(text_bounds.width - 60.0, actual_h + fs * 1.3),
                        size: Pixels(fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::DEFAULT,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(pass_x, y), text_color, bounds);
                } else if !line.is_empty() {
                    let dots: String = "\u{2022}".repeat(line.chars().count());
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: dots,
                        bounds: Size::new(text_bounds.width - 60.0, actual_h + fs * 1.3),
                        size: Pixels(fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::DEFAULT,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(pass_x, y), text_color, bounds);
                }
            }

            if is_password_block {
            } else if is_image_line {
                let trimmed = line.trim_start();
                if let (Some(alt_end), Some(path_end)) = (trimmed.find("]("), trimmed.rfind(')')) {
                    let alt = &trimmed[2..alt_end];
                    let path = &trimmed[alt_end + 2..path_end];

                    let mut drew_image = false;
                    if !path.is_empty() && path != "path" {
                        if let Some(handle) = self.state.image_cache.get(path) {
                            use iced::advanced::image::Renderer as IR;
                            let img_size = IR::measure_image(renderer, handle);

                            let (draw_w, draw_h) = if let Some(&(cw, ch)) = self.state.image_sizes.get(path) {
                                (cw, ch)
                            } else {
                                let max_w = text_bounds.width - 16.0;
                                let max_h = 400.0f32;
                                let scale = (max_w / img_size.width as f32).min(max_h / img_size.height as f32).min(1.0);
                                let w = img_size.width as f32 * scale;
                                let h = img_size.height as f32 * scale;
                                // can't mutate from draw, but resize handler uses same calculation
                                (w, h)
                            };

                            if y + draw_h + 4.0 > bounds.y && y < bounds.y + bounds.height {
                                let img_x = draw_x + 8.0;
                                let img_y = y + 2.0;
                                let img_rect = Rectangle::new(Point::new(img_x, img_y), Size::new(draw_w, draw_h));
                                let accent = Color::from_rgb(0.18, 0.55, 0.31); // same as canvas ACCENT
                                let selected = is_cursor_line;

                                if selected {
                                    let glow_pad = 6.0;
                                    let glow_rect = Rectangle::new(
                                        Point::new(img_x - glow_pad, img_y - glow_pad),
                                        Size::new(draw_w + glow_pad * 2.0, draw_h + glow_pad * 2.0),
                                    );
                                    if let Some(clip) = text_bounds.intersection(&glow_rect) {
                                        QR::fill_quad(renderer, Quad {
                                            bounds: clip,
                                            border: Border { radius: 8.0.into(), ..Border::default() },
                                            ..Quad::default()
                                        }, Background::Color(Color::from_rgba(accent.r, accent.g, accent.b, 0.2)));
                                    }
                                }

                                if let Some(clipped_area) = text_bounds.intersection(&Rectangle::new(
                                    Point::new(img_x - 2.0, img_y - 2.0),
                                    Size::new(draw_w + 4.0, draw_h + 4.0),
                                )) {
                                    QR::with_layer(renderer, clipped_area, |renderer| {
                                        let image = iced::advanced::image::Image {
                                            handle: handle.clone(),
                                            filter_method: iced::advanced::image::FilterMethod::Linear,
                                            rotation: iced::Radians(0.0),
                                            opacity: 1.0,
                                            snap: true,
                                        };
                                        IR::draw_image(renderer, image, img_rect);

                                    });
                                    if is_selected || selected {
                                        QR::with_layer(renderer, clipped_area, |renderer| {
                                            QR::fill_quad(renderer, Quad {
                                                bounds: img_rect,
                                                border: Border { radius: 4.0.into(), ..Border::default() },
                                                ..Quad::default()
                                            }, Background::Color(Color::from_rgba(accent.r, accent.g, accent.b, 0.25)));
                                        });
                                    }
                                }

                                if selected {
                                    let hs = 8.0;
                                    let hh = hs / 2.0;
                                    let corners = [
                                        (img_x - hh, img_y - hh),
                                        (img_x + draw_w - hh, img_y - hh),
                                        (img_x - hh, img_y + draw_h - hh),
                                        (img_x + draw_w - hh, img_y + draw_h - hh),
                                    ];
                                    for (cx, cy) in &corners {
                                        let hr = Rectangle::new(Point::new(*cx, *cy), Size::new(hs, hs));
                                        if let Some(clip) = text_bounds.intersection(&hr) {
                                            QR::fill_quad(renderer, Quad {
                                                bounds: clip,
                                                border: Border { radius: 2.0.into(), width: 1.5, color: accent },
                                                ..Quad::default()
                                            }, Background::Color(Color::from_rgb(0x1F as f32 / 255.0, 0x1F as f32 / 255.0, 0x1F as f32 / 255.0)));
                                        }
                                    }
                                }
                            }
                            drew_image = true;
                        }
                    }

                    if !drew_image {
                        let label = if alt.is_empty() { if path.starts_with("img:") { "Image" } else { path } } else { alt };
                        TR::fill_text(renderer, iced::advanced::Text {
                            content: format!("\u{1F5BC} {}", label),
                            bounds: Size::new(text_bounds.width, actual_h),
                            size: Pixels(fs * 0.85),
                            line_height: text::LineHeight::Relative(1.3),
                            font: Font::DEFAULT,
                            horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced,
                            wrapping: text::Wrapping::None,
                        }, Point::new(draw_x + 4.0, y), Color::from_rgb(0.5, 0.65, 0.85), bounds);
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(text_bounds.x, y), Size::new(text_bounds.width, actual_h)),
                            border: Border { radius: 4.0.into(), width: 1.0, color: Color::from_rgba(1.0, 1.0, 1.0, 0.06) },
                            ..Quad::default()
                        }, Background::Color(Color::TRANSPARENT));
                    }
                }
            } else if is_divider && !is_cursor_line && !is_selected {
            } else if trimmed.starts_with('|') && trimmed.ends_with('|') {
                let is_separator = trimmed.contains('-') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                let parsed_cells = parse_table_cells(line);
                let cell_count = if !parsed_cells.is_empty() { parsed_cells.len() } else {
                    let mut cc = 3;
                    for di in 1..5 {
                        if i >= di { let pc = parse_table_cells(&self.state.lines[i - di]);
                            if !pc.is_empty() { cc = pc.len(); break; } }
                    }
                    cc
                };

                let grid = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
                let cell_text_col = Color::from_rgb(0.6, 0.6, 0.62);
                let header_text_col = Color::from_rgb(0.45, 0.7, 0.5); // green headers

                let is_header = if i + 1 < self.state.lines.len() {
                    let next = self.state.lines[i + 1].trim_start();
                    next.starts_with('|') && next.contains('-') && next.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
                } else { false };

                let table_w = text_bounds.width;
                let cell_w = table_w / cell_count.max(1) as f32;
                let text_size = fs * 0.85;

                if !is_separator {
                    if is_cursor_line && !parsed_cells.is_empty() {
                        let (ac, _) = cursor_to_cell(line, cursor_col);
                        let cx = draw_x + ac.min(cell_count - 1) as f32 * cell_w;
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(cx, y), Size::new(cell_w, actual_h)),
                            ..Quad::default()
                        }, Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03)));
                    }

                    for ci in 1..cell_count {
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(draw_x + ci as f32 * cell_w, y), Size::new(1.0, actual_h)),
                            ..Quad::default()
                        }, Background::Color(grid));
                    }

                    let is_last_data = if i + 1 >= self.state.lines.len() { true } else {
                        let nt = self.state.lines[i + 1].trim_start();
                        !(nt.starts_with('|') && nt.ends_with('|'))
                    };
                    if !is_last_data {
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(draw_x, y + actual_h - 1.0), Size::new(table_w, 1.0)),
                            ..Quad::default()
                        }, Background::Color(grid));
                    }

                    for (ci, (_cs, _ce, cell_text)) in parsed_cells.iter().enumerate() {
                        if !cell_text.is_empty() {
                            let cx = draw_x + ci as f32 * cell_w;
                            TR::fill_text(renderer, iced::advanced::Text {
                                content: cell_text.clone(),
                                bounds: Size::new(cell_w - 16.0, actual_h),
                                size: Pixels(text_size),
                                line_height: text::LineHeight::Relative(1.3),
                                font: Font::DEFAULT,
                                horizontal_alignment: alignment::Horizontal::Left,
                                vertical_alignment: alignment::Vertical::Top,
                                shaping: text::Shaping::Advanced,
                                wrapping: text::Wrapping::None,
                            }, Point::new(cx + 8.0, y + 3.0), if is_header { header_text_col } else { cell_text_col }, bounds);
                        }
                    }
                }
            } else if is_code_block {
                let is_fence = line.trim_start().starts_with("```");
                let lang = &code_block_lang[i];
                if is_fence && !is_cursor_line {
                } else if is_fence && is_cursor_line {
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: line.to_string(),
                        bounds: Size::new(text_bounds.width, actual_h * 2.0),
                        size: Pixels(fs * 0.9),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::MONOSPACE,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::None,
                    }, Point::new(draw_x + 6.0, y),
                    Color::from_rgb(0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0), bounds);
                } else if !line.is_empty() {
                    let segs = syntax_highlight_line(line, lang, fs * 0.9);
                    let mut sx = draw_x + 6.0;
                    for seg in &segs {
                        if seg.text.is_empty() { continue; }
                        let seg_w = measure_text_width(&seg.text, fs * 0.9, Font::MONOSPACE);
                        TR::fill_text(renderer, iced::advanced::Text {
                            content: seg.text.clone(),
                            bounds: Size::new(text_bounds.width - (sx - text_bounds.x), actual_h * 2.0),
                            size: Pixels(fs * 0.9),
                            line_height: text::LineHeight::Relative(1.3),
                            font: Font::MONOSPACE,
                            horizontal_alignment: alignment::Horizontal::Left,
                            vertical_alignment: alignment::Vertical::Top,
                            shaping: text::Shaping::Advanced,
                            wrapping: text::Wrapping::None,
                        }, Point::new(sx, y), seg.color, bounds);
                        sx += seg_w;
                    }
                }
            } else if is_cursor_line || is_selected {
                // Cursor line shows raw text (with markers visible).
                // For headings, strip the marker prefix so the display text stays in place
                // and show the marker in a muted color separately.
                let is_heading = trimmed.starts_with("# ") || trimmed.starts_with("## ")
                    || trimmed.starts_with("### ") || trimmed.starts_with("#### ");
                if is_heading {
                    let marker_len = if trimmed.starts_with("#### ") { 5 }
                        else if trimmed.starts_with("### ") { 4 }
                        else if trimmed.starts_with("## ") { 3 }
                        else { 2 };
                    let leading = &line[..line.len() - trimmed.len()];
                    let marker = &trimmed[..marker_len];
                    let content = &trimmed[marker_len..];
                    // Draw marker in muted color
                    let marker_text = format!("{}{}", leading, marker);
                    let marker_w = measure_text_width(&marker_text, line_fs, line_font);
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: marker_text,
                        bounds: Size::new(marker_w + 1.0, actual_h + line_fs * 1.3),
                        size: Pixels(line_fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: line_font,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::None,
                    }, Point::new(draw_x, y), Color::from_rgba(0.85, 0.85, 0.87, 0.3), bounds);
                    // Draw content after marker
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: content.to_string(),
                        bounds: Size::new(text_bounds.width - marker_w, actual_h + line_fs * 1.3),
                        size: Pixels(line_fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: line_font,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(draw_x + marker_w, y), text_color, bounds);
                } else {
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: line.to_string(),
                        bounds: Size::new(text_bounds.width, actual_h + line_fs * 1.3),
                        size: Pixels(line_fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: line_font,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(draw_x, y), text_color, bounds);
                }
            } else {
                // Detect alignment tags
                let (align_tag, display_line) = if line.starts_with("{center}") {
                    (alignment::Horizontal::Center, &line[8..])
                } else if line.starts_with("{right}") {
                    (alignment::Horizontal::Right, &line[7..])
                } else {
                    (alignment::Horizontal::Left, line.as_str())
                };
                let (mut segments, leading_ws) = build_display_segments(display_line);
                // Apply runtime color ranges (from color picker) to segments,
                // adjusting for prefix offset between raw line and display segments.
                // ColorRange columns are in raw-line coordinates (matching cursor positions).
                // Display segments may differ due to: alignment tag stripping, leading ws separation,
                // and prefix transformation (e.g., "# " stripped, "- " → "•  ").
                if !line_colors.is_empty() {
                    let align_prefix_chars = line.chars().count() - display_line.chars().count();
                    let dl_trimmed = display_line.trim_start();
                    let leading_ws_chars = display_line.chars().count() - dl_trimmed.chars().count();
                    let (raw_stripped, display_added) = raw_to_display_prefix_offset(dl_trimmed);
                    // Total raw chars before content = align + leading_ws + raw_prefix_stripped
                    let raw_before = align_prefix_chars + leading_ws_chars + raw_stripped;
                    // Total display chars before content in segments = display_prefix_added
                    let display_before = display_added;
                    let adjusted: Vec<ColorRange> = line_colors.iter().map(|cr| {
                        let adj_start = if cr.start_col >= raw_before {
                            cr.start_col - raw_before + display_before
                        } else if cr.start_col >= align_prefix_chars + leading_ws_chars {
                            // Falls within the prefix area — map to display prefix
                            let pos_in_raw_prefix = cr.start_col - align_prefix_chars - leading_ws_chars;
                            (pos_in_raw_prefix * display_added / raw_stripped.max(1)).min(display_added)
                        } else { 0 };
                        let adj_end = if cr.end_col >= raw_before {
                            cr.end_col - raw_before + display_before
                        } else if cr.end_col >= align_prefix_chars + leading_ws_chars {
                            let pos_in_raw_prefix = cr.end_col - align_prefix_chars - leading_ws_chars;
                            (pos_in_raw_prefix * display_added / raw_stripped.max(1)).min(display_added)
                        } else { 0 };
                        ColorRange { line: cr.line, start_col: adj_start, end_col: adj_end, color: cr.color.clone() }
                    }).collect();
                    let adj_refs: Vec<&ColorRange> = adjusted.iter().collect();
                    apply_color_ranges_to_segments(&mut segments, &adj_refs);
                }
                let display_text: String = format!("{}{}", leading_ws, segments.iter().map(|s| s.text.as_str()).collect::<String>());
                let display_font = if segments.iter().any(|s| s.font.weight == iced::font::Weight::Bold) {
                    Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }
                } else { line_font };
                let display_color = segments.first().map(|s| s.color).unwrap_or(text_color);
                // Extra height buffer for wrapping: prevents clipping last wrap line
                let render_h = actual_h + line_fs * 1.3;

                // Check if line fits on one visual line — if so, use per-segment rendering
                // for correct per-word bold/color. Otherwise, fall back to single fill_text
                // with wrapping (sacrificing per-word styling for correct layout).
                let total_display_w = measure_text_width(&display_text, line_fs, display_font);
                let has_mixed_styles = segments.len() > 1 && segments.iter().any(|s|
                    s.font.weight == iced::font::Weight::Bold
                    || s.color != segments[0].color);
                let fits_one_line = total_display_w <= text_bounds.width;

                if has_mixed_styles {
                    // Per-segment rendering for correct per-word bold/color.
                    // Works for both single-line and wrapped text.
                    let leading_ws_w = measure_text_width(&leading_ws, line_fs, Font::DEFAULT);

                    if fits_one_line {
                        // Single visual line — simple left-to-right segment rendering
                        let base_x = match align_tag {
                            alignment::Horizontal::Center => {
                                draw_x + (text_bounds.width - total_display_w) / 2.0
                            }
                            alignment::Horizontal::Right => {
                                draw_x + text_bounds.width - total_display_w
                            }
                            _ => draw_x,
                        };
                        let link_green = Color::from_rgb(0.45, 0.75, 0.5);
                        let mut seg_x = base_x + leading_ws_w;
                        for seg in &segments {
                            if seg.text.is_empty() { continue; }
                            let seg_font = if seg.font.weight == iced::font::Weight::Bold {
                                Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }
                            } else { line_font };
                            let seg_fs = line_fs * seg.size_mult;
                            let seg_w = measure_text_width(&seg.text, seg_fs, seg_font);
                            TR::fill_text(renderer, iced::advanced::Text {
                                content: seg.text.clone(),
                                bounds: Size::new(seg_w + 1.0, render_h),
                                size: Pixels(seg_fs),
                                line_height: text::LineHeight::Relative(1.3),
                                font: seg_font,
                                horizontal_alignment: alignment::Horizontal::Left,
                                vertical_alignment: alignment::Vertical::Top,
                                shaping: text::Shaping::Advanced,
                                wrapping: text::Wrapping::None,
                            }, Point::new(seg_x, y), seg.color, bounds);
                            // Draw underline for link segments
                            let is_link = seg.color.r == link_green.r && seg.color.g == link_green.g && seg.color.b == link_green.b;
                            if is_link {
                                let underline_y = y + seg_fs * 1.1;
                                QR::fill_quad(renderer, Quad {
                                    bounds: Rectangle::new(Point::new(seg_x, underline_y), Size::new(seg_w, 1.0)),
                                    ..Quad::default()
                                }, Background::Color(seg.color));
                            }
                            seg_x += seg_w;
                        }
                    } else {
                        // Wrapped text with mixed styles — manual word-wrap per segment.
                        // We walk segments word by word, measuring with the correct font
                        // for each segment, and wrap to the next visual line when needed.
                        let vlh = line_fs * 1.3;
                        let avail = text_bounds.width;
                        let mut cur_x = draw_x + leading_ws_w;
                        let link_green_w = Color::from_rgb(0.45, 0.75, 0.5);
                        let mut cur_vl = 0;

                        for seg in &segments {
                            if seg.text.is_empty() { continue; }
                            let seg_font = if seg.font.weight == iced::font::Weight::Bold {
                                Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }
                            } else { line_font };
                            let seg_fs = line_fs * seg.size_mult;
                            let is_link_seg = seg.color.r == link_green_w.r && seg.color.g == link_green_w.g && seg.color.b == link_green_w.b;

                            // Split segment into words (keeping spaces attached to word starts)
                            let mut remaining = seg.text.as_str();
                            while !remaining.is_empty() {
                                // Find next word boundary (end of word + trailing spaces)
                                let word_end = remaining.find(' ')
                                    .map(|i| {
                                        // include trailing spaces
                                        let after = &remaining[i..];
                                        let spaces = after.len() - after.trim_start().len();
                                        i + spaces
                                    })
                                    .unwrap_or(remaining.len());
                                let word = &remaining[..word_end];
                                let word_w = measure_text_width(word, seg_fs, seg_font);

                                // Check if word fits on current visual line
                                if cur_x + word_w > draw_x + avail && cur_x > draw_x + 1.0 {
                                    // Wrap to next visual line
                                    cur_vl += 1;
                                    cur_x = draw_x;
                                    // Skip leading spaces on the new visual line
                                    let word_trimmed = word.trim_start();
                                    if word_trimmed.is_empty() {
                                        remaining = &remaining[word_end..];
                                        continue;
                                    }
                                    let tw = measure_text_width(word_trimmed, seg_fs, seg_font);
                                    let wy = y + cur_vl as f32 * vlh;
                                    TR::fill_text(renderer, iced::advanced::Text {
                                        content: word_trimmed.to_string(),
                                        bounds: Size::new(tw + 1.0, vlh),
                                        size: Pixels(seg_fs),
                                        line_height: text::LineHeight::Relative(1.3),
                                        font: seg_font,
                                        horizontal_alignment: alignment::Horizontal::Left,
                                        vertical_alignment: alignment::Vertical::Top,
                                        shaping: text::Shaping::Advanced,
                                        wrapping: text::Wrapping::None,
                                    }, Point::new(cur_x, wy), seg.color, bounds);
                                    if is_link_seg {
                                        QR::fill_quad(renderer, Quad { bounds: Rectangle::new(Point::new(cur_x, wy + seg_fs * 1.1), Size::new(tw, 1.0)), ..Quad::default() }, Background::Color(seg.color));
                                    }
                                    cur_x += tw;
                                } else {
                                    let wy = y + cur_vl as f32 * vlh;
                                    TR::fill_text(renderer, iced::advanced::Text {
                                        content: word.to_string(),
                                        bounds: Size::new(word_w + 1.0, vlh),
                                        size: Pixels(seg_fs),
                                        line_height: text::LineHeight::Relative(1.3),
                                        font: seg_font,
                                        horizontal_alignment: alignment::Horizontal::Left,
                                        vertical_alignment: alignment::Vertical::Top,
                                        shaping: text::Shaping::Advanced,
                                        wrapping: text::Wrapping::None,
                                    }, Point::new(cur_x, wy), seg.color, bounds);
                                    if is_link_seg {
                                        let uw = measure_text_width(word.trim_end(), seg_fs, seg_font);
                                        QR::fill_quad(renderer, Quad { bounds: Rectangle::new(Point::new(cur_x, wy + seg_fs * 1.1), Size::new(uw, 1.0)), ..Quad::default() }, Background::Color(seg.color));
                                    }
                                    cur_x += word_w;
                                }
                                remaining = &remaining[word_end..];
                            }
                        }
                    }
                } else {
                    // Single-style text (no mixed bold/color): render as one text block with wrapping
                    let text_x = match align_tag {
                        alignment::Horizontal::Center => draw_x + text_bounds.width / 2.0,
                        alignment::Horizontal::Right => draw_x + text_bounds.width,
                        _ => draw_x,
                    };
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: display_text.clone(),
                        bounds: Size::new(text_bounds.width, render_h),
                        size: Pixels(line_fs),
                        line_height: text::LineHeight::Relative(1.3),
                        font: display_font,
                        horizontal_alignment: align_tag,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::WordOrGlyph,
                    }, Point::new(text_x, y), display_color, bounds);
                    // Underline if single-style link (link is only content on the line)
                    let link_green_s = Color::from_rgb(0.45, 0.75, 0.5);
                    if display_color.r == link_green_s.r && display_color.g == link_green_s.g && display_color.b == link_green_s.b {
                        let dw = measure_text_width(&display_text, line_fs, display_font);
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(draw_x, y + line_fs * 1.1), Size::new(dw, 1.0)),
                            ..Quad::default()
                        }, Background::Color(display_color));
                    }
                }
            }

            let is_img_cur = i < image_lines.len() && image_lines[i];
            if is_cursor_line && !is_img_cur && self.state.focused && self.state.is_window_focused {
                let elapsed_ms = self.state.focus_instant
                    .map(|fi| (self.state.now - fi).as_millis() as u64)
                    .unwrap_or(0);
                let visible = (elapsed_ms / 530) % 2 == 0;
                let cursor_color = if visible {
                    Color::from_rgb(0.65, 0.65, 0.68)
                } else {
                    Color::from_rgb(0.30, 0.30, 0.32)
                };

                // Compute cursor X and Y offset.
                // cx = x position relative to draw_x
                // cursor_y_off = additional y offset (for wrapped lines or table padding)
                // All cases use the same height/centering formula based on line_fs.
                let pass_block_start = if is_password_block { password_block_range[i].map(|r| r.0) } else { None };
                let pass_visible = pass_block_start.map(|s| self.state.password_visible.contains(&s)).unwrap_or(false);
                let (cx, cursor_y_off) = if is_password_block {
                    // Cursor line always shows raw text, so measure raw text width
                    let t: String = line.chars().take(cursor_col).collect();
                    (measure_text_width(&t, fs, Font::DEFAULT) + 6.0, 0.0)
                } else if is_code_block {
                    let t: String = line.chars().take(cursor_col).collect();
                    (measure_text_width(&t, fs * 0.9, Font::MONOSPACE) + 6.0, 0.0)
                } else if trimmed.starts_with('|') && trimmed.ends_with('|')
                    && !is_separator_line(trimmed) {
                    let parsed = parse_table_cells(line);
                    let cell_count = parsed.len().max(1);
                    let cell_w = text_bounds.width / cell_count as f32;
                    let (cell_idx, col_in_cell) = cursor_to_cell(line, cursor_col);
                    let cell_x = cell_idx.min(cell_count - 1) as f32 * cell_w + 8.0;
                    let cell_text = parsed.get(cell_idx.min(parsed.len().saturating_sub(1)))
                        .map(|(_, _, t)| t.clone()).unwrap_or_default();
                    let t: String = cell_text.chars().take(col_in_cell).collect();
                    (measure_text_width(&t, fs * 0.85, Font::DEFAULT) + cell_x, 0.0)
                } else {
                    // For headings on cursor line, marker is rendered separately.
                    // Cursor before/in marker: position in marker area.
                    // Cursor in content: position relative to content start.
                    let is_heading_cur = trimmed.starts_with("# ") || trimmed.starts_with("## ")
                        || trimmed.starts_with("### ") || trimmed.starts_with("#### ");
                    if is_heading_cur && is_cursor_line {
                        let marker_len = if trimmed.starts_with("#### ") { 5 }
                            else if trimmed.starts_with("### ") { 4 }
                            else if trimmed.starts_with("## ") { 3 }
                            else { 2 };
                        let leading_len = line.len() - trimmed.len();
                        let marker_char_len = leading_len + marker_len;
                        let marker_text: String = line.chars().take(marker_char_len).collect();
                        let marker_w = measure_text_width(&marker_text, line_fs, line_font);
                        if cursor_col <= marker_char_len {
                            // Cursor is in the marker prefix
                            let t: String = line.chars().take(cursor_col).collect();
                            (measure_text_width(&t, line_fs, line_font), 0.0)
                        } else {
                            // Cursor is in the content after marker
                            let content: String = line.chars().skip(marker_char_len).collect();
                            let col_in_content = cursor_col - marker_char_len;
                            let (wrap_x, wrap_y) = wrapped_cursor_pos(&content, col_in_content, line_fs, line_font, text_bounds.width - marker_w);
                            (marker_w + wrap_x, wrap_y)
                        }
                    } else {
                        wrapped_cursor_pos(line, cursor_col, line_fs, line_font, text_bounds.width)
                    }
                };

                // For code blocks, align cursor tightly with code text (fs*0.9)
                let (cursor_h, cursor_y) = if is_code_block {
                    let code_fs = fs * 0.9;
                    (code_fs, y + cursor_y_off + 1.0)
                } else {
                    let h = line_fs * 0.95;
                    (h, y + cursor_y_off + (line_fs * 1.3 - h) / 2.0)
                };
                let cursor_rect = Rectangle::new(
                    Point::new(draw_x + cx, cursor_y),
                    Size::new(1.0, cursor_h),
                );
                if let Some(clipped) = bounds.intersection(&cursor_rect) {
                    QR::fill_quad(renderer, Quad { bounds: clipped, ..Quad::default() }, Background::Color(cursor_color));
                }
            }

            y += actual_h;
        }

        let total_content_h = y - (text_bounds.y - scroll);
        if self.scrollbar && total_content_h > bounds.height && bounds.height > 50.0 {
            let pad = 8.0;
            let track_h = (bounds.height - pad * 2.0).max(20.0);
            let bar_ratio = bounds.height / total_content_h;
            let bar_h = (track_h * bar_ratio).max(24.0).min(track_h);
            let max_scroll = (total_content_h - bounds.height).max(1.0);
            let bar_y = (scroll / max_scroll).clamp(0.0, 1.0) * (track_h - bar_h);

            let hover_zone = Rectangle::new(
                Point::new(bounds.x + bounds.width - 16.0, bounds.y),
                Size::new(16.0, bounds.height),
            );
            let is_hovered = _cursor.position().map_or(false, |p| hover_zone.contains(p));
            let (bar_w, bar_color) = if is_hovered {
                (5.0, Color::from_rgba(0.6, 0.6, 0.65, 0.5))
            } else {
                (3.0, Color::from_rgba(0.5, 0.5, 0.55, 0.35))
            };
            let bar_x = bounds.x + bounds.width - bar_w - 2.0;

            let thumb_rect = Rectangle::new(
                Point::new(bar_x, bounds.y + pad + bar_y),
                Size::new(bar_w, bar_h),
            );
            // clip to widget bounds
            if let Some(clipped) = bounds.intersection(&thumb_rect) {
                QR::fill_quad(renderer, Quad {
                    bounds: clipped,
                    border: Border { radius: (bar_w / 2.0).into(), ..Border::default() },
                    ..Quad::default()
                }, Background::Color(bar_color));
            }
        }

        QR::with_layer(renderer, bounds, |renderer| {

        if self.state.slash_menu_open {
            let filtered = filter_slash_commands(&self.state.slash_filter);
            if !filtered.is_empty() {
                let cursor_line = self.state.cursor.0;
                let mut popup_y = text_bounds.y - scroll;
                for li in 0..cursor_line {
                    if li < self.state.lines.len() {
                        popup_y += wrapped_line_height(&self.state.lines[li], fs, text_bounds.width, &self.state.image_sizes);
                    }
                }
                let cursor_line_h = if cursor_line < self.state.lines.len() {
                    wrapped_line_height(&self.state.lines[cursor_line], fs, text_bounds.width, &self.state.image_sizes)
                } else { fs * 1.3 };
                popup_y += cursor_line_h + 2.0;

                let popup_x = text_bounds.x;
                let item_h = 32.0;
                let popup_w = 170.0;
                let menu_pad = 6.0;
                let popup_h = (filtered.len() as f32 * item_h) + menu_pad * 2.0;
                let popup_y = popup_y.min(bounds.y + bounds.height - popup_h - 4.0);

                QR::fill_quad(renderer, Quad {
                    bounds: Rectangle::new(
                        Point::new(popup_x + 2.0, popup_y + 4.0),
                        Size::new(popup_w, popup_h),
                    ),
                    border: Border { radius: 8.0.into(), ..Border::default() },
                    ..Quad::default()
                }, Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3)));
                QR::fill_quad(renderer, Quad {
                    bounds: Rectangle::new(Point::new(popup_x, popup_y), Size::new(popup_w, popup_h)),
                    border: Border { radius: 8.0.into(), width: 1.0, color: Color::from_rgb(0.25, 0.25, 0.25) },
                    ..Quad::default()
                }, Background::Color(Color::from_rgb(0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0x28 as f32 / 255.0)));

                for (fi, &cmd_idx) in filtered.iter().enumerate() {
                    let cmd = &SLASH_COMMANDS[cmd_idx];
                    let item_y = popup_y + menu_pad + fi as f32 * item_h;
                    let is_sel = fi == self.state.slash_selected;

                    if is_sel {
                        QR::fill_quad(renderer, Quad {
                            bounds: Rectangle::new(Point::new(popup_x + 4.0, item_y), Size::new(popup_w - 8.0, item_h)),
                            border: Border { radius: 4.0.into(), ..Border::default() },
                            ..Quad::default()
                        }, Background::Color(Color::from_rgb(0x3A as f32 / 255.0, 0x3A as f32 / 255.0, 0x3A as f32 / 255.0)));
                    }

                    let icon_x = popup_x + 10.0;
                    let icon_y = item_y + (item_h - 14.0) / 2.0;
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: cmd.icon.to_string(),
                        bounds: Size::new(20.0, 14.0),
                        size: Pixels(10.0),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::MONOSPACE,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::None,
                    }, Point::new(icon_x, icon_y),
                    Color::from_rgb(0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0), bounds);

                    let label_x = popup_x + 34.0;
                    let label_y = item_y + (item_h - 12.0) / 2.0;
                    TR::fill_text(renderer, iced::advanced::Text {
                        content: cmd.label.to_string(),
                        bounds: Size::new(popup_w - 44.0, item_h),
                        size: Pixels(12.0),
                        line_height: text::LineHeight::Relative(1.3),
                        font: Font::DEFAULT,
                        horizontal_alignment: alignment::Horizontal::Left,
                        vertical_alignment: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: text::Wrapping::None,
                    }, Point::new(label_x, label_y),
                    Color::from_rgb(0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0),
                    bounds);
                }
            }
        }

        if let Some(block_line) = self.state.code_lang_menu {
            let langs = ["plain", "rust", "python", "javascript", "typescript", "java", "c", "cpp", "csharp", "go", "sql", "bash", "html", "css"];
            let item_h = 28.0;
            let popup_w = 130.0;
            let menu_pad = 4.0;
            let popup_h = (langs.len() as f32 * item_h) + menu_pad * 2.0;

            let mut popup_y = text_bounds.y - scroll;
            for li in 0..=block_line {
                if li < block_line && li < self.state.lines.len() {
                    popup_y += wrapped_line_height(&self.state.lines[li], fs, text_bounds.width, &self.state.image_sizes);
                }
            }
            if block_line < self.state.lines.len() {
                popup_y += wrapped_line_height(&self.state.lines[block_line], fs, text_bounds.width, &self.state.image_sizes);
            }
            let popup_x = text_bounds.x + text_bounds.width - popup_w - 30.0;
            let popup_y = popup_y.min(bounds.y + bounds.height - popup_h - 4.0);

            let bg_sec = Color::from_rgb(0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0x28 as f32 / 255.0);
            QR::fill_quad(renderer, Quad {
                bounds: Rectangle::new(Point::new(popup_x, popup_y), Size::new(popup_w, popup_h)),
                border: Border { radius: 8.0.into(), width: 1.0, color: Color::from_rgb(0.25, 0.25, 0.25) },
                ..Quad::default()
            }, Background::Color(bg_sec));

            let current_lang = &code_block_lang[block_line];
            for (li, lang_name) in langs.iter().enumerate() {
                let iy = popup_y + menu_pad + li as f32 * item_h;
                let is_current = (current_lang.is_empty() && *lang_name == "plain") || current_lang == lang_name;
                if is_current {
                    QR::fill_quad(renderer, Quad {
                        bounds: Rectangle::new(Point::new(popup_x + 3.0, iy), Size::new(popup_w - 6.0, item_h)),
                        border: Border { radius: 4.0.into(), ..Border::default() },
                        ..Quad::default()
                    }, Background::Color(Color::from_rgb(0x3A as f32 / 255.0, 0x3A as f32 / 255.0, 0x3A as f32 / 255.0)));
                }
                TR::fill_text(renderer, iced::advanced::Text {
                    content: lang_name.to_string(),
                    bounds: Size::new(popup_w - 20.0, item_h),
                    size: Pixels(12.0),
                    line_height: text::LineHeight::Relative(1.3),
                    font: Font::DEFAULT,
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Top,
                    shaping: text::Shaping::Advanced,
                    wrapping: text::Wrapping::None,
                }, Point::new(popup_x + 10.0, iy + (item_h - 12.0) / 2.0),
                Color::from_rgb(0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0), bounds);
            }
        }

        }); // end with_layer for popups
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let wstate = tree.state.downcast_mut::<WidgetState>();
        let bounds = layout.bounds();

        match &event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                let has_velocity = self.state.scroll_velocity.abs() > 0.5;
                if self.state.focused || has_velocity {
                    let tw = bounds.width - self.padding.left - self.padding.right;
                    shell.publish((self.on_edit)(MdAction::Tick(*now, tw)));
                    let next_ms = if self.state.focused {
                        let fi = self.state.focus_instant.unwrap_or(*now);
                        500 - ((*now - fi).as_millis() % 500)
                    } else { 16 }; // ~60fps for scroll animation
                    shell.request_redraw(window::RedrawRequest::At(*now + Duration::from_millis(next_ms as u64)));
                    return event::Status::Ignored;
                }
            }
            Event::Window(window::Event::Focused) => {
                shell.publish((self.on_edit)(MdAction::WindowFocus(true)));
            }
            Event::Window(window::Event::Unfocused) => {
                // Stop any in-progress drag so selection isn't corrupted on refocus
                if wstate.dragging {
                    wstate.dragging = false;
                    shell.publish((self.on_edit)(MdAction::Release));
                }
                shell.publish((self.on_edit)(MdAction::WindowFocus(false)));
            }
            _ => {}
        }

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) = &event {
            wstate.shift_held = mods.shift();
        }

        // scrollbar drag handling
        if wstate.scrollbar_dragging {
            match &event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    // use absolute cursor position so drag works even outside widget
                    let mouse_y = cursor.position().map(|p| p.y - bounds.y)
                        .or_else(|| cursor.position_in(bounds).map(|p| p.y))
                        .unwrap_or(wstate.scrollbar_drag_start_y);
                    let dy = mouse_y - wstate.scrollbar_drag_start_y;
                    let fs = self.font_size;
                    let avail_w = bounds.width - self.padding.left - self.padding.right;
                    let total_h: f32 = self.state.lines.iter()
                        .map(|l| wrapped_line_height(l, fs, avail_w, &self.state.image_sizes)).sum();
                    let max_scroll = (total_h - bounds.height).max(1.0);
                    let pad = 8.0;
                    let track_h = (bounds.height - pad * 2.0).max(20.0);
                    let bar_ratio = bounds.height / total_h;
                    let bar_h = (track_h * bar_ratio).max(24.0).min(track_h);
                    let usable_track = (track_h - bar_h).max(1.0);
                    // 1px mouse movement = 1px thumb movement → scale to scroll space
                    let target = (wstate.scrollbar_drag_start_scroll + dy * (max_scroll / usable_track)).clamp(0.0, max_scroll);
                    shell.publish((self.on_edit)(MdAction::ScrollTo(target)));
                    return event::Status::Captured;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    wstate.scrollbar_dragging = false;
                    return event::Status::Captured;
                }
                _ => {}
            }
        }

        match &event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    // check scrollbar click
                    if self.scrollbar && pos.x > bounds.width - 16.0 {
                        let ratio = (pos.y / bounds.height).clamp(0.0, 1.0);
                        let fs = self.font_size;
                        let avail_w = bounds.width - self.padding.left - self.padding.right;
                        let total_h: f32 = self.state.lines.iter()
                            .map(|l| wrapped_line_height(l, fs, avail_w, &self.state.image_sizes)).sum();
                        if total_h > bounds.height {
                            wstate.scrollbar_dragging = true;
                            wstate.scrollbar_drag_start_y = pos.y;
                            wstate.scrollbar_drag_start_scroll = self.state.scroll_offset;
                            return event::Status::Captured;
                        }
                    }
                    let x = pos.x - self.padding.left;
                    let y = pos.y - self.padding.top + self.state.scroll_offset;

                    let evt_avail_w = self.state.text_area_width;
                    if self.state.slash_menu_open {
                        let filtered = filter_slash_commands(&self.state.slash_filter);
                        if !filtered.is_empty() {
                            let cursor_line = self.state.cursor.0;
                            let scroll = self.state.scroll_offset;
                            let mut popup_y = self.padding.top - scroll;
                            for li in 0..cursor_line {
                                if li < self.state.lines.len() {
                                    popup_y += wrapped_line_height(&self.state.lines[li], self.font_size, evt_avail_w, &self.state.image_sizes);
                                }
                            }
                            let clh = if cursor_line < self.state.lines.len() {
                                wrapped_line_height(&self.state.lines[cursor_line], self.font_size, evt_avail_w, &self.state.image_sizes)
                            } else { self.font_size * 1.3 };
                            popup_y += clh + 2.0;
                            let item_h = 32.0;
                            let popup_w = 170.0;
                            let menu_pad = 6.0;
                            let popup_h = (filtered.len() as f32 * item_h) + menu_pad * 2.0;
                            let popup_y = popup_y.min(bounds.height - popup_h - 4.0);
                            let popup_x = self.padding.left;

                            if pos.x >= popup_x && pos.x <= popup_x + popup_w
                                && pos.y >= popup_y && pos.y <= popup_y + popup_h
                            {
                                let clicked_idx = ((pos.y - popup_y - menu_pad) / item_h) as usize;
                                if clicked_idx < filtered.len() {
                                    shell.publish((self.on_edit)(MdAction::SlashClickSelect(clicked_idx)));
                                    return event::Status::Captured;
                                }
                            }
                        }
                        shell.publish((self.on_edit)(MdAction::Insert('\x08')));
                        return event::Status::Captured;
                    }

                    if self.state.code_lang_menu.is_some() {
                        let langs = ["plain", "rust", "python", "javascript", "typescript", "java", "c", "cpp", "csharp", "go", "sql", "bash", "html", "css"];
                        let block_line = self.state.code_lang_menu.unwrap();
                        let item_h = 28.0;
                        let popup_w = 130.0;
                        let menu_pad = 4.0;
                        let popup_h = (langs.len() as f32 * item_h) + menu_pad * 2.0;
                        let mut popup_y = self.padding.top - self.state.scroll_offset;
                        for li in 0..block_line {
                            if li < self.state.lines.len() {
                                popup_y += wrapped_line_height(&self.state.lines[li], self.font_size, evt_avail_w, &self.state.image_sizes);
                            }
                        }
                        if block_line < self.state.lines.len() {
                            popup_y += wrapped_line_height(&self.state.lines[block_line], self.font_size, evt_avail_w, &self.state.image_sizes);
                        }
                        let tb_w = bounds.width - self.padding.left - self.padding.right;
                        let popup_x = self.padding.left + tb_w - popup_w - 30.0;
                        let popup_y = popup_y.min(bounds.height - popup_h - 4.0);

                        if pos.x >= popup_x && pos.x <= popup_x + popup_w
                            && pos.y >= popup_y && pos.y <= popup_y + popup_h
                        {
                            let clicked_idx = ((pos.y - popup_y - menu_pad) / item_h) as usize;
                            if clicked_idx < langs.len() {
                                let selected = langs[clicked_idx].to_string();
                                shell.publish((self.on_edit)(MdAction::CodeLangSelect(block_line,
                                    if selected == "plain" { String::new() } else { selected })));
                                return event::Status::Captured;
                            }
                        }
                        shell.publish((self.on_edit)(MdAction::CodeLangSelect(block_line,
                            self.state.lines.get(block_line).map(|l| l.trim_start().trim_start_matches('`').trim().to_lowercase()).unwrap_or_default())));
                        return event::Status::Captured;
                    }

                    let now = Instant::now();
                    let _shift = iced::keyboard::Modifiers::default();

                    let is_repeat = wstate.last_click_time
                        .map(|t| now.duration_since(t) < Duration::from_millis(400))
                        .unwrap_or(false)
                        && wstate.last_click_pos
                            .map(|p| (p.x - pos.x).abs() < 5.0 && (p.y - pos.y).abs() < 5.0)
                            .unwrap_or(false);

                    if is_repeat {
                        wstate.click_count += 1;
                    } else {
                        wstate.click_count = 1;
                    }
                    wstate.last_click_time = Some(now);
                    wstate.last_click_pos = Some(pos);

                    if wstate.click_count == 2 {
                        shell.publish((self.on_edit)(MdAction::DoubleClick(x, y)));
                    } else if wstate.click_count >= 3 {
                        shell.publish((self.on_edit)(MdAction::TripleClick(x, y)));
                        wstate.click_count = 0; // reset cycle
                    } else {
                        let avail_w = self.state.text_area_width;
                        let (click_line, click_y_in_line) = {
                            let mut cy = 0.0_f32;
                            let mut found = 0;
                            let mut found_cy = 0.0_f32;
                            for (li, l) in self.state.lines.iter().enumerate() {
                                let lh = wrapped_line_height(l, self.font_size, avail_w, &self.state.image_sizes);
                                if y < cy + lh { found = li; found_cy = cy; break; }
                                cy += lh;
                                found = li;
                                found_cy = cy;
                            }
                            (found, y - found_cy)
                        };
                        {
                            let mut iy = 0.0f32;
                            for li in 0..self.state.lines.len() {
                                let l = &self.state.lines[li];
                                let lh = wrapped_line_height(l, self.font_size, avail_w, &self.state.image_sizes);
                                let lt = l.trim_start();
                                let is_img = lt.starts_with("![") && lt.contains("](img:") && lt.ends_with(')');
                                if is_img && y >= iy && y < iy + lh {
                                    let img_id = lt.find("](").and_then(|a| lt.rfind(')').map(|b| &lt[a+2..b])).unwrap_or("");
                                    let (dw, dh) = self.state.image_sizes.get(img_id).copied().unwrap_or((lh - 8.0, lh - 8.0));
                                    if x > 8.0 + dw - 14.0 && y > iy + dh - 10.0 {
                                        shell.publish((self.on_edit)(MdAction::ImageResizeStart(li, pos.x, pos.y, dw, dh)));
                                        return event::Status::Captured;
                                    }
                                    shell.publish((self.on_edit)(MdAction::Click(x, iy + 1.0)));
                                    shell.publish((self.on_edit)(MdAction::Focus));
                                    wstate.dragging = true;
                                    return event::Status::Captured;
                                }
                                iy += lh;
                            }
                        }

                        if click_line < self.state.lines.len() {
                            let cl = &self.state.lines[click_line];
                            if cl.trim_start().starts_with("```") {
                                let has_closing = self.state.lines[(click_line + 1)..].iter()
                                    .any(|l| l.trim_start().starts_with("```"));
                                if has_closing {
                                    let tb = bounds.shrink(self.padding);
                                    let icon_size = 14.0;
                                    let copy_right = tb.width - 8.0;
                                    let copy_left = copy_right - icon_size;
                                    if x >= copy_left && x <= copy_right {
                                        shell.publish((self.on_edit)(MdAction::CopyCodeBlock(click_line)));
                                        return event::Status::Captured;
                                    }
                                    let lang_right = copy_left - 8.0;
                                    let lang_left = lang_right - 80.0;
                                    if x >= lang_left.max(0.0) && x <= lang_right {
                                        shell.publish((self.on_edit)(MdAction::CodeLangMenuOpen(click_line)));
                                        return event::Status::Captured;
                                    }
                                }
                            }
                        }

                        if click_line < self.state.lines.len() && click_line > 0 {
                            let prev = &self.state.lines[click_line - 1];
                            if prev.trim_start() == "%%pass" {
                                let mut count = 0;
                                for li in 0..click_line {
                                    if self.state.lines[li].trim_start() == "%%pass" { count += 1; }
                                }
                                if count % 2 == 1 {
                                    let tb = bounds.shrink(self.padding);
                                    let icon_size = 14.0;
                                    let eye_right = tb.width - 8.0;
                                    let eye_left = eye_right - icon_size;
                                    if x >= eye_left && x <= eye_right {
                                        shell.publish((self.on_edit)(MdAction::TogglePasswordVisible(click_line - 1)));
                                        return event::Status::Captured;
                                    }
                                    let copy_right = eye_left - 4.0;
                                    let copy_left = copy_right - icon_size;
                                    if x >= copy_left && x <= copy_right {
                                        shell.publish((self.on_edit)(MdAction::CopyPasswordBlock(click_line - 1)));
                                        return event::Status::Captured;
                                    }
                                }
                            }
                        }

                        if click_line < self.state.lines.len() {
                            let tl = self.state.lines[click_line].trim_start().to_string();
                            if tl.starts_with('|') && tl.ends_with('|') {
                                let is_last = if click_line + 1 >= self.state.lines.len() { true } else {
                                    let nt = self.state.lines[click_line + 1].trim_start();
                                    !(nt.starts_with('|') && nt.ends_with('|'))
                                };
                                if is_last {
                                    let line_h = crate::ui::md_widget::line_height(&tl, self.font_size);
                                    if click_y_in_line > line_h * 0.8 {
                                        if x < 50.0 {
                                            shell.publish((self.on_edit)(MdAction::TableAddRow(click_line)));
                                            return event::Status::Captured;
                                        } else if x < 100.0 {
                                            shell.publish((self.on_edit)(MdAction::TableAddCol(click_line)));
                                            return event::Status::Captured;
                                        }
                                    }
                                }
                            }
                        }

                        if !self.state.focused && click_line < self.state.lines.len() {
                            let full_line = &self.state.lines[click_line];
                            let lt = full_line.trim_start();
                            if lt.starts_with("- [ ] ") || lt.starts_with("- [x] ") || lt.starts_with("- [X] ") {
                                let leading = &full_line[..full_line.len() - lt.len()];
                                let lead_w = measure_text_width(leading, self.font_size, Font::DEFAULT);
                                let checkbox_right = lead_w + self.font_size * 1.5;
                                if x >= lead_w.max(0.0) - 4.0 && x < checkbox_right {
                                    shell.publish((self.on_edit)(MdAction::ToggleCheckbox(click_line)));
                                    return event::Status::Captured;
                                }
                            }
                        }
                        // Link click detection on rendered (non-cursor) lines
                        let is_cursor_line = self.state.focused && click_line == self.state.cursor.0;
                        if !is_cursor_line && click_line < self.state.lines.len() {
                            if let Some(url) = detect_link_click(&self.state.lines[click_line], x, click_y_in_line, self.font_size, avail_w) {
                                shell.publish((self.on_edit)(MdAction::OpenLink(url)));
                                return event::Status::Captured;
                            }
                        }
                        if wstate.shift_held {
                            shell.publish((self.on_edit)(MdAction::ShiftClick(x, y)));
                        } else {
                            shell.publish((self.on_edit)(MdAction::Click(x, y)));
                        }
                    }
                    wstate.dragging = true;
                    return event::Status::Captured;
                } else if self.state.focused {
                    shell.publish((self.on_edit)(MdAction::Unfocus));
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    let rx = pos.x - self.padding.left;
                    let ry = pos.y - self.padding.top + self.state.scroll_offset;
                    // Only move cursor if there is no active selection, so right-click
                    // preserves selected text for formatting operations.
                    if self.state.selection.is_none() {
                        shell.publish((self.on_edit)(MdAction::Click(rx, ry)));
                    }
                    shell.publish((self.on_edit)(MdAction::Focus));
                    shell.publish((self.on_edit)(MdAction::RightClick));
                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.state.image_resizing.is_some() {
                    shell.publish((self.on_edit)(MdAction::ImageResizeEnd));
                    return event::Status::Captured;
                }
                if wstate.dragging {
                    wstate.dragging = false;
                    shell.publish((self.on_edit)(MdAction::Release));
                    return event::Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.state.image_resizing.is_some() {
                    if let Some(pos) = cursor.position_in(bounds) {
                        shell.publish((self.on_edit)(MdAction::ImageResizeDrag(pos.x, pos.y)));
                        return event::Status::Captured;
                    }
                }
                if wstate.dragging {
                    // Get cursor position relative to bounds, clamping to widget edges
                    // so drag-select works even when the cursor leaves the editor area.
                    let pos_opt = cursor.position_in(bounds).or_else(|| {
                        cursor.position().map(|abs| {
                            Point::new(
                                (abs.x - bounds.x).clamp(0.0, bounds.width),
                                (abs.y - bounds.y).clamp(0.0, bounds.height),
                            )
                        })
                    });
                    if let Some(pos) = pos_opt {
                        let x = pos.x - self.padding.left;
                        let y = pos.y - self.padding.top + self.state.scroll_offset;
                        shell.publish((self.on_edit)(MdAction::DragTo(x, y)));

                        // Auto-scroll when dragging near the top or bottom edge
                        let edge_zone = 40.0_f32;
                        let scroll_speed = self.font_size * 3.0;
                        if pos.y < edge_zone {
                            let factor = 1.0 - (pos.y / edge_zone).max(0.0);
                            shell.publish((self.on_edit)(MdAction::Scroll(scroll_speed * factor)));
                        } else if pos.y > bounds.height - edge_zone {
                            let factor = 1.0 - ((bounds.height - pos.y) / edge_zone).max(0.0);
                            shell.publish((self.on_edit)(MdAction::Scroll(-scroll_speed * factor)));
                        }

                        return event::Status::Captured;
                    }
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(bounds) {
                    let lines = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y * self.font_size * 1.2,
                        mouse::ScrollDelta::Pixels { y, .. } => *y * 0.4,
                    };
                    shell.publish((self.on_edit)(MdAction::Scroll(lines)));
                    shell.request_redraw(window::RedrawRequest::NextFrame);
                    return event::Status::Captured;
                }
            }
            _ => {}
        }

        if self.state.focused {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, text: key_text, .. }) = &event {
                let ctrl = modifiers.command();
                let shift = modifiers.shift();

                if self.state.slash_menu_open {
                    let slash_action: Option<MdAction> = match key.as_ref() {
                        keyboard::Key::Named(keyboard::key::Named::Enter) => Some(MdAction::SlashSelect),
                        keyboard::Key::Named(keyboard::key::Named::Tab) => Some(MdAction::SlashSelect),
                        keyboard::Key::Named(keyboard::key::Named::ArrowDown) => Some(MdAction::SlashArrow(true)),
                        keyboard::Key::Named(keyboard::key::Named::ArrowUp) => Some(MdAction::SlashArrow(false)),
                        keyboard::Key::Named(keyboard::key::Named::Escape) => {
                            // \x08 sentinel tells MdAction::Insert to close the menu
                            shell.publish((self.on_edit)(MdAction::Insert('\x08'))); // sentinel: will be handled as close menu
                            return event::Status::Captured;
                        }
                        _ => None,
                    };
                    if let Some(a) = slash_action {
                        shell.publish((self.on_edit)(a));
                        return event::Status::Captured;
                    }
                }

                let action: Option<MdAction> = match key.as_ref() {
                    keyboard::Key::Named(keyboard::key::Named::Enter) => Some(MdAction::Enter),
                    keyboard::Key::Named(keyboard::key::Named::Backspace) => Some(MdAction::Backspace),
                    keyboard::Key::Named(keyboard::key::Named::Delete) => Some(MdAction::Delete),
                    keyboard::Key::Named(keyboard::key::Named::Space) => Some(MdAction::Insert(' ')),
                    keyboard::Key::Named(keyboard::key::Named::Tab) => {
                        if shift {
                            Some(MdAction::Unindent)
                        } else {
                            Some(MdAction::Indent)
                        }
                    }
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        // let app handle escape for search/dialog/unfocus
                        return event::Status::Ignored;
                    }
                    keyboard::Key::Character(c) if ctrl => match c.as_ref() {
                        "a" => Some(MdAction::SelectAll),
                        "c" => {
                            if let Some(sel) = self.state.selected_text() {
                                clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                            }
                            Some(MdAction::Copy)
                        }
                        "x" => {
                            if let Some(sel) = self.state.selected_text() {
                                clipboard.write(iced::advanced::clipboard::Kind::Standard, sel);
                            }
                            Some(MdAction::Cut)
                        }
                        "v" => {
                            let text = clipboard.read(iced::advanced::clipboard::Kind::Standard).unwrap_or_default();
                            Some(MdAction::Paste(text))
                        }
                        "z" => Some(MdAction::Undo),
                        "y" => Some(MdAction::Redo),
                        _ => None,
                    },
                    keyboard::Key::Named(named) => {
                        let motion = match named {
                            keyboard::key::Named::ArrowLeft if ctrl => Some(MdMotion::WordLeft),
                            keyboard::key::Named::ArrowRight if ctrl => Some(MdMotion::WordRight),
                            keyboard::key::Named::ArrowLeft => Some(MdMotion::Left),
                            keyboard::key::Named::ArrowRight => Some(MdMotion::Right),
                            keyboard::key::Named::ArrowUp => Some(MdMotion::Up),
                            keyboard::key::Named::ArrowDown => Some(MdMotion::Down),
                            keyboard::key::Named::Home if ctrl => Some(MdMotion::DocStart),
                            keyboard::key::Named::End if ctrl => Some(MdMotion::DocEnd),
                            keyboard::key::Named::Home => Some(MdMotion::Home),
                            keyboard::key::Named::End => Some(MdMotion::End),
                            _ => None,
                        };
                        motion.map(|m| if shift { MdAction::Select(m) } else { MdAction::Move(m) })
                    }
                    _ => {
                        if let Some(txt) = key_text {
                            if let Some(c) = txt.chars().find(|c| !c.is_control()) {
                                Some(MdAction::Insert(c))
                            } else { None }
                        } else { None }
                    }
                };

                if let Some(a) = action {
                    shell.publish((self.on_edit)(a));
                    return event::Status::Captured;
                }
            }
        }

        event::Status::Ignored
    }

    fn mouse_interaction(
        &self, _tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor,
        _viewport: &Rectangle, _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(layout.bounds()) {
            // scrollbar hover
            if self.scrollbar && pos.x > layout.bounds().width - 16.0 {
                return mouse::Interaction::Pointer;
            }
            let x = pos.x - self.padding.left;
            let y = pos.y - self.padding.top + self.state.scroll_offset;

            let mi_avail_w = self.state.text_area_width;
            let mut iy = 0.0f32;
            for li in 0..self.state.lines.len() {
                let l = &self.state.lines[li];
                let lh = wrapped_line_height(l, self.font_size, mi_avail_w, &self.state.image_sizes);
                let lt = l.trim_start();
                if lt.starts_with("![") && lt.contains("](img:") && lt.ends_with(')') {
                    if y >= iy && y < iy + lh {
                        let img_id = lt.find("](").and_then(|a| lt.rfind(')').map(|b| &lt[a+2..b])).unwrap_or("");
                        let (dw, dh) = self.state.image_sizes.get(img_id).copied().unwrap_or((200.0, 150.0));
                        if x > 8.0 + dw - 14.0 && y > iy + dh - 10.0 {
                            return mouse::Interaction::ResizingDiagonallyDown;
                        }
                        return mouse::Interaction::Pointer;
                    }
                }
                if lt.starts_with("- [ ] ") || lt.starts_with("- [x] ") || lt.starts_with("- [X] ") {
                    if y >= iy && y < iy + lh && x < self.font_size * 1.5 + 4.0 {
                        return mouse::Interaction::Pointer;
                    }
                }
                // Link hover: show pointer on non-cursor lines with links
                let is_cursor_line = self.state.focused && li == self.state.cursor.0;
                if !is_cursor_line && y >= iy && y < iy + lh {
                    if detect_link_click(l, x, y - iy, self.font_size, mi_avail_w).is_some() {
                        return mouse::Interaction::Pointer;
                    }
                }
                iy += lh;
            }

            if self.state.image_resizing.is_some() {
                return mouse::Interaction::ResizingDiagonallyDown;
            }

            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}


struct TextSegment {
    text: String,
    font: Font,
    color: Color,
    size_mult: f32, // multiplier on line_fs
    link_url: Option<String>,
}

/// Build display segments for a line (markers hidden, formatting applied).
/// Returns (segments, leading_whitespace_text) — the whitespace is measured separately for positioning.
/// Simple syntax highlighting for code blocks. Returns colored segments.
fn syntax_highlight_line(line: &str, lang: &str, _fs: f32) -> Vec<TextSegment> {
    let keyword_color = Color::from_rgb(0.6, 0.5, 0.85);   // purple — keywords
    let string_color = Color::from_rgb(0.85, 0.65, 0.45);   // orange — strings
    let comment_color = Color::from_rgb(0.45, 0.55, 0.45);  // muted green — comments
    let number_color = Color::from_rgb(0.7, 0.8, 0.55);     // yellow-green — numbers
    let type_color = Color::from_rgb(0.45, 0.75, 0.85);     // cyan — types
    let normal_color = Color::from_rgb(0.82, 0.82, 0.84);   // light grey — normal
    let fn_color = Color::from_rgb(0.85, 0.8, 0.5);         // gold — function names

    let keywords: &[&str] = match lang {
        "rust" | "rs" => &["fn", "let", "mut", "if", "else", "for", "while", "loop", "match", "return", "use", "pub", "struct", "enum", "impl", "self", "Self", "true", "false", "const", "static", "mod", "crate", "super", "where", "trait", "type", "as", "in", "ref", "async", "await", "move", "break", "continue"],
        "python" | "py" => &["def", "class", "if", "elif", "else", "for", "while", "return", "import", "from", "as", "with", "try", "except", "finally", "raise", "True", "False", "None", "and", "or", "not", "in", "is", "lambda", "yield", "pass", "break", "continue", "async", "await"],
        "javascript" | "js" | "typescript" | "ts" => &["function", "const", "let", "var", "if", "else", "for", "while", "return", "import", "export", "from", "class", "new", "this", "true", "false", "null", "undefined", "async", "await", "try", "catch", "throw", "switch", "case", "break", "default", "typeof", "instanceof"],
        "java" | "c" | "cpp" | "c++" | "csharp" | "cs" => &["public", "private", "protected", "static", "void", "int", "float", "double", "char", "bool", "boolean", "string", "if", "else", "for", "while", "return", "class", "new", "this", "true", "false", "null", "try", "catch", "throw", "import", "package", "final", "const", "struct", "enum", "switch", "case", "break", "continue", "override", "virtual", "abstract", "interface"],
        "go" => &["func", "var", "const", "if", "else", "for", "range", "return", "import", "package", "type", "struct", "interface", "map", "chan", "go", "defer", "select", "case", "switch", "break", "continue", "true", "false", "nil"],
        "html" | "xml" => &[],
        "css" | "scss" => &[],
        "sql" => &["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "TABLE", "INTO", "VALUES", "SET", "JOIN", "LEFT", "RIGHT", "INNER", "ON", "AND", "OR", "NOT", "NULL", "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "AS", "IN", "LIKE", "BETWEEN", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN"],
        "bash" | "sh" | "shell" => &["if", "then", "else", "fi", "for", "do", "done", "while", "case", "esac", "function", "return", "echo", "exit", "export", "source", "local", "readonly", "set", "unset"],
        _ => &["if", "else", "for", "while", "return", "function", "class", "true", "false", "null", "import", "export", "const", "let", "var", "def", "fn"],
    };

    let type_words: &[&str] = match lang {
        "rust" | "rs" => &["String", "Vec", "Option", "Result", "Box", "HashMap", "HashSet", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64", "bool", "usize", "isize", "str", "char"],
        "typescript" | "ts" => &["string", "number", "boolean", "any", "void", "never", "unknown", "object", "Array", "Promise", "Record", "Partial"],
        "java" | "csharp" | "cs" => &["String", "Integer", "Boolean", "List", "Map", "Set", "ArrayList", "HashMap", "Object"],
        "go" => &["string", "int", "int8", "int16", "int32", "int64", "float32", "float64", "bool", "byte", "rune", "error"],
        _ => &[],
    };

    let comment_prefix = match lang {
        "python" | "py" | "bash" | "sh" | "shell" => "#",
        "html" | "xml" => "<!--",
        "css" | "scss" => "/*",
        "sql" => "--",
        _ => "//",
    };

    let mut segs = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut current = String::new();

    let flush = |segs: &mut Vec<TextSegment>, current: &mut String| {
        if !current.is_empty() {
            segs.push(TextSegment { text: current.clone(), font: Font::MONOSPACE, color: normal_color, size_mult: 1.0, link_url: None });
            current.clear();
        }
    };

    while i < len {
        if line[chars.iter().take(i).map(|c| c.len_utf8()).sum::<usize>()..].starts_with(comment_prefix) {
            flush(&mut segs, &mut current);
            let rest: String = chars[i..].iter().collect();
            segs.push(TextSegment { text: rest, font: Font::MONOSPACE, color: comment_color, size_mult: 1.0, link_url: None });
            return segs;
        }

        if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
            flush(&mut segs, &mut current);
            let quote = chars[i];
            let mut s = String::new();
            s.push(chars[i]);
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len { s.push(chars[i]); i += 1; }
                s.push(chars[i]);
                i += 1;
            }
            if i < len { s.push(chars[i]); i += 1; }
            segs.push(TextSegment { text: s, font: Font::MONOSPACE, color: string_color, size_mult: 1.0, link_url: None });
            continue;
        }

        if chars[i].is_ascii_digit() && (i == 0 || !chars[i-1].is_alphanumeric()) {
            flush(&mut segs, &mut current);
            let mut n = String::new();
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'x' || chars[i] == 'b') {
                n.push(chars[i]); i += 1;
            }
            segs.push(TextSegment { text: n, font: Font::MONOSPACE, color: number_color, size_mult: 1.0, link_url: None });
            continue;
        }

        if chars[i].is_alphanumeric() || chars[i] == '_' {
            flush(&mut segs, &mut current);
            let mut word = String::new();
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                word.push(chars[i]); i += 1;
            }
            let color = if keywords.contains(&word.as_str()) {
                keyword_color
            } else if type_words.contains(&word.as_str()) {
                type_color
            } else if i < len && chars[i] == '(' {
                fn_color
            } else {
                normal_color
            };
            segs.push(TextSegment { text: word, font: Font::MONOSPACE, color, size_mult: 1.0, link_url: None });
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }
    flush(&mut segs, &mut current);
    if segs.is_empty() {
        segs.push(TextSegment { text: " ".to_string(), font: Font::MONOSPACE, color: normal_color, size_mult: 1.0, link_url: None });
    }
    segs
}

fn build_display_segments(line: &str) -> (Vec<TextSegment>, String) {
    let trimmed = line.trim_start();
    let leading = &line[..line.len() - trimmed.len()];
    let text_color = Color::from_rgb(0.85, 0.85, 0.87);
    let checked_color = Color::from_rgb(0.45, 0.45, 0.47);

    let mut segs = Vec::new();

    if trimmed.starts_with("# ") {
        segs.extend(build_inline_segments(&trimmed[2..], Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, text_color));
    } else if trimmed.starts_with("## ") {
        segs.extend(build_inline_segments(&trimmed[3..], Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, text_color));
    } else if trimmed.starts_with("### ") {
        segs.extend(build_inline_segments(&trimmed[4..], Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, text_color));
    } else if trimmed.starts_with("#### ") {
        segs.extend(build_inline_segments(&trimmed[5..], Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, text_color));
    } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        segs.push(TextSegment { text: "- [x] ".into(), font: Font::DEFAULT, color: Color::TRANSPARENT, size_mult: 1.0, link_url: None });
        segs.extend(build_inline_segments(&trimmed[6..], Font::DEFAULT, checked_color));
    } else if trimmed.starts_with("- [ ] ") {
        segs.push(TextSegment { text: "- [ ] ".into(), font: Font::DEFAULT, color: Color::TRANSPARENT, size_mult: 1.0, link_url: None });
        segs.extend(build_inline_segments(&trimmed[6..], Font::DEFAULT, text_color));
    } else if trimmed.starts_with("- ") {
        segs.push(TextSegment { text: "•  ".into(), font: Font::DEFAULT, color: text_color, size_mult: 1.0, link_url: None });
        segs.extend(build_inline_segments(&trimmed[2..], Font::DEFAULT, text_color));
    } else if trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        if let Some(dot_pos) = trimmed.find(". ") {
            let num_str = &trimmed[..dot_pos];
            if num_str.chars().all(|c| c.is_ascii_digit()) {
                let marker_color = Color::from_rgb(0.55, 0.55, 0.57);
                segs.push(TextSegment { text: format!("{}. ", num_str), font: Font::DEFAULT, color: marker_color, size_mult: 1.0, link_url: None });
                segs.extend(build_inline_segments(&trimmed[dot_pos + 2..], Font::DEFAULT, text_color));
            } else {
                segs.extend(build_inline_segments(trimmed, Font::DEFAULT, text_color));
            }
        } else {
            segs.extend(build_inline_segments(trimmed, Font::DEFAULT, text_color));
        }
    } else {
        segs.extend(build_inline_segments(trimmed, Font::DEFAULT, text_color));
    }

    (segs, leading.to_string())
}

/// Build inline segments with formatting (bold/code only).
/// Italic and quotes kept as plain text with markers visible.
fn build_inline_segments(text: &str, base_font: Font, base_color: Color) -> Vec<TextSegment> {
    let mut segs = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut normal_start = 0;

    while i < len {
        if i + 3 < len && chars[i] == '{' && chars[i + 1] == 'c' && chars[i + 2] == ':' {
            if let Some(tag_end) = chars[i + 3..].iter().position(|c| *c == '}').map(|p| i + 3 + p) {
                let color_code: String = chars[i + 3..tag_end].iter().collect();
                let search_start = tag_end + 1;
                let remaining: String = chars[search_start..].iter().collect();
                if let Some(close_pos) = remaining.find("{/c}") {
                    let content_end = search_start + close_pos;
                    if i > normal_start {
                        let t: String = chars[normal_start..i].iter().collect();
                        segs.push(TextSegment { text: t, font: base_font, color: base_color, size_mult: 1.0, link_url: None });
                    }
                    let colored_text: String = chars[tag_end + 1..content_end].iter().collect();
                    if colored_text.is_empty() {
                        // empty color tag: show raw text
                        i += 1;
                        continue;
                    }
                    let tag_color = parse_color_code(&color_code, base_color);
                    segs.push(TextSegment { text: colored_text, font: base_font, color: tag_color, size_mult: 1.0, link_url: None });
                    i = content_end + 4; // skip {/c}
                    normal_start = i;
                    continue;
                }
            }
        }

        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars, i + 2) {
                if i > normal_start {
                    let t: String = chars[normal_start..i].iter().collect();
                    segs.push(TextSegment { text: t, font: base_font, color: base_color, size_mult: 1.0, link_url: None });
                }
                let bold_text: String = chars[i + 2..end].iter().collect();
                segs.push(TextSegment {
                    text: bold_text,
                    font: Font { weight: iced::font::Weight::Bold, ..base_font },
                    color: base_color,
                    size_mult: 1.0,
                    link_url: None,
                });
                i = end + 2;
                normal_start = i;
                continue;
            }
        }
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|c| *c == '`').map(|p| i + 1 + p) {
                if i > normal_start {
                    let t: String = chars[normal_start..i].iter().collect();
                    segs.push(TextSegment { text: t, font: base_font, color: base_color, size_mult: 1.0, link_url: None });
                }
                let code_text: String = chars[i + 1..end].iter().collect();
                segs.push(TextSegment {
                    text: code_text,
                    font: base_font,
                    color: Color::from_rgb(0.55, 0.82, 0.55),
                    size_mult: 1.0,
                    link_url: None,
                });
                i = end + 1;
                normal_start = i;
                continue;
            }
        }
        // Links: [text](url) — display only the text in link color
        if chars[i] == '[' {
            if let Some(bracket_end) = chars[i + 1..].iter().position(|c| *c == ']').map(|p| i + 1 + p) {
                if bracket_end + 1 < len && chars[bracket_end + 1] == '(' {
                    if let Some(paren_end) = chars[bracket_end + 2..].iter().position(|c| *c == ')').map(|p| bracket_end + 2 + p) {
                        if i > normal_start {
                            let t: String = chars[normal_start..i].iter().collect();
                            segs.push(TextSegment { text: t, font: base_font, color: base_color, size_mult: 1.0, link_url: None });
                        }
                        let link_text: String = chars[i + 1..bracket_end].iter().collect();
                        let link_target: String = chars[bracket_end + 2..paren_end].iter().collect();
                        segs.push(TextSegment {
                            text: link_text,
                            font: base_font,
                            color: Color::from_rgb(0.45, 0.75, 0.5),
                            size_mult: 1.0,
                            link_url: Some(link_target),
                        });
                        i = paren_end + 1;
                        normal_start = i;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    if normal_start < len {
        let t: String = chars[normal_start..].iter().collect();
        segs.push(TextSegment { text: t, font: base_font, color: base_color, size_mult: 1.0, link_url: None });
    }

    if segs.is_empty() && !text.is_empty() {
        segs.push(TextSegment { text: text.to_string(), font: base_font, color: base_color, size_mult: 1.0, link_url: None });
    }
    segs
}

/// Detect if a click at `(click_x, click_y)` on a rendered line hits a link.
/// `click_y` is relative to the line's top. `avail_w` is the wrapping width.
/// Returns `Some(url)` if so.
fn detect_link_click(line: &str, click_x: f32, click_y: f32, font_size: f32, avail_w: f32) -> Option<String> {
    // Build display segments to get accurate x-positions
    let (segments, leading_ws) = build_display_segments(line);
    let display_text: String = format!("{}{}", leading_ws, segments.iter().map(|s| s.text.as_str()).collect::<String>());

    // Determine which visual line was clicked
    let vlh = font_size * 1.3;
    let clicked_vline = (click_y / vlh).floor().max(0.0) as usize;

    // Find visual line break positions
    let num_vlines = wrapped_visual_lines(&display_text, font_size, Font::DEFAULT, avail_w);
    let para = Paragraph::with_text(iced::advanced::Text {
        content: &display_text,
        bounds: Size::new(avail_w, f32::MAX),
        size: Pixels(font_size),
        line_height: text::LineHeight::Relative(1.3),
        font: Font::DEFAULT,
        horizontal_alignment: alignment::Horizontal::Left,
        vertical_alignment: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::WordOrGlyph,
    });
    let mut vline_starts: Vec<usize> = vec![0];
    for vl in 1..num_vlines {
        let vl_char = para.hit_test(Point::new(0.0, vl as f32 * vlh + 1.0))
            .map(|h| h.cursor())
            .unwrap_or(display_text.chars().count());
        vline_starts.push(vl_char);
    }
    vline_starts.push(display_text.chars().count());

    let vl = clicked_vline.min(num_vlines.saturating_sub(1));
    let vl_start_char = vline_starts[vl];
    let vl_end_char = vline_starts[vl + 1];

    // Accumulate segment positions, tracking which visual line each segment falls on
    let leading_chars = leading_ws.chars().count();
    let mut char_offset = leading_chars;
    let mut links: Vec<(f32, f32, String)> = Vec::new();

    for seg in &segments {
        let seg_chars = seg.text.chars().count();
        let seg_start = char_offset;
        let seg_end = char_offset + seg_chars;

        if seg.link_url.is_some() {
            // This segment is a link — check if it overlaps the clicked visual line
            if seg_start < vl_end_char && seg_end > vl_start_char {
                let frag_start = seg_start.max(vl_start_char);
                let frag_end = seg_end.min(vl_end_char);

                // Measure x position on this visual line
                let chars_before_on_vline: String = display_text.chars().skip(vl_start_char).take(frag_start - vl_start_char).collect();
                let chars_to_end: String = display_text.chars().skip(vl_start_char).take(frag_end - vl_start_char).collect();
                let start_x = measure_text_width(&chars_before_on_vline, font_size, Font::DEFAULT);
                let end_x = measure_text_width(&chars_to_end, font_size, Font::DEFAULT);

                let url = seg.link_url.as_ref().unwrap().clone();
                links.push((start_x, end_x, url));
            }
        }
        char_offset = seg_end;
    }

    for (start_x, end_x, url) in links {
        if click_x >= start_x && click_x <= end_x {
            return Some(url);
        }
    }
    None
}

/// Find the position of closing ** starting from `start`
fn find_double_star(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[allow(dead_code)]
fn _build_inline_segments_old(text: &str, base_font: Font, base_color: Color) -> Vec<TextSegment> {
    let highlights = highlight_inline(text, 0);
    let mut segs = Vec::new();
    for (range, hl) in &highlights {
        let chunk = &text[range.clone()];
        if chunk.is_empty() { continue; }
        match hl {
            Highlight::Marker => {}
            _ => segs.push(TextSegment {
                text: chunk.to_string(),
                font: base_font,
                color: base_color,
                size_mult: 1.0,
                link_url: None,
            }),
        }
    }
    if segs.is_empty() && !text.is_empty() {
        segs.push(TextSegment { text: text.to_string(), font: base_font, color: base_color, size_mult: 1.0, link_url: None });
    }
    segs
}

/// Build spans for formatted display (non-active lines). Markers hidden, formatting applied.
#[allow(dead_code)]
fn build_display_spans<'a>(line: &str, line_fs: f32, base_font: Font) -> Vec<iced::advanced::text::Span<'a, (), Font>> {
    let trimmed = line.trim_start();
    let text_color = Color::from_rgb(0.85, 0.85, 0.87);

    let content = if trimmed.starts_with("# ") { &trimmed[2..] }
    else if trimmed.starts_with("## ") { &trimmed[3..] }
    else if trimmed.starts_with("### ") { &trimmed[4..] }
    else if trimmed.starts_with("#### ") { &trimmed[5..] }
    else if trimmed.starts_with("> ") {
        let pl = line.len() - trimmed.len();
        &line[pl + 2..]
    }
    else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        let pl = line.len() - trimmed.len();
        let rest = &line[pl + 6..];
        let mut spans = vec![
            iced::advanced::text::Span::new("☑ ".to_string()).size(line_fs).color(Color::from_rgb(0.4, 0.8, 0.5)),
        ];
        spans.extend(build_inline_display_spans(rest, line_fs, base_font, text_color));
        return spans;
    }
    else if trimmed.starts_with("- [ ] ") {
        let pl = line.len() - trimmed.len();
        let rest = &line[pl + 6..];
        let mut spans = vec![
            iced::advanced::text::Span::new("☐ ".to_string()).size(line_fs).color(Color::from_rgb(0.5, 0.5, 0.55)),
        ];
        spans.extend(build_inline_display_spans(rest, line_fs, base_font, text_color));
        return spans;
    }
    else if trimmed.starts_with("- ") {
        let pl = line.len() - trimmed.len();
        let rest = &line[pl + 2..];
        let mut spans = vec![
            iced::advanced::text::Span::new("• ".to_string()).size(line_fs).color(Color::from_rgb(0.55, 0.75, 0.95)),
        ];
        spans.extend(build_inline_display_spans(rest, line_fs, base_font, text_color));
        return spans;
    }
    else { line };

    let base_color = if trimmed.starts_with("> ") { Color::from_rgb(0.65, 0.65, 0.7) } else { text_color };
    build_inline_display_spans(content, line_fs, base_font, base_color)
}

/// Build inline spans with formatting (bold/italic/code) applied, markers hidden.
#[allow(dead_code)]
fn build_inline_display_spans<'a>(text: &str, fs: f32, base_font: Font, base_color: Color) -> Vec<iced::advanced::text::Span<'a, (), Font>> {
    let highlights = highlight_inline(text, 0);
    let mut spans = Vec::new();
    for (range, hl) in &highlights {
        let chunk = &text[range.clone()];
        if chunk.is_empty() { continue; }
        match hl {
            Highlight::Marker => {} // HIDDEN
            Highlight::Bold => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs)
                    .font(Font { weight: iced::font::Weight::Bold, ..base_font })
                    .color(base_color));
            }
            Highlight::Italic => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs)
                    .font(Font { style: iced::font::Style::Italic, ..base_font })
                    .color(base_color));
            }
            Highlight::BoldItalic => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs)
                    .font(Font { weight: iced::font::Weight::Bold, style: iced::font::Style::Italic, ..base_font })
                    .color(base_color));
            }
            Highlight::Code => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs * 0.9)
                    .font(Font::MONOSPACE)
                    .color(Color::from_rgb(0.6, 0.85, 0.6)));
            }
            Highlight::Link => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs)
                    .color(Color::from_rgb(0.45, 0.75, 0.5)).underline(true));
            }
            _ => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs)
                    .font(base_font).color(base_color));
            }
        }
    }
    if spans.is_empty() {
        spans.push(iced::advanced::text::Span::new(" ".to_string()).size(fs));
    }
    spans
}

/// Build spans for the active line (raw markdown, markers subtly colored).
#[allow(dead_code)]
fn build_raw_spans<'a>(line: &str, fs: f32) -> Vec<iced::advanced::text::Span<'a, (), Font>> {
    let highlights = highlight_inline(line, 0);
    let mut spans = Vec::new();
    for (range, hl) in &highlights {
        let chunk = &line[range.clone()];
        if chunk.is_empty() { continue; }
        let (color, font) = match hl {
            Highlight::Marker => (Color::from_rgb(0.35, 0.35, 0.38), Font::DEFAULT),
            Highlight::Bold => (Color::from_rgb(0.92, 0.92, 0.94), Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
            Highlight::Italic => (Color::from_rgb(0.82, 0.82, 0.88), Font { style: iced::font::Style::Italic, ..Font::DEFAULT }),
            Highlight::BoldItalic => (Color::from_rgb(0.92, 0.92, 0.94), Font { weight: iced::font::Weight::Bold, style: iced::font::Style::Italic, ..Font::DEFAULT }),
            Highlight::Code => (Color::from_rgb(0.6, 0.85, 0.6), Font::MONOSPACE),
            Highlight::Link => (Color::from_rgb(0.45, 0.75, 0.5), Font::DEFAULT),
            Highlight::Heading => (Color::from_rgb(0.55, 0.75, 0.95), Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }),
            _ => (Color::from_rgb(0.85, 0.85, 0.87), Font::DEFAULT),
        };
        spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(color).font(font));
    }
    if spans.is_empty() { spans.push(iced::advanced::text::Span::new(" ".to_string()).size(fs)); }
    spans
}

/// Build spans for formatted display (markers hidden, headings large, bold/italic applied).
#[allow(dead_code)]
fn build_formatted_spans<'a>(line: &str, base_fs: f32) -> (Vec<iced::advanced::text::Span<'a, (), Font>>, Vec<DisplayChar>) {
    let trimmed = line.trim_start();
    let mut spans = Vec::new();
    let display_map = Vec::new();

    let (content, fs, base_font, base_color) = if trimmed.starts_with("# ") {
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 2..], base_fs * 1.8, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, Color::from_rgb(0.55, 0.75, 0.95))
    } else if trimmed.starts_with("## ") {
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 3..], base_fs * 1.5, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, Color::from_rgb(0.55, 0.75, 0.95))
    } else if trimmed.starts_with("### ") {
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 4..], base_fs * 1.3, Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }, Color::from_rgb(0.55, 0.75, 0.95))
    } else if trimmed.starts_with("> ") {
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 2..], base_fs, Font { style: iced::font::Style::Italic, ..Font::DEFAULT }, Color::from_rgb(0.65, 0.65, 0.7))
    } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        spans.push(iced::advanced::text::Span::new("☑ ".to_string()).size(base_fs).color(Color::from_rgb(0.4, 0.8, 0.5)));
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 6..], base_fs, Font::DEFAULT, Color::from_rgb(0.85, 0.85, 0.87))
    } else if trimmed.starts_with("- [ ] ") {
        spans.push(iced::advanced::text::Span::new("☐ ".to_string()).size(base_fs).color(Color::from_rgb(0.5, 0.5, 0.55)));
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 6..], base_fs, Font::DEFAULT, Color::from_rgb(0.85, 0.85, 0.87))
    } else if trimmed.starts_with("- ") {
        spans.push(iced::advanced::text::Span::new("• ".to_string()).size(base_fs).color(Color::from_rgb(0.55, 0.75, 0.95)));
        let prefix_len = line.len() - trimmed.len();
        (&line[prefix_len + 2..], base_fs, Font::DEFAULT, Color::from_rgb(0.85, 0.85, 0.87))
    } else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        spans.push(iced::advanced::text::Span::new("─────────────────────".to_string()).size(base_fs * 0.5).color(Color::from_rgba(1.0, 1.0, 1.0, 0.15)));
        return (spans, display_map);
    } else {
        (line, base_fs, Font::DEFAULT, Color::from_rgb(0.85, 0.85, 0.87))
    };

    let highlights = highlight_inline(content, 0);
    for (range, hl) in &highlights {
        let chunk = &content[range.clone()];
        if chunk.is_empty() { continue; }
        match hl {
            Highlight::Marker => {} // HIDDEN
            Highlight::Bold => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(Color::from_rgb(0.92, 0.92, 0.94)).font(Font { weight: iced::font::Weight::Bold, ..base_font }));
            }
            Highlight::Italic => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(Color::from_rgb(0.82, 0.82, 0.88)).font(Font { style: iced::font::Style::Italic, ..base_font }));
            }
            Highlight::BoldItalic => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(Color::from_rgb(0.92, 0.92, 0.94)).font(Font { weight: iced::font::Weight::Bold, style: iced::font::Style::Italic, ..base_font }));
            }
            Highlight::Code => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs * 0.9).color(Color::from_rgb(0.6, 0.85, 0.6)).font(Font::MONOSPACE));
            }
            Highlight::Link => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(Color::from_rgb(0.45, 0.75, 0.5)).underline(true));
            }
            _ => {
                spans.push(iced::advanced::text::Span::new(chunk.to_string()).size(fs).color(base_color).font(base_font));
            }
        }
    }

    if spans.is_empty() {
        spans.push(iced::advanced::text::Span::new(" ".to_string()).size(fs));
    }

    (spans, display_map)
}

/// Get the rendered height for a line (used by hit testing)
/// Get actual line height, accounting for custom image sizes
pub fn actual_line_height(line: &str, fs: f32, image_sizes: &HashMap<String, (f32, f32)>) -> f32 {
    let trimmed = line.trim_start();
    if trimmed.starts_with("![") && trimmed.contains("](img:") && trimmed.ends_with(')') {
        if let (Some(a), Some(b)) = (trimmed.find("]("), trimmed.rfind(')')) {
            let img_id = &trimmed[a + 2..b];
            if let Some(&(_, h)) = image_sizes.get(img_id) {
                return h + 8.0;
            }
        }
        return fs * 1.3;
    }
    line_height(line, fs)
}

pub fn line_height(line: &str, fs: f32) -> f32 {
    let trimmed = line.trim_start();
    if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains('-')
        && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ') {
        return 1.0;
    }
    let size = if trimmed.starts_with("# ") { fs * 1.8 }
    else if trimmed.starts_with("## ") { fs * 1.5 }
    else if trimmed.starts_with("### ") { fs * 1.3 }
    else if trimmed.starts_with("#### ") { fs * 1.15 }
    else { fs };
    size * 1.3
}

/// Get cursor (x, y) position within a wrapped paragraph using hit_test.
/// Returns (x_offset, y_offset) relative to the paragraph's top-left.
fn wrapped_cursor_pos(text: &str, cursor_col: usize, fs: f32, font: Font, avail_w: f32) -> (f32, f32) {
    let para = Paragraph::with_text(iced::advanced::Text {
        content: text,
        bounds: Size::new(avail_w, f32::MAX),
        size: Pixels(fs),
        line_height: iced::advanced::text::LineHeight::Relative(1.3),
        font,
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: iced::advanced::text::Shaping::Advanced,
        wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
    });

    let text_before: String = text.chars().take(cursor_col).collect();
    let para_before = Paragraph::with_text(iced::advanced::Text {
        content: &text_before,
        bounds: Size::new(avail_w, f32::MAX),
        size: Pixels(fs),
        line_height: iced::advanced::text::LineHeight::Relative(1.3),
        font,
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: iced::advanced::text::Shaping::Advanced,
        wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
    });

    let line_h = fs * 1.3;

    let total_lines = wrapped_visual_lines(text, fs, font, avail_w);
    let mut cursor_visual_line = 0;
    let mut vl_start_char = 0;
    for vl in 0..total_lines {
        let vl_start = if vl == 0 { 0 } else {
            para.hit_test(Point::new(0.0, vl as f32 * line_h + 1.0))
                .map(|h| h.cursor()).unwrap_or(0)
        };
        if cursor_col >= vl_start {
            cursor_visual_line = vl;
            vl_start_char = vl_start;
        } else {
            break;
        }
    }

    let cursor_y = cursor_visual_line as f32 * line_h;
    let chars_on_line = cursor_col.saturating_sub(vl_start_char);
    let line_text: String = text.chars().skip(vl_start_char).take(chars_on_line).collect();
    let cursor_x = measure_text_width(&line_text, fs, font);

    (cursor_x, cursor_y)
}

/// Check how many visual lines a paragraph wraps into.
fn wrapped_visual_lines(text: &str, fs: f32, font: Font, avail_w: f32) -> usize {
    if text.is_empty() || avail_w <= 0.0 { return 1; }
    let para = Paragraph::with_text(iced::advanced::Text {
        content: text,
        bounds: Size::new(avail_w, f32::MAX),
        size: Pixels(fs),
        line_height: iced::advanced::text::LineHeight::Relative(1.3),
        font,
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: iced::advanced::text::Shaping::Advanced,
        wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
    });
    let line_h = fs * 1.3;
    ((para.min_height() / line_h).ceil() as usize).max(1)
}

/// Line height accounting for word wrapping at a given width.
/// Measures DISPLAY text (markers stripped) with the correct font for headings.
pub fn wrapped_line_height(line: &str, fs: f32, avail_width: f32, image_sizes: &HashMap<String, (f32, f32)>) -> f32 {
    let base_h = actual_line_height(line, fs, image_sizes);
    if avail_width <= 0.0 || line.is_empty() { return base_h; }
    let trimmed = line.trim_start();
    if trimmed.starts_with("![") || (trimmed.starts_with('|') && trimmed.ends_with('|'))
        || trimmed.starts_with("```") || trimmed.starts_with("%%pass")
        || trimmed.starts_with("[file:") {
        return base_h;
    }
    let line_fs = if trimmed.starts_with("# ") { fs * 1.8 }
        else if trimmed.starts_with("## ") { fs * 1.5 }
        else if trimmed.starts_with("### ") { fs * 1.3 }
        else if trimmed.starts_with("#### ") { fs * 1.15 }
        else { fs };
    // Use display text (markers stripped) for height — matches what non-cursor lines render.
    let (segments, leading_ws) = build_display_segments(line);
    let display_text: String = format!("{}{}", leading_ws, segments.iter().map(|s| s.text.as_str()).collect::<String>());
    let display_font = if segments.iter().any(|s| s.font.weight == iced::font::Weight::Bold) {
        Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT }
    } else { Font::DEFAULT };
    // Quick check: if text is short enough, skip expensive Paragraph layout
    let estimated_w = display_text.len() as f32 * line_fs * 0.65;
    if estimated_w <= avail_width { return base_h; }
    // Use Paragraph to get accurate wrapped height (accounts for word boundaries)
    let para = Paragraph::with_text(iced::advanced::Text {
        content: &display_text,
        bounds: Size::new(avail_width, f32::MAX),
        size: Pixels(line_fs),
        line_height: iced::advanced::text::LineHeight::Relative(1.3),
        font: display_font,
        horizontal_alignment: iced::alignment::Horizontal::Left,
        vertical_alignment: iced::alignment::Vertical::Top,
        shaping: iced::advanced::text::Shaping::Advanced,
        wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
    });
    let para_h = para.min_height();
    if para_h > base_h { para_h } else { base_h }
}

/// Simple marker stripping for display
#[allow(dead_code)]
fn strip_markers_simple(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with("# ") { return trimmed[2..].to_string(); }
    if trimmed.starts_with("## ") { return trimmed[3..].to_string(); }
    if trimmed.starts_with("### ") { return trimmed[4..].to_string(); }
    if trimmed.starts_with("#### ") { return trimmed[5..].to_string(); }
    if trimmed.starts_with("> ") { let pl = line.len() - trimmed.len(); return line[pl+2..].to_string(); }
    if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") { let pl = line.len() - trimmed.len(); return format!("☑ {}", &line[pl+6..]); }
    if trimmed.starts_with("- [ ] ") { let pl = line.len() - trimmed.len(); return format!("☐ {}", &line[pl+6..]); }
    if trimmed.starts_with("- ") { let pl = line.len() - trimmed.len(); return format!("• {}", &line[pl+2..]); }
    if trimmed == "---" || trimmed == "***" || trimmed == "___" { return "─────────".to_string(); }
    let highlights = highlight_inline(line, 0);
    let mut result = String::new();
    for (range, hl) in &highlights {
        match hl {
            Highlight::Marker => {}
            _ => result.push_str(&line[range.clone()]),
        }
    }
    if result.is_empty() && !line.is_empty() { return line.to_string(); }
    result
}


/// Apply color ranges to text segments by splitting and recoloring overlapping segments.
fn apply_color_ranges_to_segments(segs: &mut Vec<TextSegment>, colors: &[&ColorRange]) {
    if colors.is_empty() { return; }

    let mut new_segs = Vec::new();
    let mut char_offset = 0;
    for seg in segs.iter() {
        let seg_start = char_offset;
        let seg_end = char_offset + seg.text.chars().count();
        let mut remaining = seg.text.as_str();
        let mut pos = seg_start;

        while !remaining.is_empty() {
            let mut earliest_start = seg_end;
            let mut matched_color: Option<&&ColorRange> = None;
            for cr in colors.iter() {
                if cr.start_col < earliest_start && cr.end_col > pos && cr.start_col < seg_end {
                    earliest_start = cr.start_col.max(pos);
                    matched_color = Some(cr);
                }
            }

            if let Some(cr) = matched_color {
                let cr_start = cr.start_col.max(pos);
                let cr_end = cr.end_col.min(seg_end);

                if cr_start > pos {
                    let n = cr_start - pos;
                    let (before, rest) = split_str_at_char(remaining, n);
                    new_segs.push(TextSegment { text: before.to_string(), font: seg.font, color: seg.color, size_mult: seg.size_mult, link_url: seg.link_url.clone() });
                    remaining = rest;
                    #[allow(unused_assignments)]
                    { pos = cr_start; }
                }

                let n = cr_end - cr_start;
                let (colored, rest) = split_str_at_char(remaining, n);
                let tag_color = parse_color_code(&cr.color, seg.color);
                new_segs.push(TextSegment { text: colored.to_string(), font: seg.font, color: tag_color, size_mult: seg.size_mult, link_url: seg.link_url.clone() });
                remaining = rest;
                pos = cr_end;
            } else {
                new_segs.push(TextSegment { text: remaining.to_string(), font: seg.font, color: seg.color, size_mult: seg.size_mult, link_url: seg.link_url.clone() });
                break;
            }
        }
        char_offset = seg_end;
    }
    *segs = new_segs;
}

fn split_str_at_char(s: &str, n: usize) -> (&str, &str) {
    let byte_pos = s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
    (&s[..byte_pos], &s[byte_pos..])
}

/// Returns (raw_prefix_stripped, display_prefix_added) for the line type.
/// `raw_prefix_stripped`: chars after leading whitespace that are removed from display segments.
/// `display_prefix_added`: chars added to display segments as a replacement prefix.
/// The caller uses: display_col = raw_col - leading_ws_chars - raw_prefix_stripped + display_prefix_added
fn raw_to_display_prefix_offset(trimmed: &str) -> (usize, usize) {
    if trimmed.starts_with("#### ") { (5, 0) }
    else if trimmed.starts_with("### ") { (4, 0) }
    else if trimmed.starts_with("## ") { (3, 0) }
    else if trimmed.starts_with("# ") { (2, 0) }
    else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") || trimmed.starts_with("- [ ] ") {
        (0, 0) // checkbox: "- [ ] " kept as-is in segments
    }
    else if trimmed.starts_with("- ") { (2, 3) } // bullet: "- " (2 stripped) → "•  " (3 added)
    else { (0, 0) } // numbered lists, plain text: no transformation
}

/// Strip color tags from a line, returning clean text and color ranges.
fn strip_color_tags(line: &str, line_idx: usize) -> (String, Vec<ColorRange>) {
    let mut clean = String::new();
    let mut colors = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if line.len() - i >= 4 && &line.as_bytes()[i..i+3] == b"{c:" {
            if let Some(tag_end) = line[i+3..].find('}') {
                let color_code = &line[i+3..i+3+tag_end];
                let content_start = i + 3 + tag_end + 1;
                if let Some(close_pos) = line[content_start..].find("{/c}") {
                    let content = &line[content_start..content_start+close_pos];
                    let start_col = clean.chars().count();
                    clean.push_str(content);
                    let end_col = clean.chars().count();
                    colors.push(ColorRange {
                        line: line_idx,
                        start_col,
                        end_col,
                        color: color_code.to_string(),
                    });
                    i = content_start + close_pos + 4; // skip {/c}
                    continue;
                }
            }
        }
        if let Some(c) = line[i..].chars().next() {
            clean.push(c);
            i += c.len_utf8();
        } else {
            i += 1;
        }
    }
    (clean, colors)
}

/// Parse a color code (preset letter or "h,s,v" HSV values) into a Color.
fn parse_color_code(code: &str, fallback: Color) -> Color {
    match code {
        "r" => Color::from_rgb(0.9, 0.35, 0.35),
        "o" => Color::from_rgb(0.95, 0.65, 0.25),
        "y" => Color::from_rgb(0.95, 0.85, 0.3),
        "g" => Color::from_rgb(0.35, 0.8, 0.45),
        "b" => Color::from_rgb(0.35, 0.6, 0.95),
        "p" => Color::from_rgb(0.7, 0.45, 0.9),
        "w" => Color::from_rgb(0.85, 0.85, 0.87),
        _ => {
            let parts: Vec<&str> = code.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(h), Ok(s), Ok(v)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>(), parts[2].parse::<f32>()) {
                    return hsv_to_color(h, s / 100.0, v / 100.0);
                }
            }
            fallback
        }
    }
}

/// Convert HSV to RGB Color. Matches the color picker's HSV color space.
fn hsv_to_color(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

/// Convert HSL to RGB Color.
#[allow(dead_code)]
fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    if s == 0.0 { return Color::from_rgb(l, l, l); }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let h = h / 360.0;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    Color::from_rgb(r, g, b)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

/// Measure exact pixel width of text using Paragraph.
/// Parse table cells from a raw `| cell1 | cell2 |` line.
/// Returns Vec of (content_start_col, content_end_col, trimmed_text).
/// content_start/end are char indices in the raw line pointing to the cell content (without padding).
fn is_separator_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed.contains('-') && trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

pub fn parse_table_cells(line: &str) -> Vec<(usize, usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') { return Vec::new(); }
    let offset = line.chars().count() - trimmed.chars().count();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut cells = Vec::new();
    let mut i = 1; // skip leading |
    while i < chars.len() {
        let start = i;
        while i < chars.len() && chars[i] != '|' { i += 1; }
        if i <= chars.len() {
            let raw: String = chars[start..i].iter().collect();
            let cell_text = raw.trim().to_string();
            let content_start = start + raw.chars().take_while(|c| *c == ' ').count();
            let content_end = if cell_text.is_empty() { content_start } else { content_start + cell_text.chars().count() };
            cells.push((offset + content_start, offset + content_end, cell_text));
        }
        i += 1; // skip |
    }
    cells
}

/// Find which cell the cursor is in for a table line.
/// Returns (cell_index, col_within_cell_content).
pub fn cursor_to_cell(line: &str, raw_col: usize) -> (usize, usize) {
    let trimmed = line.trim_start();
    let offset = line.chars().count() - trimmed.chars().count();
    let chars: Vec<char> = trimmed.chars().collect();
    let adj_col = raw_col.saturating_sub(offset);

    let mut cell_idx = 0;
    let mut i = 1; // skip leading |
    while i < chars.len() {
        let cell_start = i;
        while i < chars.len() && chars[i] != '|' { i += 1; }
        if adj_col <= i {
            let raw_cell: String = chars[cell_start..i].iter().collect();
            let leading_spaces = raw_cell.chars().take_while(|c| *c == ' ').count();
            let col_in_cell = adj_col.saturating_sub(cell_start).saturating_sub(leading_spaces);
            let cell_text_len = raw_cell.trim().chars().count();
            return (cell_idx, col_in_cell.min(cell_text_len));
        }
        cell_idx += 1;
        i += 1; // skip |
    }
    if cell_idx > 0 {
        let trimmed2 = line.trim_start();
        let cells: Vec<&str> = if trimmed2.len() > 2 { trimmed2[1..trimmed2.len()-1].split('|').collect() } else { Vec::new() };
        let last = cells.last().map(|c| c.trim().chars().count()).unwrap_or(0);
        (cell_idx.saturating_sub(1), last)
    } else {
        (0, 0)
    }
}

/// Convert cell index + col within cell back to raw col in the line.
pub fn cell_to_raw_col(line: &str, cell_idx: usize, col_in_cell: usize) -> usize {
    let trimmed = line.trim_start();
    let offset = line.chars().count() - trimmed.chars().count();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut ci = 0;
    let mut i = 1;
    while i < chars.len() {
        let cell_start = i;
        while i < chars.len() && chars[i] != '|' { i += 1; }
        if ci == cell_idx {
            let raw_cell: String = chars[cell_start..i].iter().collect();
            let leading_spaces = raw_cell.chars().take_while(|c| *c == ' ').count();
            return offset + cell_start + leading_spaces + col_in_cell;
        }
        ci += 1;
        i += 1;
    }
    raw_col_fallback(line)
}

fn raw_col_fallback(line: &str) -> usize { line.chars().count() }

pub fn measure_text_width(text: &str, size: f32, font: Font) -> f32 {
    if text.is_empty() { return 0.0; }
    let para = Paragraph::with_text(iced::advanced::Text {
        content: text,
        bounds: Size::new(f32::MAX, f32::MAX),
        size: Pixels(size),
        line_height: text::LineHeight::Relative(1.3),
        font,
        horizontal_alignment: alignment::Horizontal::Left,
        vertical_alignment: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    });
    para.min_width()
}

fn char_len(s: &str) -> usize { s.chars().count() }

pub fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}


pub fn md_editor<'a, Message: 'a>(
    state: &'a MdEditorState,
    on_edit: impl Fn(MdAction) -> Message + 'a,
) -> MdEditorWidget<'a, Message> {
    MdEditorWidget::new(state, on_edit)
}

impl<'a, Message: 'a> From<MdEditorWidget<'a, Message>> for Element<'a, Message> {
    fn from(w: MdEditorWidget<'a, Message>) -> Self { Element::new(w) }
}
