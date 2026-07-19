// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM Range API implementation.
//!
//! A Range represents a fragment of a document that can contain nodes
//! and parts of text nodes. It is used by the Selection API, programmatic
//! text manipulation, and many editing operations.

use vex_core::VexId;

// ── Types ────────────────────────────────────────────────────────────────────

/// Comparison constants for `compare_boundary_points`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeComparison {
    /// Compare start of this range with start of other.
    StartToStart,
    /// Compare start of this range with end of other.
    StartToEnd,
    /// Compare end of this range with start of other.
    EndToStart,
    /// Compare end of this range with end of other.
    EndToEnd,
}

impl RangeComparison {
    pub fn value(&self) -> u16 {
        match self {
            Self::StartToStart => 0,
            Self::StartToEnd => 1,
            Self::EndToStart => 2,
            Self::EndToEnd => 3,
        }
    }
}

/// A boundary point: a (node, offset) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryPoint {
    /// The container node.
    pub node: VexId,
    /// The offset within the node (character offset for text, child index for elements).
    pub offset: u32,
}

impl BoundaryPoint {
    pub fn new(node: VexId, offset: u32) -> Self {
        Self { node, offset }
    }
}

/// DOM Range — represents a contiguous part of the document tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// The start boundary point.
    start: BoundaryPoint,
    /// The end boundary point.
    end: BoundaryPoint,
    /// Whether the range has been collapsed.
    collapsed: bool,
}

impl Range {
    /// Create a new Range with both endpoints at (node, 0).
    pub fn new(node: VexId) -> Self {
        let bp = BoundaryPoint::new(node, 0);
        Self {
            start: bp,
            end: bp,
            collapsed: true,
        }
    }

    /// Create a range spanning from start to end.
    pub fn from_points(start: BoundaryPoint, end: BoundaryPoint) -> Self {
        let collapsed = start == end;
        Self {
            start,
            end,
            collapsed,
        }
    }

    pub fn start_container(&self) -> VexId {
        self.start.node
    }

    pub fn start_offset(&self) -> u32 {
        self.start.offset
    }

    pub fn end_container(&self) -> VexId {
        self.end.node
    }

    pub fn end_offset(&self) -> u32 {
        self.end.offset
    }

    pub fn collapsed(&self) -> bool {
        self.collapsed
    }

    /// Set the start point.
    pub fn set_start(&mut self, node: VexId, offset: u32) {
        self.start = BoundaryPoint::new(node, offset);
        self.update_collapsed();
    }

    /// Set the end point.
    pub fn set_end(&mut self, node: VexId, offset: u32) {
        self.end = BoundaryPoint::new(node, offset);
        self.update_collapsed();
    }

    /// Set the start before a node (offset = node's index in parent's children).
    pub fn set_start_before(&mut self, parent: VexId, child_index: u32) {
        self.start = BoundaryPoint::new(parent, child_index);
        self.update_collapsed();
    }

    /// Set the start after a node.
    pub fn set_start_after(&mut self, parent: VexId, child_index: u32) {
        self.start = BoundaryPoint::new(parent, child_index + 1);
        self.update_collapsed();
    }

    /// Set the end before a node.
    pub fn set_end_before(&mut self, parent: VexId, child_index: u32) {
        self.end = BoundaryPoint::new(parent, child_index);
        self.update_collapsed();
    }

    /// Set the end after a node.
    pub fn set_end_after(&mut self, parent: VexId, child_index: u32) {
        self.end = BoundaryPoint::new(parent, child_index + 1);
        self.update_collapsed();
    }

    /// Collapse the range to one of its boundary points.
    pub fn collapse(&mut self, to_start: bool) {
        if to_start {
            self.end = self.start;
        } else {
            self.start = self.end;
        }
        self.collapsed = true;
    }

    /// Select a node — sets the range to encompass the entire node within its parent.
    pub fn select_node(&mut self, parent: VexId, child_index: u32) {
        self.start = BoundaryPoint::new(parent, child_index);
        self.end = BoundaryPoint::new(parent, child_index + 1);
        self.update_collapsed();
    }

    /// Select the contents of a node — sets the range from (node, 0) to (node, child_count).
    pub fn select_node_contents(&mut self, node: VexId, child_count: u32) {
        self.start = BoundaryPoint::new(node, 0);
        self.end = BoundaryPoint::new(node, child_count);
        self.update_collapsed();
    }

    /// Compare boundary points of two ranges.
    /// Returns -1, 0, or 1.
    pub fn compare_boundary_points(&self, how: RangeComparison, other: &Range) -> i8 {
        let (this_bp, other_bp) = match how {
            RangeComparison::StartToStart => (&self.start, &other.start),
            RangeComparison::StartToEnd => (&self.start, &other.end),
            RangeComparison::EndToStart => (&self.end, &other.start),
            RangeComparison::EndToEnd => (&self.end, &other.end),
        };
        Self::compare_points(this_bp, other_bp)
    }

    /// Compare two boundary points.
    fn compare_points(a: &BoundaryPoint, b: &BoundaryPoint) -> i8 {
        // Compare by node ID first, then by offset
        let a_idx = a.node.index();
        let b_idx = b.node.index();
        match a_idx.cmp(&b_idx) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => match a.offset.cmp(&b.offset) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            },
        }
    }

    /// Check if a point is inside this range.
    pub fn is_point_in_range(&self, node: VexId, offset: u32) -> bool {
        let point = BoundaryPoint::new(node, offset);
        Self::compare_points(&self.start, &point) <= 0
            && Self::compare_points(&point, &self.end) <= 0
    }

    /// Clone this range.
    pub fn clone_range(&self) -> Range {
        self.clone()
    }

    fn update_collapsed(&mut self) {
        self.collapsed = self.start == self.end;
    }
}

/// A StaticRange is an immutable snapshot of a Range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticRange {
    pub start_container: VexId,
    pub start_offset: u32,
    pub end_container: VexId,
    pub end_offset: u32,
}

impl StaticRange {
    pub fn new(
        start_container: VexId,
        start_offset: u32,
        end_container: VexId,
        end_offset: u32,
    ) -> Self {
        Self {
            start_container,
            start_offset,
            end_container,
            end_offset,
        }
    }

    pub fn collapsed(&self) -> bool {
        self.start_container == self.end_container && self.start_offset == self.end_offset
    }
}

impl From<&Range> for StaticRange {
    fn from(range: &Range) -> Self {
        Self {
            start_container: range.start_container(),
            start_offset: range.start_offset(),
            end_container: range.end_container(),
            end_offset: range.end_offset(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_new() {
        let node = VexId::new(1);
        let range = Range::new(node);
        assert_eq!(range.start_container(), node);
        assert_eq!(range.start_offset(), 0);
        assert!(range.collapsed());
    }

    #[test]
    fn range_set_start_end() {
        let node_a = VexId::new(1);
        let node_b = VexId::new(2);
        let mut range = Range::new(node_a);
        range.set_start(node_a, 2);
        range.set_end(node_b, 5);
        assert_eq!(range.start_container(), node_a);
        assert_eq!(range.start_offset(), 2);
        assert_eq!(range.end_container(), node_b);
        assert_eq!(range.end_offset(), 5);
        assert!(!range.collapsed());
    }

    #[test]
    fn range_collapse_to_start() {
        let mut range = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 2),
            BoundaryPoint::new(VexId::new(2), 5),
        );
        range.collapse(true);
        assert!(range.collapsed());
        assert_eq!(range.start_container(), VexId::new(1));
        assert_eq!(range.end_container(), VexId::new(1));
        assert_eq!(range.end_offset(), 2);
    }

    #[test]
    fn range_collapse_to_end() {
        let mut range = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 2),
            BoundaryPoint::new(VexId::new(2), 5),
        );
        range.collapse(false);
        assert!(range.collapsed());
        assert_eq!(range.start_container(), VexId::new(2));
        assert_eq!(range.start_offset(), 5);
    }

    #[test]
    fn range_select_node() {
        let mut range = Range::new(VexId::new(1));
        range.select_node(VexId::new(5), 2);
        assert_eq!(range.start_container(), VexId::new(5));
        assert_eq!(range.start_offset(), 2);
        assert_eq!(range.end_offset(), 3);
    }

    #[test]
    fn range_select_node_contents() {
        let mut range = Range::new(VexId::new(1));
        range.select_node_contents(VexId::new(3), 4);
        assert_eq!(range.start_container(), VexId::new(3));
        assert_eq!(range.start_offset(), 0);
        assert_eq!(range.end_container(), VexId::new(3));
        assert_eq!(range.end_offset(), 4);
    }

    #[test]
    fn range_compare_boundary_points() {
        let range_a = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 0),
            BoundaryPoint::new(VexId::new(1), 5),
        );
        let range_b = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 3),
            BoundaryPoint::new(VexId::new(1), 8),
        );
        // range_a start < range_b start
        assert_eq!(
            range_a.compare_boundary_points(RangeComparison::StartToStart, &range_b),
            -1
        );
        // range_a end < range_b end
        assert_eq!(
            range_a.compare_boundary_points(RangeComparison::EndToEnd, &range_b),
            -1
        );
    }

    #[test]
    fn range_is_point_in_range() {
        let range = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 2),
            BoundaryPoint::new(VexId::new(1), 8),
        );
        assert!(range.is_point_in_range(VexId::new(1), 5));
        assert!(range.is_point_in_range(VexId::new(1), 2)); // start inclusive
        assert!(range.is_point_in_range(VexId::new(1), 8)); // end inclusive
        assert!(!range.is_point_in_range(VexId::new(1), 1));
        assert!(!range.is_point_in_range(VexId::new(1), 9));
    }

    #[test]
    fn static_range_from_range() {
        let range = Range::from_points(
            BoundaryPoint::new(VexId::new(1), 2),
            BoundaryPoint::new(VexId::new(3), 5),
        );
        let sr = StaticRange::from(&range);
        assert_eq!(sr.start_container, VexId::new(1));
        assert_eq!(sr.start_offset, 2);
        assert_eq!(sr.end_container, VexId::new(3));
        assert_eq!(sr.end_offset, 5);
        assert!(!sr.collapsed());
    }

    #[test]
    fn static_range_collapsed() {
        let sr = StaticRange::new(VexId::new(1), 3, VexId::new(1), 3);
        assert!(sr.collapsed());
    }

    #[test]
    fn range_comparison_values() {
        assert_eq!(RangeComparison::StartToStart.value(), 0);
        assert_eq!(RangeComparison::StartToEnd.value(), 1);
        assert_eq!(RangeComparison::EndToStart.value(), 2);
        assert_eq!(RangeComparison::EndToEnd.value(), 3);
    }
}
