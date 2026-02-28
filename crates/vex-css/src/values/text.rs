// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS text property values.

/// CSS `text-align`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}

impl TextAlign {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "left" | "start" => Some(Self::Left),
            "right" | "end" => Some(Self::Right),
            "center" => Some(Self::Center),
            "justify" => Some(Self::Justify),
            _ => None,
        }
    }
}

/// CSS `text-decoration`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    Overline,
    LineThrough,
}

impl TextDecoration {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "none" => Some(Self::None),
            "underline" => Some(Self::Underline),
            "overline" => Some(Self::Overline),
            "line-through" => Some(Self::LineThrough),
            _ => None,
        }
    }
}

/// CSS `white-space`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WhiteSpace {
    #[default]
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

impl WhiteSpace {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "normal" => Some(Self::Normal),
            "nowrap" => Some(Self::NoWrap),
            "pre" => Some(Self::Pre),
            "pre-wrap" => Some(Self::PreWrap),
            "pre-line" => Some(Self::PreLine),
            _ => None,
        }
    }
}

/// CSS `overflow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Overflow {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "scroll" => Some(Self::Scroll),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// CSS `vertical-align`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlign {
    #[default]
    Baseline,
    Top,
    Middle,
    Bottom,
    TextTop,
    TextBottom,
}

impl VerticalAlign {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "baseline" => Some(Self::Baseline),
            "top" => Some(Self::Top),
            "middle" => Some(Self::Middle),
            "bottom" => Some(Self::Bottom),
            "text-top" => Some(Self::TextTop),
            "text-bottom" => Some(Self::TextBottom),
            _ => None,
        }
    }
}

/// CSS `cursor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cursor {
    #[default]
    Auto,
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
    Crosshair,
    Wait,
    Help,
    Grab,
    Grabbing,
}

impl Cursor {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "auto" => Some(Self::Auto),
            "default" => Some(Self::Default),
            "pointer" => Some(Self::Pointer),
            "text" => Some(Self::Text),
            "move" => Some(Self::Move),
            "not-allowed" => Some(Self::NotAllowed),
            "crosshair" => Some(Self::Crosshair),
            "wait" => Some(Self::Wait),
            "help" => Some(Self::Help),
            "grab" => Some(Self::Grab),
            "grabbing" => Some(Self::Grabbing),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_align_parse() {
        assert_eq!(TextAlign::parse("center"), Some(TextAlign::Center));
        assert_eq!(TextAlign::parse("justify"), Some(TextAlign::Justify));
    }

    #[test]
    fn overflow_parse() {
        assert_eq!(Overflow::parse("hidden"), Some(Overflow::Hidden));
        assert_eq!(Overflow::parse("scroll"), Some(Overflow::Scroll));
    }

    #[test]
    fn cursor_parse() {
        assert_eq!(Cursor::parse("pointer"), Some(Cursor::Pointer));
    }
}
