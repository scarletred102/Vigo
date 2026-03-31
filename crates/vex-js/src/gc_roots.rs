// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! GC root set — tracks which DOM nodes are referenced by live JS objects.
//!
//! When a JS proxy is created for a `VexId`, that ID is added to the root set.
//! The DOM arena must not logically free (recycle) any node in the root set
//! while JS holds a reference.
//!
//! ## Design
//!
//! A simple reference-counted `HashSet<VexId>` behind `Rc<RefCell<...>>`.
//! Each time a JS proxy is built, `root(id)` is called. When the proxy is
//! collected (or manually released), `unroot(id)` is called.
//!
//! Since Boa's GC doesn't expose destructor hooks on plain JS objects,
//! `unroot` is called explicitly during cleanup rather than automatically.
//! A future enhancement could use Boa's `Trace` trait for automatic cleanup.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vex_core::VexId;

/// A reference-counted set of rooted DOM node IDs.
///
/// Each ID has a reference count — rooted once per proxy that wraps it.
#[derive(Debug, Clone)]
pub struct GcRootSet {
    inner: Rc<RefCell<HashMap<VexId, u32>>>,
}

impl GcRootSet {
    /// Create a new empty root set.
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Mark a node as rooted (increment reference count).
    pub fn root(&self, id: VexId) {
        let mut map = self.inner.borrow_mut();
        *map.entry(id).or_insert(0) += 1;
    }

    /// Unroot a node (decrement reference count).
    ///
    /// When the count reaches zero, the ID is removed from the set.
    pub fn unroot(&self, id: VexId) {
        let mut map = self.inner.borrow_mut();
        if let Some(count) = map.get_mut(&id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                map.remove(&id);
            }
        }
    }

    /// Check whether a node is currently rooted.
    pub fn is_rooted(&self, id: VexId) -> bool {
        self.inner.borrow().contains_key(&id)
    }

    /// Number of distinct rooted node IDs.
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    /// Whether the root set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    /// Get all currently rooted node IDs.
    pub fn rooted_ids(&self) -> Vec<VexId> {
        self.inner.borrow().keys().copied().collect()
    }

    /// Remove all entries (full cleanup).
    pub fn clear(&self) {
        self.inner.borrow_mut().clear();
    }
}

impl Default for GcRootSet {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_and_unroot() {
        let roots = GcRootSet::new();
        let id = VexId::new(42);

        assert!(!roots.is_rooted(id));
        roots.root(id);
        assert!(roots.is_rooted(id));
        roots.unroot(id);
        assert!(!roots.is_rooted(id));
    }

    #[test]
    fn reference_counting() {
        let roots = GcRootSet::new();
        let id = VexId::new(7);

        roots.root(id);
        roots.root(id);
        assert_eq!(roots.len(), 1); // same id, refcount = 2
        roots.unroot(id);
        assert!(roots.is_rooted(id)); // refcount = 1
        roots.unroot(id);
        assert!(!roots.is_rooted(id)); // refcount = 0, removed
    }

    #[test]
    fn multiple_ids() {
        let roots = GcRootSet::new();
        roots.root(VexId::new(1));
        roots.root(VexId::new(2));
        roots.root(VexId::new(3));
        assert_eq!(roots.len(), 3);

        roots.unroot(VexId::new(2));
        assert_eq!(roots.len(), 2);
        assert!(!roots.is_rooted(VexId::new(2)));
    }

    #[test]
    fn unroot_nonexistent_is_noop() {
        let roots = GcRootSet::new();
        roots.unroot(VexId::new(99)); // should not panic
        assert!(roots.is_empty());
    }

    #[test]
    fn clear_removes_all() {
        let roots = GcRootSet::new();
        roots.root(VexId::new(1));
        roots.root(VexId::new(2));
        roots.clear();
        assert!(roots.is_empty());
    }

    #[test]
    fn clone_shares_state() {
        let roots = GcRootSet::new();
        let clone = roots.clone();
        roots.root(VexId::new(10));
        assert!(clone.is_rooted(VexId::new(10)));
    }
}
