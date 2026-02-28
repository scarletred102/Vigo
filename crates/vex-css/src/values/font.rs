// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! CSS font property values.

/// CSS font-family value (ordered list of family names).
#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily(pub Vec<String>);

impl FontFamily {
    pub fn parse(input: &str) -> Self {
        let families = input
            .split(',')
            .map(|s| {
                let s = s.trim();
                // Strip quotes
                s.trim_matches(|c| c == '"' || c == '\'').to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        Self(families)
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self(vec!["sans-serif".to_string()])
    }
}

/// CSS font-weight (100–900 or named).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const NORMAL: Self = Self(400);
    pub const BOLD: Self = Self(700);

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "normal" => Some(Self::NORMAL),
            "bold" => Some(Self::BOLD),
            "lighter" => Some(Self(100)),
            "bolder" => Some(Self(900)),
            s => s.parse::<u16>().ok().map(|v| Self(v.clamp(1, 1000))),
        }
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// CSS font-style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "normal" => Some(Self::Normal),
            "italic" => Some(Self::Italic),
            "oblique" => Some(Self::Oblique),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_font_family() {
        let f = FontFamily::parse("\"Helvetica Neue\", Arial, sans-serif");
        assert_eq!(f.0, vec!["Helvetica Neue", "Arial", "sans-serif"]);
    }

    #[test]
    fn parse_font_weight_named() {
        assert_eq!(FontWeight::parse("bold"), Some(FontWeight::BOLD));
        assert_eq!(FontWeight::parse("normal"), Some(FontWeight::NORMAL));
    }

    #[test]
    fn parse_font_weight_numeric() {
        assert_eq!(FontWeight::parse("600"), Some(FontWeight(600)));
    }

    #[test]
    fn parse_font_style() {
        assert_eq!(FontStyle::parse("italic"), Some(FontStyle::Italic));
        assert_eq!(FontStyle::parse("normal"), Some(FontStyle::Normal));
    }
}
