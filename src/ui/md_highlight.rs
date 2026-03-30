use iced::{Color, Font};
use iced::font;
use std::ops::Range;

/// Settings for the markdown highlighter.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Settings;

/// A highlight produced by the markdown highlighter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Highlight {
    Normal,
    Bold,
    Italic,
    BoldItalic,
    Heading,
    Code,
    Marker,   // the `**`, `*`, `##`, etc. characters
    Link,
    Quote,
    ListMarker,
}

#[allow(dead_code)]
impl Highlight {
    pub fn to_format(&self) -> iced::advanced::text::highlighter::Format<Font> {
        match self {
            Highlight::Normal => iced::advanced::text::highlighter::Format {
                color: None,
                font: None,
            },
            Highlight::Bold => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.9, 0.9, 0.92)),
                font: Some(Font {
                    weight: font::Weight::Bold,
                    ..Font::DEFAULT
                }),
            },
            Highlight::Italic => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.85, 0.85, 0.9)),
                font: Some(Font {
                    style: font::Style::Italic,
                    ..Font::DEFAULT
                }),
            },
            Highlight::BoldItalic => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.9, 0.9, 0.92)),
                font: Some(Font {
                    weight: font::Weight::Bold,
                    style: font::Style::Italic,
                    ..Font::DEFAULT
                }),
            },
            Highlight::Heading => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.55, 0.75, 0.95)),
                font: Some(Font {
                    weight: font::Weight::Bold,
                    ..Font::DEFAULT
                }),
            },
            Highlight::Code => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.6, 0.85, 0.6)),
                font: Some(Font::MONOSPACE),
            },
            Highlight::Marker => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.22, 0.22, 0.24)),
                font: None,
            },
            Highlight::Link => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.45, 0.7, 0.9)),
                font: None,
            },
            Highlight::Quote => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.65, 0.65, 0.7)),
                font: Some(Font {
                    style: font::Style::Italic,
                    ..Font::DEFAULT
                }),
            },
            Highlight::ListMarker => iced::advanced::text::highlighter::Format {
                color: Some(Color::from_rgb(0.55, 0.75, 0.95)),
                font: Some(Font {
                    weight: font::Weight::Bold,
                    ..Font::DEFAULT
                }),
            },
        }
    }
}

/// Custom markdown highlighter that renders bold/italic with actual font changes.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MdHighlighter;

impl iced::advanced::text::highlighter::Highlighter for MdHighlighter {
    type Settings = Settings;
    type Highlight = Highlight;
    type Iterator<'a> = Box<dyn Iterator<Item = (Range<usize>, Self::Highlight)> + 'a>;

    fn new(_settings: &Self::Settings) -> Self {
        MdHighlighter
    }

    fn update(&mut self, _new_settings: &Self::Settings) {}

    fn change_line(&mut self, _line: usize) {}

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        Box::new(highlight_line(line).into_iter())
    }

    fn current_line(&self) -> usize {
        0
    }
}

#[allow(dead_code)]
pub fn highlight_line(line: &str) -> Vec<(Range<usize>, Highlight)> {
    let trimmed = line.trim_start();

    if trimmed.starts_with('#') {
        let hash_count = trimmed.chars().take_while(|c| *c == '#').count();
        if hash_count <= 6 && trimmed.get(hash_count..hash_count+1) == Some(" ") {
            let prefix_len = line.len() - trimmed.len();
            return vec![
                (0..prefix_len + hash_count + 1, Highlight::Marker),
                (prefix_len + hash_count + 1..line.len(), Highlight::Heading),
            ];
        }
    }

    if trimmed.starts_with("> ") {
        let prefix_len = line.len() - trimmed.len();
        return vec![
            (0..prefix_len + 2, Highlight::Marker),
            (prefix_len + 2..line.len(), Highlight::Quote),
        ];
    }

    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return vec![(0..line.len(), Highlight::Marker)];
    }

    let list_prefix = if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") {
        Some(6)
    } else if trimmed.starts_with("- ") {
        Some(2)
    } else if trimmed.len() >= 3 && trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) && trimmed.contains(". ") {
        trimmed.find(". ").map(|p| p + 2)
    } else {
        None
    };
    if let Some(marker_len) = list_prefix {
        let prefix_len = line.len() - trimmed.len();
        let mut spans = vec![(0..prefix_len + marker_len, Highlight::ListMarker)];
        spans.extend(highlight_inline(&line[prefix_len + marker_len..], prefix_len + marker_len));
        return spans;
    }

    highlight_inline(line, 0)
}

pub fn highlight_inline(text: &str, offset: usize) -> Vec<(Range<usize>, Highlight)> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut normal_start = 0;

    while i < len {
        if i + 2 < len && &bytes[i..i+3] == b"***" {
            if let Some(end) = find_closing(text, i + 3, "***") {
                push_normal(&mut spans, offset, normal_start, i);
                spans.push((offset + i..offset + i + 3, Highlight::Marker));
                spans.push((offset + i + 3..offset + end, Highlight::BoldItalic));
                spans.push((offset + end..offset + end + 3, Highlight::Marker));
                i = end + 3;
                normal_start = i;
                continue;
            }
        }
        if i + 1 < len && &bytes[i..i+2] == b"**" && (i + 2 >= len || bytes[i+2] != b'*') {
            if let Some(end) = find_closing(text, i + 2, "**") {
                push_normal(&mut spans, offset, normal_start, i);
                spans.push((offset + i..offset + i + 2, Highlight::Marker));
                spans.push((offset + i + 2..offset + end, Highlight::Bold));
                spans.push((offset + end..offset + end + 2, Highlight::Marker));
                i = end + 2;
                normal_start = i;
                continue;
            }
        }
        if bytes[i] == b'*' && (i + 1 >= len || bytes[i+1] != b'*') {
            if let Some(end) = find_closing_char(text, i + 1, b'*') {
                if end > i + 1 {
                    push_normal(&mut spans, offset, normal_start, i);
                    spans.push((offset + i..offset + i + 1, Highlight::Marker));
                    spans.push((offset + i + 1..offset + end, Highlight::Italic));
                    spans.push((offset + end..offset + end + 1, Highlight::Marker));
                    i = end + 1;
                    normal_start = i;
                    continue;
                }
            }
        }
        if bytes[i] == b'`' {
            if let Some(end) = find_closing_char(text, i + 1, b'`') {
                push_normal(&mut spans, offset, normal_start, i);
                spans.push((offset + i..offset + i + 1, Highlight::Marker));
                spans.push((offset + i + 1..offset + end, Highlight::Code));
                spans.push((offset + end..offset + end + 1, Highlight::Marker));
                i = end + 1;
                normal_start = i;
                continue;
            }
        }
        if bytes[i] == b'[' {
            if let Some(bracket_end) = find_closing_char(text, i + 1, b']') {
                if bracket_end + 1 < len && bytes[bracket_end + 1] == b'(' {
                    if let Some(paren_end) = find_closing_char(text, bracket_end + 2, b')') {
                        push_normal(&mut spans, offset, normal_start, i);
                        spans.push((offset + i..offset + paren_end + 1, Highlight::Link));
                        i = paren_end + 1;
                        normal_start = i;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    push_normal(&mut spans, offset, normal_start, len);
    spans
}

fn push_normal(spans: &mut Vec<(Range<usize>, Highlight)>, offset: usize, start: usize, end: usize) {
    if end > start {
        spans.push((offset + start..offset + end, Highlight::Normal));
    }
}

fn find_closing(text: &str, start: usize, marker: &str) -> Option<usize> {
    text[start..].find(marker).map(|p| start + p)
}

fn find_closing_char(text: &str, start: usize, ch: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    for i in start..bytes.len() {
        if bytes[i] == ch { return Some(i); }
    }
    None
}
