use iced::widget::{button, column, container, horizontal_space, row, scrollable, svg, text, text_input, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::models::FileEntry;
use crate::ui::{icons, theme};

pub fn view<'a>(
    files: &'a [FileEntry],
    file_path_input: &str,
    is_maximized: bool,
) -> Element<'a, Message> {
    let editor_style = if is_maximized { theme::editor_panel_square as fn(&iced::Theme) -> container::Style } else { theme::editor_panel_rounded };
    let header = row![
        text("File Vault").size(16).style(|_t| theme::primary_text()),
        horizontal_space(),
    ].align_y(iced::Alignment::Center);

    let add_row = row![
        text_input("File path to add...", file_path_input)
            .on_input(Message::FilePathInputChanged)
            .on_submit(Message::AddFileToVault)
            .style(theme::dialog_input)
            .size(12)
            .padding(7),
        button(
            row![svg(icons::plus_bright()).width(14).height(14), text("Add").size(12)]
                .spacing(4).align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddFileToVault)
        .style(theme::new_note_button)
        .padding([7, 12]),
    ].spacing(6).align_y(iced::Alignment::Center);

    let mut file_list = column![].spacing(4);
    for entry in files {
        let lock_icon = if entry.encrypted {
            svg(icons::lock_active()).width(14).height(14)
        } else {
            svg(icons::document_icon()).width(14).height(14)
        };

        let file_row = container(
            row![
                lock_icon,
                column![
                    text(&entry.name).size(13).style(|_t| theme::primary_text()),
                    text(entry.size_display()).size(10).style(|_t| theme::secondary_text()),
                ].spacing(2),
                horizontal_space(),
                button(svg(icons::trash_icon()).width(14).height(14))
                    .on_press(Message::DeleteVaultFile(entry.id))
                    .style(theme::icon_button)
                    .padding([4, 6]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([8, 10]),
        )
        .style(|_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_TERTIARY)),
            border: iced::Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        });

        file_list = file_list.push(file_row);
    }

    if files.is_empty() {
        file_list = file_list.push(
            container(
                text("No files stored. Add files using the path input above.")
                    .size(12)
                    .style(|_t| theme::secondary_text()),
            )
            .padding(16),
        );
    }

    let content = column![
        header,
        Space::with_height(8),
        add_row,
        Space::with_height(8),
        scrollable(file_list).direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar())).style(theme::dark_scrollable).height(Length::Fill),
    ]
    .spacing(4)
    .padding(16);

    container(content)
        .style(editor_style)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
