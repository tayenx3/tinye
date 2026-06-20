use std::str::FromStr;

use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorScheme {
    pub fg: Color,
    pub bg: Color,
    pub gutter_fg: Color,
    pub gutter_bg: Color,
    pub line_num_fg: Color,
    pub status_bar_fg: Color,
    pub status_bar_bg: Color,
}

impl ColorScheme {
    pub const DEFAULT: Self = Self {
        fg:            Color::White,
        bg:            Color::Reset,
        gutter_fg:     Color::DarkGrey,
        gutter_bg:     Color::Black,
        line_num_fg:   Color::Yellow,
        status_bar_fg: Color::Black,
        status_bar_bg: Color::Cyan,
    };
    pub const WILD_ROSES: Self = Self {
        fg:            Color::Rgb { r: 0xD9, g: 0xD0, b: 0xDE },
        bg:            Color::Rgb { r: 0x0C, g: 0x17, b: 0x13 },
        gutter_fg:     Color::Rgb { r: 0xBC, g: 0x8D, b: 0xA0 },
        gutter_bg:     Color::Rgb { r: 0xA0, g: 0x46, b: 0x68 },
        line_num_fg:   Color::Rgb { r: 0x0C, g: 0x17, b: 0x13 },
        status_bar_bg: Color::Rgb { r: 0xD9, g: 0xD0, b: 0xDE },
        status_bar_fg: Color::Rgb { r: 0x0C, g: 0x17, b: 0x13 },
    };
    pub const CERULEAN: Self = Self {
        fg:            Color::Rgb { r: 0xFF, g: 0xFF, b: 0xFF },
        bg:            Color::Rgb { r: 0x00, g: 0x17, b: 0x1F },
        gutter_fg:     Color::Rgb { r: 0x00, g: 0xA8, b: 0xE8 },
        gutter_bg:     Color::Rgb { r: 0x00, g: 0x7E, b: 0xA7 },
        line_num_fg:   Color::Rgb { r: 0x00, g: 0x34, b: 0x59 },
        status_bar_bg: Color::Rgb { r: 0x00, g: 0xA8, b: 0xE8 },
        status_bar_fg: Color::Rgb { r: 0x00, g: 0x17, b: 0x1F },
    };
    pub const GLACIER: Self = Self {
        fg:            Color::Rgb { r: 0xF8, g: 0xF8, b: 0xF8 },
        bg:            Color::Rgb { r: 0x00, g: 0x01, b: 0x00 },
        gutter_fg:     Color::Rgb { r: 0x51, g: 0x56, b: 0x64 },
        gutter_bg:     Color::Rgb { r: 0x94, g: 0xC5, b: 0xCC },
        line_num_fg:   Color::Rgb { r: 0x00, g: 0x01, b: 0x00 },
        status_bar_bg: Color::Rgb { r: 0xF8, g: 0xF8, b: 0xF8 },
        status_bar_fg: Color::Rgb { r: 0x6B, g: 0x86, b: 0x90 },
    };
}

impl FromStr for ColorScheme {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_lowercase() {
            "wild-roses" | "wild roses" | "wildroses" => Ok(Self::WILD_ROSES),
            "cerulean" => Ok(Self::CERULEAN),
            "glacier" => Ok(Self::GLACIER),
            _ => Ok(Self::DEFAULT)
        }
    }
}