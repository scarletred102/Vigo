// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Logical Properties and Values.
//!
//! Maps flow-relative (logical) properties to physical properties based on
//! writing mode and direction. Supports `inline-start`, `inline-end`,
//! `block-start`, `block-end` mappings.

// ── Types ────────────────────────────────────────────────────────────────────

/// Writing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WritingMode {
    /// Left-to-right, top-to-bottom (English, most Latin scripts).
    #[default]
    HorizontalTb,
    /// Top-to-bottom, right-to-left (Traditional Chinese, Japanese).
    VerticalRl,
    /// Top-to-bottom, left-to-right (Mongolian).
    VerticalLr,
    /// Right-to-left, top-to-bottom.
    SidewaysRl,
    /// Left-to-right, top-to-bottom at 90 degrees.
    SidewaysLr,
}

impl WritingMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "horizontal-tb" => Some(Self::HorizontalTb),
            "vertical-rl" => Some(Self::VerticalRl),
            "vertical-lr" => Some(Self::VerticalLr),
            "sideways-rl" => Some(Self::SidewaysRl),
            "sideways-lr" => Some(Self::SidewaysLr),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HorizontalTb => "horizontal-tb",
            Self::VerticalRl => "vertical-rl",
            Self::VerticalLr => "vertical-lr",
            Self::SidewaysRl => "sideways-rl",
            Self::SidewaysLr => "sideways-lr",
        }
    }

    /// Whether this writing mode is horizontal.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::HorizontalTb)
    }
}

/// CSS direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

/// Physical side of a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalSide {
    Top,
    Right,
    Bottom,
    Left,
}

/// Logical side of a box (flow-relative).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalSide {
    BlockStart,
    BlockEnd,
    InlineStart,
    InlineEnd,
}

impl LogicalSide {
    /// Map a logical side to a physical side given writing mode and direction.
    pub fn to_physical(self, wm: WritingMode, dir: Direction) -> PhysicalSide {
        match wm {
            WritingMode::HorizontalTb => match self {
                Self::BlockStart => PhysicalSide::Top,
                Self::BlockEnd => PhysicalSide::Bottom,
                Self::InlineStart => match dir {
                    Direction::Ltr => PhysicalSide::Left,
                    Direction::Rtl => PhysicalSide::Right,
                },
                Self::InlineEnd => match dir {
                    Direction::Ltr => PhysicalSide::Right,
                    Direction::Rtl => PhysicalSide::Left,
                },
            },
            WritingMode::VerticalRl | WritingMode::SidewaysRl => match self {
                Self::BlockStart => PhysicalSide::Right,
                Self::BlockEnd => PhysicalSide::Left,
                Self::InlineStart => match dir {
                    Direction::Ltr => PhysicalSide::Top,
                    Direction::Rtl => PhysicalSide::Bottom,
                },
                Self::InlineEnd => match dir {
                    Direction::Ltr => PhysicalSide::Bottom,
                    Direction::Rtl => PhysicalSide::Top,
                },
            },
            WritingMode::VerticalLr | WritingMode::SidewaysLr => match self {
                Self::BlockStart => PhysicalSide::Left,
                Self::BlockEnd => PhysicalSide::Right,
                Self::InlineStart => match dir {
                    Direction::Ltr => PhysicalSide::Top,
                    Direction::Rtl => PhysicalSide::Bottom,
                },
                Self::InlineEnd => match dir {
                    Direction::Ltr => PhysicalSide::Bottom,
                    Direction::Rtl => PhysicalSide::Top,
                },
            },
        }
    }
}

/// Logical size dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalSize {
    InlineSize,
    BlockSize,
}

impl LogicalSize {
    /// Map a logical size to physical width/height.
    pub fn to_physical(self, wm: WritingMode) -> PhysicalSize {
        if wm.is_horizontal() {
            match self {
                Self::InlineSize => PhysicalSize::Width,
                Self::BlockSize => PhysicalSize::Height,
            }
        } else {
            match self {
                Self::InlineSize => PhysicalSize::Height,
                Self::BlockSize => PhysicalSize::Width,
            }
        }
    }
}

/// Physical size dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalSize {
    Width,
    Height,
}

/// Logical property name → physical property name mapper.
pub fn map_logical_property(name: &str, wm: WritingMode, dir: Direction) -> Option<&'static str> {
    let (logical_side, prefix) = match name {
        "margin-block-start" => (LogicalSide::BlockStart, "margin"),
        "margin-block-end" => (LogicalSide::BlockEnd, "margin"),
        "margin-inline-start" => (LogicalSide::InlineStart, "margin"),
        "margin-inline-end" => (LogicalSide::InlineEnd, "margin"),
        "padding-block-start" => (LogicalSide::BlockStart, "padding"),
        "padding-block-end" => (LogicalSide::BlockEnd, "padding"),
        "padding-inline-start" => (LogicalSide::InlineStart, "padding"),
        "padding-inline-end" => (LogicalSide::InlineEnd, "padding"),
        "border-block-start-width" => (LogicalSide::BlockStart, "border-width"),
        "border-block-end-width" => (LogicalSide::BlockEnd, "border-width"),
        "border-inline-start-width" => (LogicalSide::InlineStart, "border-width"),
        "border-inline-end-width" => (LogicalSide::InlineEnd, "border-width"),
        "inset-block-start" => (LogicalSide::BlockStart, "inset"),
        "inset-block-end" => (LogicalSide::BlockEnd, "inset"),
        "inset-inline-start" => (LogicalSide::InlineStart, "inset"),
        "inset-inline-end" => (LogicalSide::InlineEnd, "inset"),
        "inline-size" => {
            return Some(if wm.is_horizontal() {
                "width"
            } else {
                "height"
            });
        }
        "block-size" => {
            return Some(if wm.is_horizontal() {
                "height"
            } else {
                "width"
            });
        }
        "min-inline-size" => {
            return Some(if wm.is_horizontal() {
                "min-width"
            } else {
                "min-height"
            });
        }
        "min-block-size" => {
            return Some(if wm.is_horizontal() {
                "min-height"
            } else {
                "min-width"
            });
        }
        "max-inline-size" => {
            return Some(if wm.is_horizontal() {
                "max-width"
            } else {
                "max-height"
            });
        }
        "max-block-size" => {
            return Some(if wm.is_horizontal() {
                "max-height"
            } else {
                "max-width"
            });
        }
        _ => return None,
    };

    let physical = logical_side.to_physical(wm, dir);
    let side_str = match physical {
        PhysicalSide::Top => "top",
        PhysicalSide::Right => "right",
        PhysicalSide::Bottom => "bottom",
        PhysicalSide::Left => "left",
    };

    Some(match prefix {
        "margin" => match side_str {
            "top" => "margin-top",
            "right" => "margin-right",
            "bottom" => "margin-bottom",
            "left" => "margin-left",
            _ => return None,
        },
        "padding" => match side_str {
            "top" => "padding-top",
            "right" => "padding-right",
            "bottom" => "padding-bottom",
            "left" => "padding-left",
            _ => return None,
        },
        "border-width" => match side_str {
            "top" => "border-top-width",
            "right" => "border-right-width",
            "bottom" => "border-bottom-width",
            "left" => "border-left-width",
            _ => return None,
        },
        "inset" => match side_str {
            "top" => "top",
            "right" => "right",
            "bottom" => "bottom",
            "left" => "left",
            _ => return None,
        },
        _ => return None,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_mode_parse() {
        assert_eq!(
            WritingMode::parse("horizontal-tb"),
            Some(WritingMode::HorizontalTb)
        );
        assert_eq!(
            WritingMode::parse("vertical-rl"),
            Some(WritingMode::VerticalRl)
        );
        assert_eq!(WritingMode::parse("invalid"), None);
    }

    #[test]
    fn direction_parse() {
        assert_eq!(Direction::parse("ltr"), Some(Direction::Ltr));
        assert_eq!(Direction::parse("rtl"), Some(Direction::Rtl));
    }

    #[test]
    fn logical_to_physical_horizontal_ltr() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Ltr;
        assert_eq!(
            LogicalSide::BlockStart.to_physical(wm, dir),
            PhysicalSide::Top
        );
        assert_eq!(
            LogicalSide::BlockEnd.to_physical(wm, dir),
            PhysicalSide::Bottom
        );
        assert_eq!(
            LogicalSide::InlineStart.to_physical(wm, dir),
            PhysicalSide::Left
        );
        assert_eq!(
            LogicalSide::InlineEnd.to_physical(wm, dir),
            PhysicalSide::Right
        );
    }

    #[test]
    fn logical_to_physical_horizontal_rtl() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Rtl;
        assert_eq!(
            LogicalSide::InlineStart.to_physical(wm, dir),
            PhysicalSide::Right
        );
        assert_eq!(
            LogicalSide::InlineEnd.to_physical(wm, dir),
            PhysicalSide::Left
        );
    }

    #[test]
    fn logical_to_physical_vertical_rl() {
        let wm = WritingMode::VerticalRl;
        let dir = Direction::Ltr;
        assert_eq!(
            LogicalSide::BlockStart.to_physical(wm, dir),
            PhysicalSide::Right
        );
        assert_eq!(
            LogicalSide::InlineStart.to_physical(wm, dir),
            PhysicalSide::Top
        );
    }

    #[test]
    fn map_margin_horizontal_ltr() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Ltr;
        assert_eq!(
            map_logical_property("margin-inline-start", wm, dir),
            Some("margin-left")
        );
        assert_eq!(
            map_logical_property("margin-inline-end", wm, dir),
            Some("margin-right")
        );
        assert_eq!(
            map_logical_property("margin-block-start", wm, dir),
            Some("margin-top")
        );
    }

    #[test]
    fn map_margin_horizontal_rtl() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Rtl;
        assert_eq!(
            map_logical_property("margin-inline-start", wm, dir),
            Some("margin-right")
        );
    }

    #[test]
    fn map_size_properties() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Ltr;
        assert_eq!(
            map_logical_property("inline-size", wm, dir),
            Some("width")
        );
        assert_eq!(
            map_logical_property("block-size", wm, dir),
            Some("height")
        );
        assert_eq!(
            map_logical_property("min-inline-size", wm, dir),
            Some("min-width")
        );
    }

    #[test]
    fn map_size_vertical() {
        let wm = WritingMode::VerticalRl;
        let dir = Direction::Ltr;
        assert_eq!(
            map_logical_property("inline-size", wm, dir),
            Some("height")
        );
        assert_eq!(
            map_logical_property("block-size", wm, dir),
            Some("width")
        );
    }

    #[test]
    fn map_inset_properties() {
        let wm = WritingMode::HorizontalTb;
        let dir = Direction::Ltr;
        assert_eq!(
            map_logical_property("inset-block-start", wm, dir),
            Some("top")
        );
        assert_eq!(
            map_logical_property("inset-inline-start", wm, dir),
            Some("left")
        );
    }

    #[test]
    fn unknown_property() {
        assert_eq!(
            map_logical_property("color", WritingMode::HorizontalTb, Direction::Ltr),
            None
        );
    }

    #[test]
    fn logical_size_to_physical() {
        assert_eq!(
            LogicalSize::InlineSize.to_physical(WritingMode::HorizontalTb),
            PhysicalSize::Width
        );
        assert_eq!(
            LogicalSize::BlockSize.to_physical(WritingMode::HorizontalTb),
            PhysicalSize::Height
        );
        assert_eq!(
            LogicalSize::InlineSize.to_physical(WritingMode::VerticalRl),
            PhysicalSize::Height
        );
    }
}
