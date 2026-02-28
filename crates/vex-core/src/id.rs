// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Lightweight node IDs and a recycling allocator.

use std::fmt;

/// An opaque handle used as an arena index for DOM nodes, layout boxes, etc.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VexId(u32);

impl VexId {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn index(self) -> u32 {
        self.0
    }
}

impl fmt::Display for VexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node#{}", self.0)
    }
}

/// Hands out sequential [`VexId`]s and recycles freed ones.
pub struct IdAllocator {
    next: u32,
    free_list: Vec<u32>,
}

impl IdAllocator {
    pub fn new() -> Self {
        Self {
            next: 0,
            free_list: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> VexId {
        if let Some(id) = self.free_list.pop() {
            VexId(id)
        } else {
            let id = self.next;
            self.next += 1;
            VexId(id)
        }
    }

    pub fn free(&mut self, id: VexId) {
        self.free_list.push(id.0);
    }
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential() {
        let mut alloc = IdAllocator::new();
        assert_eq!(alloc.alloc(), VexId::new(0));
        assert_eq!(alloc.alloc(), VexId::new(1));
        assert_eq!(alloc.alloc(), VexId::new(2));
    }

    #[test]
    fn recycle() {
        let mut alloc = IdAllocator::new();
        let a = alloc.alloc();
        let _b = alloc.alloc();
        alloc.free(a);
        let c = alloc.alloc();
        assert_eq!(c, a); // recycled
    }

    #[test]
    fn display() {
        assert_eq!(VexId::new(42).to_string(), "node#42");
    }
}
