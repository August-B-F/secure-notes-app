use std::collections::HashMap;
use iced::advanced::layout;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell, Renderer as _};
use iced::{event, keyboard, mouse};
use iced::{Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Vector};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::Message;
use crate::ui::md_widget::{self, MdEditorState};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardSide { Top, Right, Bottom, Left }
impl Default for CardSide { fn default() -> Self { Self::Top } }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanvasData {
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNode {
    pub id: String,
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub label: String,
    pub color: String,
    #[serde(default)]
    pub bg_color: Option<String>,
    /// User-set minimum height — card won't auto-shrink below this
    #[serde(default)]
    pub user_min_h: f32,
}

fn gen_edge_id() -> String { Uuid::new_v4().to_string()[..8].to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEdge {
    #[serde(default = "gen_edge_id")]
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub from_side: CardSide,
    #[serde(default)]
    pub to_side: CardSide,
}

impl CanvasData {
    pub fn from_json(s: &str) -> Self { serde_json::from_str(s).unwrap_or_default() }
    pub fn to_json(&self) -> String { serde_json::to_string(self).unwrap_or_default() }
}

const GRID: f32 = 20.0;
fn snap(v: f32) -> f32 { (v / GRID).round() * GRID }

impl CanvasNode {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            x: snap(x), y: snap(y), w: 160.0, h: 48.0,
            label: String::new(),
            color: String::from("#2D8B4E"),
            bg_color: None,
            user_min_h: 0.0,
        }
    }
    pub fn min_size_for_label(&self) -> (f32, f32) {
        // Must match md_editor rendering: font 14px, line_height 1.3x, padding 8/12
        let font_size = 14.0;
        let char_w = font_size * 0.58; // ~8.1px avg char width at 14px
        let line_h = font_size * 1.3;  // 18.2px — matches md_editor Relative(1.3)
        let pad_h = 12.0 * 2.0; // left + right padding (matches pad_lr in view)
        let pad_v = 8.0 * 2.0;  // top + bottom padding (matches pad_tb in view)
        let min_w = 80.0f32;
        let avail_chars = ((self.w - pad_h) / char_w).max(1.0) as usize;
        let mut line_count = 0usize;
        if self.label.is_empty() {
            line_count = 1;
        } else {
            // Split by \n to count ALL lines including trailing empty ones
            // (.lines() ignores trailing newlines which breaks Enter sizing)
            for line in self.label.split('\n') {
                let chars = line.chars().count();
                let wrapped = if chars == 0 { 1 } else {
                    ((chars as f32) / avail_chars as f32).ceil().max(1.0) as usize
                };
                line_count += wrapped;
            }
        }
        let content_h = line_count as f32 * line_h + pad_v;
        let h = content_h.max(48.0).max(self.user_min_h);
        (snap(min_w), h)
    }
    pub fn center(&self) -> Point { Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0) }
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
    pub fn parse_color(&self) -> Color { parse_hex(&self.color, ACCENT) }
    pub fn parse_bg_color(&self) -> Color {
        self.bg_color.as_ref().map_or(BG_DEFAULT, |h| parse_hex(h, BG_DEFAULT))
    }
    pub fn edge_dots(&self) -> [Point; 4] {
        [
            Point::new(self.x + self.w / 2.0, self.y),
            Point::new(self.x + self.w, self.y + self.h / 2.0),
            Point::new(self.x + self.w / 2.0, self.y + self.h),
            Point::new(self.x, self.y + self.h / 2.0),
        ]
    }
    pub fn side_point(&self, side: CardSide) -> Point { self.edge_dots()[side as usize] }
    pub fn dot_side(idx: usize) -> CardSide {
        [CardSide::Top, CardSide::Right, CardSide::Bottom, CardSide::Left][idx]
    }
    pub fn corner_rects(&self) -> [Rectangle; 4] {
        let s = 8.0;
        [
            Rectangle::new(Point::new(self.x - s/2.0, self.y - s/2.0), Size::new(s, s)),
            Rectangle::new(Point::new(self.x + self.w - s/2.0, self.y - s/2.0), Size::new(s, s)),
            Rectangle::new(Point::new(self.x + self.w - s/2.0, self.y + self.h - s/2.0), Size::new(s, s)),
            Rectangle::new(Point::new(self.x - s/2.0, self.y + self.h - s/2.0), Size::new(s, s)),
        ]
    }
}


fn parse_hex(hex: &str, fallback: Color) -> Color {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
        Color::from_rgb8(r, g, b)
    } else { fallback }
}

fn dist(a: Point, b: Point) -> f32 { ((a.x-b.x).powi(2) + (a.y-b.y).powi(2)).sqrt() }
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

fn control_offset(side: CardSide, d: f32) -> Vector {
    match side {
        CardSide::Top => Vector::new(0.0, -d),
        CardSide::Bottom => Vector::new(0.0, d),
        CardSide::Left => Vector::new(-d, 0.0),
        CardSide::Right => Vector::new(d, 0.0),
    }
}

fn bezier_dist(p: Point, p0: Point, p1: Point, p2: Point, p3: Point) -> f32 {
    let mut min_d = f32::MAX;
    for i in 0..=30 {
        let t = i as f32 / 30.0;
        let u = 1.0 - t;
        let bx = u*u*u*p0.x + 3.0*u*u*t*p1.x + 3.0*u*t*t*p2.x + t*t*t*p3.x;
        let by = u*u*u*p0.y + 3.0*u*u*t*p1.y + 3.0*u*t*t*p2.y + t*t*t*p3.y;
        let d = dist(p, Point::new(bx, by));
        if d < min_d { min_d = d; }
    }
    min_d
}

fn to_world(screen: Point, pan: (f32, f32), zoom: f32) -> Point {
    Point::new((screen.x - pan.0) / zoom, (screen.y - pan.1) / zoom)
}
fn to_screen(world: Point, pan: (f32, f32), zoom: f32) -> Point {
    Point::new(world.x * zoom + pan.0, world.y * zoom + pan.1)
}


#[derive(Clone, Debug)]
pub struct CanvasCtxMenu {
    pub pos: (f32, f32),
    pub target: CanvasCtxTarget,
}

#[derive(Clone, Debug)]
pub enum CanvasCtxTarget {
    Node(String),
    Edge(String),
    Empty(f32, f32),
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Corner { TL, TR, BR, BL }

enum CanvasAction {
    None,
    Dragging { node_id: String, offset: Vector, group: Vec<(String, f32, f32)> },
    Panning { last: Point },
    Selecting { start: Point, current: Point },
    Connecting { from_id: String, from_side: CardSide },
    Resizing { node_id: String, corner: Corner, start_rect: (f32, f32, f32, f32) },
}

const ACCENT: Color = Color::from_rgb(0.18, 0.55, 0.31);
const BG_DEFAULT: Color = Color::from_rgb(0.14, 0.14, 0.18);
const BG_CANVAS: Color = Color::from_rgb(0.09, 0.09, 0.10);
const DOT_R: f32 = 5.0;
const HANDLE_H: f32 = 0.0; // no accent bar — double-click to edit
const SIMPLIFY_ZOOM: f32 = 0.55; // below this zoom, show simplified cards (no editor)


pub struct CanvasEditor {
    pub data: CanvasData,
    pub card_editors: HashMap<String, MdEditorState>,
    pub selected: Vec<String>,
    pub selected_edges: Vec<String>,
    pub focused_card: Option<String>,
    pub pan: (f32, f32),
    pub zoom: f32,
    pub ctx_menu_info: Option<CanvasCtxMenu>,
    pub hover_anim: HashMap<String, f32>,
    pub select_anim: HashMap<String, f32>,
    pub edge_hover_anim: HashMap<String, f32>,
    pub last_hovered: Option<String>,
    pub viewport_size: (f32, f32),
    // Undo/redo for canvas structure (node positions, edges, additions, deletions)
    undo_stack: std::collections::VecDeque<String>, // serialized CanvasData snapshots
    redo_stack: std::collections::VecDeque<String>,
}

impl CanvasEditor {
    pub fn new() -> Self {
        Self {
            data: CanvasData::default(),
            card_editors: HashMap::new(),
            selected: Vec::new(),
            selected_edges: Vec::new(),
            focused_card: None,
            pan: (0.0, 0.0),
            zoom: 1.0,
            ctx_menu_info: None,
            hover_anim: HashMap::new(),
            select_anim: HashMap::new(),
            edge_hover_anim: HashMap::new(),
            last_hovered: None,
            viewport_size: (800.0, 600.0),
            undo_stack: std::collections::VecDeque::new(),
            redo_stack: std::collections::VecDeque::new(),
        }
    }

    /// Save current canvas state for undo.
    pub fn push_undo(&mut self) {
        self.sync_labels();
        let snapshot = self.data.to_json();
        self.undo_stack.push_back(snapshot);
        if self.undo_stack.len() > 100 { self.undo_stack.pop_front(); }
        self.redo_stack.clear();
    }

    /// Undo last canvas change.
    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop_back() {
            self.sync_labels();
            self.redo_stack.push_back(self.data.to_json());
            self.data = CanvasData::from_json(&snapshot);
            self.sync_editors();
        }
    }

    /// Redo last undone canvas change.
    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop_back() {
            self.sync_labels();
            self.undo_stack.push_back(self.data.to_json());
            self.data = CanvasData::from_json(&snapshot);
            self.sync_editors();
        }
    }

    pub fn load(&mut self, json: &str) {
        self.data = CanvasData::from_json(json);
        for node in &mut self.data.nodes {
            let (min_w, min_h) = node.min_size_for_label();
            if node.w < min_w { node.w = min_w; }
            if node.h < min_h { node.h = min_h; }
        }
        self.selected.clear();
        self.selected_edges.clear();
        self.hover_anim.clear();
        self.select_anim.clear();
        self.edge_hover_anim.clear();
        self.focused_card = None;
        self.pan = (0.0, 0.0);
        self.zoom = 1.0;
        self.sync_editors();
        // Center on cards (viewport_size may not be set yet, recenter() uses it)
        self.recenter();
    }

    /// Ensure every node has an MdEditorState and remove stale ones.
    pub fn sync_editors(&mut self) {
        let ids: Vec<String> = self.data.nodes.iter().map(|n| n.id.clone()).collect();
        for node in &self.data.nodes {
            // replace editor if label changed (e.g. after undo), or add if new
            let needs_update = self.card_editors.get(&node.id)
                .map_or(true, |ed| ed.to_body() != node.label);
            if needs_update {
                self.card_editors.insert(node.id.clone(), MdEditorState::from_body(&node.label));
            }
        }
        self.card_editors.retain(|k, _| ids.contains(k));
    }

    /// Sync all editor contents back to node labels (call before save).
    pub fn sync_labels(&mut self) {
        for node in &mut self.data.nodes {
            if let Some(editor) = self.card_editors.get(&node.id) {
                let body = editor.to_body();
                node.label = body;
            }
        }
    }

    pub fn recenter(&mut self) {
        if !self.data.nodes.is_empty() {
            let mut cx = 0.0f32;
            let mut cy = 0.0f32;
            for n in &self.data.nodes {
                cx += n.x + n.w / 2.0;
                cy += n.y + n.h / 2.0;
            }
            cx /= self.data.nodes.len() as f32;
            cy /= self.data.nodes.len() as f32;
            let (vw, vh) = self.viewport_size;
            self.pan = (vw / 2.0 - cx * self.zoom, vh / 2.0 - cy * self.zoom);
        } else {
            let (vw, vh) = self.viewport_size;
            self.pan = (vw / 2.0, vh / 2.0);
        }
    }

    pub fn fit_view(&mut self) {
        if !self.data.nodes.is_empty() {
            let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for n in &self.data.nodes {
                mnx = mnx.min(n.x); mny = mny.min(n.y);
                mxx = mxx.max(n.x + n.w); mxy = mxy.max(n.y + n.h);
            }
            let pad = 60.0;
            let cw = mxx - mnx + pad * 2.0;
            let ch = mxy - mny + pad * 2.0;
            let (vw, vh) = self.viewport_size;
            let zx = vw / cw;
            let zy = vh / ch;
            self.zoom = zx.min(zy).clamp(0.3, 2.0);
            let cx = (mnx + mxx) / 2.0;
            let cy = (mny + mxy) / 2.0;
            self.pan = (vw / 2.0 - cx * self.zoom, vh / 2.0 - cy * self.zoom);
        } else {
            let (vw, vh) = self.viewport_size;
            self.pan = (vw / 2.0, vh / 2.0);
            self.zoom = 1.0;
        }
    }

    pub fn viewport_center(&self) -> (f32, f32) {
        let (vw, vh) = self.viewport_size;
        let wp = to_world(Point::new(vw / 2.0, vh / 2.0), self.pan, self.zoom);
        (wp.x, wp.y)
    }

    pub fn tick_animations(&mut self) {
        let speed = 0.25;
        let mut dirty = false;
        for node in &self.data.nodes {
            let target = if self.last_hovered.as_ref() == Some(&node.id) { 1.0 } else { 0.0 };
            let current = self.hover_anim.get(&node.id).copied().unwrap_or(0.0);
            let new_val = current + (target - current) * speed;
            let new_val = if (new_val - target).abs() < 0.01 { target } else { new_val };
            if (new_val - current).abs() > 0.001 { self.hover_anim.insert(node.id.clone(), new_val); dirty = true; }
        }
        for node in &self.data.nodes {
            let target = if self.selected.contains(&node.id) { 1.0 } else { 0.0 };
            let current = self.select_anim.get(&node.id).copied().unwrap_or(0.0);
            let new_val = current + (target - current) * speed;
            let new_val = if (new_val - target).abs() < 0.01 { target } else { new_val };
            if (new_val - current).abs() > 0.001 { self.select_anim.insert(node.id.clone(), new_val); dirty = true; }
        }
        for edge in &self.data.edges {
            let target = if self.selected_edges.contains(&edge.id) { 1.0 } else { 0.0 };
            let current = self.edge_hover_anim.get(&edge.id).copied().unwrap_or(0.0);
            let new_val = current + (target - current) * speed;
            let new_val = if (new_val - target).abs() < 0.01 { target } else { new_val };
            if (new_val - current).abs() > 0.001 { self.edge_hover_anim.insert(edge.id.clone(), new_val); dirty = true; }
        }
        let _ = dirty; // Widget redraws every frame when animations active
    }

    /// Build the view: a custom widget with embedded md_editor children.
    pub fn view(&self) -> Element<'_, Message> {
        let z = self.zoom;
        let font_size = (14.0 * z).max(6.0);

        // Build one md_editor element per card — scale padding with zoom
        let pad_tb = (8.0 * z).max(2.0);
        let pad_lr = (12.0 * z).max(4.0);
        let cards: Vec<(String, Element<'_, Message>)> = self.data.nodes.iter()
            .filter_map(|node| {
                let editor_state = self.card_editors.get(&node.id)?;
                let nid = node.id.clone();
                let elem: Element<'_, Message> = md_widget::md_editor(
                    editor_state,
                    move |action| Message::CanvasCardEdit(nid.clone(), action),
                )
                .size(font_size)
                .padding(iced::Padding::new(pad_tb).left(pad_lr).right(pad_lr))
                .no_scrollbar()
                .into();
                Some((node.id.clone(), elem))
            })
            .collect();

        let widget = CanvasViewWidget {
            editor: self,
            cards,
        };
        Element::new(widget)
    }
}


struct CanvasViewWidget<'a> {
    editor: &'a CanvasEditor,
    cards: Vec<(String, Element<'a, Message>)>,
}

struct CanvasViewState {
    action: CanvasAction,
    cursor: Point,
    last_click: Option<(std::time::Instant, Point)>,
    ctrl_held: bool,
    shift_held: bool,
}

impl Default for CanvasViewState {
    fn default() -> Self {
        Self {
            action: CanvasAction::None,
            cursor: Point::ORIGIN,
            last_click: None,
            ctrl_held: false,
            shift_held: false,
        }
    }
}

impl<'a> Widget<Message, iced::Theme, iced::Renderer> for CanvasViewWidget<'a> {
    fn tag(&self) -> tree::Tag { tree::Tag::of::<CanvasViewState>() }
    fn state(&self) -> tree::State { tree::State::new(CanvasViewState::default()) }

    fn children(&self) -> Vec<Tree> {
        self.cards.iter().map(|(_, e)| Tree::new(e.as_widget())).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.cards.iter().map(|(_, e)| e.as_widget()).collect::<Vec<_>>());
    }

    fn size(&self) -> Size<Length> { Size::new(Length::Fill, Length::Fill) }

    fn layout(&self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let full_size = limits.max();
        let pan = self.editor.pan;
        let z = self.editor.zoom;
        let handle_screen = HANDLE_H * z;

        let simplified = z < SIMPLIFY_ZOOM;
        let mut children = vec![];
        for (i, (id, element)) in self.cards.iter().enumerate() {
            if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *id) {
                if simplified {
                    children.push(layout::Node::new(Size::ZERO));
                    continue;
                }
                let sw = node.w * z;
                let sh = node.h * z;
                let editor_h = (sh - handle_screen).max(1.0);
                let child_limits = layout::Limits::new(
                    Size::new(sw, editor_h),
                    Size::new(sw, editor_h),
                );
                let mut child_node = element.as_widget().layout(&mut tree.children[i], renderer, &child_limits);
                let sx = node.x * z + pan.0;
                let sy = node.y * z + pan.1 + handle_screen;
                child_node = child_node.move_to(Point::new(sx, sy));
                children.push(child_node);
            } else {
                children.push(layout::Node::new(Size::ZERO));
            }
        }

        layout::Node::with_children(full_size, children)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let pan = self.editor.pan;
        let z = self.editor.zoom;
        let handle_screen = HANDLE_H * z;
        let state = tree.state.downcast_ref::<CanvasViewState>();

        renderer.fill_quad(
            Quad { bounds, border: Border::default(), shadow: iced::Shadow::default() },
            Background::Color(BG_CANVAS),
        );

        let draw_pan = (pan.0 + bounds.x, pan.1 + bounds.y);
        renderer.with_layer(bounds, |renderer| {

        let node_map: HashMap<&str, &CanvasNode> = self.editor.data.nodes.iter()
            .map(|n| (n.id.as_str(), n)).collect();

        for edge in &self.editor.data.edges {
            let anim_t = self.editor.edge_hover_anim.get(&edge.id).copied().unwrap_or(0.0);
            if anim_t < 0.01 { continue; }
            let from_node = node_map.get(edge.from.as_str()).copied();
            let to_node = node_map.get(edge.to.as_str()).copied();
            if let (Some(f), Some(t)) = (from_node, to_node) {
                let fp = to_screen(f.side_point(edge.from_side), draw_pan, z);
                let tp = to_screen(t.side_point(edge.to_side), draw_pan, z);
                let d = dist(fp, tp) * 0.4;
                let cp1 = Point::new(fp.x + control_offset(edge.from_side, d).x, fp.y + control_offset(edge.from_side, d).y);
                let cp2 = Point::new(tp.x + control_offset(edge.to_side, d).x, tp.y + control_offset(edge.to_side, d).y);
                let thickness = lerp(3.0, 4.0, anim_t);
                draw_bezier_line(renderer, fp, cp1, cp2, tp, thickness + 4.0,
                    Color::from_rgba(0.22, 0.22, 0.24, anim_t), &bounds);
            }
        }
        renderer.with_layer(bounds, |renderer| {
        for edge in &self.editor.data.edges {
            let anim_t = self.editor.edge_hover_anim.get(&edge.id).copied().unwrap_or(0.0);
            let from_node = node_map.get(edge.from.as_str()).copied();
            let to_node = node_map.get(edge.to.as_str()).copied();
            if let (Some(f), Some(t)) = (from_node, to_node) {
                let fp = to_screen(f.side_point(edge.from_side), draw_pan, z);
                let tp = to_screen(t.side_point(edge.to_side), draw_pan, z);
                let d = dist(fp, tp) * 0.4;
                let cp1 = Point::new(fp.x + control_offset(edge.from_side, d).x, fp.y + control_offset(edge.from_side, d).y);
                let cp2 = Point::new(tp.x + control_offset(edge.to_side, d).x, tp.y + control_offset(edge.to_side, d).y);
                let col = Color::from_rgb(0.4, 0.4, 0.42); // always gray
                let thickness = lerp(3.0, 4.0, anim_t);
                draw_bezier_line(renderer, fp, cp1, cp2, tp, thickness, col, &bounds);
            }
        }
        }); // end pass 2 layer

        if let CanvasAction::Connecting { ref from_id, from_side } = state.action {
            if let Some(f) = node_map.get(from_id.as_str()).copied() {
                let fp = to_screen(f.side_point(from_side), draw_pan, z);
                let cp = state.cursor;
                draw_straight_line(renderer, fp, cp, 2.0, Color::from_rgba(0.18, 0.62, 0.38, 0.6), &bounds);
                let dot_r = 4.0;
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(Point::new(cp.x - dot_r, cp.y - dot_r), Size::new(dot_r * 2.0, dot_r * 2.0)),
                        border: Border { radius: dot_r.into(), ..Default::default() },
                        shadow: iced::Shadow::default(),
                    },
                    Background::Color(Color::from_rgba(0.22, 0.65, 0.40, 0.7)),
                );
            }
        }

        if let CanvasAction::Selecting { start, current } = &state.action {
            let s = to_screen(*start, draw_pan, z);
            let c = to_screen(*current, draw_pan, z);
            let rx = s.x.min(c.x);
            let ry = s.y.min(c.y);
            let rw = (s.x - c.x).abs();
            let rh = (s.y - c.y).abs();
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(Point::new(rx, ry), Size::new(rw, rh)),
                    border: Border { radius: 3.0.into(), width: 1.0, color: Color::from_rgba(0.22, 0.65, 0.38, 0.3) },
                    shadow: iced::Shadow::default(),
                },
                Background::Color(Color::from_rgba(0.18, 0.55, 0.31, 0.06)),
            );
        }

        // Each card is drawn in its own layer so later cards properly
        // occlude earlier ones (both background AND text).
        let simplified = z < SIMPLIFY_ZOOM;
        let children_layouts: Vec<Layout<'_>> = layout.children().collect();
        for (i, (id, element)) in self.cards.iter().enumerate() {
            let Some(node) = node_map.get(id.as_str()).copied() else { continue };
            let child_layout = children_layouts[i];

            let ht = self.editor.hover_anim.get(id).copied().unwrap_or(0.0);
            let st = self.editor.select_anim.get(id).copied().unwrap_or(0.0);
            let is_focused = !simplified && self.editor.focused_card.as_ref() == Some(id);

            let sx = node.x * z + draw_pan.0;
            let sy = node.y * z + draw_pan.1;
            let (sw, sh) = if simplified {
                (node.w * z, node.h * z)
            } else {
                let cw = child_layout.bounds().width;
                let eh = child_layout.bounds().height;
                (cw, eh + handle_screen)
            };

            // Wrap entire card in a layer for proper z-ordering
            // Intersect with canvas bounds so cards don't render over sidebar etc.
            let card_clip = Rectangle::new(
                Point::new(sx - 12.0, sy - 12.0),
                Size::new(sw + 24.0, sh + 24.0),
            );
            let Some(clipped) = bounds.intersection(&card_clip) else { continue };
            renderer.with_layer(clipped, |renderer| {
            let r = (8.0 * z).clamp(2.0, 14.0);

            let shadow_off = lerp(1.0, 2.0, ht);
            let shadow_a = lerp(0.10, 0.20, ht);
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(Point::new(sx + shadow_off, sy + shadow_off * 1.5), Size::new(sw, sh)),
                    border: Border { radius: r.into(), ..Default::default() },
                    shadow: iced::Shadow::default(),
                },
                Background::Color(Color::from_rgba(0.0, 0.0, 0.0, shadow_a)),
            );

            if st > 0.01 {
                let gp = lerp(0.0, 4.0, st);
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(Point::new(sx - gp, sy - gp), Size::new(sw + gp * 2.0, sh + gp * 2.0)),
                        border: Border { radius: (r + gp).into(), ..Default::default() },
                        shadow: iced::Shadow::default(),
                    },
                    Background::Color(Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.25 * st)),
                );
            }

            let base = node.parse_bg_color();
            let bright = lerp(1.0, 1.08, ht).max(lerp(1.0, 1.15, st));
            let body_col = Color::from_rgb(
                (base.r * bright).min(1.0), (base.g * bright).min(1.0), (base.b * bright).min(1.0)
            );
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(Point::new(sx, sy), Size::new(sw, sh)),
                    border: Border { radius: r.into(), ..Default::default() },
                    shadow: iced::Shadow::default(),
                },
                Background::Color(body_col),
            );

            if st > 0.01 {
                let bcol = Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, st);
                let bw = lerp(0.0, 2.0, st);
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(Point::new(sx, sy), Size::new(sw, sh)),
                        border: Border { radius: r.into(), width: bw, color: bcol },
                        shadow: iced::Shadow::default(),
                    },
                    Background::Color(Color::TRANSPARENT),
                );
            }

            if is_focused {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(Point::new(sx, sy), Size::new(sw, sh)),
                        border: Border { radius: r.into(), width: 2.0, color: ACCENT },
                        shadow: iced::Shadow::default(),
                    },
                    Background::Color(Color::TRANSPARENT),
                );
            }

            if simplified {
                let fsz = (14.0 * z).max(4.0);
                if fsz > 4.5 && !node.label.is_empty() {
                    use iced::advanced::text::{self, Renderer as TR, Paragraph as _};
                    let pad_x = (12.0 * z).max(4.0);
                    let pad_y = (8.0 * z).max(2.0);
                    let text_col = Color::from_rgb(0.85, 0.85, 0.86);
                    let max_lines = ((sh - pad_y * 2.0) / (fsz * 1.3)).max(1.0) as usize;
                    let mut ly = sy + pad_y;
                    for (li, line) in node.label.split('\n').enumerate() {
                        if li >= max_lines { break; }
                        if ly + fsz > sy + sh { break; }
                        let display = line.trim_start_matches('#').trim_start_matches(' ')
                            .trim_start_matches("- ").trim_start_matches("**").trim_end_matches("**");
                        if !display.is_empty() {
                            TR::fill_text(renderer, iced::advanced::Text {
                                content: display.to_string(),
                                bounds: Size::new(sw - pad_x * 2.0, fsz * 1.3),
                                size: iced::Pixels(fsz),
                                line_height: text::LineHeight::Relative(1.3),
                                font: iced::Font::DEFAULT,
                                horizontal_alignment: iced::alignment::Horizontal::Left,
                                vertical_alignment: iced::alignment::Vertical::Top,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::None,
                            }, Point::new(sx + pad_x, ly), text_col, bounds);
                        }
                        ly += fsz * 1.3;
                    }
                }
            } else {
                let editor_h = child_layout.bounds().height;
                let clip_rect = Rectangle::new(Point::new(sx, sy + handle_screen), Size::new(sw, editor_h));
                if let Some(clip_rect) = bounds.intersection(&clip_rect) {
                renderer.with_layer(clip_rect, |renderer| {
                    element.as_widget().draw(
                        &tree.children[i], renderer, theme, style, child_layout, cursor, viewport,
                    );
                });
                } // end clip intersection
            }

            let dot_progress = ht.max(st).max(
                if matches!(state.action, CanvasAction::Connecting { .. }) { 1.0 } else { 0.0 }
            );
            if dot_progress > 0.05 {
                for dot_world in &node.edge_dots() {
                    let sd = to_screen(*dot_world, draw_pan, z);
                    let dr = lerp(1.0, DOT_R, dot_progress);
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(Point::new(sd.x - dr - 4.0, sd.y - dr - 4.0), Size::new((dr + 4.0) * 2.0, (dr + 4.0) * 2.0)),
                            border: Border { radius: (dr + 4.0).into(), ..Default::default() },
                            shadow: iced::Shadow::default(),
                        },
                        Background::Color(Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.2 * dot_progress)),
                    );
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(Point::new(sd.x - dr, sd.y - dr), Size::new(dr * 2.0, dr * 2.0)),
                            border: Border { radius: dr.into(), width: 1.5, color: Color::from_rgba(1.0, 1.0, 1.0, 0.5 * dot_progress) },
                            shadow: iced::Shadow::default(),
                        },
                        Background::Color(Color::from_rgba(0.22, 0.72, 0.42, dot_progress)),
                    );
                }
            }
            }); // end per-card with_layer
        }
        }); // end with_layer clip
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        let bounds = layout.bounds();
        let cursor_pos = cursor.position().unwrap_or(Point::ORIGIN);
        let in_bounds = bounds.contains(cursor_pos);
        let state = tree.state.downcast_mut::<CanvasViewState>();
        if in_bounds { state.cursor = cursor_pos; }

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) = &event {
            state.ctrl_held = mods.control();
            state.shift_held = mods.shift();
        }

        let (cur_w, cur_h) = self.editor.viewport_size;
        if (bounds.width - cur_w).abs() > 1.0 || (bounds.height - cur_h).abs() > 1.0 {
            shell.publish(Message::CanvasViewportSize(bounds.width, bounds.height));
        }

        let pan = self.editor.pan;
        let z = self.editor.zoom;
        let handle_screen = HANDLE_H * z;

        if in_bounds {
            if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = &event {
                let (dx, dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => (*x, *y),
                    mouse::ScrollDelta::Pixels { x, y } => (x / 50.0, y / 50.0),
                };
                if state.ctrl_held {
                    shell.publish(Message::CanvasZoom(dy, cursor_pos.x - bounds.x, cursor_pos.y - bounds.y));
                } else if state.shift_held {
                    shell.publish(Message::CanvasPan(dy * 40.0, dx * 40.0));
                } else {
                    shell.publish(Message::CanvasPan(dx * 40.0, dy * 40.0));
                }
                return event::Status::Captured;
            }
        }

        let action_active = !matches!(state.action, CanvasAction::None);
        if action_active {
            match &event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    let sp = cursor_pos;
                    let wp = to_world(Point::new(sp.x - bounds.x, sp.y - bounds.y), pan, z);

                    let action = std::mem::replace(&mut state.action, CanvasAction::None);
                    match action {
                        CanvasAction::Dragging { ref node_id, offset, ref group } => {
                            let new_x = snap(wp.x - offset.x);
                            let new_y = snap(wp.y - offset.y);
                            if group.len() > 1 {
                                let (_, ox, oy) = group.iter().find(|(id, _, _)| id == node_id).cloned().unwrap_or_default();
                                let dx = new_x - ox;
                                let dy = new_y - oy;
                                let moves: Vec<(String, f32, f32)> = group.iter().map(|(id, gx, gy)| (id.clone(), snap(gx + dx), snap(gy + dy))).collect();
                                shell.publish(Message::CanvasMoveNodeGroup(moves));
                            } else {
                                shell.publish(Message::CanvasMoveNode(node_id.clone(), new_x, new_y));
                            }
                            state.action = action; // put back
                        }
                        CanvasAction::Panning { last } => {
                            let dx = sp.x - last.x;
                            let dy = sp.y - last.y;
                            shell.publish(Message::CanvasPan(dx, dy));
                            state.action = CanvasAction::Panning { last: sp };
                        }
                        CanvasAction::Selecting { start, .. } => {
                            state.action = CanvasAction::Selecting { start, current: wp };
                        }
                        CanvasAction::Connecting { .. } => {
                            state.action = action; // put back, just track cursor
                        }
                        CanvasAction::Resizing { ref node_id, corner, start_rect } => {
                            let (sx, sy, sw, sh) = start_rect;
                            let min_w = 80.0f32;
                            let min_h = 48.0f32;
                            #[allow(unused_assignments)]
                            let (mut nx, mut ny, mut nw, mut nh) = (sx, sy, sw, sh);
                            match corner {
                                Corner::BR => { nw = snap((wp.x - sx).max(min_w)); nh = snap((wp.y - sy).max(min_h)); nx = sx; ny = sy; }
                                Corner::BL => { nw = snap((sx + sw - wp.x).max(min_w)); nx = snap(sx + sw - nw); nh = snap((wp.y - sy).max(min_h)); ny = sy; }
                                Corner::TR => { nw = snap((wp.x - sx).max(min_w)); nx = sx; nh = snap((sy + sh - wp.y).max(min_h)); ny = snap(sy + sh - nh); }
                                Corner::TL => { nw = snap((sx + sw - wp.x).max(min_w)); nx = snap(sx + sw - nw); nh = snap((sy + sh - wp.y).max(min_h)); ny = snap(sy + sh - nh); }
                            }
                            shell.publish(Message::CanvasResizeNode(node_id.clone(), nx, ny, nw, nh));
                            state.action = action; // put back
                        }
                        CanvasAction::None => {} // shouldn't happen
                    }
                    return event::Status::Captured;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    let sp_local = Point::new(cursor_pos.x - bounds.x, cursor_pos.y - bounds.y);
                    let wp = to_world(sp_local, pan, z);
                    let action = std::mem::replace(&mut state.action, CanvasAction::None);
                    match action {
                        CanvasAction::Connecting { from_id, from_side } => {
                            if let Some(target) = self.editor.data.nodes.iter().find(|n| n.contains(wp)) {
                                if target.id != from_id {
                                    let to_side = target.edge_dots().iter().enumerate()
                                        .min_by(|(_, a), (_, b)| dist(**a, wp).partial_cmp(&dist(**b, wp)).unwrap())
                                        .map(|(i, _)| CanvasNode::dot_side(i))
                                        .unwrap_or(CardSide::Top);
                                    shell.publish(Message::CanvasAddEdge(from_id, from_side, target.id.clone(), to_side));
                                }
                            }
                        }
                        CanvasAction::Selecting { start, current } => {
                            let (mnx, mny) = (start.x.min(current.x), start.y.min(current.y));
                            let (mxx, mxy) = (start.x.max(current.x), start.y.max(current.y));
                            let sel: Vec<String> = self.editor.data.nodes.iter()
                                .filter(|n| n.x + n.w > mnx && n.x < mxx && n.y + n.h > mny && n.y < mxy)
                                .map(|n| n.id.clone()).collect();
                            if !sel.is_empty() { shell.publish(Message::CanvasMultiSelect(sel)); }
                        }
                        _ => {}
                    }
                    return event::Status::Captured;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                    state.action = CanvasAction::None;
                    return event::Status::Captured;
                }
                _ => {}
            }
        }

        if !in_bounds { return event::Status::Ignored; }

        if let Event::Keyboard(_) = &event {
            if let Some(ref focused_id) = self.editor.focused_card {
                let children_layouts: Vec<Layout<'_>> = layout.children().collect();
                for (i, (id, element)) in self.cards.iter_mut().enumerate() {
                    if id == focused_id {
                        let status = element.as_widget_mut().on_event(
                            &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                        );
                        if status == event::Status::Captured {
                            return event::Status::Captured;
                        }
                        break;
                    }
                }
            }
            if self.editor.focused_card.is_none() {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = &event {
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Delete) | keyboard::Key::Named(keyboard::key::Named::Backspace)) {
                        if !self.editor.selected.is_empty() || !self.editor.selected_edges.is_empty() {
                            shell.publish(Message::CanvasDeleteSelected);
                            return event::Status::Captured;
                        }
                    }
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                        shell.publish(Message::CanvasCardUnfocus);
                        return event::Status::Captured;
                    }
                    if modifiers.command() {
                        if let keyboard::Key::Character(c) = key {
                            match c.as_str() {
                                "z" | "Z" => { shell.publish(Message::CanvasUndo); return event::Status::Captured; }
                                "y" | "Y" => { shell.publish(Message::CanvasRedo); return event::Status::Captured; }
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = &event {
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                        shell.publish(Message::CanvasCardUnfocus);
                        return event::Status::Captured;
                    }
                }
            }
            return event::Status::Ignored;
        }

        let sp_local = Point::new(cursor_pos.x - bounds.x, cursor_pos.y - bounds.y);
        let wp = to_world(sp_local, pan, z);

        match &event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                shell.publish(Message::CanvasCloseCtxMenu);

                let now = std::time::Instant::now();
                let dbl = state.last_click.map_or(false, |(t, p)| now.duration_since(t).as_millis() < 350 && dist(p, sp_local) < 10.0);
                state.last_click = Some((now, sp_local));

                if state.ctrl_held {
                    state.action = CanvasAction::Panning { last: cursor_pos };
                    return event::Status::Captured;
                }

                for node in &self.editor.data.nodes {
                    let ht = self.editor.hover_anim.get(&node.id).copied().unwrap_or(0.0);
                    let st_anim = self.editor.select_anim.get(&node.id).copied().unwrap_or(0.0);
                    if ht > 0.3 || st_anim > 0.3 || matches!(state.action, CanvasAction::Connecting { .. }) {
                        for (di, dot) in node.edge_dots().iter().enumerate() {
                            let sd = to_screen(*dot, pan, z);
                            let sd_abs = Point::new(sd.x + bounds.x, sd.y + bounds.y);
                            if dist(sd_abs, cursor_pos) < DOT_R * 4.0 {
                                state.action = CanvasAction::Connecting { from_id: node.id.clone(), from_side: CanvasNode::dot_side(di) };
                                shell.publish(Message::CanvasSelect(Some(node.id.clone())));
                                return event::Status::Captured;
                            }
                        }
                    }
                }

                for sid in &self.editor.selected {
                    if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *sid) {
                        let corners = [Corner::TL, Corner::TR, Corner::BR, Corner::BL];
                        for (ci, cr) in node.corner_rects().iter().enumerate() {
                            let ss = to_screen(cr.position(), pan, z);
                            let ss_abs = Point::new(ss.x + bounds.x, ss.y + bounds.y);
                            let sr = Rectangle::new(ss_abs, Size::new(cr.width * z, cr.height * z));
                            if sr.contains(cursor_pos) {
                                state.action = CanvasAction::Resizing { node_id: sid.clone(), corner: corners[ci], start_rect: (node.x, node.y, node.w, node.h) };
                                return event::Status::Captured;
                            }
                        }
                    }
                }

                // Iterate in reverse so topmost card (last drawn) gets priority
                let children_layouts: Vec<Layout<'_>> = layout.children().collect();
                for (i, (id, element)) in self.cards.iter_mut().enumerate().rev() {
                    if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *id) {
                        let sx = node.x * z + pan.0 + bounds.x;
                        let sy = node.y * z + pan.1 + bounds.y;
                        let simplified = z < SIMPLIFY_ZOOM;
                        let (sw, sh) = if simplified {
                            (node.w * z, node.h * z)
                        } else {
                            (children_layouts[i].bounds().width, children_layouts[i].bounds().height)
                        };
                        let card_bounds = Rectangle::new(Point::new(sx, sy), Size::new(sw, sh));

                        if card_bounds.contains(cursor_pos) {
                            if simplified {
                                let offset = Vector::new(wp.x - node.x, wp.y - node.y);
                                let group = if self.editor.selected.contains(id) {
                                    self.editor.data.nodes.iter()
                                        .filter(|n| self.editor.selected.contains(&n.id))
                                        .map(|n| (n.id.clone(), n.x, n.y))
                                        .collect()
                                } else {
                                    vec![(id.clone(), node.x, node.y)]
                                };
                                state.action = CanvasAction::Dragging { node_id: id.clone(), offset, group };
                                if !self.editor.selected.contains(id) {
                                    shell.publish(Message::CanvasSelect(Some(id.clone())));
                                }
                                return event::Status::Captured;
                            }
                            if self.editor.focused_card.as_ref() == Some(id) {
                                let status = element.as_widget_mut().on_event(
                                    &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                                );
                                return status;
                            } else if dbl {
                                shell.publish(Message::CanvasCardFocus(id.clone()));
                                shell.publish(Message::CanvasSelect(Some(id.clone())));
                                let _status = element.as_widget_mut().on_event(
                                    &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                                );
                                return event::Status::Captured;
                            } else {
                                let offset = Vector::new(wp.x - node.x, wp.y - node.y);
                                let group = if self.editor.selected.contains(id) {
                                    self.editor.data.nodes.iter()
                                        .filter(|n| self.editor.selected.contains(&n.id))
                                        .map(|n| (n.id.clone(), n.x, n.y))
                                        .collect()
                                } else {
                                    vec![(id.clone(), node.x, node.y)]
                                };
                                state.action = CanvasAction::Dragging { node_id: id.clone(), offset, group };
                                if !self.editor.selected.contains(id) {
                                    shell.publish(Message::CanvasSelect(Some(id.clone())));
                                }
                                if self.editor.focused_card.is_some() {
                                    shell.publish(Message::CanvasCardUnfocus);
                                }
                                return event::Status::Captured;
                            }
                        }
                    }
                }

                for edge in &self.editor.data.edges {
                    let from_node = self.editor.data.nodes.iter().find(|n| n.id == edge.from);
                    let to_node = self.editor.data.nodes.iter().find(|n| n.id == edge.to);
                    if let (Some(f), Some(t)) = (from_node, to_node) {
                        let fp = to_screen(f.side_point(edge.from_side), pan, z);
                        let tp = to_screen(t.side_point(edge.to_side), pan, z);
                        let d = dist(fp, tp) * 0.4;
                        let cp1 = Point::new(fp.x + control_offset(edge.from_side, d).x, fp.y + control_offset(edge.from_side, d).y);
                        let cp2 = Point::new(tp.x + control_offset(edge.to_side, d).x, tp.y + control_offset(edge.to_side, d).y);
                        if bezier_dist(sp_local, fp, cp1, cp2, tp) < 8.0 {
                            shell.publish(Message::CanvasSelectEdge(Some(edge.id.clone())));
                            shell.publish(Message::CanvasCardUnfocus);
                            return event::Status::Captured;
                        }
                    }
                }

                shell.publish(Message::CanvasSelect(None));
                shell.publish(Message::CanvasCardUnfocus);
                state.action = CanvasAction::Selecting { start: wp, current: wp };
                return event::Status::Captured;
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.action = CanvasAction::Panning { last: cursor_pos };
                return event::Status::Captured;
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // If a card is focused, forward right-click to md_editor for text context menu
                if let Some(ref focused_id) = self.editor.focused_card {
                    if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *focused_id && n.contains(wp)) {
                        let children_layouts: Vec<Layout<'_>> = layout.children().collect();
                        for (i, (id, element)) in self.cards.iter_mut().enumerate() {
                            if id == &node.id {
                                let _status = element.as_widget_mut().on_event(
                                    &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                                );
                                return event::Status::Captured;
                            }
                        }
                    }
                }

                let screen_pos = (sp_local.x, sp_local.y);
                if let Some(node) = self.editor.data.nodes.iter().find(|n| n.contains(wp)) {
                    let nid = node.id.clone();
                    shell.publish(Message::CanvasShowCtxMenu(screen_pos.0, screen_pos.1, CanvasCtxTarget::Node(nid.clone())));
                    shell.publish(Message::CanvasSelect(Some(nid)));
                    return event::Status::Captured;
                }
                for edge in &self.editor.data.edges {
                    let from_node = self.editor.data.nodes.iter().find(|n| n.id == edge.from);
                    let to_node = self.editor.data.nodes.iter().find(|n| n.id == edge.to);
                    if let (Some(f), Some(t)) = (from_node, to_node) {
                        let fp = to_screen(f.side_point(edge.from_side), pan, z);
                        let tp = to_screen(t.side_point(edge.to_side), pan, z);
                        let d = dist(fp, tp) * 0.4;
                        let cp1 = Point::new(fp.x + control_offset(edge.from_side, d).x, fp.y + control_offset(edge.from_side, d).y);
                        let cp2 = Point::new(tp.x + control_offset(edge.to_side, d).x, tp.y + control_offset(edge.to_side, d).y);
                        if bezier_dist(sp_local, fp, cp1, cp2, tp) < 8.0 {
                            shell.publish(Message::CanvasShowCtxMenu(screen_pos.0, screen_pos.1, CanvasCtxTarget::Edge(edge.id.clone())));
                            shell.publish(Message::CanvasSelectEdge(Some(edge.id.clone())));
                            return event::Status::Captured;
                        }
                    }
                }
                shell.publish(Message::CanvasShowCtxMenu(screen_pos.0, screen_pos.1, CanvasCtxTarget::Empty(snap(wp.x - 80.0), snap(wp.y - 24.0))));
                return event::Status::Captured;
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let new_hover = self.editor.data.nodes.iter().find(|n| n.contains(wp)).map(|n| n.id.clone());
                if new_hover != self.editor.last_hovered {
                    shell.publish(Message::CanvasHover(new_hover));
                }

                if let Some(ref focused_id) = self.editor.focused_card {
                    let children_layouts: Vec<Layout<'_>> = layout.children().collect();
                    for (i, (id, element)) in self.cards.iter_mut().enumerate() {
                        if id == focused_id {
                            let _status = element.as_widget_mut().on_event(
                                &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                            );
                            break;
                        }
                    }
                }
                return event::Status::Ignored; // Let other widgets also see cursor moves
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(ref focused_id) = self.editor.focused_card {
                    let children_layouts: Vec<Layout<'_>> = layout.children().collect();
                    for (i, (id, element)) in self.cards.iter_mut().enumerate() {
                        if id == focused_id {
                            let _status = element.as_widget_mut().on_event(
                                &mut tree.children[i], event.clone(), children_layouts[i], cursor, renderer, clipboard, shell, viewport,
                            );
                            break;
                        }
                    }
                }
                return event::Status::Ignored;
            }

            _ => {}
        }

        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let Some(pos) = cursor.position() else { return mouse::Interaction::default() };
        if !bounds.contains(pos) { return mouse::Interaction::default() }

        let state = tree.state.downcast_ref::<CanvasViewState>();
        let pan = self.editor.pan;
        let z = self.editor.zoom;
        let handle_screen = HANDLE_H * z;

        match &state.action {
            CanvasAction::Dragging { .. } => return mouse::Interaction::Move,
            CanvasAction::Panning { .. } => return mouse::Interaction::Grabbing,
            CanvasAction::Connecting { .. } => return mouse::Interaction::Pointer,
            CanvasAction::Selecting { .. } => return mouse::Interaction::Pointer,
            CanvasAction::Resizing { corner, .. } => return match corner {
                Corner::TL | Corner::BR => mouse::Interaction::ResizingDiagonallyDown,
                Corner::TR | Corner::BL => mouse::Interaction::ResizingDiagonallyUp,
            },
            CanvasAction::None => {}
        }

        if state.ctrl_held { return mouse::Interaction::Grab; }

        let sp_local = Point::new(pos.x - bounds.x, pos.y - bounds.y);
        let wp = to_world(sp_local, pan, z);

        for node in &self.editor.data.nodes {
            let ht = self.editor.hover_anim.get(&node.id).copied().unwrap_or(0.0);
            let st = self.editor.select_anim.get(&node.id).copied().unwrap_or(0.0);
            if ht > 0.3 || st > 0.3 {
                for dot in &node.edge_dots() {
                    let sd = to_screen(*dot, pan, z);
                    let sd_abs = Point::new(sd.x + bounds.x, sd.y + bounds.y);
                    if dist(sd_abs, pos) < DOT_R * 4.0 { return mouse::Interaction::Pointer; }
                }
            }
        }

        for sid in &self.editor.selected {
            if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *sid) {
                let corners = node.corner_rects();
                for (ci, cr) in corners.iter().enumerate() {
                    let ss = to_screen(cr.position(), pan, z);
                    let ss_abs = Point::new(ss.x + bounds.x, ss.y + bounds.y);
                    let sr = Rectangle::new(ss_abs, Size::new(cr.width * z, cr.height * z));
                    if sr.contains(pos) {
                        return match ci {
                            0 | 2 => mouse::Interaction::ResizingDiagonallyDown,
                            _ => mouse::Interaction::ResizingDiagonallyUp,
                        };
                    }
                }
            }
        }

        let simplified = z < SIMPLIFY_ZOOM;
        let children_layouts: Vec<Layout<'_>> = layout.children().collect();
        for (i, (id, element)) in self.cards.iter().enumerate() {
            if let Some(node) = self.editor.data.nodes.iter().find(|n| n.id == *id) {
                let sx = node.x * z + pan.0 + bounds.x;
                let sy = node.y * z + pan.1 + bounds.y;
                let (sw, sh) = if simplified {
                    (node.w * z, node.h * z)
                } else {
                    (children_layouts[i].bounds().width, children_layouts[i].bounds().height)
                };
                let card_bounds = Rectangle::new(Point::new(sx, sy), Size::new(sw, sh));

                if card_bounds.contains(pos) {
                    if self.editor.focused_card.as_ref() == Some(id) {
                        return element.as_widget().mouse_interaction(
                            &tree.children[i], children_layouts[i], cursor, viewport, renderer,
                        );
                    } else {
                        return mouse::Interaction::Move; // click to select/drag
                    }
                }
            }
        }

        for edge in &self.editor.data.edges {
            let from_node = self.editor.data.nodes.iter().find(|n| n.id == edge.from);
            let to_node = self.editor.data.nodes.iter().find(|n| n.id == edge.to);
            if let (Some(f), Some(t)) = (from_node, to_node) {
                let fp = to_screen(f.side_point(edge.from_side), pan, z);
                let tp = to_screen(t.side_point(edge.to_side), pan, z);
                let d = dist(fp, tp) * 0.4;
                let cp1 = Point::new(fp.x + control_offset(edge.from_side, d).x, fp.y + control_offset(edge.from_side, d).y);
                let cp2 = Point::new(tp.x + control_offset(edge.to_side, d).x, tp.y + control_offset(edge.to_side, d).y);
                if bezier_dist(sp_local, fp, cp1, cp2, tp) < 8.0 {
                    return mouse::Interaction::Pointer;
                }
            }
        }

        mouse::Interaction::default()
    }
}

// Approximate bezier curves with densely packed circles (rounded quads).

fn draw_bezier_line(
    renderer: &mut iced::Renderer,
    p0: Point, p1: Point, p2: Point, p3: Point,
    thickness: f32, color: Color, _clip: &Rectangle,
) {
    use iced::advanced::Renderer as _;
    let total_dist = dist(p0, p1) + dist(p1, p2) + dist(p2, p3);
    // Place a dot every ~1.5px for smooth appearance
    let steps = (total_dist / 3.0).max(10.0).min(200.0) as usize;
    let half = thickness / 2.0;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let u = 1.0 - t;
        let x = u*u*u*p0.x + 3.0*u*u*t*p1.x + 3.0*u*t*t*p2.x + t*t*t*p3.x;
        let y = u*u*u*p0.y + 3.0*u*u*t*p1.y + 3.0*u*t*t*p2.y + t*t*t*p3.y;

        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(Point::new(x - half, y - half), Size::new(thickness, thickness)),
                border: Border { radius: half.into(), ..Default::default() },
                shadow: iced::Shadow::default(),
            },
            Background::Color(color),
        );
    }
}

fn draw_straight_line(
    renderer: &mut iced::Renderer,
    p0: Point, p1: Point,
    thickness: f32, color: Color, clip: &Rectangle,
) {
    use iced::advanced::Renderer as _;
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let length = (dx * dx + dy * dy).sqrt();
    let steps = (length / 3.0).max(5.0).min(100.0) as usize;
    let half = thickness / 2.0;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = p0.x + dx * t;
        let y = p0.y + dy * t;
        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(Point::new(x - half, y - half), Size::new(thickness, thickness)),
                border: Border { radius: half.into(), ..Default::default() },
                shadow: iced::Shadow::default(),
            },
            Background::Color(color),
        );
    }
}
