use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::theme;

pub fn view<'a>(title: &str, body: &str, confirm_message: Message) -> Element<'a, Message> {
    let card = container(
        column![
            text(title.to_owned()).size(18).style(|_t| theme::primary_text()),
            Space::with_height(4),
            text(body.to_owned()).size(13).style(|_t| theme::secondary_text()),
            Space::with_height(12),
            row![
                button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
                    .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
                button(text("Delete").size(13).align_x(Horizontal::Center).width(Length::Fill))
                    .on_press(confirm_message).style(theme::danger_button).padding([8, 16]).width(Length::Fill),
            ].spacing(8),
        ].spacing(2).padding(20),
    )
    .style(theme::dialog_card)
    .width(300);

    container(card)
        .style(theme::dialog_overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
