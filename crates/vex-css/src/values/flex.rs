// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS flexbox property values.

/// CSS `flex-direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "row" => Some(Self::Row),
            "row-reverse" => Some(Self::RowReverse),
            "column" => Some(Self::Column),
            "column-reverse" => Some(Self::ColumnReverse),
            _ => None,
        }
    }

    pub fn is_row(&self) -> bool {
        matches!(self, Self::Row | Self::RowReverse)
    }
}

/// CSS `flex-wrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrap {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "nowrap" => Some(Self::NoWrap),
            "wrap" => Some(Self::Wrap),
            "wrap-reverse" => Some(Self::WrapReverse),
            _ => None,
        }
    }
}

/// CSS `justify-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyContent {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "space-between" => Some(Self::SpaceBetween),
            "space-around" => Some(Self::SpaceAround),
            "space-evenly" => Some(Self::SpaceEvenly),
            _ => None,
        }
    }
}

/// CSS `align-items`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignItems {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "baseline" => Some(Self::Baseline),
            _ => None,
        }
    }
}

/// CSS `align-self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignSelf {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "auto" => Some(Self::Auto),
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "baseline" => Some(Self::Baseline),
            _ => None,
        }
    }
}

/// CSS `align-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignContent {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
}

impl AlignContent {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "space-between" => Some(Self::SpaceBetween),
            "space-around" => Some(Self::SpaceAround),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_direction_parse() {
        assert_eq!(FlexDirection::parse("row"), Some(FlexDirection::Row));
        assert_eq!(FlexDirection::parse("column"), Some(FlexDirection::Column));
        assert!(FlexDirection::Row.is_row());
        assert!(!FlexDirection::Column.is_row());
    }

    #[test]
    fn justify_content_parse() {
        assert_eq!(JustifyContent::parse("center"), Some(JustifyContent::Center));
        assert_eq!(JustifyContent::parse("space-between"), Some(JustifyContent::SpaceBetween));
    }

    #[test]
    fn align_items_parse() {
        assert_eq!(AlignItems::parse("stretch"), Some(AlignItems::Stretch));
        assert_eq!(AlignItems::parse("center"), Some(AlignItems::Center));
    }
}
