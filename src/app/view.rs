use super::*;
use iced::widget::column;

impl App {
    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        if id != self.focused_window {
            return if let Some(win) = self.other_windows.get(&id) {
                self.view_stored_window(id, win)
            } else {
                Space::new(Length::Fill, Length::Fill).into()
            };
        }

        let content = match self.vault_state {
            VaultState::Setup => dialog::password_dialog::view_setup(&self.password_input, &self.confirm_password_input, self.auth_error.as_deref(), self.window_controls_hovered, self.is_maximized),
            VaultState::Login => dialog::password_dialog::view_login(&self.password_input, self.auth_error.as_deref(), self.window_controls_hovered, self.is_maximized, self.show_password),
            VaultState::Loading => self.view_loading(),
            VaultState::Unlocked => self.view_main(),
        };
        // 1px padding makes rounded corners visible against transparent bg
        let window_style = if self.is_maximized { theme::window_container_maximized as fn(&iced::Theme) -> container::Style } else { theme::window_container };
        let pad = if self.is_maximized { 0 } else { 1 };
        let main = container(
            container(content)
                .style(window_style)
                .width(Length::Fill)
                .height(Length::Fill)
        )
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
            ..Default::default()
        })
        .padding(pad)
        .width(Length::Fill)
        .height(Length::Fill);

        if self.is_maximized {
            return main.into();
        }

        let (ww, wh) = self.window_size;
        let b: u16 = 5;
        let ww_u = (ww as u16).saturating_sub(b);
        let wh_u = (wh as u16).saturating_sub(b);

        let right_handle: Element<Message> = column![
            Space::with_height(0),
            row![
                Space::with_width(ww_u),
                mouse_area(Space::new(b, Length::Fill))
                    .on_press(Message::WindowResizeStart(ResizeEdge::Right))
                    .interaction(iced::mouse::Interaction::ResizingHorizontally),
            ].height(Length::Fill),
        ].into();

        let bottom_handle: Element<Message> = column![
            Space::with_height(wh_u),
            mouse_area(Space::new(Length::Fill, b))
                .on_press(Message::WindowResizeStart(ResizeEdge::Bottom))
                .interaction(iced::mouse::Interaction::ResizingVertically),
        ].into();

        let cb: u16 = 12;
        let corner_handle: Element<Message> = column![
            Space::with_height((wh as u16).saturating_sub(cb)),
            row![
                Space::with_width((ww as u16).saturating_sub(cb)),
                mouse_area(Space::new(cb, cb))
                    .on_press(Message::WindowResizeStart(ResizeEdge::BottomRight))
                    .interaction(iced::mouse::Interaction::ResizingDiagonallyDown),
            ],
        ].into();

        stack![main, right_handle, bottom_handle, corner_handle].into()
    }

    pub(super) fn view_loading(&self) -> Element<'_, Message> {
        use iced::alignment::Horizontal;
        let dots = match self.loading_tick % 4 {
            0 => "   ",
            1 => ".  ",
            2 => ".. ",
            _ => "...",
        };

        let title_bar = mouse_area(
            container(
                row![
                    iced::widget::image(iced::widget::image::Handle::from_bytes(include_bytes!("../../assets/logo.png").to_vec())).width(16).height(16),
                    Space::with_width(Length::Fill),
                    container(Space::new(16, 16)).padding([4, 6]),
                    Space::with_width(8),
                    self.window_controls(),
                ].spacing(8).align_y(iced::Alignment::Center).padding([8, 10]),
            )
            .style({
                let maximized = self.is_maximized;
                move |_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border { radius: if maximized { 0.0.into() } else { iced::border::top(10.0) }, ..Default::default() },
                    ..Default::default()
                }
            })
            .width(Length::Fill),
        ).on_press(Message::WindowDrag);

        let content = column![
            Space::with_height(Length::Fill),
            text(format!("Unlocking{dots}")).size(16).style(|_t| theme::primary_text()).align_x(Horizontal::Center),
            text("Deriving encryption key").size(12).style(|_t| theme::secondary_text()).align_x(Horizontal::Center),
            Space::with_height(Length::Fill),
        ]
        .spacing(4)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

        column![
            title_bar,
            container(content)
                .style({
                    let maximized = self.is_maximized;
                    move |_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::BG_PRIMARY)),
                        border: iced::Border { radius: if maximized { 0.0.into() } else { iced::border::bottom(10.0) }, ..Default::default() },
                        ..Default::default()
                    }
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ].into()
    }

    pub(super) fn view_main(&self) -> Element<'_, Message> {
        let lock_btn = button(svg(crate::ui::icons::lock_closed()).width(16).height(16))
            .on_press(Message::LockVault)
            .style(theme::icon_button)
            .padding([4, 6]);

        let title_bar = mouse_area(
            container(
                row![
                    iced::widget::image(iced::widget::image::Handle::from_bytes(include_bytes!("../../assets/logo.png").to_vec())).width(16).height(16),
                    Space::with_width(Length::Fill),
                    lock_btn,
                    Space::with_width(8),
                    self.window_controls(),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .padding([8, 10]),
            )
            .style({
                let maximized = self.is_maximized;
                move |_t: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border {
                        radius: if maximized { 0.0.into() } else { iced::border::top(10.0) },
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .width(Length::Fill),
        )
        .on_press(Message::WindowDrag)
        .on_enter(Message::HoverItem(None));

        let tags = tags_panel::view(
            &self.active_view,
            &self.folders,
            self.all_count,
            self.fav_count,
            &self.folder_counts,
            &self.context_menu,
            &self.dragging,
            self.renaming_folder,
            &self.folder_rename_buffer,
            self.hovered_item,
            self.is_maximized,
        );

        let is_folder_view = matches!(self.active_view, ActiveView::Folder(_));
        let active_folder_id = match &self.active_view { ActiveView::Folder(id) => Some(*id), _ => None };
        let notes = notes_list::view(
            &self.notes,
            self.selected_note.as_ref().map(|n| n.id),
            &self.search_query,
            &self.context_menu,
            self.renaming_note,
            &self.rename_buffer,
            is_folder_view,
            &self.subfolders,
            &self.subfolder_notes,
            &self.expanded_folders,
            &self.dragging,
            active_folder_id,
            self.sort_mode,
            self.sort_menu_open,
            &self.multi_selected,
            self.renaming_folder,
            &self.folder_rename_buffer,
            self.ctrl_held,
            self.shift_held,
            &self.multi_selected_folders,
            self.hovered_item,
        );

        let right_panel: Element<Message> = {
            let panel = if self.show_settings {
                settings_view::view(self)
            } else if let Some(ref note) = self.selected_note {
                editor::view(note, &self.editor_title, &self.editor_content, self.editor_dirty, &self.password_data, &self.password_notes_content, self.show_password, self.show_password_gen, &self.password_gen_options, self.copied_field.as_deref(), &self.canvas_editor, &self.canvas_color_editing, self.color_hue, self.color_sat, self.color_lit, self.session_decrypted.contains_key(&note.id), self.encrypting, &self.note_password_input, self.auth_error.as_deref(), self.is_maximized, self.toolbar_move_open, &self.folders, self.setting_font_size, self.editor_search_open, &self.editor_search_query, self.editor_search_index, self.editor_search_case_sensitive, &self.line_editor)
            } else {
                empty_state::view(self.is_maximized)
            };
            mouse_area(panel).on_enter(Message::HoverItem(None)).into()
        };

        let right_panel: Element<Message> = if !self.file_transfers.is_empty() {
            use iced::widget::{stack, container, column, row, text, Space};
            use std::sync::atomic::Ordering;
            let bar_w = 200.0;
            let count = self.file_transfers.len();

            let toast_content: Element<Message> = if count <= 3 {
                let mut toasts = column![].spacing(4);
                for (_id, label, progress_arc) in &self.file_transfers {
                    let pct = progress_arc.load(Ordering::Relaxed) as f32 / 1000.0;
                    let fill_w = (bar_w * pct).max(2.0);
                    let pct_text = format!("{}%", (pct * 100.0) as u32);
                    let toast = container(
                        column![
                            row![
                                text(label.clone()).size(11).style(|_t| theme::primary_text()),
                                Space::with_width(iced::Length::Fill),
                                text(pct_text).size(10).style(|_t| theme::secondary_text()),
                            ].align_y(iced::Alignment::Center),
                            stack![
                                container(Space::new(bar_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(theme::BG_TERTIARY)),
                                    border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                                }),
                                container(Space::new(fill_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(iced::Color::from_rgb(0.18, 0.55, 0.31))),
                                    border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                                }),
                            ]
                        ].spacing(5)
                    )
                    .style(|_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                        border: iced::Border { radius: 8.0.into(), ..Default::default() }, ..Default::default()
                    })
                    .padding([10, 14]).width(230);
                    toasts = toasts.push(toast);
                }
                toasts.into()
            } else {
                let total_pct: f32 = self.file_transfers.iter()
                    .map(|(_, _, p)| p.load(Ordering::Relaxed) as f32 / 1000.0)
                    .sum::<f32>() / count as f32;
                let fill_w = (bar_w * total_pct).max(2.0);
                let done = self.file_transfers.iter().filter(|(_, _, p)| p.load(Ordering::Relaxed) >= 1000).count();
                let label = format!("Processing {} files ({}/{})", count, done, count);

                container(
                    column![
                        row![
                            text(label).size(11).style(|_t| theme::primary_text()),
                            Space::with_width(iced::Length::Fill),
                            text(format!("{}%", (total_pct * 100.0) as u32)).size(10).style(|_t| theme::secondary_text()),
                        ].align_y(iced::Alignment::Center),
                        stack![
                            container(Space::new(bar_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(theme::BG_TERTIARY)),
                                border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                            }),
                            container(Space::new(fill_w, 3)).style(|_t: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(iced::Color::from_rgb(0.18, 0.55, 0.31))),
                                border: iced::Border { radius: 1.5.into(), ..Default::default() }, ..Default::default()
                            }),
                        ]
                    ].spacing(5)
                )
                .style(|_t: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::BG_SECONDARY)),
                    border: iced::Border { radius: 8.0.into(), ..Default::default() }, ..Default::default()
                })
                .padding([10, 14]).width(230).into()
            };

            let overlay = container(
                container(toast_content).padding([8, 8])
            )
            .width(iced::Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top);

            stack![right_panel, overlay].into()
        } else {
            right_panel
        };

        let content_row: Element<Message> = if self.show_sidebar {
            row![tags, notes, right_panel].into()
        } else {
            row![right_panel].into()
        };
        let mut main_layout: Element<Message> = column![
            title_bar,
            content_row,
        ].into();

        if let Some(ref drag_item) = self.dragging {
            let (label, ghost_color) = match drag_item {
                DragItem::Note(id) => {
                    let n = self.notes.iter().find(|n| n.id == *id);
                    let multi = self.multi_selected.len() + self.multi_selected_folders.len();
                    let name = if multi > 1 && self.multi_selected.contains(id) {
                        format!("{} items", multi)
                    } else {
                        n.map(|n| if n.title.is_empty() { "Untitled".to_string() } else { n.title.clone() }).unwrap_or_default()
                    };
                    let color = n.map(|n| n.color.to_iced_color()).unwrap_or(iced::Color::from_rgb(0.3, 0.3, 0.35));
                    (name, color)
                }
                DragItem::Folder(id) => {
                    let f = self.folders.iter().chain(self.subfolders.iter()).find(|f| f.id == *id);
                    let multi = self.multi_selected.len() + self.multi_selected_folders.len();
                    let name = if multi > 1 && self.multi_selected_folders.contains(id) {
                        format!("{} items", multi)
                    } else {
                        f.map(|f| f.name.clone()).unwrap_or_default()
                    };
                    let color = f.map(|f| f.color.to_iced_color()).unwrap_or(iced::Color::from_rgb(0.3, 0.3, 0.35));
                    (name, color)
                }
            };
            let (cx, cy) = self.cursor_pos;
            let ghost = container(
                text(label).size(11).style(|_t| iced::widget::text::Style { color: Some(iced::Color::WHITE) })
            )
            .style(move |_t: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(ghost_color.r, ghost_color.g, ghost_color.b, 0.85))),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .padding([3, 8]);

            let ghost_positioned: Element<Message> = column![
                Space::with_height(cy.max(0.0) as u16),
                row![Space::with_width(cx.max(0.0) as u16 + 10), container(ghost).width(Length::Shrink)].width(Length::Shrink),
            ].into();

            main_layout = stack![main_layout, ghost_positioned].into();
        }

        let mut result: Element<Message> = if let Some(ref dialog_kind) = self.active_dialog {
            let dialog_view = self.view_dialog(dialog_kind);
            let da = self.dialog_anim;
            let blocking_overlay: Element<Message> = mouse_area(
                container(
                    container(Space::new(Length::Fill, Length::Fill))
                        .style(move |_t: &iced::Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35 * da))),
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .height(Length::Fill)
                ).width(Length::Fill).height(Length::Fill)
            )
            .on_press(Message::CloseDialog)
            .on_right_press(Message::CloseDialog)
            .on_enter(Message::HoverItem(None))
            .into();
            stack![main_layout, stack![blocking_overlay, dialog_view]].into()
        } else if let Some(ref ctx) = self.context_menu {
            let ctx_overlay = self.view_context_menu(ctx);
            stack![main_layout, ctx_overlay].into()
        } else {
            main_layout
        };

        if let Some(toast_time) = self.zoom_toast {
            if toast_time.elapsed() < Duration::from_millis(1200) {
                let opacity = if toast_time.elapsed() > Duration::from_millis(800) {
                    1.0 - (toast_time.elapsed().as_millis() as f32 - 800.0) / 400.0
                } else { 1.0 };
                let zoom_label = format!("{}%", (self.gui_scale * 100.0).round() as u32);
                let toast_pill = container(
                    text(zoom_label).size(11).style(move |_t| iced::widget::text::Style {
                        color: Some(iced::Color::from_rgba(
                            0x8D as f32 / 255.0, 0x8D as f32 / 255.0, 0x8D as f32 / 255.0, opacity
                        )),
                    })
                )
                .style(move |_t: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0x28 as f32 / 255.0, 0.9 * opacity
                    ))),
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .padding([5, 10]);
                let toast: Element<Message> = column![
                    Space::with_height(42),
                    row![Space::with_width(Length::Fill), toast_pill, Space::with_width(12)],
                ].into();
                result = stack![result, toast].into();
            }
        }
        result
    }
}
