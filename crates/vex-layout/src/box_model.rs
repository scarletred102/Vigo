// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Box model: `LayoutBox`, `BoxType`, `Dimensions`, and edge accessors.

use vex_core::{Insets, Point, Rect, Size, VexId};

/// What kind of formatting context a box participates in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxType {
    Block,
    Inline,
    InlineBlock,
    Flex,
    Grid,
    /// Wrapper box not tied to a DOM element (e.g. anonymous block for mixed content).
    Anonymous,
}

/// Resolved dimensions and edges for a layout box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimensions {
    /// Content area position and size (absolute coordinates).
    pub content: Rect,
    pub padding: Insets,
    pub border: Insets,
    pub margin: Insets,
}

impl Dimensions {
    /// The padding box = content area expanded by padding.
    pub fn padding_box(&self) -> Rect {
        expand(self.content, self.padding)
    }

    /// The border box = padding box expanded by border widths.
    pub fn border_box(&self) -> Rect {
        expand(self.padding_box(), self.border)
    }

    /// The margin box = border box expanded by margins.
    pub fn margin_box(&self) -> Rect {
        expand(self.border_box(), self.margin)
    }
}

/// Expand a rect outward by the given insets.
fn expand(r: Rect, i: Insets) -> Rect {
    Rect {
        origin: Point {
            x: r.origin.x - i.left,
            y: r.origin.y - i.top,
        },
        size: Size {
            width: r.size.width + i.left + i.right,
            height: r.size.height + i.top + i.bottom,
        },
    }
}

/// A node in the layout tree.
#[derive(Debug, Clone)]
pub struct LayoutBox {
    /// The DOM node this box corresponds to (`None` for anonymous boxes).
    pub node_id: Option<VexId>,
    pub box_type: BoxType,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,

    /// Clip rect for `overflow: hidden`.
    pub clip_rect: Option<Rect>,
    /// Scroll offset (for scrollable containers).
    pub scroll_offset: Point,
}

impl LayoutBox {
    pub fn new(node_id: Option<VexId>, box_type: BoxType) -> Self {
        Self {
            node_id,
            box_type,
            dimensions: Dimensions::default(),
            children: Vec::new(),
            clip_rect: None,
            scroll_offset: Point::default(),
        }
    }

    /// Shorthand for the content rect.
    pub fn content_rect(&self) -> Rect {
        self.dimensions.content
    }

    /// Shorthand for the border box rect.
    pub fn border_box(&self) -> Rect {
        self.dimensions.border_box()
    }

    /// Shorthand for the margin box rect.
    pub fn margin_box(&self) -> Rect {
        self.dimensions.margin_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_expand_outward() {
        let d = Dimensions {
            content: Rect::new(50.0, 50.0, 200.0, 100.0),
            padding: Insets::new(10.0, 10.0, 10.0, 10.0),
            border: Insets::new(2.0, 2.0, 2.0, 2.0),
            margin: Insets::new(20.0, 20.0, 20.0, 20.0),
        };
        let pb = d.padding_box();
        assert_eq!(pb.origin.x, 40.0);
        assert_eq!(pb.size.width, 220.0);

        let bb = d.border_box();
        assert_eq!(bb.origin.x, 38.0);
        assert_eq!(bb.size.width, 224.0);

        let mb = d.margin_box();
        assert_eq!(mb.origin.x, 18.0);
        assert_eq!(mb.size.width, 264.0);
    }

    #[test]
    fn layout_box_creation() {
        let b = LayoutBox::new(Some(VexId::new(5)), BoxType::Block);
        assert_eq!(b.node_id, Some(VexId::new(5)));
        assert_eq!(b.box_type, BoxType::Block);
        assert!(b.children.is_empty());
    }

    #[test]
    fn zero_insets_passthrough() {
        let d = Dimensions {
            content: Rect::new(10.0, 20.0, 100.0, 50.0),
            ..Default::default()
        };
        assert_eq!(d.padding_box(), d.content);
        assert_eq!(d.border_box(), d.content);
        assert_eq!(d.margin_box(), d.content);
    }
}
