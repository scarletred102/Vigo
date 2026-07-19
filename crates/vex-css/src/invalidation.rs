// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Style invalidation — tracks which elements need re-styling after DOM mutations.
//!
//! When the DOM changes (attribute set, class toggled, element inserted/removed,
//! text content modified), the invalidation system marks affected nodes as "dirty"
//! so the next style recalc only processes changed subtrees instead of
//! recomputing the entire document.
//!
//! This module provides:
//!
//! - [`InvalidationMap`] — per-document dirty set of `VexId` nodes.
//! - [`DomMutation`] — enum describing what changed.
//! - [`invalidate`] — given a mutation, marks the appropriate nodes dirty.

use std::collections::HashSet;

use vex_core::VexId;
use vex_dom::document::Document;

/// A mutation that occurred in the DOM.
#[derive(Debug, Clone, PartialEq)]
pub enum DomMutation {
    /// An attribute was set / changed / removed on `node`.
    AttributeChanged { node: VexId, name: String },
    /// A class was added or removed on `node`.
    ClassChanged { node: VexId },
    /// The inline `style` attribute changed on `node`.
    InlineStyleChanged { node: VexId },
    /// A child was inserted/appended under `parent`.
    ChildInserted { parent: VexId, child: VexId },
    /// A child was removed from `parent`.
    ChildRemoved { parent: VexId, child: VexId },
    /// The text content of a text node changed.
    TextChanged { node: VexId },
    /// A node was moved (remove + insert).
    NodeMoved {
        old_parent: VexId,
        new_parent: VexId,
        node: VexId,
    },
}

/// Tracks dirty nodes that need style recalculation.
#[derive(Debug, Default, Clone)]
pub struct InvalidationMap {
    /// Nodes whose own computed style is stale.
    dirty_self: HashSet<VexId>,
    /// Nodes whose *descendants* may need re-styling
    /// (e.g., parent class change that affects child selectors).
    dirty_descendants: HashSet<VexId>,
    /// Whether the entire document needs a full restyle
    /// (e.g., stylesheet was added/removed).
    full_restyle: bool,
}

impl InvalidationMap {
    /// Create an empty invalidation map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a single node as needing re-styling.
    pub fn mark_dirty(&mut self, id: VexId) {
        self.dirty_self.insert(id);
    }

    /// Mark a node's descendants as needing re-styling.
    pub fn mark_descendants_dirty(&mut self, id: VexId) {
        self.dirty_descendants.insert(id);
    }

    /// Mark the entire document for full restyle.
    pub fn mark_full_restyle(&mut self) {
        self.full_restyle = true;
    }

    /// Whether a node needs its own style recalculated.
    pub fn is_dirty(&self, id: VexId) -> bool {
        self.full_restyle || self.dirty_self.contains(&id)
    }

    /// Whether a node's descendants need re-styling.
    pub fn has_dirty_descendants(&self, id: VexId) -> bool {
        self.full_restyle || self.dirty_descendants.contains(&id)
    }

    /// Whether a full restyle is needed.
    pub fn needs_full_restyle(&self) -> bool {
        self.full_restyle
    }

    /// Whether there are any dirty nodes at all.
    pub fn has_dirty_nodes(&self) -> bool {
        self.full_restyle || !self.dirty_self.is_empty() || !self.dirty_descendants.is_empty()
    }

    /// Number of individually dirty nodes (not counting full restyle).
    pub fn dirty_count(&self) -> usize {
        self.dirty_self.len()
    }

    /// Clear all dirty state after a restyle pass.
    pub fn clear(&mut self) {
        self.dirty_self.clear();
        self.dirty_descendants.clear();
        self.full_restyle = false;
    }

    /// Iterate over all individually dirty node IDs.
    pub fn dirty_nodes(&self) -> impl Iterator<Item = VexId> + '_ {
        self.dirty_self.iter().copied()
    }

    /// Iterate over nodes whose descendants are dirty.
    pub fn dirty_descendant_roots(&self) -> impl Iterator<Item = VexId> + '_ {
        self.dirty_descendants.iter().copied()
    }
}

/// Process a DOM mutation and mark the appropriate nodes dirty.
///
/// For class/attribute/style changes, only the affected node is marked dirty
/// plus its ancestors with `dirty_descendants` (since selectors like
/// `.parent .child` can propagate changes downward).
///
/// For structural changes (insert/remove), both parent and child subtrees
/// are marked.
pub fn invalidate(map: &mut InvalidationMap, doc: &Document, mutation: &DomMutation) {
    match mutation {
        DomMutation::AttributeChanged { node, name } => {
            map.mark_dirty(*node);
            // `id` and `class` changes can affect selector matching globally.
            if name == "id" || name == "class" {
                map.mark_descendants_dirty(*node);
                mark_ancestors_dirty(map, doc, *node);
            }
        }
        DomMutation::ClassChanged { node } => {
            map.mark_dirty(*node);
            map.mark_descendants_dirty(*node);
            mark_ancestors_dirty(map, doc, *node);
        }
        DomMutation::InlineStyleChanged { node } => {
            // Inline style only affects the node itself.
            map.mark_dirty(*node);
        }
        DomMutation::ChildInserted { parent, child } => {
            map.mark_dirty(*child);
            map.mark_dirty(*parent);
            map.mark_descendants_dirty(*parent);
            // Siblings may be affected by :first-child, :nth-child, etc.
            mark_siblings_dirty(map, doc, *parent);
        }
        DomMutation::ChildRemoved { parent, child } => {
            map.mark_dirty(*parent);
            map.mark_dirty(*child);
            map.mark_descendants_dirty(*parent);
            mark_siblings_dirty(map, doc, *parent);
        }
        DomMutation::TextChanged { node } => {
            map.mark_dirty(*node);
            // Text changes can affect :empty pseudo-class on parent.
            if let Some(parent_id) = doc.arena().get(*node).parent {
                map.mark_dirty(parent_id);
            }
        }
        DomMutation::NodeMoved {
            old_parent,
            new_parent,
            node,
        } => {
            map.mark_dirty(*node);
            map.mark_dirty(*old_parent);
            map.mark_dirty(*new_parent);
            map.mark_descendants_dirty(*old_parent);
            map.mark_descendants_dirty(*new_parent);
            mark_siblings_dirty(map, doc, *old_parent);
            mark_siblings_dirty(map, doc, *new_parent);
        }
    }
}

/// Mark all ancestors of a node as having dirty descendants.
fn mark_ancestors_dirty(map: &mut InvalidationMap, doc: &Document, node: VexId) {
    let mut current = node;
    while let Some(parent) = doc.arena().get(current).parent {
        map.mark_descendants_dirty(parent);
        current = parent;
    }
}

/// Mark all children of a parent as dirty (for sibling-related pseudo-classes).
fn mark_siblings_dirty(map: &mut InvalidationMap, doc: &Document, parent: VexId) {
    let arena = doc.arena();
    let mut child_opt = arena.get(parent).first_child;
    while let Some(child) = child_opt {
        map.mark_dirty(child);
        child_opt = arena.get(child).next_sibling;
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use vex_dom::document::Document;
    use vex_dom::node::Namespace;

    fn simple_doc() -> Document {
        let mut doc = Document::new();
        let body = doc.create_element("body", Namespace::Html);
        let root = doc.root();
        doc.append_child(root, body);
        let div = doc.create_element("div", Namespace::Html);
        doc.append_child(body, div);
        let span = doc.create_element("span", Namespace::Html);
        doc.append_child(div, span);
        doc
    }

    #[test]
    fn empty_map_has_no_dirty() {
        let map = InvalidationMap::new();
        assert!(!map.has_dirty_nodes());
        assert_eq!(map.dirty_count(), 0);
        assert!(!map.needs_full_restyle());
    }

    #[test]
    fn mark_dirty_single_node() {
        let mut map = InvalidationMap::new();
        let id = VexId::new(5);
        map.mark_dirty(id);
        assert!(map.is_dirty(id));
        assert!(!map.is_dirty(VexId::new(6)));
        assert_eq!(map.dirty_count(), 1);
    }

    #[test]
    fn mark_full_restyle() {
        let mut map = InvalidationMap::new();
        map.mark_full_restyle();
        assert!(map.needs_full_restyle());
        // Every node is "dirty" under full restyle.
        assert!(map.is_dirty(VexId::new(999)));
        assert!(map.has_dirty_descendants(VexId::new(999)));
    }

    #[test]
    fn clear_resets_everything() {
        let mut map = InvalidationMap::new();
        map.mark_dirty(VexId::new(1));
        map.mark_full_restyle();
        map.clear();
        assert!(!map.has_dirty_nodes());
        assert!(!map.needs_full_restyle());
        assert_eq!(map.dirty_count(), 0);
    }

    #[test]
    fn attribute_change_marks_node() {
        let doc = simple_doc();
        let mut map = InvalidationMap::new();
        // Change a non-class/non-id attribute on div (id=2).
        invalidate(
            &mut map,
            &doc,
            &DomMutation::AttributeChanged {
                node: VexId::new(2),
                name: "title".to_string(),
            },
        );
        assert!(map.is_dirty(VexId::new(2)));
        // Should NOT mark descendants dirty for a generic attribute.
        assert!(!map.has_dirty_descendants(VexId::new(2)));
    }

    #[test]
    fn class_change_marks_node_and_ancestors() {
        let doc = simple_doc();
        let mut map = InvalidationMap::new();
        // Change class on div (id=2). Body is parent (id=1).
        invalidate(
            &mut map,
            &doc,
            &DomMutation::ClassChanged {
                node: VexId::new(2),
            },
        );
        assert!(map.is_dirty(VexId::new(2)));
        assert!(map.has_dirty_descendants(VexId::new(2)));
        // Ancestor (body=1) should have dirty_descendants.
        assert!(map.has_dirty_descendants(VexId::new(1)));
    }

    #[test]
    fn inline_style_only_marks_self() {
        let doc = simple_doc();
        let mut map = InvalidationMap::new();
        invalidate(
            &mut map,
            &doc,
            &DomMutation::InlineStyleChanged {
                node: VexId::new(3),
            },
        );
        assert!(map.is_dirty(VexId::new(3)));
        assert!(!map.has_dirty_descendants(VexId::new(3)));
        assert_eq!(map.dirty_count(), 1);
    }

    #[test]
    fn child_insert_marks_parent_and_siblings() {
        let doc = simple_doc();
        let mut map = InvalidationMap::new();
        invalidate(
            &mut map,
            &doc,
            &DomMutation::ChildInserted {
                parent: VexId::new(2),
                child: VexId::new(10),
            },
        );
        assert!(map.is_dirty(VexId::new(10))); // new child
        assert!(map.is_dirty(VexId::new(2))); // parent
        assert!(map.has_dirty_descendants(VexId::new(2)));
        // Sibling span(3) should be dirty.
        assert!(map.is_dirty(VexId::new(3)));
    }

    #[test]
    fn text_change_marks_self_and_parent() {
        let mut doc = Document::new();
        let body = doc.create_element("body", Namespace::Html);
        let root = doc.root();
        doc.append_child(root, body);
        let text = doc.create_text("hello");
        doc.append_child(body, text);

        let mut map = InvalidationMap::new();
        invalidate(&mut map, &doc, &DomMutation::TextChanged { node: text });
        assert!(map.is_dirty(text));
        assert!(map.is_dirty(body)); // parent affected by :empty
    }

    #[test]
    fn dirty_nodes_iterator() {
        let mut map = InvalidationMap::new();
        map.mark_dirty(VexId::new(1));
        map.mark_dirty(VexId::new(5));
        map.mark_dirty(VexId::new(3));

        let mut nodes: Vec<u32> = map.dirty_nodes().map(|id| id.index()).collect();
        nodes.sort();
        assert_eq!(nodes, vec![1, 3, 5]);
    }
}
