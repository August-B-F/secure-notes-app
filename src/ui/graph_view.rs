use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::app::Message;
use crate::models::NotePreview;

/// Simple graph view showing notes as nodes.
pub struct GraphState {
    cache: Cache,
    pub nodes: Vec<GraphNode>,
}

pub struct GraphNode {
    pub id: uuid::Uuid,
    pub title: String,
    pub color: Color,
    pub position: Point,
}

impl GraphState {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
            nodes: Vec::new(),
        }
    }

    pub fn update_from_notes(&mut self, notes: &[NotePreview]) {
        // Only rebuild if count changed
        if self.nodes.len() != notes.len() {
            self.nodes.clear();
            let count = notes.len();
            for (i, note) in notes.iter().enumerate() {
                // Arrange in a circle
                let angle = (i as f32 / count.max(1) as f32) * std::f32::consts::TAU;
                let radius = 150.0 + (count as f32 * 10.0).min(200.0);
                let x = 400.0 + angle.cos() * radius;
                let y = 300.0 + angle.sin() * radius;
                self.nodes.push(GraphNode {
                    id: note.id,
                    title: if note.title.is_empty() { "Untitled".to_string() } else { note.title.clone() },
                    color: note.color.to_iced_color(),
                    position: Point::new(x, y),
                });
            }
            self.cache.clear();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl canvas::Program<Message> for &GraphState {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgb(0x1F as f32 / 255.0, 0x1F as f32 / 255.0, 0x1F as f32 / 255.0),
            );

            for i in 0..self.nodes.len().saturating_sub(1) {
                let a = &self.nodes[i];
                let b = &self.nodes[i + 1];
                let path = Path::line(a.position, b.position);
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgba(0.3, 0.3, 0.4, 0.3))
                        .with_width(1.0),
                );
            }

            for node in &self.nodes {
                let circle = Path::circle(node.position, 20.0);
                frame.fill(&circle, node.color);

                let label = Text {
                    content: if node.title.len() > 12 {
                        format!("{}...", &node.title[..10])
                    } else {
                        node.title.clone()
                    },
                    position: Point::new(node.position.x, node.position.y + 28.0),
                    color: Color::from_rgb(0.85, 0.85, 0.85),
                    size: 11.0.into(),
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    ..Text::default()
                };
                frame.fill_text(label);
            }

            if self.nodes.is_empty() {
                let title = Text {
                    content: "Graph View — Create notes to see them here".to_string(),
                    position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                    color: Color::from_rgb(0.55, 0.55, 0.55),
                    size: 16.0.into(),
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    ..Text::default()
                };
                frame.fill_text(title);
            }
        });

        vec![geometry]
    }
}
