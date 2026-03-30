use iced::widget::svg;

fn handle(svg_data: &'static [u8]) -> svg::Handle {
    svg::Handle::from_memory(svg_data)
}

macro_rules! icon {
    ($name:ident, $svg:expr) => {
        #[allow(dead_code)]
        pub fn $name() -> svg::Handle {
            handle($svg)
        }
    };
}

icon!(settings_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='3'/><path d='M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z'/></svg>");

icon!(win_minimize, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#282828' stroke-width='5' stroke-linecap='round'><line x1='6' y1='12' x2='18' y2='12'/></svg>");
icon!(win_maximize, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#282828' stroke-width='4' stroke-linecap='round' stroke-linejoin='round'><polyline points='15 3 21 3 21 9'/><polyline points='9 21 3 21 3 15'/><line x1='21' y1='3' x2='14' y2='10'/><line x1='3' y1='21' x2='10' y2='14'/></svg>");
icon!(win_close, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#282828' stroke-width='5' stroke-linecap='round'><line x1='6' y1='6' x2='18' y2='18'/><line x1='18' y1='6' x2='6' y2='18'/></svg>");
icon!(close_light, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='4' stroke-linecap='round'><line x1='6' y1='6' x2='18' y2='18'/><line x1='18' y1='6' x2='6' y2='18'/></svg>");


icon!(plus_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round'><line x1='12' y1='5' x2='12' y2='19'/><line x1='5' y1='12' x2='19' y2='12'/></svg>");

icon!(plus_bright, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#D9D9D9' stroke-width='3' stroke-linecap='round'><line x1='12' y1='5' x2='12' y2='19'/><line x1='5' y1='12' x2='19' y2='12'/></svg>");

icon!(search_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round'><circle cx='11' cy='11' r='7'/><line x1='16.5' y1='16.5' x2='21' y2='21'/></svg>");

icon!(save_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><path d='M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4'/><polyline points='7 10 12 15 17 10'/><line x1='12' y1='15' x2='12' y2='3'/></svg>");

icon!(crosshair_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#D9D9D9' stroke-width='2.5' stroke-linecap='round'><circle cx='12' cy='12' r='8'/><line x1='12' y1='2' x2='12' y2='6'/><line x1='12' y1='18' x2='12' y2='22'/><line x1='2' y1='12' x2='6' y2='12'/><line x1='18' y1='12' x2='22' y2='12'/></svg>");
icon!(fit_view_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#D9D9D9' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><polyline points='15 3 21 3 21 9'/><polyline points='9 21 3 21 3 15'/><polyline points='21 3 14 10'/><polyline points='3 21 10 14'/></svg>");

icon!(fmt_bold, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><path d='M6 4h8a4 4 0 014 4 4 4 0 01-4 4H6z'/><path d='M6 12h9a4 4 0 014 4 4 4 0 01-4 4H6z'/></svg>");
icon!(fmt_italic, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><line x1='19' y1='4' x2='10' y2='4'/><line x1='14' y1='20' x2='5' y2='20'/><line x1='15' y1='4' x2='9' y2='20'/></svg>");
icon!(fmt_heading, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><line x1='4' y1='4' x2='4' y2='20'/><line x1='18' y1='4' x2='18' y2='20'/><line x1='4' y1='12' x2='18' y2='12'/></svg>");
icon!(fmt_list, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round'><line x1='9' y1='6' x2='20' y2='6'/><line x1='9' y1='12' x2='20' y2='12'/><line x1='9' y1='18' x2='20' y2='18'/><circle cx='5' cy='6' r='1' fill='#8D8D8D'/><circle cx='5' cy='12' r='1' fill='#8D8D8D'/><circle cx='5' cy='18' r='1' fill='#8D8D8D'/></svg>");
icon!(fmt_checkbox, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='5' width='14' height='14' rx='2'/><polyline points='7 12 10 15 16 9'/></svg>");
icon!(fmt_code, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><polyline points='16 18 22 12 16 6'/><polyline points='8 6 2 12 8 18'/></svg>");
icon!(fmt_quote, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none'><path d='M10 8c-2.2 0-4 1.8-4 4v4h4v-4H8c0-1.1.9-2 2-2V8zm8 0c-2.2 0-4 1.8-4 4v4h4v-4h-2c0-1.1.9-2 2-2V8z'/></svg>");
icon!(fmt_divider, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round'><line x1='3' y1='12' x2='21' y2='12'/></svg>");

icon!(chevron_right, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='9 18 15 12 9 6'/></svg>");
icon!(chevron_up, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 15 12 9 18 15'/></svg>");
icon!(chevron_down, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'/></svg>");

icon!(copy_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='9' y='9' width='13' height='13' rx='2' ry='2'/><path d='M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1'/></svg>");
icon!(copy_check, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#4DC86A' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='20 6 9 17 4 12'/></svg>");
icon!(eye_open, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><path d='M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z'/><circle cx='12' cy='12' r='3'/></svg>");
icon!(eye_closed, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><path d='M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24'/><line x1='1' y1='1' x2='23' y2='23'/></svg>");
icon!(dice_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='2' y='2' width='20' height='20' rx='3'/><circle cx='8' cy='8' r='1.2' fill='#8D8D8D'/><circle cx='16' cy='8' r='1.2' fill='#8D8D8D'/><circle cx='8' cy='16' r='1.2' fill='#8D8D8D'/><circle cx='16' cy='16' r='1.2' fill='#8D8D8D'/><circle cx='12' cy='12' r='1.2' fill='#8D8D8D'/></svg>");
icon!(dice_icon_white, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#FFFFFF' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='2' y='2' width='20' height='20' rx='3'/><circle cx='8' cy='8' r='1.2' fill='#FFFFFF'/><circle cx='16' cy='8' r='1.2' fill='#FFFFFF'/><circle cx='8' cy='16' r='1.2' fill='#FFFFFF'/><circle cx='16' cy='16' r='1.2' fill='#FFFFFF'/><circle cx='12' cy='12' r='1.2' fill='#FFFFFF'/></svg>");

icon!(open_in_new_window, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><path d='M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6'/><polyline points='15 3 21 3 21 9'/><line x1='10' y1='14' x2='21' y2='3'/></svg>");


icon!(folder_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none'><path d='M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z'/></svg>");

icon!(move_folder_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none' stroke-linecap='round' stroke-linejoin='round'><path d='M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z'/><line x1='9' y1='14' x2='15' y2='14' stroke='#1F1F1F' stroke-width='2'/><polyline points='12 11 15 14 12 17' stroke='#1F1F1F' stroke-width='2'/></svg>");


icon!(star_outline, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none'><polygon points='12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2'/></svg>");

icon!(star_filled, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#E5D54D' stroke='none'><polygon points='12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2'/></svg>");


icon!(pin_outline, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='#8D8D8D' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><line x1='12' y1='17' x2='12' y2='24' stroke-width='2.5'/><path d='M5 17h14v-1.76a2 2 0 00-1.11-1.79l-1.78-.89A2 2 0 0115 10.76V6h1a2 2 0 000-4H8a2 2 0 000 4h1v4.76a2 2 0 01-1.11 1.79l-1.78.89A2 2 0 005 15.24z'/></svg>");

icon!(pin_filled, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#D9D9D9' stroke='#D9D9D9' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><line x1='12' y1='17' x2='12' y2='24' stroke-width='2.5'/><path d='M5 17h14v-1.76a2 2 0 00-1.11-1.79l-1.78-.89A2 2 0 0115 10.76V6h1a2 2 0 000-4H8a2 2 0 000 4h1v4.76a2 2 0 01-1.11 1.79l-1.78.89A2 2 0 005 15.24z'/></svg>");


icon!(lock_closed, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='11' width='18' height='11' rx='2' ry='2' fill='#8D8D8D' stroke-width='2.5'/><path d='M7 11V7a5 5 0 0110 0v4' stroke-width='3.5'/></svg>");

icon!(lock_active, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#E5D54D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='11' width='18' height='11' rx='2' ry='2' fill='#E5D54D'/><path d='M7 11V7a5 5 0 0110 0v4'/></svg>");

icon!(unlock_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='11' width='18' height='11' rx='2' ry='2' fill='#8D8D8D'/><path d='M7 11V7a5 5 0 019.9-1'/></svg>");


icon!(trash_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><polyline points='3 6 5 6 21 6'/><path d='M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2' fill='#8D8D8D'/></svg>");

icon!(trash_danger, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#E54D4D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><polyline points='3 6 5 6 21 6'/><path d='M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2' fill='#E54D4D'/></svg>");
icon!(trash_muted, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#6B3333' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><polyline points='3 6 5 6 21 6'/><path d='M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2' fill='#6B3333'/></svg>");


icon!(pencil_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='#8D8D8D' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7'/><path d='M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z'/></svg>");

icon!(document_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='#8D8D8D' stroke-width='1' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z'/><polyline points='14 2 14 8 20 8' fill='none' stroke-width='2'/><line x1='16' y1='13' x2='8' y2='13' stroke='#1F1F1F' stroke-width='2'/><line x1='16' y1='17' x2='8' y2='17' stroke='#1F1F1F' stroke-width='2'/></svg>");

icon!(document_lock, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='#8D8D8D' stroke-width='1' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z'/><polyline points='14 2 14 8 20 8' fill='none' stroke-width='2'/><rect x='8' y='13' width='8' height='6' rx='1' fill='#1F1F1F'/><path d='M10 13v-1a2 2 0 014 0v1' fill='none' stroke='#1F1F1F' stroke-width='2'/></svg>");

icon!(key_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8D8D8D' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><path d='M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.78 7.78 5.5 5.5 0 017.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4'/></svg>");

icon!(unlock_danger, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#E54D4D' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='11' width='18' height='11' rx='2' ry='2' fill='#E54D4D'/><path d='M7 11V7a5 5 0 019.9-1'/><line x1='8' y1='15' x2='16' y2='19'/><line x1='16' y1='15' x2='8' y2='19'/></svg>");

icon!(palette_icon, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#8D8D8D' stroke='none'><circle cx='13.5' cy='6.5' r='2.5'/><circle cx='17.5' cy='10.5' r='2.5'/><circle cx='8.5' cy='7.5' r='2.5'/><circle cx='6.5' cy='12.5' r='2.5'/><path d='M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.93 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.04-.23-.29-.38-.63-.38-1.01 0-.83.67-1.5 1.5-1.5H16c3.31 0 6-2.69 6-6 0-5.17-4.49-9-10-9z' fill='none' stroke='#8D8D8D' stroke-width='2.5'/></svg>");


/// Folder icon — colored
pub fn folder_colored(color: iced::Color) -> svg::Handle {
    let hex = format!("{:02X}{:02X}{:02X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    let svg_str = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#{hex}' stroke='none'><path d='M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z'/></svg>"
    );
    svg::Handle::from_memory(svg_str.into_bytes())
}

/// Simple text lines icon — colored
pub fn note_text_icon(color: iced::Color) -> svg::Handle {
    let hex = format!("{:02X}{:02X}{:02X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    let svg_str = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#{hex}' stroke-width='2.5' stroke-linecap='round'><line x1='4' y1='7' x2='20' y2='7'/><line x1='4' y1='12' x2='16' y2='12'/><line x1='4' y1='17' x2='12' y2='17'/></svg>"
    );
    svg::Handle::from_memory(svg_str.into_bytes())
}

/// Simple lock/key icon — colored
pub fn note_password_icon(color: iced::Color) -> svg::Handle {
    let hex = format!("{:02X}{:02X}{:02X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    let svg_str = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#{hex}' stroke='#{hex}' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect x='5' y='11' width='14' height='10' rx='2'/><path d='M8 11V7a4 4 0 018 0v4' fill='none' stroke-width='2.5'/></svg>"
    );
    svg::Handle::from_memory(svg_str.into_bytes())
}

/// Simple canvas/grid icon — colored
pub fn note_canvas_icon(color: iced::Color) -> svg::Handle {
    let hex = format!("{:02X}{:02X}{:02X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    let svg_str = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#{hex}' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><rect x='3' y='3' width='8' height='8' rx='1'/><rect x='13' y='13' width='8' height='8' rx='1'/><line x1='7' y1='11' x2='7' y2='13'/><line x1='17' y1='11' x2='17' y2='13'/></svg>"
    );
    svg::Handle::from_memory(svg_str.into_bytes())
}

pub fn note_file_icon(color: iced::Color) -> svg::Handle {
    let hex = format!("{:02X}{:02X}{:02X}", (color.r * 255.0) as u8, (color.g * 255.0) as u8, (color.b * 255.0) as u8);
    let svg_str = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#{hex}' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z'/><polyline points='14 2 14 8 20 8'/><line x1='16' y1='13' x2='8' y2='13'/><line x1='16' y1='17' x2='8' y2='17'/></svg>"
    );
    svg::Handle::from_memory(svg_str.into_bytes())
}

icon!(file_large, b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#6B6B70' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z'/><polyline points='14 2 14 8 20 8'/><line x1='16' y1='13' x2='8' y2='13'/><line x1='16' y1='17' x2='8' y2='17'/></svg>");
