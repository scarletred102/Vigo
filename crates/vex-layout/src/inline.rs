// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Inline-level layout: line boxes, text fragments, text-align.
//!
//! When a block container has only inline children, they are laid
//! out left-to-right, wrapping into line boxes when the available
//! width is exhausted. Text measurement is delegated to [`crate::text::TextEngine`].

use std::collections::HashMap;

use vex_core::{Rect, VexId};
use vex_css::ComputedStyle;
use vex_css::values::text::TextAlign;
use vex_dom::{NodeArena, NodeData};

use crate::box_model::LayoutBox;
use crate::text::TextEngine;

/// A single horizontal line box containing fragments.
#[derive(Debug)]
pub struct LineBox {
    /// Fragments on this line (index into parent's children vec).
    pub fragments: Vec<InlineFragment>,
    /// Total width consumed.
    pub width: f32,
    /// Height of the tallest fragment.
    pub height: f32,
}

/// A chunk of content placed on a line.
#[derive(Debug, Clone)]
pub struct InlineFragment {
    /// Index into the parent layout box's `children`.
    pub child_index: usize,
    /// Position within the line box.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Lay out inline children of a block layout box into line boxes.
///
/// This modifies the children's positions in-place and returns the
/// total height consumed by the inline formatting context.
pub fn layout_inline_children(
    layout_box: &mut LayoutBox,
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
) -> f32 {
    let available_width = layout_box.dimensions.content.size.width;
    let content_x = layout_box.dimensions.content.origin.x;
    let content_y = layout_box.dimensions.content.origin.y;

    // Determine text-align from the parent's style
    let text_align = layout_box
        .node_id
        .and_then(|id| styles.get(&id))
        .map(|s| s.text_align)
        .unwrap_or(TextAlign::Left);

    // Build line boxes from children
    let lines = build_line_boxes(
        &mut layout_box.children,
        arena,
        styles,
        text_engine,
        available_width,
    );

    // Apply positions
    let mut cursor_y = 0.0_f32;
    for line in &lines {
        let offset_x = align_offset(text_align, line.width, available_width);

        for frag in &line.fragments {
            let child = &mut layout_box.children[frag.child_index];
            child.dimensions.content = Rect::new(
                content_x + offset_x + frag.x,
                content_y + cursor_y + frag.y,
                frag.width,
                frag.height,
            );
        }
        cursor_y += line.height;
    }

    cursor_y
}

/// Build line boxes from a list of inline children.
fn build_line_boxes(
    children: &mut [LayoutBox],
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
    available_width: f32,
) -> Vec<LineBox> {
    let mut lines: Vec<LineBox> = Vec::new();
    let mut current_line = LineBox {
        fragments: Vec::new(),
        width: 0.0,
        height: 0.0,
    };

    for (i, child) in children.iter().enumerate() {
        // Measure the child
        let (child_w, child_h) = measure_inline_child(child, arena, styles, text_engine, available_width);

        // Does it fit on the current line?
        if current_line.width + child_w > available_width && !current_line.fragments.is_empty() {
            // Wrap: finish current line, start new one
            lines.push(current_line);
            current_line = LineBox {
                fragments: Vec::new(),
                width: 0.0,
                height: 0.0,
            };
        }

        current_line.fragments.push(InlineFragment {
            child_index: i,
            x: current_line.width,
            y: 0.0,
            width: child_w,
            height: child_h,
        });
        current_line.width += child_w;
        current_line.height = current_line.height.max(child_h);
    }

    // Don't forget the last line
    if !current_line.fragments.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Measure the natural size of an inline child.
fn measure_inline_child(
    child: &LayoutBox,
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
    available_width: f32,
) -> (f32, f32) {
    let node_id = match child.node_id {
        Some(id) => id,
        None => return (0.0, 0.0),
    };

    let node = arena.get(node_id);

    match &node.data {
        NodeData::Text(text) => {
            let style = styles.get(&node_id);
            // Walk up to parent for font info if the text node doesn't have its own style
            let (font_size, line_height) = if let Some(s) = style {
                (s.font_size, s.line_height)
            } else {
                // Use the parent's style as fallback
                let parent_style = node
                    .parent
                    .and_then(|pid| styles.get(&pid));
                match parent_style {
                    Some(ps) => (ps.font_size, ps.line_height),
                    None => (16.0, 19.2),
                }
            };
            text_engine.measure(text.trim(), font_size, line_height, available_width)
        }
        _ => {
            // Element — use its explicit width/height or default
            let style = styles.get(&node_id);
            let w = style.map(|s| s.width).unwrap_or(f32::NAN);
            let h = style.map(|s| s.height).unwrap_or(f32::NAN);
            (
                if w.is_nan() { 0.0 } else { w },
                if h.is_nan() { 0.0 } else { h },
            )
        }
    }
}

/// Compute the x offset for line alignment.
fn align_offset(align: TextAlign, line_width: f32, container_width: f32) -> f32 {
    let free = (container_width - line_width).max(0.0);
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Right => free,
        TextAlign::Center => free / 2.0,
        TextAlign::Justify => 0.0, // TODO: distribute space between words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;

    #[test]
    fn align_offset_left() {
        assert_eq!(align_offset(TextAlign::Left, 200.0, 800.0), 0.0);
    }

    #[test]
    fn align_offset_right() {
        assert_eq!(align_offset(TextAlign::Right, 200.0, 800.0), 600.0);
    }

    #[test]
    fn align_offset_center() {
        assert_eq!(align_offset(TextAlign::Center, 200.0, 800.0), 300.0);
    }

    #[test]
    fn align_offset_overflow_clamps() {
        // Line wider than container → offset = 0
        assert_eq!(align_offset(TextAlign::Center, 900.0, 800.0), 0.0);
    }

    #[test]
    fn empty_children_produce_no_lines() {
        let mut parent = LayoutBox::new(None, BoxType::Block);
        parent.dimensions.content = Rect::new(0.0, 0.0, 800.0, 0.0);
        let arena = vex_dom::NodeArena::default();
        let styles = HashMap::new();
        let mut engine = TextEngine::new();
        let h = layout_inline_children(&mut parent, &arena, &styles, &mut engine);
        assert_eq!(h, 0.0);
    }
}
