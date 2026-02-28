// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS `display` property values.

/// CSS `display` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Display {
    Block,
    #[default]
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    InlineGrid,
    None,
    Contents,
    Table,
    TableRow,
    TableCell,
    ListItem,
}

impl Display {
    /// Parse from a CSS value string.
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "block" => Some(Self::Block),
            "inline" => Some(Self::Inline),
            "inline-block" => Some(Self::InlineBlock),
            "flex" => Some(Self::Flex),
            "inline-flex" => Some(Self::InlineFlex),
            "grid" => Some(Self::Grid),
            "inline-grid" => Some(Self::InlineGrid),
            "none" => Some(Self::None),
            "contents" => Some(Self::Contents),
            "table" => Some(Self::Table),
            "table-row" => Some(Self::TableRow),
            "table-cell" => Some(Self::TableCell),
            "list-item" => Some(Self::ListItem),
            _ => None,
        }
    }

    /// Whether this display value generates a block-level box.
    pub fn is_block_level(&self) -> bool {
        matches!(self, Self::Block | Self::Flex | Self::Grid | Self::Table | Self::ListItem)
    }

    /// Whether this is an inline-level display.
    pub fn is_inline_level(&self) -> bool {
        matches!(self, Self::Inline | Self::InlineBlock | Self::InlineFlex | Self::InlineGrid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_displays() {
        assert_eq!(Display::parse("block"), Some(Display::Block));
        assert_eq!(Display::parse("inline-block"), Some(Display::InlineBlock));
        assert_eq!(Display::parse("flex"), Some(Display::Flex));
        assert_eq!(Display::parse("none"), Some(Display::None));
        assert_eq!(Display::parse("garbage"), None);
    }

    #[test]
    fn block_level_check() {
        assert!(Display::Block.is_block_level());
        assert!(Display::Flex.is_block_level());
        assert!(!Display::Inline.is_block_level());
    }
}
