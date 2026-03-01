// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Block-level layout: width calculation, child positioning, height calculation.
//!
//! Implements the CSS 2.1 §10.3 / §10.6 block formatting context.

use std::collections::HashMap;

use vex_core::{Insets, VexId};
use vex_css::values::box_model::BoxSizing;
use vex_css::values::Overflow;
use vex_css::ComputedStyle;

use crate::box_model::{BoxType, LayoutBox};

/// Containing block = the available area a box can lay out inside.
#[derive(Debug, Clone, Copy)]
pub struct ContainingBlock {
    pub width: f32,
    pub height: f32,
}

/// Perform layout on a block-level box and all descendants.
pub fn layout_block(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    // 1. Resolve width + horizontal margins
    calculate_block_width(layout_box, containing.width, styles);
    // 2. Position + layout children
    layout_block_children(layout_box, styles);
    // 3. Resolve height
    calculate_block_height(layout_box, styles);
    // 4. Apply overflow clip
    apply_overflow_clip(layout_box, styles);
}

/// CSS 2.1 §10.3.3 — calculate the used width of a block box.
///
/// `margin-left + border-left + padding-left + width + padding-right + border-right + margin-right = containing width`
fn calculate_block_width(
    layout_box: &mut LayoutBox,
    containing_width: f32,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    let style = match layout_box.node_id.and_then(|id| styles.get(&id)) {
        Some(s) => s,
        None => {
            // Anonymous box — takes full containing width
            layout_box.dimensions.content.size.width = containing_width;
            return;
        }
    };

    let width = style.width; // NaN = auto
    let margin_left = style.margin_left;
    let margin_right = style.margin_right;

    let padding = Insets::new(
        style.padding_top,
        style.padding_right,
        style.padding_bottom,
        style.padding_left,
    );
    let border = Insets::new(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    );

    layout_box.dimensions.padding = padding;
    layout_box.dimensions.border = border;

    let horiz_edges = padding.left + padding.right + border.left + border.right;

    let auto_width = width.is_nan();
    let auto_ml = margin_left.is_nan();
    let auto_mr = margin_right.is_nan();

    let mut used_width;
    let used_ml;
    let used_mr;

    if auto_width {
        // Auto width: absorb remaining space, auto margins become 0
        used_ml = if auto_ml { 0.0 } else { margin_left };
        used_mr = if auto_mr { 0.0 } else { margin_right };
        used_width = containing_width - horiz_edges - used_ml - used_mr;
        used_width = used_width.max(0.0);
    } else {
        let mut w = adjust_for_box_sizing(width, style.box_sizing, &padding, &border);
        w = clamp_dimension(w, style.min_width, style.max_width);

        let remaining = containing_width - w - horiz_edges;

        if !auto_ml && !auto_mr {
            // Over-constrained: adjust margin-right
            used_ml = margin_left;
            used_mr = remaining - margin_left;
        } else if auto_ml && auto_mr {
            // Center: split equally
            let half = (remaining / 2.0).max(0.0);
            used_ml = half;
            used_mr = remaining - half;
        } else if auto_ml {
            used_mr = margin_right;
            used_ml = remaining - margin_right;
        } else {
            used_ml = margin_left;
            used_mr = remaining - margin_left;
        }
        used_width = w;
    }

    layout_box.dimensions.margin = Insets::new(
        if style.margin_top.is_nan() {
            0.0
        } else {
            style.margin_top
        },
        used_mr,
        if style.margin_bottom.is_nan() {
            0.0
        } else {
            style.margin_bottom
        },
        used_ml,
    );
    layout_box.dimensions.content.size.width = used_width;
}

/// Layout block children vertically, stacking them top-to-bottom.
fn layout_block_children(layout_box: &mut LayoutBox, styles: &HashMap<VexId, ComputedStyle>) {
    let d = &layout_box.dimensions;
    let containing = ContainingBlock {
        width: d.content.size.width,
        height: d.content.size.height,
    };

    // Start position for first child
    let mut cursor_y = 0.0_f32;
    let mut prev_margin_bottom = 0.0_f32;

    // Take children to avoid borrow issues
    let mut children = std::mem::take(&mut layout_box.children);

    for child in &mut children {
        match child.box_type {
            BoxType::Block | BoxType::Anonymous | BoxType::Flex => {
                // Recursively layout the child
                layout_block(child, containing, styles);

                // Margin collapsing: overlap adjacent margins
                let child_margin_top = child.dimensions.margin.top;
                let collapsed = collapse_margins(prev_margin_bottom, child_margin_top);
                cursor_y += collapsed - prev_margin_bottom;

                // Position child
                child.dimensions.content.origin.x = layout_box.dimensions.content.origin.x
                    + child.dimensions.margin.left
                    + child.dimensions.padding.left
                    + child.dimensions.border.left;

                child.dimensions.content.origin.y = layout_box.dimensions.content.origin.y
                    + cursor_y
                    + child.dimensions.margin.top
                    + child.dimensions.padding.top
                    + child.dimensions.border.top;

                // Advance cursor
                cursor_y += child.dimensions.margin_box().size.height;
                prev_margin_bottom = child.dimensions.margin.bottom;
            }
            BoxType::Inline | BoxType::InlineBlock => {
                // Inline children in a block context get treated as a single line
                // (proper inline layout is handled by inline.rs)
                child.dimensions.content.origin.x = layout_box.dimensions.content.origin.x;
                child.dimensions.content.origin.y =
                    layout_box.dimensions.content.origin.y + cursor_y;
                cursor_y += child.dimensions.margin_box().size.height;
                prev_margin_bottom = 0.0;
            }
        }
    }

    layout_box.children = children;
}

/// CSS 2.1 §10.6.3 — calculate the height of a block box.
fn calculate_block_height(layout_box: &mut LayoutBox, styles: &HashMap<VexId, ComputedStyle>) {
    let style = layout_box.node_id.and_then(|id| styles.get(&id));

    // Explicit height?
    let explicit_height = style.map(|s| s.height).unwrap_or(f32::NAN);

    if !explicit_height.is_nan() {
        let padding = &layout_box.dimensions.padding;
        let border = &layout_box.dimensions.border;
        let box_sizing = style.map(|s| s.box_sizing).unwrap_or(BoxSizing::ContentBox);
        let mut h = adjust_for_box_sizing_vert(explicit_height, box_sizing, padding, border);
        let min_h = style.map(|s| s.min_height).unwrap_or(0.0);
        let max_h = style.map(|s| s.max_height).unwrap_or(f32::INFINITY);
        h = clamp_dimension(h, min_h, max_h);
        layout_box.dimensions.content.size.height = h;
    } else {
        // Auto height: sum of children's margin boxes
        let auto_height = layout_box
            .children
            .iter()
            .map(|c| c.dimensions.margin_box().size.height)
            .sum::<f32>();
        let min_h = style.map(|s| s.min_height).unwrap_or(0.0);
        let max_h = style.map(|s| s.max_height).unwrap_or(f32::INFINITY);
        layout_box.dimensions.content.size.height = clamp_dimension(auto_height, min_h, max_h);
    }
}

/// Set a clip rect when overflow is hidden or scroll.
fn apply_overflow_clip(layout_box: &mut LayoutBox, styles: &HashMap<VexId, ComputedStyle>) {
    if let Some(style) = layout_box.node_id.and_then(|id| styles.get(&id)) {
        if matches!(
            style.overflow,
            Overflow::Hidden | Overflow::Scroll | Overflow::Auto
        ) {
            layout_box.clip_rect = Some(layout_box.dimensions.border_box());
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Adjust for `box-sizing: border-box` (horizontal).
fn adjust_for_box_sizing(
    width: f32,
    box_sizing: BoxSizing,
    padding: &Insets,
    border: &Insets,
) -> f32 {
    match box_sizing {
        BoxSizing::ContentBox => width,
        BoxSizing::BorderBox => {
            (width - padding.left - padding.right - border.left - border.right).max(0.0)
        }
    }
}

/// Adjust for `box-sizing: border-box` (vertical).
fn adjust_for_box_sizing_vert(
    height: f32,
    box_sizing: BoxSizing,
    padding: &Insets,
    border: &Insets,
) -> f32 {
    match box_sizing {
        BoxSizing::ContentBox => height,
        BoxSizing::BorderBox => {
            (height - padding.top - padding.bottom - border.top - border.bottom).max(0.0)
        }
    }
}

/// Clamp a dimension between min and max.
fn clamp_dimension(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, if max.is_infinite() { f32::MAX } else { max })
}

/// Collapse two adjacent vertical margins per CSS 2.1 §8.3.1.
/// Returns the effective margin (max of both, respecting negative margins).
fn collapse_margins(margin_a: f32, margin_b: f32) -> f32 {
    if margin_a >= 0.0 && margin_b >= 0.0 {
        margin_a.max(margin_b)
    } else if margin_a < 0.0 && margin_b < 0.0 {
        margin_a.min(margin_b) // Most negative
    } else {
        margin_a + margin_b // One positive, one negative: sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::Rect;

    fn style_with_width(w: f32) -> ComputedStyle {
        ComputedStyle {
            width: w,
            ..Default::default()
        }
    }

    #[test]
    fn auto_width_fills_container() {
        let mut styles = HashMap::new();
        let id = VexId::new(1);
        styles.insert(id, ComputedStyle::default()); // width: NaN = auto

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        calculate_block_width(&mut b, 800.0, &styles);
        assert_eq!(b.dimensions.content.size.width, 800.0);
    }

    #[test]
    fn explicit_width_centers_with_auto_margins() {
        let mut styles = HashMap::new();
        let id = VexId::new(1);
        let mut s = style_with_width(400.0);
        s.margin_left = f32::NAN;
        s.margin_right = f32::NAN;
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        calculate_block_width(&mut b, 800.0, &styles);
        assert_eq!(b.dimensions.content.size.width, 400.0);
        assert_eq!(b.dimensions.margin.left, 200.0);
        assert_eq!(b.dimensions.margin.right, 200.0);
    }

    #[test]
    fn border_box_sizing() {
        let mut styles = HashMap::new();
        let id = VexId::new(1);
        let mut s = style_with_width(200.0);
        s.box_sizing = BoxSizing::BorderBox;
        s.padding_left = 10.0;
        s.padding_right = 10.0;
        s.border_left_width = 2.0;
        s.border_right_width = 2.0;
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        calculate_block_width(&mut b, 800.0, &styles);
        // 200 - 10 - 10 - 2 - 2 = 176 content width
        assert_eq!(b.dimensions.content.size.width, 176.0);
    }

    #[test]
    fn margin_collapse_both_positive() {
        assert_eq!(collapse_margins(20.0, 30.0), 30.0);
    }

    #[test]
    fn margin_collapse_both_negative() {
        assert_eq!(collapse_margins(-10.0, -20.0), -20.0);
    }

    #[test]
    fn margin_collapse_mixed() {
        assert_eq!(collapse_margins(20.0, -10.0), 10.0);
    }

    #[test]
    fn auto_height_sums_children() {
        let styles = HashMap::new();
        let mut parent = LayoutBox::new(None, BoxType::Block);

        // Add two children with known heights
        let mut c1 = LayoutBox::new(None, BoxType::Block);
        c1.dimensions.content.size.height = 50.0;
        let mut c2 = LayoutBox::new(None, BoxType::Block);
        c2.dimensions.content.size.height = 70.0;

        parent.children = vec![c1, c2];
        calculate_block_height(&mut parent, &styles);
        assert_eq!(parent.dimensions.content.size.height, 120.0);
    }

    #[test]
    fn explicit_height_respected() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        let s = ComputedStyle {
            height: 300.0,
            ..Default::default()
        };
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        calculate_block_height(&mut b, &styles);
        assert_eq!(b.dimensions.content.size.height, 300.0);
    }

    #[test]
    fn clamp_applies_min_max() {
        assert_eq!(clamp_dimension(50.0, 100.0, 500.0), 100.0); // min
        assert_eq!(clamp_dimension(600.0, 100.0, 500.0), 500.0); // max
        assert_eq!(clamp_dimension(250.0, 100.0, 500.0), 250.0); // pass-through
    }

    #[test]
    fn overflow_hidden_sets_clip() {
        let id = VexId::new(1);
        let mut styles = HashMap::new();
        let s = ComputedStyle {
            overflow: Overflow::Hidden,
            ..Default::default()
        };
        styles.insert(id, s);

        let mut b = LayoutBox::new(Some(id), BoxType::Block);
        b.dimensions.content = Rect::new(0.0, 0.0, 100.0, 100.0);
        apply_overflow_clip(&mut b, &styles);
        assert!(b.clip_rect.is_some());
    }
}
