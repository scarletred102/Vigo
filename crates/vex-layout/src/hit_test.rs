// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Layout tree hit-testing — find the deepest element at a screen position.
//!
//! Used for click events, hover detection, cursor changes, and context menus.
//! Walks the layout tree in reverse paint order (last child first) so that
//! visually-on-top elements are found first.

use vex_core::{Point, VexId};

use crate::box_model::LayoutBox;

/// Find the deepest layout box whose border-box contains the given point.
///
/// Coordinates are in content space (caller should adjust for scroll offset).
/// Returns `None` if the point is outside every layout box.
pub fn hit_test(root: &LayoutBox, x: f32, y: f32) -> Option<VexId> {
    hit_test_recursive(root, Point::new(x, y))
}

/// Recursive hit-test in reverse paint order.
fn hit_test_recursive(layout_box: &LayoutBox, point: Point) -> Option<VexId> {
    let border_box = layout_box.dimensions.border_box();

    // Early-out if point is outside this box's border-box.
    if !border_box.contains(point) {
        return None;
    }

    // If this box clips overflow, children outside the content rect are invisible.
    if let Some(clip) = layout_box.clip_rect {
        if !clip.contains(point) {
            // Point is inside border-box but outside clip — still this box,
            // but no children are hit.
            return layout_box.node_id;
        }
    }

    // Traverse children in reverse order (last painted = topmost layer).
    for child in layout_box.children.iter().rev() {
        if let Some(hit) = hit_test_recursive(child, point) {
            return Some(hit);
        }
    }

    // No child was hit — return this box if it's tied to a DOM node.
    layout_box.node_id
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::{BoxType, Dimensions};
    use vex_core::{Insets, Rect, Size};

    fn make_box(id: Option<VexId>, x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        LayoutBox {
            node_id: id,
            box_type: BoxType::Block,
            dimensions: Dimensions {
                content: Rect {
                    origin: Point::new(x, y),
                    size: Size::new(w, h),
                },
                padding: Insets::default(),
                border: Insets::default(),
                margin: Insets::default(),
            },
            children: Vec::new(),
            clip_rect: None,
            scroll_offset: Point::default(),
        }
    }

    #[test]
    fn hit_test_returns_none_outside_root() {
        let root = make_box(Some(VexId::new(1)), 0.0, 0.0, 100.0, 100.0);
        assert_eq!(hit_test(&root, 200.0, 200.0), None);
    }

    #[test]
    fn hit_test_returns_root_inside() {
        let root = make_box(Some(VexId::new(1)), 0.0, 0.0, 100.0, 100.0);
        assert_eq!(hit_test(&root, 50.0, 50.0), Some(VexId::new(1)));
    }

    #[test]
    fn hit_test_returns_deepest_child() {
        let child = make_box(Some(VexId::new(2)), 10.0, 10.0, 50.0, 50.0);
        let mut root = make_box(Some(VexId::new(1)), 0.0, 0.0, 100.0, 100.0);
        root.children.push(child);

        // Inside child → returns child
        assert_eq!(hit_test(&root, 20.0, 20.0), Some(VexId::new(2)));
        // Outside child but inside root → returns root
        assert_eq!(hit_test(&root, 80.0, 80.0), Some(VexId::new(1)));
    }

    #[test]
    fn hit_test_reverse_paint_order() {
        // Two overlapping children — second child is "on top"
        let child_a = make_box(Some(VexId::new(2)), 10.0, 10.0, 50.0, 50.0);
        let child_b = make_box(Some(VexId::new(3)), 30.0, 30.0, 50.0, 50.0);
        let mut root = make_box(Some(VexId::new(1)), 0.0, 0.0, 200.0, 200.0);
        root.children.push(child_a);
        root.children.push(child_b);

        // Overlap area at (40, 40) — child_b (last) wins
        assert_eq!(hit_test(&root, 40.0, 40.0), Some(VexId::new(3)));
    }

    #[test]
    fn hit_test_with_border_box() {
        let mut bx = make_box(Some(VexId::new(1)), 20.0, 20.0, 60.0, 60.0);
        bx.dimensions.border = Insets::new(5.0, 5.0, 5.0, 5.0);

        // Point at (16, 20) is inside the border-box (content.x - border.left = 15)
        assert_eq!(hit_test(&bx, 16.0, 20.0), Some(VexId::new(1)));
        // Point at (14, 20) is outside the border-box
        assert_eq!(hit_test(&bx, 14.0, 20.0), None);
    }

    #[test]
    fn hit_test_anonymous_box_skipped() {
        // Anonymous box has no node_id
        let child = make_box(Some(VexId::new(2)), 10.0, 10.0, 50.0, 50.0);
        let mut anon = make_box(None, 0.0, 0.0, 100.0, 100.0);
        anon.children.push(child);
        let mut root = make_box(Some(VexId::new(1)), 0.0, 0.0, 200.0, 200.0);
        root.children.push(anon);

        // Inside child → returns child
        assert_eq!(hit_test(&root, 20.0, 20.0), Some(VexId::new(2)));
        // Inside anon but outside child → returns None from anon, falls through to root
        assert_eq!(hit_test(&root, 80.0, 80.0), Some(VexId::new(1)));
    }
}
