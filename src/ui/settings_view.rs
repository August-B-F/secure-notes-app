use iced::widget::{button, column, container, horizontal_space, row, scrollable, svg, text, text_input, Space};
use iced::{Element, Length};

use crate::app::{App, Message};
use crate::ui::{icons, theme};

pub fn view<'a>(app: &'a App) -> Element<'a, Message> {
    let is_maximized = app.is_maximized;
    let editor_style = if is_maximized {
        theme::editor_panel_square as fn(&iced::Theme) -> iced::widget::container::Style
    } else {
        theme::editor_panel_rounded
    };

    let header = row![
        svg(icons::settings_icon()).width(20).height(20),
        Space::with_width(8),
        text("Settings").size(18).style(|_t| theme::primary_text()),
    ]
    .align_y(iced::Alignment::Center);

    let perf_section = settings_section(
        "Performance",
        vec![
            option_row(
                "Framerate",
                "Controls UI refresh rate. Lower values use less CPU",
                &["15", "30", "60"],
                app.setting_framerate,
                |v| Message::SetFramerate(v.parse().unwrap()),
            ),
        ],
    );

    let editor_section = settings_section(
        "Editor",
        vec![
            toggle_row(
                "Auto-save",
                "Automatically save after a delay when you stop typing",
                app.setting_auto_save,
                Message::ToggleAutoSave,
            ),
            option_row(
                "Auto-save delay",
                "Seconds of inactivity before saving",
                &["1", "2", "3", "5", "10"],
                app.setting_auto_save_delay,
                |v| Message::SetAutoSaveDelay(v.parse().unwrap()),
            ),
            option_row(
                "Font size",
                "Text size in the note editor",
                &["12", "13", "14", "15", "16", "18", "20", "24"],
                app.setting_font_size,
                |v| Message::SetEditorFontSize(v.parse().unwrap()),
            ),
            container({
                let zoom_pct = format!("{}%", (app.gui_scale * 100.0) as u32);
                let left = column![
                    text("UI Zoom").size(13).style(|_t| theme::primary_text()),
                    text("Scale the entire interface. Ctrl+/- to adjust").size(10).style(|_t| theme::secondary_text()),
                ].spacing(2);
                let zoom_btns = row![
                    button(text("\u{2212}").size(13)).on_press(Message::ZoomOut).style(theme::secondary_button).padding([4, 10]),
                    text(zoom_pct).size(12).style(|_t| theme::primary_text()).width(iced::Length::Fixed(45.0)).align_x(iced::alignment::Horizontal::Center),
                    button(text("+").size(13)).on_press(Message::ZoomIn).style(theme::secondary_button).padding([4, 10]),
                    Space::with_width(4),
                    button(text("Reset").size(11)).on_press(Message::ZoomReset).style(theme::secondary_button).padding([4, 8]),
                ].spacing(4).align_y(iced::Alignment::Center);
                row![left, horizontal_space(), zoom_btns].align_y(iced::Alignment::Center)
            }).padding([12, 0]).into(),
        ],
    );

    let canvas_section = settings_section(
        "Canvas",
        vec![
            option_row(
                "Grid snap",
                "Snap nodes to grid when moving. 0 disables snapping",
                &["0", "10", "20", "40"],
                app.setting_grid_size,
                |v| Message::SetCanvasGridSize(v.parse().unwrap()),
            ),
        ],
    );

    let mut security_rows: Vec<Element<'a, Message>> = vec![
        info_row("Encryption", "AES-256-GCM", Some("Industry-standard authenticated encryption")),
        info_row("Key derivation", "Argon2id", Some("64 MB memory, 3 iterations")),
    ];

    // Change password: collapsible inline form
    let chevron = if app.show_change_password {
        icons::chevron_down()
    } else {
        icons::chevron_right()
    };
    let change_pw_header: Element<'a, Message> = container(
        button(
            row![
                svg(chevron).width(10).height(10),
                svg(icons::key_icon()).width(14).height(14),
                text("Change master password").size(13).style(|_t| theme::primary_text()),
            ].spacing(8).align_y(iced::Alignment::Center),
        )
        .on_press(Message::OpenChangeVaultPasswordDialog)
        .style(|_t: &iced::Theme, status| {
            let bg = if matches!(status, iced::widget::button::Status::Hovered) {
                iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04)
            } else {
                iced::Color::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .padding([8, 4])
        .width(Length::Fill),
    )
    .padding([6, 0])
    .width(Length::Fill)
    .into();
    security_rows.push(change_pw_header);

    if app.show_change_password {
        let mut form = column![
            column![
                text("Current password").size(10).style(|_t| theme::secondary_text()),
                text_input("", &app.vault_old_password)
                    .on_input(Message::VaultOldPasswordChanged)
                    .on_submit(Message::SubmitChangeVaultPassword)
                    .secure(true).style(theme::dialog_input).size(12).padding(7),
            ].spacing(3),
            column![
                text("New password").size(10).style(|_t| theme::secondary_text()),
                text_input("", &app.vault_new_password)
                    .on_input(Message::VaultNewPasswordChanged)
                    .secure(true).style(theme::dialog_input).size(12).padding(7),
            ].spacing(3),
            column![
                text("Confirm new password").size(10).style(|_t| theme::secondary_text()),
                text_input("", &app.vault_new_password_confirm)
                    .on_input(Message::VaultNewPasswordConfirmChanged)
                    .on_submit(Message::SubmitChangeVaultPassword)
                    .secure(true).style(theme::dialog_input).size(12).padding(7),
            ].spacing(3),
        ].spacing(6);

        if let Some(ref err) = app.auth_error {
            form = form.push(text(err.clone()).size(11).style(|_t| theme::danger_text()));
        }

        form = form.push(Space::with_height(4));
        form = form.push(
            row![
                horizontal_space(),
                button(text("Change password").size(11).style(|_t| iced::widget::text::Style { color: Some(iced::Color::WHITE) }))
                    .on_press(Message::SubmitChangeVaultPassword)
                    .style(|_t: &iced::Theme, _s| iced::widget::button::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(0.16, 0.40, 0.25))),
                        border: iced::Border { radius: 5.0.into(), ..Default::default() },
                        text_color: iced::Color::WHITE,
                        ..Default::default()
                    })
                    .padding([6, 14]),
            ]
        );
        form = form.push(Space::with_height(2));

        security_rows.push(
            container(form)
                .padding([10, 32])
                .width(Length::Fill)
                .into()
        );
    }

    security_rows.push(action_row(
        "Lock vault",
        "Lock the vault and require password to access",
        button(text("Lock").size(11).style(|_t| theme::primary_text()))
            .on_press(Message::LockVault)
            .style(theme::secondary_button)
            .padding([4, 12]),
    ));

    let security_section = settings_section("Security", security_rows);

    let data_section = settings_section(
        "Data",
        vec![
            info_row("Notes", &format!("{}", app.all_count), None),
            info_row("Folders", &format!("{}", app.folders.len()), None),
            info_row("Storage", "Local SQLite (WAL)", Some("Data stored locally in an encrypted database")),
        ],
    );

    let shortcuts_section = settings_section(
        "Keyboard shortcuts",
        vec![
            shortcut_row("New note", "Ctrl + N"),
            shortcut_row("New folder", "Ctrl + Shift + N"),
            shortcut_row("Save", "Ctrl + S"),
            shortcut_row("Bold", "Ctrl + B"),
            shortcut_row("Italic", "Ctrl + I"),
            shortcut_row("Cancel / Close", "Escape"),
            shortcut_row("New window", "Ctrl + Shift + W"),
            shortcut_row("Copy notes/folders", "Ctrl + C"),
            shortcut_row("Paste (duplicate)", "Ctrl + V"),
            shortcut_row("Open in new window", "Alt + Click"),
        ],
    );

    let about_section = settings_section(
        "About",
        vec![
            info_row("Version", "0.1.0", None),
            info_row("Framework", "iced 0.13", None),
            info_row("Language", "Rust", None),
        ],
    );

    let content = scrollable(
        column![
            header,
            Space::with_height(16),
            perf_section,
            Space::with_height(12),
            editor_section,
            Space::with_height(12),
            canvas_section,
            Space::with_height(12),
            security_section,
            Space::with_height(12),
            data_section,
            Space::with_height(12),
            shortcuts_section,
            Space::with_height(12),
            about_section,
            Space::with_height(24),
        ]
        .spacing(4)
        .padding([16, 24]),
    )
    .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
    .style(theme::dark_scrollable)
    .height(Length::Fill);

    container(content)
        .style(editor_style)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}


fn settings_section<'a>(title: &str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut col = column![
        text(title.to_owned())
            .size(11)
            .style(|_t| iced::widget::text::Style {
                color: Some(iced::Color::from_rgb(0.45, 0.45, 0.48)),
            }),
        Space::with_height(6),
    ]
    .spacing(0);

    for (i, row_elem) in rows.into_iter().enumerate() {
        if i > 0 {
            col = col.push(
                container(Space::new(Length::Fill, 1)).style(|_t: &iced::Theme| {
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
                        ..Default::default()
                    }
                }),
            );
        }
        col = col.push(row_elem);
    }

    container(col)
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::BG_SECONDARY)),
            border: iced::Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([12, 16])
        .width(Length::Fill)
        .into()
}


fn info_row<'a>(label: &str, value: &str, description: Option<&str>) -> Element<'a, Message> {
    let mut left = column![text(label.to_owned()).size(13).style(|_t| theme::primary_text())].spacing(2);
    if let Some(desc) = description {
        left = left.push(text(desc.to_owned()).size(10).style(|_t| theme::secondary_text()));
    }

    container(
        row![left, horizontal_space(), text(value.to_owned()).size(12).style(|_t| theme::secondary_text())]
            .align_y(iced::Alignment::Center),
    )
    .padding([10, 0])
    .width(Length::Fill)
    .into()
}

fn option_row<'a, F>(
    label: &str,
    description: &str,
    options: &[&str],
    current: u32,
    on_select: F,
) -> Element<'a, Message>
where
    F: Fn(&str) -> Message + 'a,
{
    let left = column![
        text(label.to_owned()).size(13).style(|_t| theme::primary_text()),
        text(description.to_owned()).size(10).style(|_t| theme::secondary_text()),
    ]
    .spacing(2);

    let mut btns = row![].spacing(4);
    for &opt in options {
        let is_active = opt.parse::<u32>().ok() == Some(current);
        let msg = on_select(opt);
        btns = btns.push(
            button(text(opt.to_owned()).size(11).style(move |_t| {
                if is_active {
                    iced::widget::text::Style { color: Some(iced::Color::WHITE) }
                } else {
                    theme::secondary_text()
                }
            }))
            .on_press(msg)
            .style(move |_t: &iced::Theme, status| {
                let bg = if is_active {
                    iced::Color::from_rgb(0.25, 0.55, 0.35)
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
            .padding([4, 10]),
        );
    }

    container(
        row![left, horizontal_space(), btns].align_y(iced::Alignment::Center),
    )
    .padding([10, 0])
    .width(Length::Fill)
    .into()
}

fn toggle_row<'a>(
    label: &str,
    description: &str,
    enabled: bool,
    msg: Message,
) -> Element<'a, Message> {
    let left = column![
        text(label.to_owned()).size(13).style(|_t| theme::primary_text()),
        text(description.to_owned()).size(10).style(|_t| theme::secondary_text()),
    ]
    .spacing(2);

    let toggle_text = if enabled { "On" } else { "Off" };
    let toggle = button(text(toggle_text).size(11).style(move |_t| {
        if enabled {
            iced::widget::text::Style { color: Some(iced::Color::WHITE) }
        } else {
            theme::secondary_text()
        }
    }))
    .on_press(msg)
    .style(move |_t: &iced::Theme, _status| {
        let bg = if enabled {
            iced::Color::from_rgb(0.25, 0.55, 0.35)
        } else {
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06)
        };
        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border { radius: 10.0.into(), ..Default::default() },
            text_color: iced::Color::WHITE,
            ..Default::default()
        }
    })
    .padding([4, 14]);

    container(
        row![left, horizontal_space(), toggle].align_y(iced::Alignment::Center),
    )
    .padding([10, 0])
    .width(Length::Fill)
    .into()
}

fn action_row<'a>(
    label: &str,
    description: &str,
    action: iced::widget::Button<'a, Message>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label.to_owned()).size(13).style(|_t| theme::primary_text()),
                text(description.to_owned()).size(10).style(|_t| theme::secondary_text()),
            ].spacing(2),
            horizontal_space(),
            action,
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([10, 0])
    .width(Length::Fill)
    .into()
}

fn shortcut_row<'a>(action: &str, shortcut: &str) -> Element<'a, Message> {
    container(
        row![
            text(action.to_owned()).size(13).style(|_t| theme::primary_text()),
            horizontal_space(),
            container(
                text(shortcut.to_owned())
                    .size(11)
                    .style(|_t| iced::widget::text::Style {
                        color: Some(iced::Color::from_rgb(0.55, 0.55, 0.58)),
                    }),
            )
            .style(|_t: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([3, 8]),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .into()
}
