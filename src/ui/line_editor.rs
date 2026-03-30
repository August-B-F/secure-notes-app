use iced::Element;

use crate::app::Message;
use crate::ui::md_widget::{self, MdEditorState};

// Re-export
pub use crate::ui::md_widget::MdEditorState as LineEditorState;

// Compatibility shim — the real state is MdEditorState
impl MdEditorState {
    /// No-op compatibility
    pub fn sync_active_to_lines(&mut self) {}
    pub fn activate(&mut self, _index: usize) {}
    pub fn deactivate(&mut self) {}
    pub fn sync_to_lines(&mut self) {}
}

pub fn view<'a>(
    state: &'a MdEditorState,
    font_size: u32,
) -> Element<'a, Message> {
    md_widget::md_editor(state, |action| Message::MdEdit(action))
        .size(font_size as f32)
        .into()
}
