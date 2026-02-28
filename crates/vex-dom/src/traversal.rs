// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM tree traversal iterators.

use vex_core::VexId;

use crate::arena::NodeArena;

/// Iterates over the direct children of a node.
pub struct Children<'a> {
    arena: &'a NodeArena,
    next: Option<VexId>,
}

impl<'a> Children<'a> {
    pub fn new(arena: &'a NodeArena, parent: VexId) -> Self {
        Self {
            arena,
            next: arena.get(parent).first_child,
        }
    }
}

impl Iterator for Children<'_> {
    type Item = VexId;

    fn next(&mut self) -> Option<VexId> {
        let current = self.next?;
        self.next = self.arena.get(current).next_sibling;
        Some(current)
    }
}

/// Depth-first pre-order traversal of a subtree (excluding the root).
pub struct Descendants<'a> {
    arena: &'a NodeArena,
    root: VexId,
    current: Option<VexId>,
}

impl<'a> Descendants<'a> {
    pub fn new(arena: &'a NodeArena, root: VexId) -> Self {
        Self {
            arena,
            root,
            current: arena.get(root).first_child,
        }
    }
}

impl Iterator for Descendants<'_> {
    type Item = VexId;

    fn next(&mut self) -> Option<VexId> {
        let current = self.current?;

        // Advance: try first_child, else walk up looking for next_sibling.
        let node = self.arena.get(current);
        if node.first_child.is_some() {
            self.current = node.first_child;
        } else {
            let mut walk = current;
            loop {
                if walk == self.root {
                    self.current = None;
                    break;
                }
                let w = self.arena.get(walk);
                if w.next_sibling.is_some() {
                    self.current = w.next_sibling;
                    break;
                }
                walk = w.parent.unwrap_or(self.root);
            }
        }

        Some(current)
    }
}

/// Walks up the ancestor chain (parent, grandparent, …) from a starting node.
pub struct Ancestors<'a> {
    arena: &'a NodeArena,
    current: Option<VexId>,
}

impl<'a> Ancestors<'a> {
    pub fn new(arena: &'a NodeArena, node: VexId) -> Self {
        Self {
            arena,
            current: arena.get(node).parent,
        }
    }
}

impl Iterator for Ancestors<'_> {
    type Item = VexId;

    fn next(&mut self) -> Option<VexId> {
        let current = self.current?;
        self.current = self.arena.get(current).parent;
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::NodeArena;
    use crate::node::NodeData;
    use crate::tree::append_child;

    /// Build a small tree:
    ///   root
    ///   ├── a
    ///   │   ├── a1
    ///   │   └── a2
    ///   └── b
    fn build_tree() -> (NodeArena, VexId, VexId, VexId, VexId, VexId) {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let a = arena.alloc(NodeData::Text("a".into()));
        let a1 = arena.alloc(NodeData::Text("a1".into()));
        let a2 = arena.alloc(NodeData::Text("a2".into()));
        let b = arena.alloc(NodeData::Text("b".into()));

        append_child(&mut arena, root, a);
        append_child(&mut arena, a, a1);
        append_child(&mut arena, a, a2);
        append_child(&mut arena, root, b);

        (arena, root, a, a1, a2, b)
    }

    #[test]
    fn children_of_root() {
        let (arena, root, a, _, _, b) = build_tree();
        let kids: Vec<_> = Children::new(&arena, root).collect();
        assert_eq!(kids, vec![a, b]);
    }

    #[test]
    fn children_of_leaf() {
        let (arena, _, _, a1, _, _) = build_tree();
        let kids: Vec<_> = Children::new(&arena, a1).collect();
        assert!(kids.is_empty());
    }

    #[test]
    fn descendants_depth_first() {
        let (arena, root, a, a1, a2, b) = build_tree();
        let order: Vec<_> = Descendants::new(&arena, root).collect();
        assert_eq!(order, vec![a, a1, a2, b]);
    }

    #[test]
    fn ancestors_walk_up() {
        let (arena, root, a, a1, _, _) = build_tree();
        let chain: Vec<_> = Ancestors::new(&arena, a1).collect();
        assert_eq!(chain, vec![a, root]);
    }
}
