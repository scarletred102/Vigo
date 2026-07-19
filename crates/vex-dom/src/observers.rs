// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! IntersectionObserver — watches when elements enter/leave the viewport.
//! ResizeObserver — watches when elements change size.
//!
//! Simplified implementations of the Intersection Observer and Resize Observer APIs.

use std::collections::HashMap;

use vex_core::geometry::Rect;
use vex_core::VexId;

/// An intersection entry for a single target element.
#[derive(Debug, Clone)]
pub struct IntersectionEntry {
    /// The observed element.
    pub target: VexId,
    /// Bounding rect of the target.
    pub bounding_client_rect: Rect,
    /// Portion of the root that is used for checking intersection.
    pub root_bounds: Option<Rect>,
    /// Intersection rectangle (overlap area).
    pub intersection_rect: Rect,
    /// Ratio of intersection area to target bounding area (0.0 - 1.0).
    pub intersection_ratio: f32,
    /// Whether the target is considered "intersecting" based on thresholds.
    pub is_intersecting: bool,
    /// Time of the observation (frame timestamp).
    pub time: f64,
}

/// Options for creating an IntersectionObserver.
#[derive(Debug, Clone)]
pub struct IntersectionObserverInit {
    /// Root element to use as the viewport. `None` means the document viewport.
    pub root: Option<VexId>,
    /// Margin around the root (CSS-like: top right bottom left).
    pub root_margin: [f32; 4],
    /// Thresholds at which to fire callbacks (default: [0.0]).
    pub thresholds: Vec<f32>,
}

impl Default for IntersectionObserverInit {
    fn default() -> Self {
        Self {
            root: None,
            root_margin: [0.0; 4],
            thresholds: vec![0.0],
        }
    }
}

/// Per-target tracking state.
#[derive(Debug, Clone)]
struct TargetState {
    /// Last known intersection ratio.
    last_ratio: f32,
    /// Last threshold index that was crossed.
    last_threshold_index: Option<usize>,
}

/// An IntersectionObserver instance.
#[derive(Debug)]
pub struct IntersectionObserver {
    options: IntersectionObserverInit,
    targets: HashMap<VexId, TargetState>,
    pending_entries: Vec<IntersectionEntry>,
}

impl IntersectionObserver {
    /// Create a new observer with the given options.
    pub fn new(options: IntersectionObserverInit) -> Self {
        let mut opts = options;
        if opts.thresholds.is_empty() {
            opts.thresholds.push(0.0);
        }
        opts.thresholds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Self {
            options: opts,
            targets: HashMap::new(),
            pending_entries: Vec::new(),
        }
    }

    /// Start observing a target element.
    pub fn observe(&mut self, target: VexId) {
        self.targets.entry(target).or_insert(TargetState {
            last_ratio: -1.0, // Force initial notification.
            last_threshold_index: None,
        });
    }

    /// Stop observing a target element.
    pub fn unobserve(&mut self, target: VexId) {
        self.targets.remove(&target);
    }

    /// Stop observing all targets.
    pub fn disconnect(&mut self) {
        self.targets.clear();
        self.pending_entries.clear();
    }

    /// Check all targets against the viewport rect.
    ///
    /// `element_rects` maps each observed VexId to its current bounding rect.
    /// `viewport` is the root bounds (screen/viewport rect).
    /// `time` is the current frame timestamp.
    pub fn check(&mut self, viewport: Rect, element_rects: &HashMap<VexId, Rect>, time: f64) {
        let root_viewport = if let Some(root_id) = self.options.root {
            element_rects.get(&root_id).copied().unwrap_or(viewport)
        } else {
            viewport
        };

        let root_bounds = apply_root_margin(root_viewport, &self.options.root_margin);

        for (target, state) in self.targets.iter_mut() {
            let target_rect = match element_rects.get(target) {
                Some(r) => *r,
                None => continue,
            };

            let intersection = rect_intersection(root_bounds, target_rect);
            let target_area = target_rect.size.width * target_rect.size.height;
            let ratio = if target_area > 0.0 {
                let inter_area = intersection.size.width * intersection.size.height;
                (inter_area / target_area).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let threshold_idx = find_threshold_index(&self.options.thresholds, ratio);

            // Only notify if threshold crossing changed.
            if threshold_idx != state.last_threshold_index || state.last_ratio < 0.0 {
                let is_intersecting = ratio > 0.0
                    || (ratio == 0.0 && self.options.thresholds.first().is_some_and(|&t| t == 0.0));

                self.pending_entries.push(IntersectionEntry {
                    target: *target,
                    bounding_client_rect: target_rect,
                    root_bounds: Some(root_bounds),
                    intersection_rect: intersection,
                    intersection_ratio: ratio,
                    is_intersecting,
                    time,
                });

                state.last_ratio = ratio;
                state.last_threshold_index = threshold_idx;
            }
        }
    }

    /// Drain all pending intersection entries.
    pub fn take_entries(&mut self) -> Vec<IntersectionEntry> {
        std::mem::take(&mut self.pending_entries)
    }

    /// Number of targets being observed.
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }

    /// The thresholds configured.
    pub fn thresholds(&self) -> &[f32] {
        &self.options.thresholds
    }
}

/// Apply root margin to the viewport rect.
fn apply_root_margin(viewport: Rect, margin: &[f32; 4]) -> Rect {
    Rect::new(
        viewport.origin.x - margin[3],
        viewport.origin.y - margin[0],
        viewport.size.width + margin[1] + margin[3],
        viewport.size.height + margin[0] + margin[2],
    )
}

/// Compute the intersection rectangle of two rects.
fn rect_intersection(a: Rect, b: Rect) -> Rect {
    let x = a.origin.x.max(b.origin.x);
    let y = a.origin.y.max(b.origin.y);
    let right = (a.origin.x + a.size.width).min(b.origin.x + b.size.width);
    let bottom = (a.origin.y + a.size.height).min(b.origin.y + b.size.height);
    let w = (right - x).max(0.0);
    let h = (bottom - y).max(0.0);
    Rect::new(x, y, w, h)
}

/// Find which threshold index the ratio falls into.
fn find_threshold_index(thresholds: &[f32], ratio: f32) -> Option<usize> {
    for (i, &threshold) in thresholds.iter().enumerate().rev() {
        if ratio >= threshold {
            return Some(i);
        }
    }
    None
}

// ── ResizeObserver ───────────────────────────────────────────────────

/// A resize entry for a single target element.
#[derive(Debug, Clone)]
pub struct ResizeEntry {
    /// The observed element.
    pub target: VexId,
    /// Content box size.
    pub content_rect: Rect,
    /// Border box width.
    pub border_box_width: f32,
    /// Border box height.
    pub border_box_height: f32,
}

/// A ResizeObserver instance.
///
/// Tracks elements and fires entries when their size changes.
#[derive(Debug, Default)]
pub struct ResizeObserver {
    targets: HashMap<VexId, (f32, f32)>, // last known (width, height)
    pending_entries: Vec<ResizeEntry>,
}

impl ResizeObserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start observing a target element.
    pub fn observe(&mut self, target: VexId) {
        self.targets.entry(target).or_insert((-1.0, -1.0)); // Force initial notification.
    }

    /// Stop observing a target element.
    pub fn unobserve(&mut self, target: VexId) {
        self.targets.remove(&target);
    }

    /// Stop observing all targets.
    pub fn disconnect(&mut self) {
        self.targets.clear();
        self.pending_entries.clear();
    }

    /// Check all targets for size changes.
    ///
    /// `element_rects` maps each observed VexId to its current bounding rect.
    pub fn check(&mut self, element_rects: &HashMap<VexId, Rect>) {
        for (target, last_size) in self.targets.iter_mut() {
            let rect = match element_rects.get(target) {
                Some(r) => *r,
                None => continue,
            };

            let w = rect.size.width;
            let h = rect.size.height;

            if (w - last_size.0).abs() > 0.01 || (h - last_size.1).abs() > 0.01 {
                self.pending_entries.push(ResizeEntry {
                    target: *target,
                    content_rect: rect,
                    border_box_width: w,
                    border_box_height: h,
                });
                *last_size = (w, h);
            }
        }
    }

    /// Drain all pending resize entries.
    pub fn take_entries(&mut self) -> Vec<ResizeEntry> {
        std::mem::take(&mut self.pending_entries)
    }

    /// Number of targets being observed.
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::new(0.0, 0.0, 1920.0, 1080.0)
    }

    fn make_rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(x, y, w, h)
    }

    // ── IntersectionObserver tests ──

    #[test]
    fn intersection_fully_visible() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit::default());
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(100.0, 100.0, 200.0, 200.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert!((entries[0].intersection_ratio - 1.0).abs() < 0.001);
        assert!(entries[0].is_intersecting);
    }

    #[test]
    fn intersection_not_visible() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit::default());
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(2000.0, 2000.0, 100.0, 100.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].intersection_ratio.abs() < 0.001);
    }

    #[test]
    fn intersection_partially_visible() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit {
            thresholds: vec![0.0, 0.5, 1.0],
            ..Default::default()
        });
        let id = VexId::new(1);
        obs.observe(id);

        // Element is 200x200, half off the right edge of viewport.
        let mut rects = HashMap::new();
        rects.insert(id, make_rect(1820.0, 100.0, 200.0, 200.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].intersection_ratio > 0.2);
        assert!(entries[0].intersection_ratio < 0.6);
        assert!(entries[0].is_intersecting);
    }

    #[test]
    fn intersection_no_duplicate_notifications() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit::default());
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(100.0, 100.0, 200.0, 200.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);

        // Same position, same frame → no new entry.
        obs.check(viewport(), &rects, 1.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn intersection_threshold_crossing() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit {
            thresholds: vec![0.0, 0.5, 1.0],
            ..Default::default()
        });
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();

        // Start fully visible.
        rects.insert(id, make_rect(100.0, 100.0, 200.0, 200.0));
        obs.check(viewport(), &rects, 0.0);
        obs.take_entries(); // Consume initial.

        // Move to partially visible (50%).
        rects.insert(id, make_rect(1820.0, 100.0, 200.0, 200.0));
        obs.check(viewport(), &rects, 1.0);
        let entries = obs.take_entries();
        // Threshold crossed from 1.0 to ~0.5 → should fire.
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn intersection_root_margin() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit {
            root_margin: [50.0, 50.0, 50.0, 50.0],
            ..Default::default()
        });
        let id = VexId::new(1);
        obs.observe(id);

        // Element just outside viewport, but within margin.
        let mut rects = HashMap::new();
        rects.insert(id, make_rect(1930.0, 100.0, 50.0, 50.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_intersecting);
    }

    #[test]
    fn intersection_uses_custom_root() {
        let root_id = VexId::new(50);
        let mut obs = IntersectionObserver::new(IntersectionObserverInit {
            root: Some(root_id),
            ..Default::default()
        });

        let target = VexId::new(1);
        obs.observe(target);

        let mut rects = HashMap::new();
        rects.insert(root_id, make_rect(100.0, 100.0, 120.0, 120.0));
        rects.insert(target, make_rect(110.0, 110.0, 50.0, 50.0));

        obs.check(viewport(), &rects, 0.0);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_intersecting);
        assert!((entries[0].intersection_ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn intersection_observe_unobserve() {
        let mut obs = IntersectionObserver::new(IntersectionObserverInit::default());
        let id1 = VexId::new(1);
        let id2 = VexId::new(2);
        obs.observe(id1);
        obs.observe(id2);
        assert_eq!(obs.target_count(), 2);

        obs.unobserve(id1);
        assert_eq!(obs.target_count(), 1);

        obs.disconnect();
        assert_eq!(obs.target_count(), 0);
    }

    // ── ResizeObserver tests ──

    #[test]
    fn resize_initial_notification() {
        let mut obs = ResizeObserver::new();
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(0.0, 0.0, 300.0, 200.0));

        obs.check(&rects);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].border_box_width, 300.0);
        assert_eq!(entries[0].border_box_height, 200.0);
    }

    #[test]
    fn resize_no_change_no_notification() {
        let mut obs = ResizeObserver::new();
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(0.0, 0.0, 300.0, 200.0));

        obs.check(&rects);
        obs.take_entries();

        // Same size → no notification.
        obs.check(&rects);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn resize_size_change_fires() {
        let mut obs = ResizeObserver::new();
        let id = VexId::new(1);
        obs.observe(id);

        let mut rects = HashMap::new();
        rects.insert(id, make_rect(0.0, 0.0, 300.0, 200.0));
        obs.check(&rects);
        obs.take_entries();

        // Width changes.
        rects.insert(id, make_rect(0.0, 0.0, 400.0, 200.0));
        obs.check(&rects);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].border_box_width, 400.0);
    }

    #[test]
    fn resize_multiple_targets() {
        let mut obs = ResizeObserver::new();
        let id1 = VexId::new(1);
        let id2 = VexId::new(2);
        obs.observe(id1);
        obs.observe(id2);

        let mut rects = HashMap::new();
        rects.insert(id1, make_rect(0.0, 0.0, 100.0, 100.0));
        rects.insert(id2, make_rect(0.0, 0.0, 200.0, 200.0));

        obs.check(&rects);
        let entries = obs.take_entries();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn resize_observe_unobserve() {
        let mut obs = ResizeObserver::new();
        obs.observe(VexId::new(1));
        obs.observe(VexId::new(2));
        assert_eq!(obs.target_count(), 2);

        obs.unobserve(VexId::new(1));
        assert_eq!(obs.target_count(), 1);

        obs.disconnect();
        assert_eq!(obs.target_count(), 0);
    }

    // ── Helper function tests ──

    #[test]
    fn rect_intersection_fully_contained() {
        let outer = make_rect(0.0, 0.0, 1000.0, 1000.0);
        let inner = make_rect(100.0, 100.0, 200.0, 200.0);
        let result = rect_intersection(outer, inner);
        assert_eq!(result.origin.x, 100.0);
        assert_eq!(result.origin.y, 100.0);
        assert_eq!(result.size.width, 200.0);
        assert_eq!(result.size.height, 200.0);
    }

    #[test]
    fn rect_intersection_no_overlap() {
        let a = make_rect(0.0, 0.0, 100.0, 100.0);
        let b = make_rect(200.0, 200.0, 100.0, 100.0);
        let result = rect_intersection(a, b);
        assert_eq!(result.size.width, 0.0);
        assert_eq!(result.size.height, 0.0);
    }

    #[test]
    fn rect_intersection_partial() {
        let a = make_rect(0.0, 0.0, 100.0, 100.0);
        let b = make_rect(50.0, 50.0, 100.0, 100.0);
        let result = rect_intersection(a, b);
        assert_eq!(result.origin.x, 50.0);
        assert_eq!(result.origin.y, 50.0);
        assert_eq!(result.size.width, 50.0);
        assert_eq!(result.size.height, 50.0);
    }

    #[test]
    fn find_threshold_index_basic() {
        let thresholds = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        assert_eq!(find_threshold_index(&thresholds, 0.0), Some(0));
        assert_eq!(find_threshold_index(&thresholds, 0.3), Some(1));
        assert_eq!(find_threshold_index(&thresholds, 0.5), Some(2));
        assert_eq!(find_threshold_index(&thresholds, 0.9), Some(3));
        assert_eq!(find_threshold_index(&thresholds, 1.0), Some(4));
    }

    #[test]
    fn apply_root_margin_basic() {
        let vp = make_rect(0.0, 0.0, 1920.0, 1080.0);
        let result = apply_root_margin(vp, &[10.0, 20.0, 30.0, 40.0]);
        assert_eq!(result.origin.x, -40.0);
        assert_eq!(result.origin.y, -10.0);
        assert_eq!(result.size.width, 1920.0 + 20.0 + 40.0);
        assert_eq!(result.size.height, 1080.0 + 10.0 + 30.0);
    }
}
