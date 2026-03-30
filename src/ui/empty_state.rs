use iced::alignment::Horizontal;
use iced::widget::{column, container, svg, text, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::{icons, theme};

pub fn view<'a>(is_maximized: bool) -> Element<'a, Message> {
    let editor_style = if is_maximized { theme::editor_panel_square as fn(&iced::Theme) -> iced::widget::container::Style } else { theme::editor_panel_rounded };

    container(
        column![
            Space::with_height(Length::Fill),
            svg(icons::document_icon()).width(28).height(28),
            Space::with_height(6),
            text("No note selected")
                .size(14)
                .style(|_t| theme::secondary_text()),
            Space::with_height(4),
            text("Select a note or press Ctrl+N")
                .size(11)
                .style(|_t| iced::widget::text::Style {
                    color: Some(iced::Color::from_rgb(0.35, 0.35, 0.38)),
                }),
            Space::with_height(Length::Fill),
        ]
        .spacing(0)
        .align_x(Horizontal::Center)
        .width(Length::Fill),
    )
    .style(editor_style)
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .into()
}
