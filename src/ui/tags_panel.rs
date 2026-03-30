use iced::widget::{button, column, container, horizontal_space, mouse_area, row, svg, text, text_input, Space};
use iced::{Element, Length};
use uuid::Uuid;

use crate::app::{ActiveView, ContextMenu, Message};
use crate::models::Folder;
use crate::ui::{icons, theme};

pub fn view<'a>(
    active_view: &'a ActiveView,
    folders: &'a [Folder],
    all_count: usize,
    fav_count: usize,
    folder_counts: &'a [(Uuid, usize)],
    context_menu: &Option<ContextMenu>,
    dragging: &'a Option<crate::app::DragItem>,
    renaming_folder: Option<Uuid>,
    folder_rename_buffer: &str,
    hovered_item: Option<Uuid>,
    is_maximized: bool,
) -> Element<'a, Message> {
    let all_btn: Element<Message> = mouse_area(
        button(
            row![
                text("All").size(13),
                horizontal_space(),
                text(all_count.to_string()).size(11).style(|_t| theme::secondary_text()),
            ].align_y(iced::Alignment::Center),
        )
        .on_press(Message::SelectView(ActiveView::AllNotes))
        .style(theme::tag_button(matches!(active_view, ActiveView::AllNotes)))
        .padding([6, 10])
        .width(Length::Fill)
    ).on_enter(Message::HoverItem(None)).into();

    let fav_btn: Element<Message> = mouse_area(
        button(
            row![
                svg(icons::star_outline()).width(14).height(14),
                text("Favorites").size(13),
                horizontal_space(),
                text(fav_count.to_string()).size(11).style(|_t| theme::secondary_text()),
            ].spacing(5).align_y(iced::Alignment::Center),
        )
        .on_press(Message::SelectView(ActiveView::Favorites))
        .style(theme::tag_button(matches!(active_view, ActiveView::Favorites)))
        .padding([6, 10])
        .width(Length::Fill)
    ).on_enter(Message::HoverItem(None)).into();

    let sep = container(Space::new(Length::Fill, 1)).style(|_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_TERTIARY)),
        ..Default::default()
    });

    let mut folder_list = column![].spacing(1);
    let is_dragging = dragging.is_some();
    for folder in folders.iter().filter(|f| f.parent_id.is_none()) {
        let fid = folder.id;
        let is_active = matches!(active_view, ActiveView::Folder(id) if *id == fid);
        let is_ctx = matches!(context_menu, Some(ContextMenu::Tag(id)) if *id == fid);
        let count = folder_counts.iter().find(|(id, _)| *id == fid).map(|(_, c)| *c).unwrap_or(0);
        let dot = container(Space::new(7, 7)).style(theme::color_dot(folder.color.to_iced_color()));

        if is_dragging {
            let drop_msg = dragging.as_ref().map(|item| match item {
                crate::app::DragItem::Folder(src_id) => {
                    // Only reorder if dragged folder is also a root folder
                    let src_is_root = folders.iter().any(|f| f.id == *src_id && f.parent_id.is_none());
                    if src_is_root {
                        Message::ReorderDrop(item.clone(), fid)
                    } else {
                        Message::DropOnFolder(item.clone(), Some(fid))
                    }
                }
                crate::app::DragItem::Note(_) => Message::DropOnFolder(item.clone(), Some(fid)),
            });
            // Live preview: when dragging a folder, hovering reorders in real time
            let hover_msg = match dragging.as_ref() {
                Some(crate::app::DragItem::Folder(src_id)) if *src_id != fid => {
                    Some(Message::ReorderPreview(*src_id, fid))
                }
                _ => None,
            };
            let folder_row = container(
                row![
                    dot,
                    text(&folder.name).size(12).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.75, 0.75, 0.78)) }),
                    horizontal_space(),
                    text(count.to_string()).size(11).style(|_t| theme::secondary_text()),
                ].spacing(5).align_y(iced::Alignment::Center),
            )
            .style(|_t: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(theme::BG_HOVER)),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([5, 8])
            .width(Length::Fill);

            let mut ma = mouse_area(folder_row);
            if let Some(msg) = drop_msg {
                ma = ma.on_release(msg);
            }
            if let Some(msg) = hover_msg {
                ma = ma.on_enter(msg);
            }
            let tag_item: Element<Message> = ma.into();
            folder_list = folder_list.push(tag_item);
        } else if renaming_folder == Some(fid) {
            let folder_row = container(
                row![
                    dot,
                    text_input("Folder name...", folder_rename_buffer)
                        .on_input(Message::RenameFolderChanged)
                        .on_submit(Message::RenameFolderSubmit)
                        .style(theme::inline_rename_input)
                        .size(12)
                        .padding([2, 4])
                        .id(text_input::Id::new("inline_folder_rename")),
                ].spacing(5).align_y(iced::Alignment::Center),
            )
            .style(theme::note_container(is_active, is_ctx))
            .padding([5, 8])
            .width(Length::Fill);
            folder_list = folder_list.push(folder_row);
        } else {
            let mut folder_row_items = row![
                dot,
                text(&folder.name).size(12).style(|_t| iced::widget::text::Style { color: Some(iced::Color::from_rgb(0.75, 0.75, 0.78)) }),
                horizontal_space(),
            ].spacing(5).align_y(iced::Alignment::Center);
            if folder.is_favorite {
                folder_row_items = folder_row_items.push(svg(icons::star_filled()).width(10).height(10));
            }
            folder_row_items = folder_row_items.push(text(count.to_string()).size(11).style(|_t| theme::secondary_text()));
            let is_hovered = hovered_item == Some(fid);
            let folder_row = container(folder_row_items)
                .style(theme::note_container_hover(is_active, is_ctx, is_hovered))
                .padding([5, 8])
                .width(Length::Fill);

            let tag_item: Element<Message> = mouse_area(folder_row)
                .on_press(Message::DragPotential(crate::app::DragItem::Folder(fid)))
                .on_release(Message::SelectView(ActiveView::Folder(fid)))
                .on_right_press(Message::ToggleContextMenu(ContextMenu::Tag(fid)))
                .on_enter(Message::HoverItem(Some(fid)))
                .into();
            folder_list = folder_list.push(tag_item);
        }
    }

    // Empty area: right-click for menu, drop here to make root / reorder to end
    let mut empty_ma = mouse_area(
        container(Space::new(Length::Fill, Length::Fill)).style(theme::tags_panel),
    )
    .on_right_press(Message::ToggleContextMenu(ContextMenu::TagsEmpty));
    if let Some(item) = dragging.as_ref() {
        empty_ma = empty_ma.on_release(Message::DropOnFolder(item.clone(), None));
        if let crate::app::DragItem::Folder(src_id) = item {
            empty_ma = empty_ma.on_enter(Message::ReorderToEnd(*src_id));
        }
    } else {
        empty_ma = empty_ma.on_enter(Message::HoverItem(None));
    }
    let empty_area = empty_ma;

    let sep2 = container(Space::new(Length::Fill, 1)).style(|_t: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_TERTIARY)),
        ..Default::default()
    });

    let settings_btn: Element<Message> = mouse_area(
        button(
            row![svg(icons::settings_icon()).width(14).height(14), text("Settings").size(12)]
                .spacing(5).align_y(iced::Alignment::Center),
        )
        .on_press(Message::ShowSettings)
        .style(theme::tag_button(false))
        .padding([5, 8])
        .width(Length::Fill)
    ).on_enter(Message::HoverItem(None)).into();

    let content = column![
        container(column![Space::with_height(8), all_btn, fav_btn, sep, folder_list].spacing(3).padding([6, 8])),
        empty_area,
        container(column![sep2, settings_btn].spacing(3).padding([6, 8])),
    ];

    let panel_style = if is_maximized { theme::tags_panel_square as fn(&iced::Theme) -> container::Style } else { theme::tags_panel_rounded };
    container(content).style(panel_style).width(140).height(Length::Fill).into()
}
