use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderColor {
    Red, Orange, Yellow, Green, Blue, Purple, Gray,
    Pink, Teal, Indigo, Lime, Amber, Cyan, Rose,
    Violet, Emerald, Sky, Coral, Slate, Mint,
}

impl FolderColor {
    /// Full palette for color pickers.
    pub const PALETTE: &[FolderColor] = &[
        Self::Red, Self::Rose, Self::Pink, Self::Orange,
        Self::Amber, Self::Yellow, Self::Lime, Self::Green,
        Self::Emerald, Self::Mint, Self::Teal, Self::Cyan,
        Self::Sky, Self::Blue, Self::Indigo, Self::Violet,
        Self::Purple, Self::Coral, Self::Gray, Self::Slate,
    ];

    /// Subset for quick selectors.
    #[allow(dead_code)]
    pub const ALL: [FolderColor; 7] = [
        Self::Green, Self::Orange, Self::Red,
        Self::Blue, Self::Purple, Self::Yellow, Self::Gray,
    ];

    pub fn to_iced_color(self) -> iced::Color {
        match self {
            Self::Red     => iced::Color::from_rgb8(0xE5, 0x4D, 0x4D),
            Self::Rose    => iced::Color::from_rgb8(0xF4, 0x3F, 0x5E),
            Self::Pink    => iced::Color::from_rgb8(0xEC, 0x48, 0x99),
            Self::Orange  => iced::Color::from_rgb8(0xE5, 0x9E, 0x4D),
            Self::Amber   => iced::Color::from_rgb8(0xF5, 0x9E, 0x0B),
            Self::Yellow  => iced::Color::from_rgb8(0xE5, 0xD5, 0x4D),
            Self::Lime    => iced::Color::from_rgb8(0x84, 0xCC, 0x16),
            Self::Green   => iced::Color::from_rgb8(0x4D, 0xC8, 0x6A),
            Self::Emerald => iced::Color::from_rgb8(0x10, 0xB9, 0x81),
            Self::Mint    => iced::Color::from_rgb8(0x34, 0xD3, 0x99),
            Self::Teal    => iced::Color::from_rgb8(0x14, 0xB8, 0xA6),
            Self::Cyan    => iced::Color::from_rgb8(0x06, 0xB6, 0xD4),
            Self::Sky     => iced::Color::from_rgb8(0x38, 0xBD, 0xF8),
            Self::Blue    => iced::Color::from_rgb8(0x4D, 0x9E, 0xE5),
            Self::Indigo  => iced::Color::from_rgb8(0x63, 0x66, 0xF1),
            Self::Violet  => iced::Color::from_rgb8(0x8B, 0x5C, 0xF6),
            Self::Purple  => iced::Color::from_rgb8(0x9E, 0x4D, 0xE5),
            Self::Coral   => iced::Color::from_rgb8(0xFB, 0x71, 0x85),
            Self::Gray    => iced::Color::from_rgb8(0x6B, 0x72, 0x80),
            Self::Slate   => iced::Color::from_rgb8(0x47, 0x55, 0x69),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red", Self::Rose => "Rose", Self::Pink => "Pink",
            Self::Orange => "Orange", Self::Amber => "Amber", Self::Yellow => "Yellow",
            Self::Lime => "Lime", Self::Green => "Green", Self::Emerald => "Emerald",
            Self::Mint => "Mint", Self::Teal => "Teal", Self::Cyan => "Cyan",
            Self::Sky => "Sky", Self::Blue => "Blue", Self::Indigo => "Indigo",
            Self::Violet => "Violet", Self::Purple => "Purple", Self::Coral => "Coral",
            Self::Gray => "Gray", Self::Slate => "Slate",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Red" => Self::Red, "Rose" => Self::Rose, "Pink" => Self::Pink,
            "Orange" => Self::Orange, "Amber" => Self::Amber, "Yellow" => Self::Yellow,
            "Lime" => Self::Lime, "Green" => Self::Green, "Emerald" => Self::Emerald,
            "Mint" => Self::Mint, "Teal" => Self::Teal, "Cyan" => Self::Cyan,
            "Sky" => Self::Sky, "Blue" => Self::Blue, "Indigo" => Self::Indigo,
            "Violet" => Self::Violet, "Purple" => Self::Purple, "Coral" => Self::Coral,
            "Slate" => Self::Slate, _ => Self::Gray,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub color: FolderColor,
    pub sort_order: i32,
    pub collapsed: bool,
    pub is_favorite: bool,
}

impl Folder {
    pub fn new(name: String, color: FolderColor, parent_id: Option<Uuid>) -> Self {
        Self { id: Uuid::new_v4(), parent_id, name, color, sort_order: 0, collapsed: false, is_favorite: false }
    }
}

/// Build tree structure: returns (root folders, children map).
#[allow(dead_code)]
pub fn build_tree(folders: &[Folder]) -> Vec<(usize, &Folder)> {
    let mut result = Vec::new();
    fn collect<'a>(folders: &'a [Folder], parent: Option<Uuid>, depth: usize, result: &mut Vec<(usize, &'a Folder)>) {
        for f in folders {
            if f.parent_id == parent {
                result.push((depth, f));
                collect(folders, Some(f.id), depth + 1, result);
            }
        }
    }
    collect(folders, None, 0, &mut result);
    result
}
