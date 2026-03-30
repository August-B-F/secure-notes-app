use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, svg, text, text_input, tooltip, Space};
use iced::{Element, Length};
use uuid::Uuid;

use crate::app::Message;
use crate::models::{Folder, FolderColor, NoteType};
use crate::ui::{color_picker, icons, theme};

pub fn view<'a>(
    title_value: &str,
    selected_type: NoteType,
    _selected_color: FolderColor,
    _selected_folder: Option<Uuid>,
    _folders: &'a [Folder],
    hue: f32, sat: f32, lit: f32,
) -> Element<'a, Message> {
    let type_row = row![
        type_icon_btn(icons::note_text_icon(iced::Color::from_rgb(0.85, 0.85, 0.87)), "Text note", NoteType::Text, selected_type == NoteType::Text),
        type_icon_btn(icons::note_password_icon(iced::Color::from_rgb(0.85, 0.85, 0.87)), "Password", NoteType::Password, selected_type == NoteType::Password),
        type_icon_btn(icons::note_canvas_icon(iced::Color::from_rgb(0.85, 0.85, 0.87)), "Canvas", NoteType::Canvas, selected_type == NoteType::Canvas),
    ].spacing(8);

    let title_input = text_input("Give it a name...", title_value)
        .on_input(Message::CreateDialogTitleChanged)
        .on_submit(Message::SubmitCreateNote)
        .style(theme::dialog_input)
        .size(14)
        .padding(10);

    let picker = color_picker::view(hue, sat, lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);

    let actions = row![
        button(text("Cancel").size(14).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::CloseDialog).style(theme::secondary_button).padding([10, 20]).width(Length::Fill),
        button(text("Create").size(14).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::SubmitCreateNote).style(theme::submit_button).padding([10, 20]).width(Length::Fill),
    ].spacing(10);

    let card = container(
        column![
            text("New Note").size(18).style(|_t| theme::primary_text()),

            Space::with_height(14),
            type_row,

            Space::with_height(12),
            title_input,

            Space::with_height(12),
            picker,

            Space::with_height(16),
            actions,
        ].padding(24).width(Length::Fill),
    )
    .style(theme::dialog_card)
    .width(320);

    container(container(card).center_x(Length::Fill).center_y(Length::Fill))
        .style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).into()
}

fn type_icon_btn<'a>(icon: iced::widget::svg::Handle, hint: &'a str, note_type: NoteType, selected: bool) -> Element<'a, Message> {
    let bg = if selected {
        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1)
    } else {
        iced::Color::TRANSPARENT
    };
    let border_col = if selected {
        iced::Color::from_rgb(0.18, 0.55, 0.31)
    } else {
        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06)
    };

    let btn = button(
        svg(icon).width(20).height(20)
    )
    .on_press(Message::CreateDialogTypeChanged(note_type))
    .style(move |_t: &iced::Theme, status: iced::widget::button::Status| {
        let b = match status {
            iced::widget::button::Status::Hovered => iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            _ => bg,
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(b)),
            border: iced::Border {
                radius: 8.0.into(),
                width: if selected { 1.5 } else { 1.0 },
                color: border_col,
            },
            ..Default::default()
        }
    })
    .padding([10, 14]);

    tooltip(btn, hint, tooltip::Position::Bottom)
        .style(|_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.15, 0.15, 0.18))),
            border: iced::Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding(6)
        .gap(4)
        .into()
}
