use iced::mouse;
use iced::widget::canvas::{self, Cache, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, row, text_input, Space};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::app::Message;
use crate::models::FolderColor;
use crate::ui::theme;

#[allow(dead_code)]
const SV_W: f32 = 240.0;
const SV_H: f32 = 100.0;
const HUE_H: f32 = 14.0;
const GAP: f32 = 8.0;

struct PickerProgram { hue: f32, sat: f32, val: f32 }

enum PickerAction { None, DraggingSV, DraggingHue }
impl Default for PickerAction { fn default() -> Self { Self::None } }

pub struct PickerInteraction { action: PickerAction }
impl Default for PickerInteraction { fn default() -> Self { Self { action: PickerAction::None } } }

impl canvas::Program<Message> for PickerProgram {
    type State = PickerInteraction;

    fn update(&self, state: &mut PickerInteraction, event: Event, bounds: Rectangle, cursor: mouse::Cursor) -> (canvas::event::Status, Option<Message>) {
        let is_dragging = !matches!(state.action, PickerAction::None);

        // When dragging, use absolute cursor position so drag continues outside bounds
        let pos = if is_dragging {
            cursor.position_in(bounds).or_else(|| {
                cursor.position().map(|abs| Point::new(abs.x - bounds.x, abs.y - bounds.y))
            })
        } else {
            cursor.position_in(bounds)
        };
        let Some(pos) = pos else { return (canvas::event::Status::Ignored, None) };

        let w = bounds.width;
        let sv_rect = Rectangle::new(Point::ORIGIN, iced::Size::new(w, SV_H));
        let hue_rect = Rectangle::new(Point::new(0.0, SV_H + GAP), iced::Size::new(w, HUE_H));

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if sv_rect.contains(pos) {
                    state.action = PickerAction::DraggingSV;
                    let s = (pos.x / w).clamp(0.0, 1.0);
                    let v = (1.0 - pos.y / SV_H).clamp(0.0, 1.0);
                    return (canvas::event::Status::Captured, Some(Message::ColorPickerSVChanged(s * 100.0, v * 100.0)));
                }
                if hue_rect.contains(pos) {
                    state.action = PickerAction::DraggingHue;
                    let h = (pos.x / w * 360.0).clamp(0.0, 360.0);
                    return (canvas::event::Status::Captured, Some(Message::ColorPickerHue(h)));
                }
                (canvas::event::Status::Ignored, None)
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                match state.action {
                    PickerAction::DraggingSV => {
                        let s = (pos.x / w).clamp(0.0, 1.0);
                        let v = (1.0 - pos.y / SV_H).clamp(0.0, 1.0);
                        (canvas::event::Status::Captured, Some(Message::ColorPickerSVChanged(s * 100.0, v * 100.0)))
                    }
                    PickerAction::DraggingHue => {
                        let h = (pos.x / w * 360.0).clamp(0.0, 360.0);
                        (canvas::event::Status::Captured, Some(Message::ColorPickerHue(h)))
                    }
                    PickerAction::None => (canvas::event::Status::Ignored, None),
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                if is_dragging {
                    state.action = PickerAction::None;
                    (canvas::event::Status::Captured, None)
                } else {
                    (canvas::event::Status::Ignored, None)
                }
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn draw(&self, _state: &PickerInteraction, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let cache = Cache::new();
        let geo = cache.draw(renderer, bounds.size(), |frame: &mut Frame| {
            let w = bounds.width;
            let sx = 50; let sy = 30;
            let cw = w / sx as f32;
            let ch = SV_H / sy as f32;
            for xi in 0..sx {
                for yi in 0..sy {
                    let s = (xi as f32 + 0.5) / sx as f32;
                    let v = 1.0 - (yi as f32 + 0.5) / sy as f32;
                    frame.fill_rectangle(Point::new(xi as f32 * cw, yi as f32 * ch), iced::Size::new(cw + 0.5, ch + 0.5), hsv_to_rgb(self.hue, s, v));
                }
            }
            let ix = self.sat * w;
            let iy = (1.0 - self.val) * SV_H;
            frame.stroke(&Path::circle(Point::new(ix, iy), 5.0), Stroke::default().with_color(Color::WHITE).with_width(2.0));
            frame.stroke(&Path::circle(Point::new(ix, iy), 6.0), Stroke::default().with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.4)).with_width(1.0));
            let hy = SV_H + GAP;
            let hs = 72;
            let hcw = w / hs as f32;
            for i in 0..hs {
                frame.fill_rectangle(Point::new(i as f32 * hcw, hy), iced::Size::new(hcw + 0.5, HUE_H), hsv_to_rgb(i as f32 / hs as f32 * 360.0, 1.0, 1.0));
            }
            let hx = self.hue / 360.0 * w;
            frame.fill_rectangle(Point::new(hx - 1.5, hy - 1.0), iced::Size::new(3.0, HUE_H + 2.0), Color::WHITE);
        });
        vec![geo]
    }
}

pub fn view<'a>(
    hue: f32, saturation: f32, lightness: f32,
    _on_hue: fn(f32) -> Message, _on_sat: fn(f32) -> Message, _on_lit: fn(f32) -> Message,
) -> Element<'a, Message> {
    let s = saturation / 100.0;
    let v = lightness / 100.0;
    let preview_color = hsv_to_rgb(hue, s, v);
    let hex = format!("#{:02X}{:02X}{:02X}", (preview_color.r * 255.0) as u8, (preview_color.g * 255.0) as u8, (preview_color.b * 255.0) as u8);

    let picker = Canvas::new(PickerProgram { hue, sat: s, val: v })
        .width(Length::Fill)
        .height(SV_H + GAP + HUE_H);

    let preview_row = row![
        container(Space::new(22, 22))
            .style(move |_t: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(preview_color)),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }),
        Space::with_width(8),
        text_input("", &hex)
            .on_input(Message::ColorPickerHexInput)
            .size(12)
            .width(75)
            .padding(2)
            .style(|_t: &iced::Theme, _s| iced::widget::text_input::Style {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: iced::Border { radius: 3.0.into(), width: 0.0, color: Color::TRANSPARENT },
                icon: Color::from_rgb(0.6, 0.6, 0.6),
                placeholder: Color::from_rgb(0.6, 0.6, 0.6),
                value: Color::from_rgb(0.85, 0.85, 0.87),
                selection: Color::from_rgb(0.25, 0.35, 0.45),
            }),
    ].align_y(iced::Alignment::Center);

    let mut presets = row![].spacing(3);
    for &c in FolderColor::PALETTE.iter().take(10) {
        presets = presets.push(
            iced::widget::button(Space::new(16, 16))
                .on_press(Message::ColorPickerPreset(c))
                .style(theme::color_dot_button(c.to_iced_color(), false))
                .padding(0),
        );
    }

    column![
        picker,
        Space::with_height(6),
        preview_row,
        Space::with_height(6),
        presets,
    ].spacing(0).into()
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

#[allow(dead_code)]
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}
