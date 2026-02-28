// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tree mutation helpers: append, insert, remove, detach, reparent.

use vex_core::VexId;

use crate::arena::NodeArena;

/// Append `child` as the last child of `parent`.
pub fn append_child(arena: &mut NodeArena, parent: VexId, child: VexId) {
    // Detach from any previous parent first.
    detach(arena, child);

    arena.get_mut(child).parent = Some(parent);

    let old_last = arena.get(parent).last_child;
    if let Some(last_id) = old_last {
        arena.get_mut(last_id).next_sibling = Some(child);
        arena.get_mut(child).prev_sibling = Some(last_id);
    } else {
        arena.get_mut(parent).first_child = Some(child);
    }
    arena.get_mut(parent).last_child = Some(child);
}

/// Insert `child` immediately before `reference` in `parent`'s children.
pub fn insert_before(arena: &mut NodeArena, parent: VexId, child: VexId, reference: VexId) {
    detach(arena, child);

    arena.get_mut(child).parent = Some(parent);

    let prev = arena.get(reference).prev_sibling;
    arena.get_mut(child).next_sibling = Some(reference);
    arena.get_mut(child).prev_sibling = prev;
    arena.get_mut(reference).prev_sibling = Some(child);

    if let Some(prev_id) = prev {
        arena.get_mut(prev_id).next_sibling = Some(child);
    } else {
        arena.get_mut(parent).first_child = Some(child);
    }
}

/// Unlink `child` from `parent`'s children. The node stays in the arena.
pub fn remove_child(arena: &mut NodeArena, parent: VexId, child: VexId) {
    let prev = arena.get(child).prev_sibling;
    let next = arena.get(child).next_sibling;

    if let Some(prev_id) = prev {
        arena.get_mut(prev_id).next_sibling = next;
    } else {
        arena.get_mut(parent).first_child = next;
    }

    if let Some(next_id) = next {
        arena.get_mut(next_id).prev_sibling = prev;
    } else {
        arena.get_mut(parent).last_child = prev;
    }

    arena.get_mut(child).parent = None;
    arena.get_mut(child).prev_sibling = None;
    arena.get_mut(child).next_sibling = None;
}

/// Detach a node from its current parent (if any).
pub fn detach(arena: &mut NodeArena, node: VexId) {
    if let Some(parent) = arena.get(node).parent {
        remove_child(arena, parent, node);
    }
}

/// Move every child of `source` to become a child of `destination` (in order).
pub fn reparent_children(arena: &mut NodeArena, source: VexId, destination: VexId) {
    while let Some(child) = arena.get(source).first_child {
        remove_child(arena, source, child);
        append_child(arena, destination, child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeData;

    fn text(arena: &mut NodeArena, s: &str) -> VexId {
        arena.alloc(NodeData::Text(s.into()))
    }

    #[test]
    fn append_three_children() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");
        let b = text(&mut arena, "b");
        let c = text(&mut arena, "c");

        append_child(&mut arena, p, a);
        append_child(&mut arena, p, b);
        append_child(&mut arena, p, c);

        assert_eq!(arena.get(p).first_child, Some(a));
        assert_eq!(arena.get(p).last_child, Some(c));
        assert_eq!(arena.get(a).next_sibling, Some(b));
        assert_eq!(arena.get(b).next_sibling, Some(c));
        assert_eq!(arena.get(c).prev_sibling, Some(b));
        assert_eq!(arena.get(a).parent, Some(p));
    }

    #[test]
    fn insert_before_middle() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");
        let c = text(&mut arena, "c");
        let b = text(&mut arena, "b");

        append_child(&mut arena, p, a);
        append_child(&mut arena, p, c);
        insert_before(&mut arena, p, b, c);

        assert_eq!(arena.get(p).first_child, Some(a));
        assert_eq!(arena.get(a).next_sibling, Some(b));
        assert_eq!(arena.get(b).next_sibling, Some(c));
        assert_eq!(arena.get(p).last_child, Some(c));
    }

    #[test]
    fn insert_before_first() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let b = text(&mut arena, "b");
        let a = text(&mut arena, "a");

        append_child(&mut arena, p, b);
        insert_before(&mut arena, p, a, b);

        assert_eq!(arena.get(p).first_child, Some(a));
        assert_eq!(arena.get(a).next_sibling, Some(b));
        assert_eq!(arena.get(p).last_child, Some(b));
    }

    #[test]
    fn remove_middle_child() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");
        let b = text(&mut arena, "b");
        let c = text(&mut arena, "c");

        append_child(&mut arena, p, a);
        append_child(&mut arena, p, b);
        append_child(&mut arena, p, c);
        remove_child(&mut arena, p, b);

        assert_eq!(arena.get(p).first_child, Some(a));
        assert_eq!(arena.get(a).next_sibling, Some(c));
        assert_eq!(arena.get(c).prev_sibling, Some(a));
        assert_eq!(arena.get(p).last_child, Some(c));
        assert_eq!(arena.get(b).parent, None);
    }

    #[test]
    fn remove_only_child() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");

        append_child(&mut arena, p, a);
        remove_child(&mut arena, p, a);

        assert_eq!(arena.get(p).first_child, None);
        assert_eq!(arena.get(p).last_child, None);
    }

    #[test]
    fn reparent_all_children() {
        let mut arena = NodeArena::new();
        let src = arena.alloc(NodeData::Document);
        let dst = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");
        let b = text(&mut arena, "b");

        append_child(&mut arena, src, a);
        append_child(&mut arena, src, b);
        reparent_children(&mut arena, src, dst);

        assert_eq!(arena.get(src).first_child, None);
        assert_eq!(arena.get(dst).first_child, Some(a));
        assert_eq!(arena.get(dst).last_child, Some(b));
        assert_eq!(arena.get(a).parent, Some(dst));
    }

    #[test]
    fn detach_from_parent() {
        let mut arena = NodeArena::new();
        let p = arena.alloc(NodeData::Document);
        let a = text(&mut arena, "a");

        append_child(&mut arena, p, a);
        detach(&mut arena, a);

        assert_eq!(arena.get(p).first_child, None);
        assert_eq!(arena.get(a).parent, None);
    }
}
