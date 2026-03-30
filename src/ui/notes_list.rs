use iced::widget::{
    column, container, horizontal_space, mouse_area, row, scrollable, svg, text,
    text_input,
};
use iced::{Element, Length};
use uuid::Uuid;

use crate::app::{ContextMenu, Message};
use crate::models::{Folder, NotePreview};
use crate::ui::{icons, theme};

pub fn view<'a>(
    notes: &'a [NotePreview],
    selected_note_id: Option<Uuid>,
    search_query: &str,
    context_menu: &Option<ContextMenu>,
    renaming_note: Option<Uuid>,
    rename_buffer: &str,
    is_folder_view: bool,
    subfolders: &'a [Folder],
    subfolder_notes: &'a [(Uuid, Vec<NotePreview>)],
    expanded_folders: &std::collections::HashSet<Uuid>,
    dragging: &'a Option<crate::app::DragItem>,
    active_folder_id: Option<Uuid>,
    sort_mode: crate::app::SortMode,
    sort_menu_open: bool,
    multi_selected: &std::collections::HashSet<Uuid>,
    renaming_folder: Option<Uuid>,
    folder_rename_buffer: &str,
    ctrl_held: bool,
    shift_held: bool,
    multi_selected_folders: &std::collections::HashSet<Uuid>,
    hovered_item: Option<Uuid>,
) -> Element<'a, Message> {
    use iced::widget::button;

    let mut search_row = row![
        svg(icons::search_icon()).width(14).height(14),
        text_input("Search...", search_query)
            .on_input(Message::SearchQueryChanged)
            .style(theme::search_input)
            .size(13)
            .padding(5),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let sort_btn = button(
        svg(sort_icon(sort_mode)).width(14).height(14)
    )
    .on_press(Message::ToggleSortMenu)
    .style(theme::icon_button)
    .padding([6, 6]);
    search_row = search_row.push(sort_btn);

    let header = container(search_row.padding([0, 4]))
        .style(|_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_TERTIARY)),
            border: iced::Border { radius: 6.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding([5, 8]);

    let mut file_list = column![].spacing(3);

    if is_folder_view {
        let mut items: Vec<Element<Message>> = Vec::new();
        collect_folder_children(
            &mut items, active_folder_id, subfolders, subfolder_notes,
            expanded_folders, context_menu, dragging, selected_note_id,
            renaming_note, rename_buffer, 0, multi_selected,
            renaming_folder, folder_rename_buffer, ctrl_held, shift_held, multi_selected_folders, hovered_item,
        );
        for preview in notes {
            items.push(render_note(preview, selected_note_id, context_menu, renaming_note, rename_buffer, 0, dragging, multi_selected, hovered_item));
        }
        for item in items {
            file_list = file_list.push(item);
        }
    } else {
        for subfolder in subfolders.iter().filter(|f| f.is_favorite) {
            let sf_notes = subfolder_notes.iter().find(|(id, _)| *id == subfolder.id).map(|(_, n)| n.as_slice()).unwrap_or(&[]);
            let folder_color = subfolder.color.to_iced_color();
            let sid = subfolder.id;
            let is_expanded = expanded_folders.contains(&sid);
            let is_ctx = matches!(context_menu, Some(ContextMenu::Tag(id)) if *id == sid);
            let is_folder_selected = multi_selected_folders.contains(&sid);
            let is_dragging = dragging.is_some();

            let arrow_icon = if is_expanded {
                iced::widget::svg::Handle::from_memory(b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'/></svg>".to_vec())
            } else {
                iced::widget::svg::Handle::from_memory(b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='9 18 15 12 9 6'/></svg>".to_vec())
            };
            let note_count = sf_notes.len();
            let mut folder_items = row![
                svg(arrow_icon).width(10).height(10),
                svg(icons::folder_colored(folder_color)).width(16).height(16),
                text(&subfolder.name).size(13).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.75, 0.75, 0.78)) }),
                horizontal_space(),
            ].spacing(4).align_y(iced::Alignment::Center);
            if subfolder.is_favorite {
                folder_items = folder_items.push(svg(icons::star_filled()).width(10).height(10));
            }
            if note_count > 0 {
                folder_items = folder_items.push(text(note_count.to_string()).size(10).style(|_t| theme::secondary_text()));
            }

            let is_hovered = hovered_item == Some(sid);
            let is_drop_target = is_dragging && !matches!(&dragging, Some(crate::app::DragItem::Folder(id)) if *id == sid);
            let bg = if is_folder_selected { theme::BG_SELECTED } else if is_drop_target { theme::BG_HOVER } else if is_ctx { theme::BG_HOVER } else if is_hovered { iced::Color::from_rgba(1.0, 1.0, 1.0, 0.015) } else { theme::TRANSPARENT };
            let folder_container = container(folder_items)
                .style(move |_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(bg)),
                    border: iced::Border {
                        radius: 6.0.into(),
                        width: if is_folder_selected { 1.0 } else { 0.0 },
                        color: if is_folder_selected { iced::Color::from_rgb(0.18, 0.55, 0.31) } else { theme::TRANSPARENT },
                    },
                    ..Default::default()
                })
                .padding([5, 8])
                .width(Length::Fill);

            let is_this_dragged = matches!(&dragging, Some(crate::app::DragItem::Folder(id)) if *id == sid);
            if is_dragging && !is_this_dragged {
                let drop_msg = dragging.as_ref().map(|item| Message::DropOnFolder(item.clone(), Some(sid)));
                let folder_item: Element<Message> = if let Some(msg) = drop_msg {
                    mouse_area(folder_container).on_release(msg).into()
                } else {
                    folder_container.into()
                };
                file_list = file_list.push(folder_item);
            } else {
                let release_msg = if is_dragging { Message::DragEnd } else { Message::ToggleFolderSelect(sid) };
                let folder_item: Element<Message> = mouse_area(folder_container)
                    .on_press(Message::DragPotential(crate::app::DragItem::Folder(sid)))
                    .on_release(release_msg)
                    .on_right_press(if is_dragging { Message::DragEnd } else { Message::ToggleContextMenu(ContextMenu::Tag(sid)) })
                    .on_enter(Message::HoverItem(Some(sid)))
                    .into();
                file_list = file_list.push(folder_item);
            }

            if is_expanded {
                for preview in sf_notes {
                    file_list = file_list.push(render_note(preview, selected_note_id, context_menu, renaming_note, rename_buffer, 16, dragging, multi_selected, hovered_item));
                }
                let mut child_items: Vec<Element<Message>> = Vec::new();
                collect_folder_children(
                    &mut child_items, Some(sid), subfolders, subfolder_notes,
                    expanded_folders, context_menu, dragging, selected_note_id,
                    renaming_note, rename_buffer, 1, multi_selected,
                    renaming_folder, folder_rename_buffer, ctrl_held, shift_held, multi_selected_folders, hovered_item,
                );
                for item in child_items {
                    file_list = file_list.push(item);
                }
            }
        }
        for preview in notes {
            file_list = file_list.push(render_note(preview, selected_note_id, context_menu, renaming_note, rename_buffer, 0, dragging, multi_selected, hovered_item));
        }
    }

    let scrollable_list = scrollable(file_list.padding([0, 6]))
        .direction(iced::widget::scrollable::Direction::Vertical(theme::thin_scrollbar()))
        .style(theme::dark_scrollable);

    // Empty area BELOW scrollable — catches right-click and drop in empty space, fills remaining height
    let empty_area: Element<Message> = if dragging.is_some() {
        let drop_msg = dragging.as_ref().map(|item| Message::DropOnFolder(item.clone(), active_folder_id));
        let mut ma = mouse_area(
            container(iced::widget::Space::new(Length::Fill, Length::Fill))
                .height(Length::Fill)
        );
        if let Some(msg) = drop_msg {
            ma = ma.on_release(msg);
        }
        ma.into()
    } else {
        mouse_area(
            container(iced::widget::Space::new(Length::Fill, Length::Fill))
                .height(Length::Fill)
        )
        .on_right_press(Message::ToggleContextMenu(ContextMenu::NotesEmpty))
        .on_enter(Message::HoverItem(None))
        .into()
    };

    let list_area: Element<Message> = column![scrollable_list, empty_area].height(Length::Fill).into();

    let search_header: Element<Message> = mouse_area(container(header).padding([10, 6]))
        .on_enter(Message::HoverItem(None))
        .into();

    let main_content = column![
        search_header,
        list_area,
    ];

    let content: Element<Message> = if sort_menu_open {
        use iced::widget::stack;
        let sort_option = |label: &str, mode: crate::app::SortMode| -> Element<Message> {
            let is_active = sort_mode == mode;
            let bg = if is_active { theme::BG_TERTIARY } else { theme::TRANSPARENT };
            button(
                row![
                    svg(sort_icon(mode)).width(12).height(12),
                    text(label.to_owned()).size(11).style(move |_t| iced::widget::text::Style {
                        color: Some(if is_active { iced::Color::from_rgb(0.85, 0.85, 0.87) } else { iced::Color::from_rgb(0.6, 0.6, 0.63) })
                    }),
                ].spacing(6).align_y(iced::Alignment::Center)
            )
            .on_press(Message::SetSortMode(mode))
            .style(move |_t: &iced::Theme, status: iced::widget::button::Status| {
                let b = match status {
                    iced::widget::button::Status::Hovered => theme::BG_HOVER,
                    _ => bg,
                };
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(b)),
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }
            })
            .padding([5, 8])
            .width(Length::Fill)
            .into()
        };
        let menu = container(
            column![
                sort_option("Modified", crate::app::SortMode::Modified),
                sort_option("Oldest", crate::app::SortMode::Created),
                sort_option("A → Z", crate::app::SortMode::NameAZ),
                sort_option("Z → A", crate::app::SortMode::NameZA),
                sort_option("Type", crate::app::SortMode::Type),
            ].spacing(1).padding(4)
        )
        .style(theme::context_menu_container)
        .width(120);

        let backdrop = mouse_area(
            container(iced::widget::Space::new(Length::Fill, 800))
        ).on_press(Message::ToggleSortMenu);

        let positioned_menu = column![
            iced::widget::Space::with_height(36),
            row![iced::widget::Space::with_width(Length::Fill), container(menu).padding([0, 4])],
        ];

        stack![main_content, backdrop, positioned_menu].into()
    } else {
        main_content.into()
    };

    container(content)
        .style(theme::notes_panel)
        .width(240)
        .height(Length::Fill)
        .into()
}

fn render_note<'a>(
    preview: &'a NotePreview,
    selected_note_id: Option<Uuid>,
    context_menu: &Option<ContextMenu>,
    renaming_note: Option<Uuid>,
    rename_buffer: &str,
    indent: u16,
    dragging: &Option<crate::app::DragItem>,
    multi_selected: &std::collections::HashSet<Uuid>,
    hovered_item: Option<Uuid>,
) -> Element<'a, Message> {
    let is_selected = selected_note_id.map_or(false, |id| id == preview.id);
    let is_multi = multi_selected.contains(&preview.id);
    let is_dragging = dragging.is_some();
    let title_display = if preview.title.is_empty() { "Untitled" } else { &preview.title };
    let single_dragged = matches!(dragging, Some(crate::app::DragItem::Note(id)) if *id == preview.id);
    let being_dragged = single_dragged || (is_dragging && is_multi);

    let note_color = preview.color.to_iced_color();
    let dot = match preview.note_type {
        crate::models::NoteType::Text => svg(icons::note_text_icon(note_color)).width(14).height(14),
        crate::models::NoteType::Password => svg(icons::note_password_icon(note_color)).width(14).height(14),
        crate::models::NoteType::Canvas => svg(icons::note_canvas_icon(note_color)).width(14).height(14),
        crate::models::NoteType::File => svg(icons::note_file_icon(note_color)).width(14).height(14),
    };

    let mut status_icons = row![].spacing(4).align_y(iced::Alignment::Center);
    if preview.is_favorite {
        status_icons = status_icons.push(svg(icons::star_filled()).width(12).height(12));
    }
    if preview.is_pinned {
        status_icons = status_icons.push(svg(icons::pin_filled()).width(12).height(12));
    }

    let is_renaming = renaming_note == Some(preview.id);

    let tree_line: Element<Message> = if indent > 0 {
        iced::widget::Space::with_width(indent).into()
    } else {
        iced::widget::Space::with_width(0).into()
    };

    let title_row: Element<Message> = if is_renaming {
        row![
            tree_line,
            dot,
            text_input("Enter name...", rename_buffer)
                .on_input(Message::RenameNoteChanged)
                .on_submit(Message::RenameNoteSubmit)
                .style(theme::inline_rename_input)
                .size(12)
                .padding([2, 4])
                .id(text_input::Id::new("inline_rename")),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        row![
            tree_line,
            dot,
            text(title_display).size(12).style(|_t| theme::primary_text()),
            horizontal_space(),
            status_icons,
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center)
        .into()
    };

    let show_snippet = preview.note_type != crate::models::NoteType::Canvas && preview.note_type != crate::models::NoteType::File && !preview.snippet.is_empty();
    let mut card_content = column![title_row].spacing(2).padding([6, 8]);
    if show_snippet {
        card_content = card_content.push(
            row![iced::widget::Space::with_width(indent + 20), text(&preview.snippet).size(10).style(|_t| theme::secondary_text())],
        );
    }

    let is_ctx_target = matches!(context_menu, Some(ContextMenu::NoteItem(id)) | Some(ContextMenu::NoteColor(id)) if *id == preview.id);

    let nid = preview.id;

    let is_hovered = hovered_item == Some(preview.id);
    let bg = if being_dragged {
        iced::Color::from_rgba(0.18, 0.55, 0.31, 0.12)
    } else if is_multi {
        theme::BG_SELECTED
    } else if is_selected {
        theme::BG_TERTIARY
    } else if is_ctx_target {
        theme::BG_HOVER
    } else if is_hovered {
        iced::Color::from_rgba(1.0, 1.0, 1.0, 0.015)
    } else {
        theme::TRANSPARENT
    };

    let card_container = container(card_content)
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                radius: 6.0.into(),
                width: if (is_selected || is_multi) && !being_dragged { 1.0 } else { 0.0 },
                color: if is_selected || is_multi { iced::Color::from_rgb(0.18, 0.55, 0.31) } else { theme::TRANSPARENT },
            },
            ..Default::default()
        })
        .width(Length::Fill);

    let release_msg = if is_dragging { Message::DragEnd } else { Message::SelectNote(nid) };

    mouse_area(card_container)
        .on_press(Message::DragPotential(crate::app::DragItem::Note(nid)))
        .on_release(release_msg)
        .on_right_press(if is_dragging { Message::DragEnd } else { Message::ToggleContextMenu(ContextMenu::NoteItem(nid)) })
        .on_enter(Message::HoverItem(Some(nid)))
        .into()
}

fn collect_folder_children<'a>(
    items: &mut Vec<Element<'a, Message>>,
    parent_id: Option<Uuid>,
    all_subfolders: &'a [Folder],
    subfolder_notes: &'a [(Uuid, Vec<NotePreview>)],
    expanded_folders: &std::collections::HashSet<Uuid>,
    context_menu: &Option<ContextMenu>,
    dragging: &'a Option<crate::app::DragItem>,
    selected_note_id: Option<Uuid>,
    renaming_note: Option<Uuid>,
    rename_buffer: &str,
    depth: u16,
    multi_selected: &std::collections::HashSet<Uuid>,
    renaming_folder: Option<Uuid>,
    folder_rename_buffer: &str,
    ctrl_held: bool,
    shift_held: bool,
    multi_selected_folders: &std::collections::HashSet<Uuid>,
    hovered_item: Option<Uuid>,
) {
    let children: Vec<&Folder> = all_subfolders.iter()
        .filter(|f| f.parent_id == parent_id)
        .collect();

    let indent = depth * 16;
    let is_drop_target = dragging.is_some();
    let being_dragged_id = match dragging.as_ref() {
        Some(crate::app::DragItem::Folder(id)) => Some(*id),
        _ => None,
    };

    for subfolder in children {
        let is_expanded = expanded_folders.contains(&subfolder.id);
        let sf_notes = subfolder_notes.iter().find(|(id, _)| *id == subfolder.id).map(|(_, n)| n.as_slice()).unwrap_or(&[]);
        let note_count = sf_notes.len();
        let subfolder_count = all_subfolders.iter().filter(|f| f.parent_id == Some(subfolder.id)).count();
        let total = note_count + subfolder_count;

        let arrow_icon = if is_expanded {
            iced::widget::svg::Handle::from_memory(b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'/></svg>".to_vec())
        } else {
            iced::widget::svg::Handle::from_memory(b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='9 18 15 12 9 6'/></svg>".to_vec())
        };
        let is_this_dragged = being_dragged_id == Some(subfolder.id);
        let is_ctx = matches!(context_menu, Some(ContextMenu::Tag(id)) if *id == subfolder.id);
        let sid = subfolder.id;

        let tree_indent: Element<Message> = if indent > 0 {
            iced::widget::Space::with_width(indent).into()
        } else {
            iced::widget::Space::with_width(0).into()
        };

        let folder_color = subfolder.color.to_iced_color();

        let count_text = if total > 0 { format!("{}", total) } else { String::new() };
        let is_folder_renaming = renaming_folder == Some(sid);

        let folder_row_content = if is_folder_renaming {
            row![
                tree_indent,
                svg(arrow_icon).width(10).height(10),
                svg(icons::folder_colored(folder_color)).width(16).height(16),
                text_input("Folder name...", folder_rename_buffer)
                    .on_input(Message::RenameFolderChanged)
                    .on_submit(Message::RenameFolderSubmit)
                    .style(theme::inline_rename_input)
                    .size(13)
                    .padding([2, 4])
                    .id(text_input::Id::new("inline_folder_rename")),
            ].spacing(4).align_y(iced::Alignment::Center)
        } else {
            {
                let mut r = row![
                    tree_indent,
                    svg(arrow_icon).width(10).height(10),
                    svg(icons::folder_colored(folder_color)).width(16).height(16),
                    text(&subfolder.name).size(13).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.75, 0.75, 0.78)) }),
                    horizontal_space(),
                ].spacing(4).align_y(iced::Alignment::Center);
                if subfolder.is_favorite {
                    r = r.push(svg(icons::star_filled()).width(10).height(10));
                }
                r.push(text(count_text).size(10).style(|_t| theme::secondary_text()))
            }
        };

        let is_folder_selected = multi_selected_folders.contains(&sid);
        let is_folder_being_dragged = is_this_dragged || (is_drop_target && is_folder_selected);
        let is_hovered = hovered_item == Some(sid);
        let bg = if is_folder_being_dragged {
            iced::Color::from_rgba(0.18, 0.55, 0.31, 0.12)
        } else if is_folder_selected {
            theme::BG_SELECTED
        } else if is_drop_target {
            theme::BG_HOVER
        } else if is_ctx {
            theme::BG_HOVER
        } else if is_hovered {
            iced::Color::from_rgba(1.0, 1.0, 1.0, 0.015)
        } else {
            theme::TRANSPARENT
        };

        let folder_container = container(folder_row_content)
            .style(move |_t: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border {
                    radius: 6.0.into(),
                    width: if is_folder_selected { 1.0 } else { 0.0 },
                    color: if is_folder_selected { iced::Color::from_rgb(0.18, 0.55, 0.31) } else { theme::TRANSPARENT },
                },
                ..Default::default()
            })
            .padding([5, 8])
            .width(Length::Fill);

        if is_drop_target && !is_this_dragged {
            let drop_msg = dragging.as_ref().map(|item| Message::DropOnFolder(item.clone(), Some(sid)));
            let folder_item: Element<Message> = if let Some(msg) = drop_msg {
                mouse_area(folder_container).on_release(msg).into()
            } else {
                folder_container.into()
            };
            items.push(folder_item);
        } else {
            let is_dragging = dragging.is_some();
            let release_msg = if is_dragging { Message::DragEnd } else { Message::ToggleFolderSelect(sid) };
            let folder_item: Element<Message> = mouse_area(folder_container)
                .on_press(Message::DragPotential(crate::app::DragItem::Folder(sid)))
                .on_release(release_msg)
                .on_right_press(if is_dragging { Message::DragEnd } else { Message::ToggleContextMenu(ContextMenu::Tag(sid)) })
                .on_enter(Message::HoverItem(Some(sid)))
                .into();
            items.push(folder_item);
        }

        if is_expanded {
            let has_children = !sf_notes.is_empty() || subfolder_count > 0;
            for preview in sf_notes {
                items.push(render_note(preview, selected_note_id, context_menu, renaming_note, rename_buffer, indent + 16, dragging, multi_selected, hovered_item));
            }
            collect_folder_children(
                items, Some(sid), all_subfolders, subfolder_notes,
                expanded_folders, context_menu, dragging, selected_note_id,
                renaming_note, rename_buffer, depth + 1, multi_selected,
                renaming_folder, folder_rename_buffer, ctrl_held, shift_held, multi_selected_folders, hovered_item,
            );
            if has_children {
                let sep: Element<Message> = container(
                    row![
                        iced::widget::Space::with_width(indent + 16),
                        container(iced::widget::Space::new(Length::Fill, 1))
                            .style(|_t: &iced::Theme| container::Style {
                                background: Some(iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.015))),
                                ..Default::default()
                            }),
                    ]
                ).padding([3, 8]).into();
                items.push(sep);
            }
        }
    }
}

fn sort_icon(mode: crate::app::SortMode) -> iced::widget::svg::Handle {
    let svg_data: &[u8] = match mode {
        crate::app::SortMode::Modified => b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round'><line x1='4' y1='6' x2='16' y2='6'/><line x1='4' y1='12' x2='12' y2='12'/><line x1='4' y1='18' x2='8' y2='18'/></svg>",
        crate::app::SortMode::Created => b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round'><line x1='4' y1='6' x2='8' y2='6'/><line x1='4' y1='12' x2='12' y2='12'/><line x1='4' y1='18' x2='16' y2='18'/></svg>",
        crate::app::SortMode::NameAZ => b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><line x1='12' y1='5' x2='12' y2='19'/><polyline points='7 14 12 19 17 14'/></svg>",
        crate::app::SortMode::NameZA => b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><line x1='12' y1='19' x2='12' y2='5'/><polyline points='7 10 12 5 17 10'/></svg>",
        crate::app::SortMode::Type => b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none'><rect x='3' y='3' width='8' height='8' rx='2'/><rect x='13' y='3' width='8' height='8' rx='2'/><rect x='3' y='13' width='8' height='8' rx='2'/><rect x='13' y='13' width='8' height='8' rx='2'/></svg>",
    };
    iced::widget::svg::Handle::from_memory(svg_data.to_vec())
}
