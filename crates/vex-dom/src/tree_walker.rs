// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM TreeWalker and NodeIterator implementations.
//!
//! TreeWalker provides tree-traversal with filtering.
//! NodeIterator provides flat sequential traversal with filtering.

use crate::arena::NodeArena;
use crate::node::NodeData;
use vex_core::VexId;

// ── Node Filter ──────────────────────────────────────────────────────────────

/// What to show when traversing the DOM tree.
/// Bitmask flags matching the DOM specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhatToShow(u32);

impl WhatToShow {
    pub const ALL: Self = Self(0xFFFF_FFFF);
    pub const ELEMENT: Self = Self(0x1);
    pub const TEXT: Self = Self(0x4);
    pub const COMMENT: Self = Self(0x80);
    pub const DOCUMENT: Self = Self(0x100);

    /// Check if the given node data variant passes this filter.
    pub fn accepts(&self, data: &NodeData) -> bool {
        let bit = match data {
            NodeData::Element(_) => 0x1,
            NodeData::Text(_) => 0x4,
            NodeData::Comment(_) => 0x80,
            NodeData::Document => 0x100,
            NodeData::Doctype { .. } => 0x400,
        };
        (self.0 & bit) != 0
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Result of a node filter callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// Accept the node.
    Accept,
    /// Reject the node (skip it and its subtree for TreeWalker, skip for NodeIterator).
    Reject,
    /// Skip the node but process its children.
    Skip,
}

// ── TreeWalker ───────────────────────────────────────────────────────────────

/// A TreeWalker object to navigate a DOM tree.
pub struct TreeWalker {
    root: VexId,
    what_to_show: WhatToShow,
    current_node: VexId,
}

impl TreeWalker {
    /// Create a new TreeWalker.
    pub fn new(root: VexId, what_to_show: WhatToShow) -> Self {
        Self {
            root,
            what_to_show,
            current_node: root,
        }
    }

    pub fn root(&self) -> VexId {
        self.root
    }

    pub fn current_node(&self) -> VexId {
        self.current_node
    }

    pub fn set_current_node(&mut self, node: VexId) {
        self.current_node = node;
    }

    pub fn what_to_show(&self) -> WhatToShow {
        self.what_to_show
    }

    /// Move to the parent node.
    pub fn parent_node(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut node = self.current_node;
        while node != self.root {
            let parent = arena.get(node).parent?;
            if self.what_to_show.accepts(&arena.get(parent).data) {
                self.current_node = parent;
                return Some(parent);
            }
            node = parent;
        }
        None
    }

    /// Move to the first child that passes the filter.
    pub fn first_child(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut candidate = arena.get(self.current_node).first_child;
        while let Some(id) = candidate {
            if self.what_to_show.accepts(&arena.get(id).data) {
                self.current_node = id;
                return Some(id);
            }
            candidate = arena.get(id).next_sibling;
        }
        None
    }

    /// Move to the last child that passes the filter.
    pub fn last_child(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut candidate = arena.get(self.current_node).last_child;
        while let Some(id) = candidate {
            if self.what_to_show.accepts(&arena.get(id).data) {
                self.current_node = id;
                return Some(id);
            }
            candidate = arena.get(id).prev_sibling;
        }
        None
    }

    /// Move to the next sibling that passes the filter.
    pub fn next_sibling(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut candidate = arena.get(self.current_node).next_sibling;
        while let Some(id) = candidate {
            if self.what_to_show.accepts(&arena.get(id).data) {
                self.current_node = id;
                return Some(id);
            }
            candidate = arena.get(id).next_sibling;
        }
        None
    }

    /// Move to the previous sibling that passes the filter.
    pub fn previous_sibling(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut candidate = arena.get(self.current_node).prev_sibling;
        while let Some(id) = candidate {
            if self.what_to_show.accepts(&arena.get(id).data) {
                self.current_node = id;
                return Some(id);
            }
            candidate = arena.get(id).prev_sibling;
        }
        None
    }

    /// Move to the next node in document order that passes the filter.
    pub fn next_node(&mut self, arena: &NodeArena) -> Option<VexId> {
        let mut node = self.current_node;

        loop {
            // Try first child
            if let Some(child) = arena.get(node).first_child {
                node = child;
                if self.what_to_show.accepts(&arena.get(node).data) {
                    self.current_node = node;
                    return Some(node);
                }
                continue;
            }

            // Try next sibling, walking up if needed
            loop {
                if let Some(sibling) = arena.get(node).next_sibling {
                    node = sibling;
                    if self.what_to_show.accepts(&arena.get(node).data) {
                        self.current_node = node;
                        return Some(node);
                    }
                    break; // Go back to outer loop to try this node's children
                }
                // Walk up to parent
                if node == self.root {
                    return None;
                }
                match arena.get(node).parent {
                    Some(parent) => node = parent,
                    None => return None,
                }
                if node == self.root {
                    return None;
                }
            }
        }
    }
}

// ── NodeIterator ─────────────────────────────────────────────────────────────

/// A NodeIterator for flat sequential traversal of the DOM tree.
pub struct NodeIterator {
    root: VexId,
    what_to_show: WhatToShow,
    reference_node: VexId,
    pointer_before_reference: bool,
}

impl NodeIterator {
    /// Create a new NodeIterator.
    pub fn new(root: VexId, what_to_show: WhatToShow) -> Self {
        Self {
            root,
            what_to_show,
            reference_node: root,
            pointer_before_reference: true,
        }
    }

    pub fn root(&self) -> VexId {
        self.root
    }

    pub fn reference_node(&self) -> VexId {
        self.reference_node
    }

    pub fn pointer_before_reference(&self) -> bool {
        self.pointer_before_reference
    }

    /// Get the next node in document order.
    pub fn next_node(&mut self, arena: &NodeArena) -> Option<VexId> {
        if self.pointer_before_reference {
            self.pointer_before_reference = false;
            if self.what_to_show.accepts(&arena.get(self.reference_node).data) {
                return Some(self.reference_node);
            }
        }

        // Walk in document order from reference
        let next = self.next_in_document_order(arena, self.reference_node)?;
        self.reference_node = next;
        if self.what_to_show.accepts(&arena.get(next).data) {
            Some(next)
        } else {
            self.next_node(arena)
        }
    }

    /// Get the next node in document order (depth-first pre-order).
    fn next_in_document_order(&self, arena: &NodeArena, from: VexId) -> Option<VexId> {
        // Try first child
        if let Some(child) = arena.get(from).first_child {
            return Some(child);
        }

        // Try next sibling, walking up
        let mut node = from;
        loop {
            if let Some(sibling) = arena.get(node).next_sibling {
                return Some(sibling);
            }
            if node == self.root {
                return None;
            }
            node = arena.get(node).parent?;
            if node == self.root {
                return None;
            }
        }
    }

    /// Detach — no-op per the current spec.
    pub fn detach(&self) {
        // No-op per spec
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::node::Namespace;

    fn build_test_tree() -> (Document, VexId, VexId, VexId, VexId) {
        let mut doc = Document::new();
        let root = doc.root();
        let div = doc.create_element("div", Namespace::Html);
        let p = doc.create_element("p", Namespace::Html);
        let text = doc.create_text("hello");
        doc.append_child(root, div);
        doc.append_child(div, p);
        doc.append_child(p, text);
        (doc, root, div, p, text)
    }

    #[test]
    fn what_to_show_all() {
        let ws = WhatToShow::ALL;
        assert!(ws.accepts(&NodeData::Element(crate::node::ElementData {
            tag_name: "div".into(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
            state: Default::default(),
        })));
        assert!(ws.accepts(&NodeData::Text("hello".into())));
        assert!(ws.accepts(&NodeData::Comment("c".into())));
        assert!(ws.accepts(&NodeData::Document));
    }

    #[test]
    fn what_to_show_element_only() {
        let ws = WhatToShow::ELEMENT;
        assert!(ws.accepts(&NodeData::Element(crate::node::ElementData {
            tag_name: "div".into(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
            state: Default::default(),
        })));
        assert!(!ws.accepts(&NodeData::Text("hello".into())));
        assert!(!ws.accepts(&NodeData::Comment("c".into())));
    }

    #[test]
    fn tree_walker_first_child() {
        let (doc, root, div, _p, _text) = build_test_tree();
        let mut tw = TreeWalker::new(root, WhatToShow::ELEMENT);
        let child = tw.first_child(doc.arena());
        assert_eq!(child, Some(div));
    }

    #[test]
    fn tree_walker_parent() {
        let (doc, _root, div, p, _text) = build_test_tree();
        let mut tw = TreeWalker::new(div, WhatToShow::ELEMENT);
        tw.set_current_node(p);
        let parent = tw.parent_node(doc.arena());
        assert_eq!(parent, Some(div));
    }

    #[test]
    fn tree_walker_text_filter() {
        let (doc, root, _div, _p, text) = build_test_tree();
        let mut tw = TreeWalker::new(root, WhatToShow::TEXT);
        let found = tw.next_node(doc.arena());
        assert_eq!(found, Some(text));
    }

    #[test]
    fn node_iterator_all() {
        let (doc, root, div, p, text) = build_test_tree();
        let mut iter = NodeIterator::new(root, WhatToShow::ALL);
        assert_eq!(iter.next_node(doc.arena()), Some(root));
        assert_eq!(iter.next_node(doc.arena()), Some(div));
        assert_eq!(iter.next_node(doc.arena()), Some(p));
        assert_eq!(iter.next_node(doc.arena()), Some(text));
        assert_eq!(iter.next_node(doc.arena()), None);
    }

    #[test]
    fn node_iterator_elements_only() {
        let (doc, root, div, p, _text) = build_test_tree();
        let mut iter = NodeIterator::new(root, WhatToShow::ELEMENT);
        // Root is Document type — skip
        let first = iter.next_node(doc.arena());
        assert_eq!(first, Some(div));
        let second = iter.next_node(doc.arena());
        assert_eq!(second, Some(p));
        let third = iter.next_node(doc.arena());
        assert_eq!(third, None);
    }

    #[test]
    fn filter_result_values() {
        assert_ne!(FilterResult::Accept, FilterResult::Reject);
        assert_ne!(FilterResult::Reject, FilterResult::Skip);
    }
}
