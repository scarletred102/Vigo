// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Positioned layout: relative, absolute, fixed offsets.
//!
//! Applied as a post-pass after normal flow layout has completed.

use std::collections::HashMap;

use vex_core::{Rect, VexId};
use vex_css::values::position::Position;
use vex_css::ComputedStyle;

use crate::box_model::LayoutBox;

/// Apply position offsets to all positioned boxes in the layout tree.
pub fn apply_positions(
    root: &mut LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    viewport: Rect,
) {
    apply_positions_recursive(root, styles, viewport, viewport);
}

fn apply_positions_recursive(
    layout_box: &mut LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    containing_block: Rect,
    viewport: Rect,
) {
    let style = layout_box.node_id.and_then(|id| styles.get(&id));
    let position = style.map(|s| s.position).unwrap_or(Position::Static);

    match position {
        Position::Relative => {
            apply_relative(layout_box, style);
        }
        Position::Absolute => {
            apply_absolute(layout_box, style, containing_block);
        }
        Position::Fixed => {
            apply_fixed(layout_box, style, viewport);
        }
        Position::Sticky => {
            apply_sticky(layout_box, style, containing_block, viewport);
        }
        Position::Static => {
            // Static: no offset.
        }
    }

    // Determine the containing block for descendants
    let new_containing = if position.is_positioned() {
        // This box becomes a containing block for absolutely positioned descendants
        layout_box.dimensions.content
    } else {
        containing_block
    };

    for child in &mut layout_box.children {
        apply_positions_recursive(child, styles, new_containing, viewport);
    }
}

/// Relative positioning: offset from normal flow position.
fn apply_relative(layout_box: &mut LayoutBox, style: Option<&ComputedStyle>) {
    let style = match style {
        Some(s) => s,
        None => return,
    };

    let dx = if !style.left.is_nan() {
        style.left
    } else if !style.right.is_nan() {
        -style.right
    } else {
        0.0
    };

    let dy = if !style.top.is_nan() {
        style.top
    } else if !style.bottom.is_nan() {
        -style.bottom
    } else {
        0.0
    };

    layout_box.dimensions.content.origin.x += dx;
    layout_box.dimensions.content.origin.y += dy;
}

/// Absolute positioning: placed relative to the nearest positioned ancestor.
fn apply_absolute(layout_box: &mut LayoutBox, style: Option<&ComputedStyle>, containing: Rect) {
    let style = match style {
        Some(s) => s,
        None => return,
    };

    let m = &layout_box.dimensions.margin;
    let p = &layout_box.dimensions.padding;
    let b = &layout_box.dimensions.border;

    // Horizontal
    if !style.left.is_nan() {
        layout_box.dimensions.content.origin.x =
            containing.origin.x + style.left + m.left + p.left + b.left;
    } else if !style.right.is_nan() {
        let content_w = layout_box.dimensions.content.size.width;
        layout_box.dimensions.content.origin.x = containing.origin.x + containing.size.width
            - style.right
            - content_w
            - m.right
            - p.right
            - b.right;
    }

    // Vertical
    if !style.top.is_nan() {
        layout_box.dimensions.content.origin.y =
            containing.origin.y + style.top + m.top + p.top + b.top;
    } else if !style.bottom.is_nan() {
        let content_h = layout_box.dimensions.content.size.height;
        layout_box.dimensions.content.origin.y = containing.origin.y + containing.size.height
            - style.bottom
            - content_h
            - m.bottom
            - p.bottom
            - b.bottom;
    }

    // Auto width for absolute boxes: shrink-to-fit (use containing block width)
    if style.width.is_nan() && !style.left.is_nan() && !style.right.is_nan() {
        let used =
            style.left + style.right + m.left + m.right + p.left + p.right + b.left + b.right;
        layout_box.dimensions.content.size.width = (containing.size.width - used).max(0.0);
    }

    // Auto height
    if style.height.is_nan() && !style.top.is_nan() && !style.bottom.is_nan() {
        let used =
            style.top + style.bottom + m.top + m.bottom + p.top + p.bottom + b.top + b.bottom;
        layout_box.dimensions.content.size.height = (containing.size.height - used).max(0.0);
    }
}

/// Fixed positioning: placed relative to the viewport.
fn apply_fixed(layout_box: &mut LayoutBox, style: Option<&ComputedStyle>, viewport: Rect) {
    // Fixed is the same as absolute, but relative to the viewport.
    apply_absolute(layout_box, style, viewport);
}

/// Sticky positioning: clamp relative movement to viewport thresholds while
/// staying within the containing block bounds.
fn apply_sticky(
    layout_box: &mut LayoutBox,
    style: Option<&ComputedStyle>,
    containing: Rect,
    viewport: Rect,
) {
    let Some(style) = style else {
        return;
    };

    let mut x = layout_box.dimensions.content.origin.x;
    let mut y = layout_box.dimensions.content.origin.y;
    let w = layout_box.dimensions.content.size.width;
    let h = layout_box.dimensions.content.size.height;

    if !style.top.is_nan() {
        let sticky_top = viewport.origin.y + style.top;
        y = y.max(sticky_top);
    }
    if !style.bottom.is_nan() {
        let sticky_bottom = viewport.origin.y + viewport.size.height - style.bottom - h;
        y = y.min(sticky_bottom);
    }
    if !style.left.is_nan() {
        let sticky_left = viewport.origin.x + style.left;
        x = x.max(sticky_left);
    }
    if !style.right.is_nan() {
        let sticky_right = viewport.origin.x + viewport.size.width - style.right - w;
        x = x.min(sticky_right);
    }

    // Sticky box should remain inside its containing block.
    let max_x = containing.origin.x + containing.size.width - w;
    let max_y = containing.origin.y + containing.size.height - h;
    x = x.clamp(containing.origin.x, max_x.max(containing.origin.x));
    y = y.clamp(containing.origin.y, max_y.max(containing.origin.y));

    layout_box.dimensions.content.origin.x = x;
    layout_box.dimensions.content.origin.y = y;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;

    fn style_positioned(position: Position, top: f32, left: f32) -> ComputedStyle {
        ComputedStyle {
            position,
            top,
            left,
            ..Default::default()
        }
    }

    #[test]
    fn relative_offsets_applied() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        styles.insert(id, style_positioned(Position::Relative, 10.0, 20.0));

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(100.0, 100.0, 200.0, 50.0);

        apply_positions(&mut b, &styles, Rect::new(0.0, 0.0, 1280.0, 720.0));

        assert_eq!(b.dimensions.content.origin.x, 120.0);
        assert_eq!(b.dimensions.content.origin.y, 110.0);
    }

    #[test]
    fn absolute_positions_from_containing_block() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        styles.insert(id, style_positioned(Position::Absolute, 50.0, 30.0));

        let mut b = LayoutBox::new(Some(id), BoxType::Block);

        let containing = Rect::new(100.0, 100.0, 600.0, 400.0);

        // Call the inner function directly
        apply_absolute(&mut b, styles.get(&id), containing);

        assert_eq!(b.dimensions.content.origin.x, 130.0);
        assert_eq!(b.dimensions.content.origin.y, 150.0);
    }

    #[test]
    fn fixed_uses_viewport() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        styles.insert(id, style_positioned(Position::Fixed, 0.0, 0.0));

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(500.0, 500.0, 100.0, 30.0);

        let viewport = Rect::new(0.0, 0.0, 1280.0, 720.0);
        apply_positions(&mut b, &styles, viewport);

        assert_eq!(b.dimensions.content.origin.x, 0.0);
        assert_eq!(b.dimensions.content.origin.y, 0.0);
    }

    #[test]
    fn static_position_unchanged() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        styles.insert(id, ComputedStyle::default()); // position: Static

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(50.0, 60.0, 200.0, 100.0);

        apply_positions(&mut b, &styles, Rect::new(0.0, 0.0, 1280.0, 720.0));

        assert_eq!(b.dimensions.content.origin.x, 50.0);
        assert_eq!(b.dimensions.content.origin.y, 60.0);
    }

    #[test]
    fn relative_right_bottom_offsets() {
        let id = VexId::new(1);
        let s = ComputedStyle {
            position: Position::Relative,
            right: 15.0,
            bottom: 25.0,
            ..Default::default()
        };
        // left and top are NAN (auto)
        let mut styles = HashMap::new();
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(100.0, 100.0, 50.0, 50.0);

        apply_positions(&mut b, &styles, Rect::new(0.0, 0.0, 1280.0, 720.0));

        assert_eq!(b.dimensions.content.origin.x, 85.0); // 100 - 15
        assert_eq!(b.dimensions.content.origin.y, 75.0); // 100 - 25
    }

    #[test]
    fn sticky_top_clamps_to_viewport_threshold() {
        let id = VexId::new(1);
        let s = ComputedStyle {
            position: Position::Sticky,
            top: 10.0,
            ..Default::default()
        };
        let mut styles = HashMap::new();
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(0.0, 20.0, 100.0, 40.0);

        // Simulate scrolled viewport by moving viewport origin.
        let viewport = Rect::new(0.0, 100.0, 1280.0, 720.0);
        apply_positions(&mut b, &styles, viewport);

        assert_eq!(b.dimensions.content.origin.y, 110.0);
    }
}
