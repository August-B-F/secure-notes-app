use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, stack, svg, text, text_editor,
    text_input, tooltip, Space,
};
use iced::{Element, Length};

use crate::app::Message;
use crate::models::{Note, NoteType, PasswordData};
use crate::ui::{canvas_editor::{CanvasEditor, CanvasCtxTarget}, color_picker, icons, password_editor, theme};
use iced::widget::mouse_area;

pub fn view<'a>(
    note: &'a Note,
    _editor_title: &str,
    _editor_content: &'a text_editor::Content,
    is_dirty: bool,
    password_data: &PasswordData,
    password_notes_content: &'a text_editor::Content,
    show_password: bool,
    show_password_gen: bool,
    password_gen_options: &crate::models::note::PasswordGenOptions,
    copied_field: Option<&str>,
    canvas: &'a CanvasEditor,
    canvas_color_editing: &Option<String>,
    color_hue: f32,
    color_sat: f32,
    color_lit: f32,
    is_session_decrypted: bool,
    is_encrypting: bool,
    note_password: &str,
    auth_error: Option<&str>,
    is_maximized: bool,
    toolbar_move_open: bool,
    folders: &'a [crate::models::Folder],
    font_size: u32,
    search_open: bool,
    search_query: &'a str,
    search_index: usize,
    search_case_sensitive: bool,
    line_editor_state: &'a crate::ui::line_editor::LineEditorState,
) -> Element<'a, Message> {
    let editor_style = if is_maximized { theme::editor_panel_square as fn(&iced::Theme) -> iced::widget::container::Style } else { theme::editor_panel_rounded };
    let pin_label = if note.is_pinned { "Unpin" } else { "Pin" };
    let pin_icon = if note.is_pinned { icons::pin_filled() } else { icons::pin_outline() };
    let pin_btn = tip(button(svg(pin_icon).width(16).height(16))
        .on_press(Message::TogglePin(note.id))
        .style(theme::toolbar_action(note.is_pinned))
        .padding([6, 8]), pin_label);

    let fav_label = if note.is_favorite { "Unfavorite" } else { "Favorite" };
    let fav_icon = if note.is_favorite { icons::star_filled() } else { icons::star_outline() };
    let fav_btn = tip(button(svg(fav_icon).width(16).height(16))
        .on_press(Message::ToggleFavorite(note.id))
        .style(theme::toolbar_action(note.is_favorite))
        .padding([6, 8]), fav_label);

    let (enc_icon, enc_msg, enc_tip) = if note.is_encrypted && is_session_decrypted {
        (icons::unlock_icon(), Message::LockNote, "Lock")
    } else if note.is_encrypted {
        (icons::lock_active(), Message::OpenDecryptDialog(note.id), "Unlock")
    } else {
        (icons::lock_closed(), Message::OpenEncryptDialog(note.id), "Encrypt")
    };
    let _enc_btn = tip(button(svg(enc_icon).width(16).height(16))
        .on_press(enc_msg)
        .style(theme::toolbar_action(note.is_encrypted))
        .padding([6, 8]), enc_tip);

    let folder_btn = tip(button(svg(icons::move_folder_icon()).width(16).height(16))
        .on_press(Message::OpenMoveFolderPicker(note.id))
        .style(theme::icon_button)
        .padding([6, 8]), "Move");

    let _delete_btn = tip_danger(button(svg(icons::trash_muted()).width(16).height(16))
        .on_press(Message::OpenDeleteNoteDialog(note.id))
        .style(theme::icon_button)
        .padding([6, 8]), "Delete");

    let mut action_row = row![pin_btn, fav_btn, folder_btn].spacing(2);

    if note.is_encrypted && is_session_decrypted {
        let change_pw_btn = tip(button(svg(icons::key_icon()).width(16).height(16))
            .on_press(Message::ChangeEncryptionPassword(note.id))
            .style(theme::icon_button)
            .padding([6, 8]), "Change password");
        let remove_enc_btn = tip(button(svg(icons::unlock_danger()).width(16).height(16))
            .on_press(Message::RemoveEncryption(note.id))
            .style(theme::icon_button)
            .padding([6, 8]), "Remove encryption");
        action_row = action_row.push(change_pw_btn);
        action_row = action_row.push(remove_enc_btn);
    }

    let title_display = if note.title.is_empty() { "Untitled" } else { &note.title };
    let toolbar_title = text(title_display).size(13).style(|_t| theme::secondary_text());

    let status = if is_encrypting { "Encrypting..." } else if is_dirty { "Editing..." } else { "" };
    let status_text = text(status).size(11).style(|_t| theme::secondary_text());

    let toolbar: Element<Message> = mouse_area(container(
        row![action_row, horizontal_space(), status_text, toolbar_title]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([6, 12]),
    )
    .style(theme::toolbar_container)
    .width(Length::Fill))
    .on_enter(Message::HoverItem(None))
    .into();

    if note.is_encrypted && !is_session_decrypted {
        let lock_content: Element<Message> = if is_encrypting {
            container(
                column![
                    Space::with_height(Length::Fill),
                    container(svg(icons::lock_active()).width(36).height(36)).center_x(Length::Fill),
                    Space::with_height(8),
                    text("Decrypting...").size(14).style(|_t| theme::secondary_text()).align_x(iced::alignment::Horizontal::Center).width(Length::Fill),
                    Space::with_height(Length::Fill),
                ].spacing(4)
            ).width(Length::Fill).height(Length::Fill).into()
        } else {
            let nid = note.id;
            let eye_icon = if show_password { icons::eye_open() } else { icons::eye_closed() };
            let password_field = stack![
                text_input("Password...", note_password)
                    .on_input(Message::NotePasswordInputChanged)
                    .on_submit(Message::SubmitDecrypt(nid))
                    .secure(!show_password)
                    .style(theme::dialog_input)
                    .size(14)
                    .padding(iced::Padding::new(10.0).right(40.0))
                    .id(text_input::Id::new("decrypt_password")),
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
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Center),
            ];
            let mut inner = column![
                container(svg(icons::lock_active()).width(36).height(36)).center_x(Length::Fill),
                Space::with_height(8),
                text("Encrypted").size(16).style(|_t| theme::primary_text()),
                Space::with_height(12),
                password_field,
            ].spacing(4).align_x(iced::alignment::Horizontal::Center).max_width(300);

            if let Some(err) = auth_error {
                inner = inner.push(
                    text(err.to_owned()).size(11).style(|_t| theme::danger_text())
                );
            }

            container(inner)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        return container(column![toolbar, lock_content])
            .style(editor_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let body_view: Element<Message> = match note.note_type {
        NoteType::Password => {
            scrollable(password_editor::view(password_data, password_notes_content, show_password, show_password_gen, password_gen_options, copied_field))
                .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
                .style(theme::dark_scrollable)
                .height(Length::Fill)
                .into()
        }
        NoteType::Canvas => {
            let tool_btn = |icon_handle, msg: Message| -> Element<Message> {
                button(svg(icon_handle).width(16).height(16))
                    .on_press(msg)
                    .style(theme::icon_button)
                    .padding([6, 6])
                    .into()
            };

            let right_toolbar = container(
                column![
                    tool_btn(icons::plus_bright(), Message::CanvasAddNodeCenter),
                    tool_btn(icons::fit_view_icon(), Message::CanvasFitView),
                    tool_btn(icons::crosshair_icon(), Message::CanvasRecenter),
                ]
                .spacing(2)
                .padding([6, 4])
            )
            .style(|_t: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.10, 0.10, 0.12, 0.7))),
                border: iced::Border { radius: 8.0.into(), ..Default::default() },
                ..Default::default()
            });

            let canvas_view = canvas.view();
            let canvas_with_tools = stack![
                canvas_view,
                container(
                    row![
                        Space::with_width(Length::Fill),
                        container(right_toolbar).padding([0, 8]),
                    ]
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_y(Length::Fill),
            ];

            let canvas_body: Element<Message> = if let Some(ref ctx) = canvas.ctx_menu_info {
                let (mx, my) = ctx.pos;
                let menu_items: Vec<Element<Message>> = match &ctx.target {
                    CanvasCtxTarget::Node(nid) => vec![
                        ctx_menu_btn(icons::pencil_icon(), "Edit", Message::CanvasCardFocus(nid.clone()), false),
                        iced::widget::mouse_area(
                            button(
                                row![svg(icons::palette_icon()).width(14).height(14), text("Color  \u{203A}").size(12).style(|_t| theme::primary_text())]
                                    .spacing(8).align_y(iced::Alignment::Center),
                            )
                            .on_press(Message::CanvasOpenColorPicker(nid.clone()))
                            .style(theme::context_menu_button)
                            .padding([5, 10])
                            .width(Length::Fill)
                        ).on_enter(Message::CanvasOpenColorPicker(nid.clone())).into(),
                        ctx_menu_btn(icons::trash_danger(), "Delete", Message::CanvasDeleteSelected, true),
                    ],
                    CanvasCtxTarget::Edge(_eid) => vec![
                        ctx_menu_btn(icons::trash_danger(), "Delete", Message::CanvasDeleteSelected, true),
                    ],
                    CanvasCtxTarget::Empty(x, y) => vec![
                        ctx_menu_btn(icons::plus_icon(), "Add card", Message::CanvasAddNode(*x, *y), false),
                    ],
                };

                let menu: Element<Message> = container(
                    column(menu_items).spacing(1).padding(4)
                )
                .style(theme::context_menu_container)
                .width(160)
                .into();

                let menu_row: Element<Message> = if canvas_color_editing.is_some() {
                    let picker = color_picker::view(color_hue, color_sat, color_lit, Message::ColorPickerHue, Message::ColorPickerSat, Message::ColorPickerLit);
                    let picker_panel: Element<Message> = container(
                        column![picker].spacing(2).padding(12)
                    )
                    .style(theme::context_menu_container)
                    .width(240)
                    .into();
                    row![menu, Space::with_width(4), picker_panel].into()
                } else {
                    menu
                };

                let backdrop: Element<Message> = mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                )
                .on_press(Message::CanvasCloseCtxMenu)
                .into();

                let clamped_mx = mx.max(0.0);
                let clamped_my = my.max(0.0);
                let positioned: Element<Message> = column![
                    Space::with_height(Length::Fixed(clamped_my)),
                    row![
                        Space::with_width(Length::Fixed(clamped_mx)),
                        mouse_area(menu_row).on_press(Message::None),
                    ].width(Length::Shrink),
                ].into();

                stack![canvas_with_tools, backdrop, positioned].into()
            } else if canvas_color_editing.is_some() {
                canvas_with_tools.into()
            } else {
                canvas_with_tools.into()
            };

            // use column for layout, but wrap in stack for dropdown overlays
            let canvas_col: Element<Message> = column![toolbar, canvas_body].into();

            if toolbar_move_open {
                let nid = note.id;
                let current_folder = note.folder_id;
                let mut move_items = column![].spacing(1);
                let none_selected = current_folder.is_none();
                let none_bg = if none_selected { theme::BG_TERTIARY } else { theme::TRANSPARENT };
                move_items = move_items.push(
                    button(row![svg(icons::folder_icon()).width(14).height(14), text("No Folder").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                        .on_press(Message::MoveNoteToFolder(nid, None))
                        .style(move |_t: &iced::Theme, status: button::Status| {
                            let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => none_bg };
                            button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                        })
                        .padding([6, 10]).width(Length::Fill)
                );
                for f in folders {
                    if f.parent_id.is_some() { continue; }
                    let fid = f.id;
                    let is_current = current_folder == Some(fid);
                    let folder_color = f.color.to_iced_color();
                    let current_bg = if is_current { theme::BG_TERTIARY } else { theme::TRANSPARENT };
                    move_items = move_items.push(
                        button(row![svg(icons::folder_colored(folder_color)).width(14).height(14), text(&f.name).size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                            .on_press(Message::MoveNoteToFolder(nid, Some(fid)))
                            .style(move |_t: &iced::Theme, status: button::Status| {
                                let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => current_bg };
                                button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                            })
                            .padding([6, 10]).width(Length::Fill)
                    );
                }
                let dropdown = container(scrollable(move_items.padding(4)).direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar())).style(theme::dark_scrollable))
                    .style(theme::context_menu_container).width(200).max_height(300);
                let backdrop: Element<Message> = mouse_area(container(Space::new(Length::Fill, Length::Fill))).on_press(Message::OpenMoveFolderPicker(nid)).into();
                let overlay = container(column![Space::with_height(36), row![Space::with_width(100), container(dropdown)]]).width(Length::Fill).height(Length::Fill);
                return container(stack![canvas_col, backdrop, overlay])
                    .style(editor_style).width(Length::Fill).height(Length::Fill).into();
            }

            return container(canvas_col)
                .style(editor_style)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        NoteType::File => {
            let file_view = crate::ui::file_viewer::view(&note.body, note.id);
            return container(column![toolbar, file_view])
                .style(editor_style)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        NoteType::Text => {
            let editor_area = crate::ui::line_editor::view(line_editor_state, font_size);

            if search_open {
                let match_count = line_editor_state.search_matches.len();

                let match_label = if search_query.is_empty() {
                    String::new()
                } else if match_count == 0 {
                    "0 of 0".to_string()
                } else {
                    format!("{} of {}", search_index + 1, match_count)
                };

                let case_btn = button(
                    text("Aa").size(12).style(move |_t| iced::widget::text::Style {
                        color: Some(if search_case_sensitive {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        }),
                    })
                )
                .on_press(Message::ToggleSearchCaseSensitive)
                .style(theme::search_toggle_button(search_case_sensitive))
                .padding([2, 5]);

                let input_row = container(
                    row![
                        text_input("Find", search_query)
                            .on_input(Message::SearchQueryEditorChanged)
                            .on_submit(Message::SearchNext)
                            .id(text_input::Id::new("editor_search"))
                            .style(theme::search_field_transparent)
                            .size(13)
                            .padding([4, 6])
                            .width(Length::Fill),
                        case_btn,
                    ].spacing(2).align_y(iced::Alignment::Center).padding([0, 4])
                )
                .style(theme::search_input_container)
                .width(Length::Fixed(220.0));

                let search_bar = container(
                    row![
                        input_row,
                        text(match_label).size(11).style(|_t| iced::widget::text::Style {
                            color: Some(theme::TEXT_SECONDARY),
                        }).width(Length::Shrink),
                        button(svg(icons::chevron_up()).width(14).height(14))
                            .on_press(Message::SearchPrev).style(theme::search_nav_button).padding([4, 5]),
                        button(svg(icons::chevron_down()).width(14).height(14))
                            .on_press(Message::SearchNext).style(theme::search_nav_button).padding([4, 5]),
                        button(svg(icons::close_light()).width(12).height(12))
                            .on_press(Message::ToggleSearch).style(theme::search_nav_button).padding([4, 5]),
                    ].spacing(4).align_y(iced::Alignment::Center).padding([4, 6])
                )
                .style(|_t: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border { radius: 0.0.into(), width: 0.0, color: iced::Color::TRANSPARENT },
                    shadow: iced::Shadow { color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35), offset: iced::Vector::new(0.0, 3.0), blur_radius: 6.0 },
                    ..Default::default()
                });
                let positioned: Element<'a, Message> = container(row![horizontal_space(), search_bar].padding([0, 0])).width(Length::Fill).into();
                stack![editor_area, positioned].into()
            } else {
                editor_area
            }
        }
    };

    let main_col = column![toolbar, body_view];

    if toolbar_move_open {
        let nid = note.id;
        let current_folder = note.folder_id;
        let mut move_items = column![].spacing(1);

        let none_selected = current_folder.is_none();
        let none_bg = if none_selected { theme::BG_TERTIARY } else { theme::TRANSPARENT };
        move_items = move_items.push(
            button(row![svg(icons::folder_icon()).width(14).height(14), text("No Folder").size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                .on_press(Message::MoveNoteToFolder(nid, None))
                .style(move |_t: &iced::Theme, status: button::Status| {
                    let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => none_bg };
                    button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                })
                .padding([6, 10]).width(Length::Fill)
        );
        for f in folders {
            if f.parent_id.is_some() { continue; }
            let fid = f.id;
            let is_current = current_folder == Some(fid);
            let folder_color = f.color.to_iced_color();
            let current_bg = if is_current { theme::BG_TERTIARY } else { theme::TRANSPARENT };
            move_items = move_items.push(
                button(row![svg(icons::folder_colored(folder_color)).width(14).height(14), text(&f.name).size(12).style(|_t| theme::primary_text())].spacing(8).align_y(iced::Alignment::Center))
                    .on_press(Message::MoveNoteToFolder(nid, Some(fid)))
                    .style(move |_t: &iced::Theme, status: button::Status| {
                        let bg = match status { button::Status::Hovered => theme::BG_HOVER, _ => current_bg };
                        button::Style { background: Some(iced::Background::Color(bg)), border: iced::Border { radius: 6.0.into(), ..Default::default() }, text_color: theme::TEXT_PRIMARY, ..Default::default() }
                    })
                    .padding([6, 10]).width(Length::Fill)
            );
        }

        let dropdown = container(
            scrollable(move_items.padding(4))
                .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
                .style(theme::dark_scrollable)
        )
        .style(theme::context_menu_container)
        .width(200)
        .max_height(300);

        let backdrop: Element<Message> = mouse_area(
            container(Space::new(Length::Fill, Length::Fill))
        ).on_press(Message::OpenMoveFolderPicker(nid)).into();

        let overlay = container(
            column![
                Space::with_height(36), // toolbar height offset
                row![
                    Space::with_width(100), // approximate position under move button
                    container(dropdown),
                ],
            ]
        ).width(Length::Fill).height(Length::Fill);

        container(stack![main_col, backdrop, overlay])
            .style(editor_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(main_col)
            .style(editor_style)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn tip<'a>(content: iced::widget::Button<'a, Message>, label: &'a str) -> Element<'a, Message> {
    tooltip(
        content,
        text(label).size(11).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.55, 0.55, 0.58)) }),
        tooltip::Position::Bottom,
    )
    .style(|_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.13, 0.13, 0.15))),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .padding(4)
    .gap(4)
    .into()
}

fn tip_danger<'a>(content: iced::widget::Button<'a, Message>, label: &'a str) -> Element<'a, Message> {
    tooltip(
        content,
        text(label).size(11).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.75, 0.3, 0.3)) }),
        tooltip::Position::Bottom,
    )
    .style(|_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.13, 0.13, 0.15))),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .padding(4)
    .gap(4)
    .into()
}

fn ctx_menu_btn<'a>(icon: iced::widget::svg::Handle, label: &str, msg: Message, danger: bool) -> Element<'a, Message> {
    let style = if danger { theme::context_menu_danger_button } else { theme::context_menu_button };
    let text_style = if danger { theme::danger_text() } else { theme::primary_text() };
    iced::widget::mouse_area(
        button(
            row![svg(icon).width(14).height(14), text(label.to_owned()).size(12).style(move |_t| text_style)]
                .spacing(8).align_y(iced::Alignment::Center),
        )
        .on_press(msg)
        .style(style)
        .padding([5, 10])
        .width(Length::Fill)
    )
    .on_enter(Message::CloseSubmenus)
    .into()
}
