// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Selection API — DOM Range and Selection objects.
//!
//! Implements the core of the [Selection API](https://w3c.github.io/selection-api/)
//! and [DOM Range](https://dom.spec.whatwg.org/#ranges):
//!
//! - `Range`: a contiguous region of the DOM identified by (start_node, start_offset)
//!   and (end_node, end_offset).
//! - `Selection`: the user's current highlighted selection, backed by a single Range.

use vex_core::VexId;

use crate::arena::NodeArena;
use crate::node::NodeData;
use crate::traversal::Descendants;

// ── Range ────────────────────────────────────────────────────────────────────

/// A boundary point in the DOM, consisting of a node and an offset.
///
/// For text nodes the offset is a character (UTF-16 code-unit) index.
/// For element nodes the offset is a child index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryPoint {
    pub node: VexId,
    pub offset: u32,
}

impl BoundaryPoint {
    pub fn new(node: VexId, offset: u32) -> Self {
        Self { node, offset }
    }
}

/// Comparison result for boundary points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointOrder {
    Before,
    Equal,
    After,
}

/// A contiguous range within the DOM tree.
///
/// Mirrors the W3C `Range` interface. `start` is always <= `end` (tree order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// The start boundary point.
    pub start: BoundaryPoint,
    /// The end boundary point.
    pub end: BoundaryPoint,
}

impl Range {
    /// Create a collapsed range at a single boundary point.
    pub fn collapsed(node: VexId, offset: u32) -> Self {
        let bp = BoundaryPoint::new(node, offset);
        Self {
            start: bp,
            end: bp,
        }
    }

    /// Create a range from explicit start and end points.
    pub fn new(start_node: VexId, start_offset: u32, end_node: VexId, end_offset: u32) -> Self {
        Self {
            start: BoundaryPoint::new(start_node, start_offset),
            end: BoundaryPoint::new(end_node, end_offset),
        }
    }

    /// Whether the range is collapsed (start == end).
    pub fn is_collapsed(&self) -> bool {
        self.start == self.end
    }

    /// Return the common ancestor container of start and end nodes.
    pub fn common_ancestor_container(&self, arena: &NodeArena) -> Option<VexId> {
        if self.start.node == self.end.node {
            return Some(self.start.node);
        }

        // Collect ancestors of start node.
        let start_ancestors = ancestor_chain(arena, self.start.node);
        let end_ancestors = ancestor_chain(arena, self.end.node);

        // Find the deepest common ancestor.
        start_ancestors
            .iter()
            .find(|a| end_ancestors.contains(a))
            .copied()
    }

    /// Collapse the range to its start point.
    pub fn collapse_to_start(&mut self) {
        self.end = self.start;
    }

    /// Collapse the range to its end point.
    pub fn collapse_to_end(&mut self) {
        self.start = self.end;
    }

    /// Set the start boundary point.
    pub fn set_start(&mut self, node: VexId, offset: u32) {
        self.start = BoundaryPoint::new(node, offset);
        // If start is now after end, collapse to start.
        if self.start.node == self.end.node && self.start.offset > self.end.offset {
            self.end = self.start;
        }
    }

    /// Set the end boundary point.
    pub fn set_end(&mut self, node: VexId, offset: u32) {
        self.end = BoundaryPoint::new(node, offset);
        // If end is before start, collapse to end.
        if self.start.node == self.end.node && self.end.offset < self.start.offset {
            self.start = self.end;
        }
    }

    /// Select a single node — start is (parent, index), end is (parent, index + 1).
    pub fn select_node(&mut self, arena: &NodeArena, node_id: VexId) {
        let parent = match arena.get(node_id).parent {
            Some(p) => p,
            None => return,
        };

        let index = child_index(arena, parent, node_id);
        self.start = BoundaryPoint::new(parent, index as u32);
        self.end = BoundaryPoint::new(parent, (index + 1) as u32);
    }

    /// Select the contents of a node (start = (node, 0), end = (node, child_count or text_len)).
    pub fn select_node_contents(&mut self, arena: &NodeArena, node_id: VexId) {
        let length = node_length_with_arena(arena, node_id);
        self.start = BoundaryPoint::new(node_id, 0);
        self.end = BoundaryPoint::new(node_id, length);
    }

    /// Clone the range for non-destructive operations.
    pub fn clone_range(&self) -> Range {
        self.clone()
    }

    /// Extract the text content within this range.
    pub fn text_content(&self, arena: &NodeArena) -> String {
        if self.is_collapsed() {
            return String::new();
        }

        if self.start.node == self.end.node {
            // Same node — extract substring if text.
            let node = arena.get(self.start.node);
            if let NodeData::Text(ref t) = node.data {
                let start = self.start.offset as usize;
                let end = self.end.offset as usize;
                return safe_substring(t, start, end);
            }
        }

        // Multi-node range: collect text from descendants in tree order.
        let common_ancestor = match self.common_ancestor_container(arena) {
            Some(id) => id,
            None => return String::new(),
        };

        let mut result = String::new();
        let mut in_range = false;

        for desc in Descendants::new(arena, common_ancestor) {
            if desc == self.start.node {
                in_range = true;
                let node = arena.get(desc);
                if let NodeData::Text(ref t) = node.data {
                    let start = self.start.offset as usize;
                    if desc == self.end.node {
                        let end = self.end.offset as usize;
                        result.push_str(&safe_substring(t, start, end));
                        break;
                    }
                    result.push_str(&safe_substring(t, start, t.len()));
                }
                continue;
            }

            if desc == self.end.node {
                let node = arena.get(desc);
                if let NodeData::Text(ref t) = node.data {
                    let end = self.end.offset as usize;
                    result.push_str(&safe_substring(t, 0, end));
                }
                break;
            }

            if in_range {
                let node = arena.get(desc);
                if let NodeData::Text(ref t) = node.data {
                    result.push_str(t);
                }
            }
        }

        result
    }
}

// ── Selection ────────────────────────────────────────────────────────────────

/// The direction of a Selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionDirection {
    /// The anchor is before or at the focus (normal left-to-right selection).
    #[default]
    Forward,
    /// The focus is before the anchor (reverse / right-to-left selection).
    Backward,
    /// Direction is unspecified.
    None,
}

/// The user's current text selection in a document.
///
/// A Selection wraps a single Range. The *anchor* is the point where the user
/// started selecting, and the *focus* is the current end of the selection.
#[derive(Debug, Clone)]
pub struct Selection {
    /// The underlying range (if any).
    range: Option<Range>,
    /// Which end of the range is the anchor vs focus.
    direction: SelectionDirection,
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

impl Selection {
    pub fn new() -> Self {
        Self {
            range: None,
            direction: SelectionDirection::None,
        }
    }

    /// The anchor node (where the selection started).
    pub fn anchor_node(&self) -> Option<VexId> {
        self.range.as_ref().map(|r| match self.direction {
            SelectionDirection::Backward => r.end.node,
            _ => r.start.node,
        })
    }

    /// The anchor offset.
    pub fn anchor_offset(&self) -> u32 {
        self.range
            .as_ref()
            .map(|r| match self.direction {
                SelectionDirection::Backward => r.end.offset,
                _ => r.start.offset,
            })
            .unwrap_or(0)
    }

    /// The focus node (current end of selection).
    pub fn focus_node(&self) -> Option<VexId> {
        self.range.as_ref().map(|r| match self.direction {
            SelectionDirection::Backward => r.start.node,
            _ => r.end.node,
        })
    }

    /// The focus offset.
    pub fn focus_offset(&self) -> u32 {
        self.range
            .as_ref()
            .map(|r| match self.direction {
                SelectionDirection::Backward => r.start.offset,
                _ => r.end.offset,
            })
            .unwrap_or(0)
    }

    /// Whether the selection is collapsed (empty).
    pub fn is_collapsed(&self) -> bool {
        self.range.as_ref().map_or(true, |r| r.is_collapsed())
    }

    /// Number of ranges in this selection (always 0 or 1).
    pub fn range_count(&self) -> usize {
        usize::from(self.range.is_some())
    }

    /// Get the range at the given index (only index 0 is valid).
    pub fn get_range_at(&self, index: usize) -> Option<&Range> {
        if index == 0 {
            self.range.as_ref()
        } else {
            Option::None
        }
    }

    /// Remove all ranges (clear selection).
    pub fn remove_all_ranges(&mut self) {
        self.range = None;
        self.direction = SelectionDirection::None;
    }

    /// Add a range (replaces any existing range, per spec behavior).
    pub fn add_range(&mut self, range: Range) {
        self.direction = SelectionDirection::Forward;
        self.range = Some(range);
    }

    /// Collapse the selection to a single point.
    pub fn collapse(&mut self, node: VexId, offset: u32) {
        self.range = Some(Range::collapsed(node, offset));
        self.direction = SelectionDirection::None;
    }

    /// Collapse to the start of the current range.
    pub fn collapse_to_start(&mut self) {
        if let Some(ref r) = self.range {
            let start = r.start;
            self.range = Some(Range::collapsed(start.node, start.offset));
            self.direction = SelectionDirection::None;
        }
    }

    /// Collapse to the end of the current range.
    pub fn collapse_to_end(&mut self) {
        if let Some(ref r) = self.range {
            let end = r.end;
            self.range = Some(Range::collapsed(end.node, end.offset));
            self.direction = SelectionDirection::None;
        }
    }

    /// Extend the selection's focus to the given point.
    pub fn extend(&mut self, node: VexId, offset: u32) {
        if let Some(ref mut range) = self.range {
            match self.direction {
                SelectionDirection::Backward => {
                    range.start = BoundaryPoint::new(node, offset);
                }
                _ => {
                    range.end = BoundaryPoint::new(node, offset);
                }
            }
        } else {
            // No range yet — create collapsed at focus point.
            self.range = Some(Range::collapsed(node, offset));
        }
    }

    /// Select all children of a node.
    pub fn select_all_children(&mut self, arena: &NodeArena, node_id: VexId) {
        let length = node_length_with_arena(arena, node_id);
        self.range = Some(Range::new(node_id, 0, node_id, length));
        self.direction = SelectionDirection::Forward;
    }

    /// Get the selection as a string (the text content of the range).
    pub fn to_string(&self, arena: &NodeArena) -> String {
        match self.range {
            Some(ref r) => r.text_content(arena),
            None => String::new(),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build the ancestor chain for a node (including itself), root-last.
fn ancestor_chain(arena: &NodeArena, mut node_id: VexId) -> Vec<VexId> {
    let mut chain = vec![node_id];
    while let Some(parent) = arena.get(node_id).parent {
        chain.push(parent);
        node_id = parent;
    }
    chain
}

/// Find the index of `child_id` among the children of `parent_id`.
fn child_index(arena: &NodeArena, parent_id: VexId, child_id: VexId) -> usize {
    let mut index = 0;
    let mut cursor = arena.get(parent_id).first_child;
    while let Some(id) = cursor {
        if id == child_id {
            return index;
        }
        index += 1;
        cursor = arena.get(id).next_sibling;
    }
    index
}

/// Arena-aware version of node length for element nodes.
fn node_length_with_arena(arena: &NodeArena, node_id: VexId) -> u32 {
    let node = arena.get(node_id);
    match &node.data {
        NodeData::Text(t) => t.len() as u32,
        NodeData::Comment(c) => c.len() as u32,
        _ => {
            let mut count = 0u32;
            let mut cursor = node.first_child;
            while let Some(id) = cursor {
                count += 1;
                cursor = arena.get(id).next_sibling;
            }
            count
        }
    }
}

/// Safely substring a string by byte offsets, clamping to bounds.
fn safe_substring(s: &str, start: usize, end: usize) -> String {
    let start = start.min(s.len());
    let end = end.min(s.len());
    if start >= end {
        return String::new();
    }
    s[start..end].to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::node::Namespace;

    fn make_doc() -> Document {
        Document::new()
    }

    #[test]
    fn collapsed_range() {
        let node = VexId::new(1);
        let r = Range::collapsed(node, 5);
        assert!(r.is_collapsed());
        assert_eq!(r.start.offset, 5);
        assert_eq!(r.end.offset, 5);
    }

    #[test]
    fn range_new() {
        let n1 = VexId::new(1);
        let n2 = VexId::new(2);
        let r = Range::new(n1, 0, n2, 10);
        assert!(!r.is_collapsed());
        assert_eq!(r.start.node, n1);
        assert_eq!(r.end.node, n2);
    }

    #[test]
    fn collapse_to_start() {
        let mut r = Range::new(VexId::new(1), 3, VexId::new(2), 7);
        r.collapse_to_start();
        assert!(r.is_collapsed());
        assert_eq!(r.end.node, VexId::new(1));
        assert_eq!(r.end.offset, 3);
    }

    #[test]
    fn collapse_to_end() {
        let mut r = Range::new(VexId::new(1), 3, VexId::new(2), 7);
        r.collapse_to_end();
        assert!(r.is_collapsed());
        assert_eq!(r.start.node, VexId::new(2));
        assert_eq!(r.start.offset, 7);
    }

    #[test]
    fn common_ancestor_same_node() {
        let doc = make_doc();
        let root = doc.root();
        let r = Range::new(root, 0, root, 1);
        assert_eq!(r.common_ancestor_container(doc.arena()), Some(root));
    }

    #[test]
    fn common_ancestor_parent_child() {
        let mut doc = make_doc();
        let root = doc.root();
        let child = doc.create_element("div", Namespace::Html);
        doc.append_child(root, child);
        let grandchild = doc.create_element("span", Namespace::Html);
        doc.append_child(child, grandchild);

        let r = Range::new(child, 0, grandchild, 0);
        assert_eq!(r.common_ancestor_container(doc.arena()), Some(child));
    }

    #[test]
    fn common_ancestor_siblings() {
        let mut doc = make_doc();
        let root = doc.root();
        let c1 = doc.create_element("p", Namespace::Html);
        doc.append_child(root, c1);
        let c2 = doc.create_element("p", Namespace::Html);
        doc.append_child(root, c2);

        let r = Range::new(c1, 0, c2, 0);
        assert_eq!(r.common_ancestor_container(doc.arena()), Some(root));
    }

    #[test]
    fn text_content_same_text_node() {
        let mut doc = make_doc();
        let root = doc.root();
        let text = doc.create_text("Hello, world!");
        doc.append_child(root, text);

        let r = Range::new(text, 7, text, 12);
        assert_eq!(r.text_content(doc.arena()), "world");
    }

    #[test]
    fn text_content_collapsed_is_empty() {
        let doc = make_doc();
        let r = Range::collapsed(doc.root(), 0);
        assert_eq!(r.text_content(doc.arena()), "");
    }

    #[test]
    fn text_content_multi_node() {
        let mut doc = make_doc();
        let root = doc.root();
        let t1 = doc.create_text("Hello ");
        doc.append_child(root, t1);
        let t2 = doc.create_text("brave ");
        doc.append_child(root, t2);
        let t3 = doc.create_text("world");
        doc.append_child(root, t3);

        let r = Range::new(t1, 0, t3, 5);
        assert_eq!(r.text_content(doc.arena()), "Hello brave world");
    }

    #[test]
    fn select_node_contents() {
        let mut doc = make_doc();
        let root = doc.root();
        let text = doc.create_text("Test data");
        doc.append_child(root, text);

        let mut r = Range::collapsed(text, 0);
        r.select_node_contents(doc.arena(), text);
        assert_eq!(r.start.offset, 0);
        assert_eq!(r.end.offset, 9);
    }

    #[test]
    fn select_node() {
        let mut doc = make_doc();
        let root = doc.root();
        let p = doc.create_element("p", Namespace::Html);
        doc.append_child(root, p);

        let mut r = Range::collapsed(root, 0);
        r.select_node(doc.arena(), p);
        assert_eq!(r.start.node, root);
        assert_eq!(r.start.offset, 0);
        assert_eq!(r.end.node, root);
        assert_eq!(r.end.offset, 1);
    }

    #[test]
    fn selection_default_is_empty() {
        let sel = Selection::new();
        assert!(sel.is_collapsed());
        assert_eq!(sel.range_count(), 0);
        assert!(sel.anchor_node().is_none());
    }

    #[test]
    fn selection_add_range() {
        let mut sel = Selection::new();
        let r = Range::new(VexId::new(1), 0, VexId::new(1), 5);
        sel.add_range(r);
        assert_eq!(sel.range_count(), 1);
        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor_node(), Some(VexId::new(1)));
        assert_eq!(sel.focus_node(), Some(VexId::new(1)));
        assert_eq!(sel.focus_offset(), 5);
    }

    #[test]
    fn selection_collapse() {
        let mut sel = Selection::new();
        sel.add_range(Range::new(VexId::new(1), 0, VexId::new(1), 5));
        sel.collapse(VexId::new(2), 3);
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor_node(), Some(VexId::new(2)));
    }

    #[test]
    fn selection_remove_all_ranges() {
        let mut sel = Selection::new();
        sel.add_range(Range::new(VexId::new(1), 0, VexId::new(1), 5));
        sel.remove_all_ranges();
        assert_eq!(sel.range_count(), 0);
        assert!(sel.is_collapsed());
    }

    #[test]
    fn selection_collapse_to_start_and_end() {
        let mut sel = Selection::new();
        sel.add_range(Range::new(VexId::new(1), 2, VexId::new(1), 8));

        let mut sel2 = sel.clone();
        sel.collapse_to_start();
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor_offset(), 2);

        sel2.collapse_to_end();
        assert!(sel2.is_collapsed());
        assert_eq!(sel2.anchor_offset(), 8);
    }

    #[test]
    fn selection_extend() {
        let mut sel = Selection::new();
        sel.collapse(VexId::new(1), 0);
        sel.extend(VexId::new(1), 10);
        assert!(!sel.is_collapsed());
        assert_eq!(sel.focus_offset(), 10);
    }

    #[test]
    fn selection_select_all_children() {
        let mut doc = make_doc();
        let root = doc.root();
        let t1 = doc.create_text("A");
        doc.append_child(root, t1);
        let t2 = doc.create_text("B");
        doc.append_child(root, t2);

        let mut sel = Selection::new();
        sel.select_all_children(doc.arena(), root);
        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor_node(), Some(root));
        assert_eq!(sel.anchor_offset(), 0);
        assert_eq!(sel.focus_offset(), 2);
    }

    #[test]
    fn selection_to_string() {
        let mut doc = make_doc();
        let root = doc.root();
        let text = doc.create_text("Hello, world!");
        doc.append_child(root, text);

        let mut sel = Selection::new();
        sel.add_range(Range::new(text, 7, text, 12));
        assert_eq!(sel.to_string(doc.arena()), "world");
    }

    #[test]
    fn selection_backward_direction() {
        let mut sel = Selection::new();
        sel.add_range(Range::new(VexId::new(1), 0, VexId::new(1), 10));
        sel.direction = SelectionDirection::Backward;
        assert_eq!(sel.anchor_node(), Some(VexId::new(1)));
        assert_eq!(sel.anchor_offset(), 10);
        assert_eq!(sel.focus_offset(), 0);
    }

    #[test]
    fn range_set_start_collapses_if_past_end() {
        let node = VexId::new(1);
        let mut r = Range::new(node, 0, node, 5);
        r.set_start(node, 8);
        assert!(r.is_collapsed());
        assert_eq!(r.start.offset, 8);
        assert_eq!(r.end.offset, 8);
    }

    #[test]
    fn range_set_end_collapses_if_before_start() {
        let node = VexId::new(1);
        let mut r = Range::new(node, 5, node, 10);
        r.set_end(node, 2);
        assert!(r.is_collapsed());
        assert_eq!(r.start.offset, 2);
        assert_eq!(r.end.offset, 2);
    }

    #[test]
    fn clone_range() {
        let r = Range::new(VexId::new(1), 3, VexId::new(2), 7);
        let r2 = r.clone_range();
        assert_eq!(r, r2);
    }

    #[test]
    fn node_length_text() {
        let mut doc = make_doc();
        let text = doc.create_text("Hello");
        assert_eq!(node_length_with_arena(doc.arena(), text), 5);
    }

    #[test]
    fn node_length_with_arena_element() {
        let mut doc = make_doc();
        let root = doc.root();
        let c1 = doc.create_element("a", Namespace::Html);
        doc.append_child(root, c1);
        let c2 = doc.create_element("b", Namespace::Html);
        doc.append_child(root, c2);
        assert_eq!(node_length_with_arena(doc.arena(), root), 2);
    }
}
