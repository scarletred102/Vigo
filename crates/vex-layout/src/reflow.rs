// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Incremental reflow planning and execution helpers.
//!
//! This module offers a lightweight invalidation plan for layout updates.
//! It supports a no-dirty fast path that reuses the previous layout tree,
//! while falling back to full layout when needed.

use std::collections::{HashMap, HashSet};

use vex_core::{Size, VexId};
use vex_css::ComputedStyle;
use vex_dom::Document;

use crate::{layout_document, LayoutBox};

/// Reflow invalidation plan.
#[derive(Debug, Clone, Default)]
pub struct ReflowPlan {
    /// Dirty nodes that require layout refresh.
    pub dirty_nodes: HashSet<VexId>,
    /// Force full reflow regardless of dirty-node list.
    pub full_reflow: bool,
}

impl ReflowPlan {
    /// Mark one node dirty.
    pub fn mark_dirty(&mut self, node_id: VexId) {
        self.dirty_nodes.insert(node_id);
    }

    /// Mark many nodes dirty.
    pub fn mark_dirty_many<I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = VexId>,
    {
        self.dirty_nodes.extend(ids);
    }

    /// Clear dirty marks and full-reflow flag.
    pub fn clear(&mut self) {
        self.dirty_nodes.clear();
        self.full_reflow = false;
    }

    /// Whether this plan requires a layout recomputation.
    pub fn needs_reflow(&self) -> bool {
        self.full_reflow || !self.dirty_nodes.is_empty()
    }
}

/// Execute layout with an invalidation plan.
///
/// - If no dirty nodes and no full-reflow request, reuses previous tree if provided.
/// - Otherwise, computes a fresh layout tree.
pub fn reflow_document(
    document: &Document,
    styles: &HashMap<VexId, ComputedStyle>,
    viewport: Size,
    previous: Option<&LayoutBox>,
    plan: &ReflowPlan,
) -> LayoutBox {
    if !plan.needs_reflow() {
        if let Some(prev) = previous {
            return prev.clone();
        }
    }

    layout_document(document, styles, viewport)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_dump;

    #[test]
    fn no_dirty_plan_reuses_previous_tree() {
        let doc = vex_html::parse_html("<html><body><div>Hello</div></body></html>");
        let styles = vex_css::compute_styles(&doc, &[], Size::new(800.0, 600.0));
        let previous = layout_document(&doc, &styles, Size::new(800.0, 600.0));

        let plan = ReflowPlan::default();
        let next = reflow_document(
            &doc,
            &styles,
            Size::new(800.0, 600.0),
            Some(&previous),
            &plan,
        );

        assert_eq!(debug_dump(&next), debug_dump(&previous));
    }

    #[test]
    fn dirty_plan_forces_relayout() {
        let doc = vex_html::parse_html("<html><body><div>Hello</div></body></html>");
        let styles = vex_css::compute_styles(&doc, &[], Size::new(800.0, 600.0));
        let previous = layout_document(&doc, &styles, Size::new(800.0, 600.0));

        let mut plan = ReflowPlan::default();
        plan.mark_dirty(doc.root());

        let next = reflow_document(
            &doc,
            &styles,
            Size::new(400.0, 300.0),
            Some(&previous),
            &plan,
        );

        // Different viewport + forced reflow should produce different geometry.
        assert_ne!(
            next.dimensions.content.size.width,
            previous.dimensions.content.size.width
        );
    }
}
