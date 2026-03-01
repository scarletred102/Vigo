// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Arena-allocated node storage for the DOM tree.

use vex_core::VexId;

use crate::node::{Node, NodeData};

/// A growable arena of DOM [`Node`]s, indexed by [`VexId`].
///
/// Nodes are never individually deallocated — the whole arena is
/// dropped when the owning [`Document`](crate::Document) is dropped.
#[derive(Debug)]
pub struct NodeArena {
    nodes: Vec<Node>,
}

impl NodeArena {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Allocate a new node and return its id.
    pub fn alloc(&mut self, data: NodeData) -> VexId {
        let id = VexId::new(self.nodes.len() as u32);
        self.nodes.push(Node {
            id,
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            data,
        });
        id
    }

    pub fn get(&self, id: VexId) -> &Node {
        &self.nodes[id.index() as usize]
    }

    pub fn get_mut(&mut self, id: VexId) -> &mut Node {
        &mut self.nodes[id.index() as usize]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for NodeArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_sequential_ids() {
        let mut arena = NodeArena::new();
        let a = arena.alloc(NodeData::Document);
        let b = arena.alloc(NodeData::Text("hi".into()));
        assert_eq!(a, VexId::new(0));
        assert_eq!(b, VexId::new(1));
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn get_round_trip() {
        let mut arena = NodeArena::new();
        let id = arena.alloc(NodeData::Comment("test".into()));
        assert!(matches!(arena.get(id).data, NodeData::Comment(ref s) if s == "test"));
    }
}
