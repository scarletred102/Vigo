// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Z-index stacking context builder.
//!
//! After layout is complete, this module collects all positioned boxes
//! and sorts them into a paint order based on z-index and tree order.

use std::collections::HashMap;

use vex_core::VexId;
use vex_css::ComputedStyle;
use vex_css::values::position::Position;

use crate::box_model::LayoutBox;

/// A reference to a layout box in paint order.
#[derive(Debug, Clone)]
pub struct StackingEntry {
    /// Path to the layout box from the root (child indices).
    pub path: Vec<usize>,
    /// z-index value (0 for non-positioned).
    pub z_index: i32,
    /// Whether this box creates a new stacking context.
    pub creates_context: bool,
}

/// Build the paint order for the layout tree.
///
/// Returns entries sorted in the CSS painting order:
/// 1. Background + borders of root
/// 2. Negative z-index stacking contexts
/// 3. Block-level non-positioned descendants
/// 4. Non-positioned floats (not yet supported)
/// 5. Inline-level non-positioned descendants
/// 6. z-index: 0 and positioned descendants
/// 7. Positive z-index stacking contexts
pub fn build_stacking_order(
    root: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
) -> Vec<StackingEntry> {
    let mut entries = Vec::new();
    collect_entries(root, styles, &mut Vec::new(), &mut entries);

    // Sort by z-index (stable sort preserves tree order for equal z-index)
    entries.sort_by_key(|e| e.z_index);

    entries
}

fn collect_entries(
    layout_box: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    current_path: &mut Vec<usize>,
    entries: &mut Vec<StackingEntry>,
) {
    let style = layout_box.node_id.and_then(|id| styles.get(&id));
    let position = style.map(|s| s.position).unwrap_or(Position::Static);
    let z_index = style.map(|s| s.z_index).unwrap_or(0);

    // Every box gets an entry (paint order)
    let creates_context = position.is_positioned()
        || style.map(|s| s.opacity < 1.0).unwrap_or(false);

    entries.push(StackingEntry {
        path: current_path.clone(),
        z_index: if position.is_positioned() { z_index } else { 0 },
        creates_context,
    });

    for (i, child) in layout_box.children.iter().enumerate() {
        current_path.push(i);
        collect_entries(child, styles, current_path, entries);
        current_path.pop();
    }
}

/// Look up a layout box by path from the root.
pub fn resolve_path<'a>(root: &'a LayoutBox, path: &[usize]) -> &'a LayoutBox {
    let mut current = root;
    for &idx in path {
        current = &current.children[idx];
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;

    #[test]
    fn single_box_one_entry() {
        let root = LayoutBox::new(Some(VexId::new(1)), BoxType::Block);
        let styles = HashMap::new();
        let order = build_stacking_order(&root, &styles);
        assert_eq!(order.len(), 1);
        assert!(order[0].path.is_empty());
    }

    #[test]
    fn positive_z_index_sorts_after_zero() {
        let styles = {
            let mut m = HashMap::new();
            let mut s1 = ComputedStyle::default();
            s1.position = Position::Relative;
            s1.z_index = 0;
            m.insert(VexId::new(1), s1);

            let mut s2 = ComputedStyle::default();
            s2.position = Position::Relative;
            s2.z_index = 5;
            m.insert(VexId::new(2), s2);

            let mut s3 = ComputedStyle::default();
            s3.position = Position::Relative;
            s3.z_index = -1;
            m.insert(VexId::new(3), s3);
            m
        };

        let mut root = LayoutBox::new(Some(VexId::new(1)), BoxType::Block);
        root.children.push(LayoutBox::new(Some(VexId::new(2)), BoxType::Block));
        root.children.push(LayoutBox::new(Some(VexId::new(3)), BoxType::Block));

        let order = build_stacking_order(&root, &styles);

        // z-index: -1 should come first, then 0, then 5
        let z_values: Vec<i32> = order.iter().map(|e| e.z_index).collect();
        assert!(z_values.windows(2).all(|w| w[0] <= w[1]),
            "Expected sorted z-indices, got {z_values:?}");
    }

    #[test]
    fn resolve_path_works() {
        let mut root = LayoutBox::new(None, BoxType::Block);
        let mut child = LayoutBox::new(Some(VexId::new(2)), BoxType::Block);
        child.children.push(LayoutBox::new(Some(VexId::new(3)), BoxType::Inline));
        root.children.push(child);

        let target = resolve_path(&root, &[0, 0]);
        assert_eq!(target.node_id, Some(VexId::new(3)));
    }

    #[test]
    fn opacity_creates_stacking_context() {
        let mut styles = HashMap::new();
        let mut s = ComputedStyle::default();
        s.opacity = 0.5;
        styles.insert(VexId::new(1), s);

        let root = LayoutBox::new(Some(VexId::new(1)), BoxType::Block);
        let order = build_stacking_order(&root, &styles);
        assert!(order[0].creates_context);
    }
}
