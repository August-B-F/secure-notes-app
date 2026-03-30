use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::models::FolderColor;
use crate::ui::{color_picker, theme};

pub fn view<'a>(title: &str, name_value: &str, _selected_color: FolderColor, confirm_message: Message, hue: f32, sat: f32, lit: f32) -> Element<'a, Message> {
    let picker = color_picker::view(hue, sat, lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);

    let card = container(
        column![
            text(title.to_owned()).size(18).style(|_t| theme::primary_text()),
            text_input("Folder name...", name_value)
                .on_input(Message::FolderNameInputChanged)
                .on_submit(confirm_message.clone())
                .style(theme::dialog_input).size(13).padding(8),
            text("Color").size(11).style(|_t| theme::secondary_text()),
            picker,
            Space::with_height(4),
            row![
                button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
                    .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
                button(text("Save").size(13).align_x(Horizontal::Center).width(Length::Fill))
                    .on_press(confirm_message).style(theme::submit_button).padding([8, 16]).width(Length::Fill),
            ].spacing(8),
        ].spacing(8).max_width(300).padding(20),
    ).style(theme::dialog_card);

    container(container(card).center_x(Length::Fill).center_y(Length::Fill))
        .style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).into()
}
