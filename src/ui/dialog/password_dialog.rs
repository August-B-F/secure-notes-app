use iced::alignment::Horizontal;
use iced::widget::{button, column, container, mouse_area, row, svg, text, text_input, Space};
use iced::{Element, Length};

use crate::app::Message;
use crate::ui::{icons, theme};

/// Window buttons for auth screens (always visible).
fn window_buttons<'a>(controls_hovered: bool, is_maximized: bool) -> Element<'a, Message> {
    let h = controls_hovered;
    let min_content: Element<Message> = if h { svg(icons::win_minimize()).width(8).height(8).into() } else { Space::new(12, 12).into() };
    let max_content: Element<Message> = if h { svg(icons::win_maximize()).width(8).height(8).into() } else { Space::new(12, 12).into() };
    let close_content: Element<Message> = if h { svg(icons::win_close()).width(8).height(8).into() } else { Space::new(12, 12).into() };
    let p = if h { 2 } else { 0 };
    let controls: Element<Message> = mouse_area(
        row![
            button(min_content).on_press(Message::WindowMinimize)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0xE5, 0xD5, 0x4D), false)).padding(p),
            button(max_content).on_press(Message::WindowMaximize)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0x4D, 0xC8, 0x6A), false)).padding(p),
            button(close_content).on_press(Message::WindowClose)
                .style(theme::color_dot_button(iced::Color::from_rgb8(0xE5, 0x4D, 0x4D), false)).padding(p),
        ].spacing(8).align_y(iced::Alignment::Center)
    )
    .on_enter(Message::WindowControlsHover(true))
    .on_exit(Message::WindowControlsHover(false))
    .into();

    mouse_area(
        container(
            row![
                iced::widget::image(iced::widget::image::Handle::from_path("assets/logo.png")).width(16).height(16),
                Space::with_width(Length::Fill),
                // Invisible placeholder matching the lock button size in the unlocked view
                container(Space::new(16, 16)).padding([4, 6]),
                Space::with_width(8),
                controls,
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([8, 10]),
        )
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SECONDARY)),
            border: iced::Border {
                radius: if is_maximized { 0.0.into() } else { iced::border::top(10.0) },
                ..Default::default()
            },
            ..Default::default()
        })
        .width(Length::Fill),
    )
    .on_press(Message::WindowDrag)
    .into()
}

/// Setup screen — first time creating a password.
pub fn view_setup<'a>(password: &str, confirm_password: &str, error: Option<&str>, controls_hovered: bool, is_maximized: bool) -> Element<'a, Message> {
    let mut form = column![
        Space::with_height(Length::Fill),
        svg(icons::key_icon()).width(36).height(36),
        Space::with_height(8),
        text("Create a password").size(20).style(|_t| theme::primary_text()).align_x(Horizontal::Center),
        text("This password will protect your notes").size(12).style(|_t| theme::secondary_text()).align_x(Horizontal::Center),
        Space::with_height(10),
        text_input("Password", password).on_input(Message::PasswordInputChanged).on_submit(Message::SubmitSetup).secure(true).style(theme::dialog_input).size(14).padding(10),
        text_input("Confirm password", confirm_password).on_input(Message::ConfirmPasswordInputChanged).on_submit(Message::SubmitSetup).secure(true).style(theme::dialog_input).size(14).padding(10),
    ].spacing(8).align_x(Horizontal::Center).max_width(270);

    if let Some(err) = error {
        form = form.push(text(err.to_owned()).size(12).style(|_t| theme::danger_text()));
    }
    form = form.push(Space::with_height(6));
    form = form.push(button(text("Create").size(14).align_x(Horizontal::Center).width(Length::Fill)).on_press(Message::SubmitSetup).style(theme::submit_button).padding([10, 20]).width(Length::Fill));
    form = form.push(Space::with_height(Length::Fill));

    let bottom_style = move |_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_PRIMARY)),
        border: iced::Border { radius: if is_maximized { 0.0.into() } else { iced::border::bottom(10.0) }, ..Default::default() },
        ..Default::default()
    };
    container(column![
        window_buttons(controls_hovered, is_maximized),
        container(form).style(bottom_style).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).padding([0, 40]),
    ]).width(Length::Fill).height(Length::Fill).into()
}

/// Login screen — enter password to unlock.
pub fn view_login<'a>(password: &str, error: Option<&str>, controls_hovered: bool, is_maximized: bool, show_password: bool) -> Element<'a, Message> {
    let eye_icon = if show_password { icons::eye_open() } else { icons::eye_closed() };
    let password_field = iced::widget::stack![
        text_input("Password", password)
            .on_input(Message::PasswordInputChanged)
            .on_submit(Message::SubmitLogin)
            .secure(!show_password)
            .style(theme::dialog_input)
            .size(14)
            .padding(iced::Padding::new(10.0).right(40.0)),
        container(
            row![
                button(svg(eye_icon).width(16).height(16))
                    .on_press(Message::TogglePasswordVisibility)
                    .style(theme::icon_button)
                    .padding([6, 8]),
                Space::with_width(4),
            ].align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .align_y(iced::alignment::Vertical::Center),
    ];
    let mut form = column![
        Space::with_height(Length::Fill),
        svg(icons::lock_closed()).width(36).height(36),
        Space::with_height(8),
        text("Welcome back").size(20).style(|_t| theme::primary_text()).align_x(Horizontal::Center),
        text("Enter your password to continue").size(12).style(|_t| theme::secondary_text()).align_x(Horizontal::Center),
        Space::with_height(10),
        password_field,
    ].spacing(8).align_x(Horizontal::Center).max_width(300);

    if let Some(err) = error {
        form = form.push(text(err.to_owned()).size(12).style(|_t| theme::danger_text()));
    }
    form = form.push(Space::with_height(6));
    form = form.push(button(text("Unlock").size(14).align_x(Horizontal::Center).width(Length::Fill)).on_press(Message::SubmitLogin).style(theme::submit_button).padding([10, 20]).width(Length::Fill));
    form = form.push(Space::with_height(Length::Fill));

    let bottom_style = move |_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_PRIMARY)),
        border: iced::Border { radius: if is_maximized { 0.0.into() } else { iced::border::bottom(10.0) }, ..Default::default() },
        ..Default::default()
    };
    container(column![
        window_buttons(controls_hovered, is_maximized),
        container(form).style(bottom_style).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).padding([0, 40]),
    ]).width(Length::Fill).height(Length::Fill).into()
}

/// Two password fields + strength indicator. Used for encrypt and change password.
pub fn view_encrypt<'a>(note_id: uuid::Uuid, password: &str, confirm: &str, error: Option<&str>) -> Element<'a, Message> {
    view_password_form(note_id, password, confirm, error, "Encrypt Note", Message::SubmitEncrypt(note_id))
}

pub fn view_change_password<'a>(note_id: uuid::Uuid, password: &str, confirm: &str, error: Option<&str>) -> Element<'a, Message> {
    view_password_form(note_id, password, confirm, error, "Change Password", Message::SubmitChangePassword(note_id))
}

fn view_password_form<'a>(_note_id: uuid::Uuid, password: &str, confirm: &str, error: Option<&str>, title: &str, confirm_msg: Message) -> Element<'a, Message> {
    let strength = password_strength(password);
    let strength_color = match strength {
        0..=1 => iced::Color::from_rgb(0.9, 0.3, 0.3),
        2 => iced::Color::from_rgb(0.9, 0.7, 0.2),
        _ => iced::Color::from_rgb(0.3, 0.8, 0.4),
    };
    let strength_label = match strength {
        0 => "",
        1 => "Weak",
        2 => "Fair",
        3 => "Good",
        _ => "Strong",
    };
    let strength_bar = container(Space::new(Length::Fill, 3))
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(strength_color)),
            border: iced::Border { radius: 2.0.into(), ..Default::default() },
            ..Default::default()
        })
        .width(Length::FillPortion((strength.max(1) * 25) as u16));

    let mut content = column![
        svg(icons::lock_closed()).width(32).height(32),
        text(title.to_owned()).size(18).style(|_t| theme::primary_text()),
        Space::with_height(4),
        text("Password").size(11).style(|_t| theme::secondary_text()),
        text_input("Enter password...", password)
            .on_input(Message::NotePasswordInputChanged)
            .secure(true).style(theme::dialog_input).size(13).padding(8),
        row![strength_bar, Space::with_width(Length::Fill), text(strength_label).size(10).style(move |_t| iced::widget::text::Style { color: Some(strength_color) })].spacing(8).align_y(iced::Alignment::Center),
        Space::with_height(2),
        text("Confirm password").size(11).style(|_t| theme::secondary_text()),
        text_input("Repeat password...", confirm)
            .on_input(Message::NotePasswordConfirmChanged)
            .on_submit(confirm_msg.clone())
            .secure(true).style(theme::dialog_input).size(13).padding(8),
    ].spacing(4).align_x(Horizontal::Center);

    if let Some(err) = error {
        content = content.push(text(err.to_owned()).size(11).style(|_t| theme::danger_text()));
    }

    content = content.push(Space::with_height(6));
    content = content.push(row![
        button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
        button(text("Confirm").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(confirm_msg).style(theme::submit_button).padding([8, 16]).width(Length::Fill),
    ].spacing(8));

    let card = container(content.max_width(300).padding(20)).style(theme::dialog_card).width(320);
    container(card).style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill).into()
}

/// Decrypt dialog — single password field.
pub fn view_decrypt<'a>(note_id: uuid::Uuid, password: &str, error: Option<&str>) -> Element<'a, Message> {
    let mut content = column![
        svg(icons::lock_active()).width(32).height(32),
        text("Unlock Note").size(18).style(|_t| theme::primary_text()),
        text("Enter the password to view this note").size(12).style(|_t| theme::secondary_text()),
        Space::with_height(4),
        text_input("Password...", password)
            .on_input(Message::NotePasswordInputChanged)
            .on_submit(Message::SubmitDecrypt(note_id))
            .secure(true).style(theme::dialog_input).size(13).padding(10),
    ].spacing(8).align_x(Horizontal::Center);

    if let Some(err) = error {
        content = content.push(text(err.to_owned()).size(11).style(|_t| theme::danger_text()));
    }

    content = content.push(Space::with_height(4));
    content = content.push(row![
        button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::CloseDialog).style(theme::secondary_button).padding([8, 16]).width(Length::Fill),
        button(text("Unlock").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::SubmitDecrypt(note_id)).style(theme::submit_button).padding([8, 16]).width(Length::Fill),
    ].spacing(8));

    let card = container(content.max_width(300).padding(20)).style(theme::dialog_card).width(320);
    container(card).style(theme::dialog_overlay).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill).into()
}

/// Change vault master password dialog.
#[allow(dead_code)]
pub fn view_change_vault_password<'a>(
    old_password: &str,
    new_password: &str,
    confirm_password: &str,
    error: Option<&str>,
) -> Element<'a, Message> {
    let strength = password_strength(new_password);
    let strength_color = match strength {
        0..=1 => iced::Color::from_rgb(0.9, 0.3, 0.3),
        2 => iced::Color::from_rgb(0.9, 0.7, 0.2),
        3 => iced::Color::from_rgb(0.3, 0.8, 0.4),
        _ => iced::Color::from_rgb(0.3, 0.8, 0.4),
    };
    let strength_label = match strength {
        0 => "",
        1 => "Weak",
        2 => "Fair",
        3 => "Good",
        _ => "Strong",
    };

    let strength_bar = row![
        container(Space::new(Length::Fill, 3))
            .style(move |_t: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(strength_color)),
                border: iced::Border { radius: 2.0.into(), ..Default::default() },
                ..Default::default()
            })
            .width(Length::FillPortion(strength.max(1) as u16 * 25)),
        Space::with_width(Length::Fill),
        text(strength_label).size(10).style(move |_t| iced::widget::text::Style { color: Some(strength_color) }),
    ].spacing(8).align_y(iced::Alignment::Center);

    let mut content = column![
        svg(icons::key_icon()).width(28).height(28),
        Space::with_height(4),
        text("Change Master Password").size(16).style(|_t| theme::primary_text()),
        text("Enter your current password and choose a new one").size(11).style(|_t| theme::secondary_text()),
        Space::with_height(8),

        text("Current password").size(11).style(|_t| theme::secondary_text()),
        text_input("Current password...", old_password)
            .on_input(Message::VaultOldPasswordChanged)
            .on_submit(Message::SubmitChangeVaultPassword)
            .secure(true).style(theme::dialog_input).size(13).padding(10),

        Space::with_height(8),
        text("New password").size(11).style(|_t| theme::secondary_text()),
        text_input("New password...", new_password)
            .on_input(Message::VaultNewPasswordChanged)
            .secure(true).style(theme::dialog_input).size(13).padding(10),
        strength_bar,

        Space::with_height(4),
        text("Confirm new password").size(11).style(|_t| theme::secondary_text()),
        text_input("Confirm new password...", confirm_password)
            .on_input(Message::VaultNewPasswordConfirmChanged)
            .on_submit(Message::SubmitChangeVaultPassword)
            .secure(true).style(theme::dialog_input).size(13).padding(10),
    ].spacing(4).align_x(Horizontal::Center);

    if let Some(err) = error {
        content = content.push(Space::with_height(2));
        content = content.push(text(err.to_owned()).size(11).style(|_t| theme::danger_text()));
    }

    content = content.push(Space::with_height(8));
    content = content.push(row![
        button(text("Cancel").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::CloseDialog).style(theme::secondary_button).padding([10, 16]).width(Length::Fill),
        button(text("Change").size(13).align_x(Horizontal::Center).width(Length::Fill))
            .on_press(Message::SubmitChangeVaultPassword).style(theme::submit_button).padding([10, 16]).width(Length::Fill),
    ].spacing(8));

    let card = container(content.max_width(320).padding([24, 24]))
        .style(theme::dialog_card)
        .width(360);
    container(card)
        .style(theme::dialog_overlay)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn password_strength(pw: &str) -> u32 {
    if pw.is_empty() { return 0; }
    let mut score = 0u32;
    if pw.len() >= 6 { score += 1; }
    if pw.len() >= 10 { score += 1; }
    if pw.chars().any(|c| c.is_uppercase()) && pw.chars().any(|c| c.is_lowercase()) { score += 1; }
    if pw.chars().any(|c| c.is_numeric()) { score += 1; }
    score.min(4)
}
