// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS box model values: sides, border-style, box-sizing.

/// Generic 4-side value, used for margin, padding, border-width.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxSide<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Clone> BoxSide<T> {
    pub fn uniform(value: T) -> Self {
        Self {
            top: value.clone(),
            right: value.clone(),
            bottom: value.clone(),
            left: value,
        }
    }
}

impl<T: Default> Default for BoxSide<T> {
    fn default() -> Self {
        Self {
            top: T::default(),
            right: T::default(),
            bottom: T::default(),
            left: T::default(),
        }
    }
}

/// CSS `border-style` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BorderStyle {
    #[default]
    None,
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
    Hidden,
}

impl BorderStyle {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "none" => Some(Self::None),
            "solid" => Some(Self::Solid),
            "dashed" => Some(Self::Dashed),
            "dotted" => Some(Self::Dotted),
            "double" => Some(Self::Double),
            "groove" => Some(Self::Groove),
            "ridge" => Some(Self::Ridge),
            "inset" => Some(Self::Inset),
            "outset" => Some(Self::Outset),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }
}

/// CSS `box-sizing` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

impl BoxSizing {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "content-box" => Some(Self::ContentBox),
            "border-box" => Some(Self::BorderBox),
            _ => None,
        }
    }
}

/// CSS `visibility` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    Collapse,
}

impl Visibility {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "collapse" => Some(Self::Collapse),
            _ => None,
        }
    }
}

/// CSS `float` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Float {
    #[default]
    None,
    Left,
    Right,
}

impl Float {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "none" => Some(Self::None),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }
}

/// CSS `clear` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Clear {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl Clear {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "none" => Some(Self::None),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_side_uniform() {
        let side = BoxSide::uniform(10.0_f32);
        assert_eq!(side.top, 10.0);
        assert_eq!(side.left, 10.0);
    }

    #[test]
    fn border_style_parse() {
        assert_eq!(BorderStyle::parse("solid"), Some(BorderStyle::Solid));
        assert_eq!(BorderStyle::parse("dashed"), Some(BorderStyle::Dashed));
        assert_eq!(BorderStyle::parse("bad"), None);
    }

    #[test]
    fn box_sizing_parse() {
        assert_eq!(BoxSizing::parse("border-box"), Some(BoxSizing::BorderBox));
        assert_eq!(BoxSizing::parse("content-box"), Some(BoxSizing::ContentBox));
    }

    #[test]
    fn float_parse() {
        assert_eq!(Float::parse("left"), Some(Float::Left));
        assert_eq!(Float::parse("right"), Some(Float::Right));
        assert_eq!(Float::parse("none"), Some(Float::None));
        assert_eq!(Float::parse("center"), None);
    }

    #[test]
    fn clear_parse() {
        assert_eq!(Clear::parse("left"), Some(Clear::Left));
        assert_eq!(Clear::parse("right"), Some(Clear::Right));
        assert_eq!(Clear::parse("both"), Some(Clear::Both));
        assert_eq!(Clear::parse("none"), Some(Clear::None));
        assert_eq!(Clear::parse("invalid"), None);
    }
}
