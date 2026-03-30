use iced::widget::{button, column, container, row, svg, text, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::{icons, theme};

/// Parse file metadata from the note body: [file:UUID:filename:size]
pub fn parse_file_body(body: &str) -> Option<(String, String, String)> {
    let trimmed = body.trim();
    if !trimmed.starts_with("[file:") || !trimmed.ends_with(']') {
        return None;
    }
    let inner = &trimmed[6..trimmed.len()-1];
    let colon1 = inner.find(':')?;
    let file_id = inner[..colon1].to_string();
    let rest = &inner[colon1+1..];
    let colon2 = rest.rfind(':')?;
    let filename = rest[..colon2].to_string();
    let size_str = rest[colon2+1..].to_string();
    Some((file_id, filename, size_str))
}

fn file_ext_label(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_uppercase();
    if ext.len() <= 5 && !ext.is_empty() { ext } else { "FILE".to_string() }
}

fn file_category(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "PDF Document",
        "doc" | "docx" => "Word Document",
        "xls" | "xlsx" => "Spreadsheet",
        "ppt" | "pptx" => "Presentation",
        "zip" | "rar" | "7z" | "tar" | "gz" => "Archive",
        "mp3" | "wav" | "ogg" | "flac" | "aac" => "Audio",
        "mp4" | "avi" | "mkv" | "mov" | "webm" => "Video",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" => "Image",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "java" | "go" => "Source Code",
        "txt" | "md" | "csv" | "json" | "xml" | "yaml" | "toml" => "Text File",
        "exe" | "msi" | "dmg" => "Program",
        _ => "File",
    }
}

pub fn view<'a>(body: &str, note_id: uuid::Uuid) -> Element<'a, Message> {
    let Some((file_id, filename, size_str)) = parse_file_body(body) else {
        return container(
            text("File data unavailable").size(13).style(|_t| theme::secondary_text())
        )
        .width(Length::Fill).height(Length::Fill)
        .center_x(Length::Fill).center_y(Length::Fill)
        .into();
    };

    let ext_label = file_ext_label(&filename);
    let category = file_category(&filename);

    let icon_box = container(
        svg(icons::file_large()).width(40).height(40)
    )
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_TERTIARY)),
        border: iced::Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    })
    .padding(16);

    let ext_pill = container(
        text(ext_label).size(10).style(|_t| theme::primary_text())
    )
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.18, 0.55, 0.31))),
        border: iced::Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    })
    .padding([1, 6]);

    let info_col = column![
        text(filename.clone()).size(15).style(|_t| theme::primary_text()),
        row![
            text(category).size(12).style(|_t| theme::secondary_text()),
            text(" \u{00B7} ").size(12).style(|_t| theme::secondary_text()),
            text(size_str).size(12).style(|_t| theme::secondary_text()),
        ],
    ].spacing(3);

    let header = row![
        column![icon_box, ext_pill].spacing(6).align_x(iced::Alignment::Center),
        info_col,
    ].spacing(16).align_y(iced::Alignment::Center);

    let divider = container(Space::new(Length::Fill, 1))
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::BG_TERTIARY)),
            ..Default::default()
        });

    let save_btn = button(
        row![
            svg(icons::save_icon()).width(14).height(14),
            text("Save").size(12).style(|_t| theme::primary_text())
        ].spacing(6).align_y(iced::Alignment::Center)
    )
    .on_press(Message::FileExport(file_id.clone(), filename.clone()))
    .style(theme::submit_button)
    .padding([8, 16]);

    let save_as_btn = button(
        row![
            svg(icons::save_icon()).width(14).height(14),
            text("Save As").size(12).style(|_t| theme::primary_text())
        ].spacing(6).align_y(iced::Alignment::Center)
    )
    .on_press(Message::FileExportAs(file_id.clone(), filename.clone()))
    .style(theme::secondary_button)
    .padding([8, 16]);

    let delete_btn = button(
        text("Delete").size(12).style(|_t| iced::widget::text::Style {
            color: Some(theme::DANGER)
        })
    )
    .on_press(Message::OpenDeleteNoteDialog(note_id))
    .style(|_t: &iced::Theme, status: iced::widget::button::Status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => iced::Color::from_rgba(0.9, 0.3, 0.3, 0.12),
            _ => theme::TRANSPARENT,
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border { radius: 6.0.into(), ..Default::default() },
            text_color: theme::DANGER,
            ..Default::default()
        }
    })
    .padding([8, 16]);

    let card = container(
        column![
            header,
            Space::with_height(16),
            divider,
            Space::with_height(16),
            row![save_btn, save_as_btn, Space::with_width(Length::Fill), delete_btn]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        ]
        .padding(24)
    )
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_SECONDARY)),
        border: iced::Border { radius: 12.0.into(), ..Default::default() },
        ..Default::default()
    })
    .max_width(420);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
