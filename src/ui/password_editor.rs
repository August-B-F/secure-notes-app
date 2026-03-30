use iced::widget::{button, column, container, horizontal_space, row, svg, text, text_editor, text_input, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::models::note::PasswordGenOptions;
use crate::models::PasswordData;
use crate::ui::{icons, theme};

pub fn view<'a>(
    data: &PasswordData,
    notes_content: &'a text_editor::Content,
    show_password: bool,
    show_gen_panel: bool,
    gen_options: &PasswordGenOptions,
    copied_field: Option<&str>,
) -> Element<'a, Message> {
    let mut content = column![].spacing(12).padding([16, 24]);

    content = content.push(copyable_field("Website", "website", &data.website, "https://...", false, Message::PasswordWebsiteChanged, copied_field));

    content = content.push(copyable_field("Email", "email", &data.email, "email@example.com", false, Message::PasswordEmailChanged, copied_field));

    content = content.push(copyable_field("Username", "username", &data.username, "username", false, Message::PasswordUsernameChanged, copied_field));

    let pw_input = text_input("", &data.password)
        .on_input(Message::PasswordValueChanged)
        .secure(!show_password)
        .style(theme_input_inner)
        .size(14)
        .padding([10, 10]);

    let is_pw_copied = copied_field == Some("password");
    let copy_icon = if is_pw_copied { icons::copy_check() } else { icons::copy_icon() };

    let pw_field_inner = row![
        pw_input,
        ghost_btn(svg(copy_icon).width(14).height(14).into(), Message::CopyField("password".into(), data.password.clone())),
        ghost_btn(svg(if show_password { icons::eye_closed() } else { icons::eye_open() }).width(14).height(14).into(), Message::TogglePasswordVisibility),
        ghost_btn(svg(icons::dice_icon()).width(14).height(14).into(), Message::TogglePasswordGenPanel),
    ].spacing(2).align_y(iced::Alignment::Center);

    let pw_container = container(pw_field_inner)
        .style(theme::dialog_input_container)
        .padding([0, 6])
        .width(Length::Fill);

    let strength = password_strength(&data.password);
    let strength_color = match strength {
        0..=1 => iced::Color::from_rgb(0.9, 0.3, 0.3),
        2 => iced::Color::from_rgb(0.9, 0.7, 0.2),
        3 => iced::Color::from_rgb(0.5, 0.8, 0.4),
        _ => iced::Color::from_rgb(0.3, 0.85, 0.45),
    };
    let strength_label = match strength {
        0 => "",
        1 => "Weak",
        2 => "Fair",
        3 => "Good",
        _ => "Strong",
    };
    let strength_bar = container(Space::new(Length::Fill, 3))
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(strength_color)),
            border: iced::Border { radius: 2.0.into(), ..Default::default() },
            ..Default::default()
        })
        .width(Length::FillPortion(strength.max(1) as u16 * 20));
    let strength_row = row![
        strength_bar,
        Space::with_width(Length::Fill),
        text(strength_label).size(10).style(move |_t| iced::widget::text::Style { color: Some(strength_color) }),
    ].spacing(8).align_y(iced::Alignment::Center);

    content = content.push(column![
        text("Password").size(12).style(|_t| theme::secondary_text()),
        pw_container,
        strength_row,
    ].spacing(4));

    if show_gen_panel {
        let len = gen_options.length;
        let len_row = row![
            text("Length").size(12).style(|_t| theme::secondary_text()),
            horizontal_space(),
            len_btn(8, len), len_btn(12, len), len_btn(16, len), len_btn(20, len),
            len_btn(24, len), len_btn(32, len), len_btn(48, len),
        ].spacing(3).align_y(iced::Alignment::Center);

        let opts_row = row![
            toggle_chip("ABC", gen_options.uppercase, Message::PasswordGenToggleUpper),
            toggle_chip("abc", gen_options.lowercase, Message::PasswordGenToggleLower),
            toggle_chip("123", gen_options.numbers, Message::PasswordGenToggleNumbers),
            toggle_chip("!@#", gen_options.symbols, Message::PasswordGenToggleSymbols),
            horizontal_space(),
            button(
                row![
                    svg(icons::dice_icon_white()).width(14).height(14),
                    text("Generate").size(12).style(|_t| iced::widget::text::Style { color: Some(iced::Color::WHITE) }),
                ].spacing(6).align_y(iced::Alignment::Center)
            )
            .on_press(Message::GeneratePassword)
            .style(|_t: &iced::Theme, _s| iced::widget::button::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(0.22, 0.52, 0.32))),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                text_color: iced::Color::WHITE,
                ..Default::default()
            })
            .padding([6, 16]),
        ].spacing(6).align_y(iced::Alignment::Center);

        content = content.push(column![len_row, opts_row].spacing(8));
    }

    content = content.push(column![
        text("Notes").size(12).style(|_t| theme::secondary_text()),
        container(
            text_editor(notes_content)
                .on_action(Message::PasswordNotesAction)
                .placeholder("Additional notes...")
                .style(theme::body_editor)
                .size(14)
                .padding([8, 10])
                .height(120),
        )
        .style(theme::dialog_input_container)
        .width(Length::Fill),
    ].spacing(4));

    if !data.custom_fields.is_empty() {
        let mut fields_col = column![
            text("Custom Fields").size(12).style(|_t| theme::secondary_text()),
        ].spacing(6);

        for (i, field) in data.custom_fields.iter().enumerate() {
            let label_input = text_input("Label", &field.label)
                .on_input(move |v| Message::CustomFieldLabelChanged(i, v))
                .style(theme::dialog_input)
                .size(13)
                .padding(8)
                .width(Length::FillPortion(3));

            let field_name = format!("custom_{}", i);
            let is_cf_copied = copied_field == Some(field_name.as_str());
            let cf_copy_icon = if is_cf_copied { icons::copy_check() } else { icons::copy_icon() };
            let hide_icon = if field.hidden { icons::eye_open() } else { icons::eye_closed() };

            let value_inner = row![
                text_input("Value", &field.value)
                    .on_input(move |v| Message::CustomFieldValueChanged(i, v))
                    .secure(field.hidden)
                    .style(theme_input_inner)
                    .size(13)
                    .padding([8, 8]),
                ghost_btn(svg(cf_copy_icon).width(12).height(12).into(), Message::CopyField(field_name, field.value.clone())),
                ghost_btn(svg(hide_icon).width(12).height(12).into(), Message::ToggleCustomFieldHidden(i)),
            ].spacing(2).align_y(iced::Alignment::Center);

            let value_container = container(value_inner)
                .style(theme::dialog_input_container)
                .padding([0, 4])
                .width(Length::FillPortion(5));

            let del_btn = button(text("\u{00d7}").size(14).style(|_t| iced::widget::text::Style {
                color: Some(iced::Color::from_rgb(0.8, 0.3, 0.3)),
            }))
            .on_press(Message::RemoveCustomField(i))
            .style(theme::icon_button)
            .padding([4, 8]);

            fields_col = fields_col.push(
                row![label_input, value_container, del_btn]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
            );
        }
        content = content.push(fields_col);
    }

    content = content.push(
        button(
            row![
                text("+").size(14).style(|_t| theme::primary_text()),
                text("Add custom field").size(12).style(|_t| theme::secondary_text()),
            ].spacing(6).align_y(iced::Alignment::Center)
        )
        .on_press(Message::AddCustomField)
        .style(|_t: &iced::Theme, status| {
            let bg = if matches!(status, iced::widget::button::Status::Hovered) {
                iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06)
            } else {
                iced::Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                },
                text_color: iced::Color::WHITE,
                ..Default::default()
            }
        })
        .padding([8, 12])
        .width(Length::Fill),
    );

    content.into()
}


/// A ghost button — no background, just the icon. Hover shows subtle highlight.
fn ghost_btn<'a>(content: Element<'a, Message>, msg: Message) -> iced::widget::Button<'a, Message> {
    button(content)
        .on_press(msg)
        .style(|_t: &iced::Theme, status| {
            let bg = if matches!(status, iced::widget::button::Status::Hovered) {
                iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                iced::Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                text_color: iced::Color::WHITE,
                ..Default::default()
            }
        })
        .padding([6, 6])
}

/// A labeled text field with a copy button inside the input container.
fn copyable_field<'a>(
    label: &str,
    field_name: &str,
    value: &str,
    placeholder: &str,
    secure: bool,
    on_input: fn(String) -> Message,
    copied_field: Option<&str>,
) -> Element<'a, Message> {
    let is_copied = copied_field == Some(field_name);
    let copy_icon = if is_copied { icons::copy_check() } else { icons::copy_icon() };

    let input = text_input(placeholder, value)
        .on_input(on_input)
        .secure(secure)
        .style(theme_input_inner)
        .size(14)
        .padding([10, 10]);

    let inner = row![
        input,
        ghost_btn(svg(copy_icon).width(14).height(14).into(), Message::CopyField(field_name.to_owned(), value.to_owned())),
    ].spacing(2).align_y(iced::Alignment::Center);

    column![
        text(label.to_owned()).size(12).style(|_t| theme::secondary_text()),
        container(inner)
            .style(theme::dialog_input_container)
            .padding([0, 6])
            .width(Length::Fill),
    ]
    .spacing(4)
    .into()
}

/// Transparent text_input style (used inside a styled container).
fn theme_input_inner(_theme: &iced::Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(iced::Color::TRANSPARENT),
        border: iced::Border { width: 0.0, radius: 0.0.into(), color: iced::Color::TRANSPARENT },
        icon: iced::Color::from_rgb(0.5, 0.5, 0.5),
        placeholder: iced::Color::from_rgb(0.4, 0.4, 0.42),
        value: iced::Color::from_rgb(0.88, 0.88, 0.9),
        selection: iced::Color::from_rgba(0.3, 0.6, 0.9, 0.35),
    }
}

fn len_btn(n: u32, current: u32) -> iced::widget::Button<'static, Message> {
    let active = n == current;
    button(text(n.to_string()).size(11).style(move |_t| {
        if active {
            iced::widget::text::Style { color: Some(iced::Color::WHITE) }
        } else {
            theme::secondary_text()
        }
    }))
    .on_press(Message::PasswordGenLength(n))
    .style(move |_t: &iced::Theme, status| {
        let bg = if active {
            iced::Color::from_rgb(0.22, 0.52, 0.32)
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        } else {
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04)
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border { radius: 4.0.into(), ..Default::default() },
            text_color: iced::Color::WHITE,
            ..Default::default()
        }
    })
    .padding([4, 8])
}

fn toggle_chip<'a>(label: &str, enabled: bool, msg: Message) -> Element<'a, Message> {
    button(text(label.to_owned()).size(11).style(move |_t| {
        if enabled {
            iced::widget::text::Style { color: Some(iced::Color::WHITE) }
        } else {
            theme::secondary_text()
        }
    }))
    .on_press(msg)
    .style(move |_t: &iced::Theme, _s| {
        let bg = if enabled {
            iced::Color::from_rgb(0.25, 0.55, 0.35)
        } else {
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04)
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border { radius: 10.0.into(), ..Default::default() },
            text_color: iced::Color::WHITE,
            ..Default::default()
        }
    })
    .padding([4, 10])
    .into()
}

fn password_strength(pw: &str) -> u32 {
    if pw.is_empty() { return 0; }
    let mut score = 0u32;
    if pw.len() >= 6 { score += 1; }
    if pw.len() >= 12 { score += 1; }
    if pw.chars().any(|c| c.is_uppercase()) && pw.chars().any(|c| c.is_lowercase()) { score += 1; }
    if pw.chars().any(|c| c.is_numeric()) { score += 1; }
    if pw.chars().any(|c| "!@#$%^&*-_+=?".contains(c)) { score += 1; }
    score.min(5)
}
