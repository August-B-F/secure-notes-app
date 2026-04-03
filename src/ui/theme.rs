use iced::widget::{button, container, scrollable, svg, text, text_editor, text_input};
use iced::{Border, Color, Theme};

pub const BG_PRIMARY: Color = Color::from_rgb(0x1F as f32 / 255.0, 0x1F as f32 / 255.0, 0x1F as f32 / 255.0); // #1F1F1F
pub const BG_SECONDARY: Color = Color::from_rgb(0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0x28 as f32 / 255.0); // #282828
pub const BG_TERTIARY: Color = Color::from_rgb(0x32 as f32 / 255.0, 0x32 as f32 / 255.0, 0x32 as f32 / 255.0); // #323232
pub const BG_HOVER: Color = Color::from_rgb(0x3A as f32 / 255.0, 0x3A as f32 / 255.0, 0x3A as f32 / 255.0); // #3A3A3A
pub const BG_SELECTED: Color = Color::from_rgb(0x40 as f32 / 255.0, 0x40 as f32 / 255.0, 0x40 as f32 / 255.0); // #404040
pub const TEXT_PRIMARY: Color = Color::from_rgb(0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0, 0xD9 as f32 / 255.0); // #D9D9D9
pub const TEXT_SECONDARY: Color = Color::from_rgb(0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0); // #8D8D8D
pub const DANGER: Color = Color::from_rgb(0xE5 as f32 / 255.0, 0x4D as f32 / 255.0, 0x4D as f32 / 255.0);
pub const TRANSPARENT: Color = Color::TRANSPARENT;

pub fn window_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
        },
        ..Default::default()
    }
}

pub fn window_container_maximized(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn tags_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        ..Default::default()
    }
}

pub fn tags_panel_rounded(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        border: Border {
            radius: iced::border::bottom_left(10.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn tags_panel_square(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        ..Default::default()
    }
}

pub fn notes_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_SECONDARY)),
        border: Border {
            width: 0.0,
            color: TRANSPARENT,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn editor_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        ..Default::default()
    }
}

pub fn editor_panel_rounded(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        border: Border {
            radius: iced::border::bottom_right(10.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn editor_panel_square(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        ..Default::default()
    }
}

pub fn toolbar_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_SECONDARY)),
        border: Border {
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn dialog_overlay(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
        ..Default::default()
    }
}

pub fn dialog_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_SECONDARY)),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn context_menu_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_SECONDARY)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgb(0.25, 0.25, 0.25),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: iced::Vector::new(2.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn note_card(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(iced::Background::Color(if selected {
            BG_SELECTED
        } else {
            BG_SECONDARY
        })),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn color_dot(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(iced::Background::Color(color)),
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[allow(dead_code)]
pub fn separator(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_TERTIARY)),
        ..Default::default()
    }
}

pub fn tag_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    tag_button_ctx(active, false)
}

pub fn tag_button_ctx(active: bool, ctx_target: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => BG_SELECTED,
            _ if active => BG_SELECTED,
            _ if ctx_target => BG_HOVER,
            _ => TRANSPARENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border {
                radius: 6.0.into(),
                width: if active { 1.0 } else { 0.0 },
                color: if active { Color::from_rgb(0.18, 0.55, 0.31) } else { TRANSPARENT },
            },
            ..Default::default()
        }
    }
}

#[allow(dead_code)]
pub fn note_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    note_button_ctx(selected, false)
}

#[allow(dead_code)]
pub fn note_button_ctx(selected: bool, ctx_target: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered if !selected => BG_HOVER,
            _ if selected => BG_TERTIARY,
            _ if ctx_target => BG_HOVER,
            _ => TRANSPARENT,
        };
        let left_border = if selected {
            Color::from_rgb(0.18, 0.55, 0.31)
        } else {
            TRANSPARENT
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                radius: 8.0.into(),
                width: if selected { 1.0 } else { 0.0 },
                color: left_border,
            },
            ..Default::default()
        }
    }
}

pub fn note_container(selected: bool, ctx_target: bool) -> impl Fn(&Theme) -> container::Style {
    note_container_hover(selected, ctx_target, false)
}

pub fn note_container_hover(selected: bool, ctx_target: bool, hovered: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        let bg = if selected {
            BG_TERTIARY
        } else if ctx_target {
            BG_HOVER
        } else if hovered {
            Color::from_rgba(1.0, 1.0, 1.0, 0.015)
        } else {
            TRANSPARENT
        };
        container::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border {
                radius: 8.0.into(),
                width: if selected { 1.0 } else { 0.0 },
                color: if selected { Color::from_rgb(0.18, 0.55, 0.31) } else { TRANSPARENT },
            },
            ..Default::default()
        }
    }
}

pub fn context_menu_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => TRANSPARENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn context_menu_danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => TRANSPARENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: DANGER,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => TRANSPARENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn window_control_button(
    hover_color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let (bg, text) = match status {
            button::Status::Hovered | button::Status::Pressed => {
                (hover_color, Color::from_rgb(0.15, 0.15, 0.15))
            }
            _ => (TRANSPARENT, TEXT_SECONDARY),
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: text,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn transparent_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(TRANSPARENT)),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn svg_hover_color(
    hover_color: Color,
) -> impl Fn(&Theme, svg::Status) -> svg::Style {
    move |_theme: &Theme, status: svg::Status| {
        match status {
            svg::Status::Hovered => svg::Style { color: Some(hover_color) },
            svg::Status::Idle => svg::Style { color: Some(TEXT_SECONDARY) },
        }
    }
}

pub fn submit_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => BG_TERTIARY,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(1.0, 0.35, 0.35),
        _ => DANGER,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => BG_TERTIARY,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn color_dot_button(
    color: Color,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => {
                Color::from_rgba(
                    (color.r * 1.15).min(1.0),
                    (color.g * 1.15).min(1.0),
                    (color.b * 1.15).min(1.0),
                    color.a,
                )
            }
            _ => color,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            border: Border {
                radius: 10.0.into(),
                width: if selected { 2.0 } else { 0.0 },
                color: Color::WHITE,
            },
            ..Default::default()
        }
    }
}

pub fn search_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(BG_TERTIARY),
        border: Border {
            radius: 8.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

#[allow(dead_code)]
pub fn title_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(TRANSPARENT),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

pub fn dialog_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(BG_PRIMARY),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: BG_TERTIARY,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

/// Transparent text input inside the search container (no border/bg — container provides those)
pub fn search_field_transparent(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(TRANSPARENT),
        border: Border { radius: 0.0.into(), width: 0.0, color: TRANSPARENT },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

/// Container wrapping the search input field
pub fn search_input_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_SECONDARY)),
        border: Border { radius: 4.0.into(), width: 1.0, color: BG_TERTIARY },
        ..Default::default()
    }
}

/// Toggle button for search options (Aa case sensitive) — highlighted when active
pub fn search_toggle_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, _status: button::Status| {
        let bg = if active { BG_SELECTED } else { TRANSPARENT };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: TEXT_SECONDARY,
            border: Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub fn search_nav_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(TRANSPARENT)),
        text_color: TEXT_SECONDARY,
        border: Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Container styled like a dialog_input — used to wrap a transparent text_input + inline buttons.
pub fn dialog_input_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BG_PRIMARY)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: BG_TERTIARY,
        },
        ..Default::default()
    }
}

pub fn inline_rename_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(TRANSPARENT),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

#[allow(dead_code)]
pub fn line_editor_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(TRANSPARENT),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: Color::from_rgb(0.68, 0.68, 0.70),
        selection: BG_SELECTED,
    }
}

/// Live preview editor: transparent when unfocused (rendered view shows through),
/// opaque with solid background when focused (covers rendered view, shows raw markdown).
#[allow(dead_code)]
pub fn live_preview_editor(_theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused);
    text_editor::Style {
        background: if focused {
            iced::Background::Color(BG_PRIMARY)
        } else {
            iced::Background::Color(TRANSPARENT)
        },
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: if focused { TEXT_SECONDARY } else { TRANSPARENT },
        placeholder: if focused { TEXT_SECONDARY } else { TRANSPARENT },
        value: if focused { TEXT_PRIMARY } else { TRANSPARENT },
        selection: if focused { BG_SELECTED } else { TRANSPARENT },
    }
}

pub fn body_editor(_theme: &Theme, _status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: iced::Background::Color(TRANSPARENT),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_PRIMARY,
        selection: BG_SELECTED,
    }
}

pub fn thin_scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().width(3).scroller_width(3).margin(1)
}

pub fn dark_scrollable(_theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                color: BG_HOVER,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                color: BG_HOVER,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
            },
        },
        gap: None,
    }
}

pub fn toolbar_action(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme: &Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            _ if active => BG_TERTIARY,
            _ => TRANSPARENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[allow(dead_code)]
pub fn new_note_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        _ => BG_TERTIARY,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn primary_text() -> text::Style {
    text::Style {
        color: Some(TEXT_PRIMARY),
    }
}

pub fn secondary_text() -> text::Style {
    text::Style {
        color: Some(TEXT_SECONDARY),
    }
}

pub fn danger_text() -> text::Style {
    text::Style {
        color: Some(DANGER),
    }
}
