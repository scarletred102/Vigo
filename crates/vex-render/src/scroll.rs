// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Scroll state — tracks viewport scroll offset and content bounds.
//!
//! Used by the rendering pipeline to translate the display list and by
//! the event system to handle `MouseScroll` events.

/// Tracks the scroll position within a scrollable container.
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Horizontal scroll offset (pixels).
    pub offset_x: f32,
    /// Vertical scroll offset (pixels).
    pub offset_y: f32,
    /// Total content width (pixels).
    pub content_width: f32,
    /// Total content height (pixels).
    pub content_height: f32,
    /// Viewport width (pixels).
    pub viewport_width: f32,
    /// Viewport height (pixels).
    pub viewport_height: f32,
}

impl ScrollState {
    /// Create a new scroll state at the origin.
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            content_width: viewport_width,
            content_height: viewport_height,
            viewport_width,
            viewport_height,
        }
    }

    /// Update the content size (typically after layout).
    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.content_width = width;
        self.content_height = height;
        // Clamp offset if content shrank.
        self.clamp();
    }

    /// Update the viewport size (typically after resize).
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.clamp();
    }

    /// Scroll by a delta, clamped to content bounds.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.offset_x += dx;
        self.offset_y += dy;
        self.clamp();
    }

    /// Scroll to an absolute position, clamped to content bounds.
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.offset_x = x;
        self.offset_y = y;
        self.clamp();
    }

    /// Whether the user can scroll further down.
    pub fn can_scroll_down(&self) -> bool {
        self.offset_y < self.max_scroll_y()
    }

    /// Whether the user can scroll further up.
    pub fn can_scroll_up(&self) -> bool {
        self.offset_y > 0.0
    }

    /// Whether the user can scroll further right.
    pub fn can_scroll_right(&self) -> bool {
        self.offset_x < self.max_scroll_x()
    }

    /// Whether the user can scroll further left.
    pub fn can_scroll_left(&self) -> bool {
        self.offset_x > 0.0
    }

    /// Maximum vertical scroll offset.
    fn max_scroll_y(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    /// Maximum horizontal scroll offset.
    fn max_scroll_x(&self) -> f32 {
        (self.content_width - self.viewport_width).max(0.0)
    }

    /// Clamp offsets to valid range.
    fn clamp(&mut self) {
        self.offset_x = self.offset_x.clamp(0.0, self.max_scroll_x());
        self.offset_y = self.offset_y.clamp(0.0, self.max_scroll_y());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_at_origin() {
        let s = ScrollState::new(800.0, 600.0);
        assert_eq!(s.offset_x, 0.0);
        assert_eq!(s.offset_y, 0.0);
        assert!(!s.can_scroll_down());
        assert!(!s.can_scroll_up());
    }

    #[test]
    fn scroll_down_clamped() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 2000.0);
        assert!(s.can_scroll_down());

        s.scroll_by(0.0, 100.0);
        assert_eq!(s.offset_y, 100.0);

        // Scroll past bottom → clamped.
        s.scroll_by(0.0, 99999.0);
        assert_eq!(s.offset_y, 1400.0); // 2000 - 600
        assert!(!s.can_scroll_down());
        assert!(s.can_scroll_up());
    }

    #[test]
    fn scroll_up_clamped() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 2000.0);

        // Can't scroll up from origin.
        s.scroll_by(0.0, -100.0);
        assert_eq!(s.offset_y, 0.0);
    }

    #[test]
    fn scroll_to() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 2000.0);

        s.scroll_to(0.0, 500.0);
        assert_eq!(s.offset_y, 500.0);

        // Beyond bounds.
        s.scroll_to(0.0, 5000.0);
        assert_eq!(s.offset_y, 1400.0);
    }

    #[test]
    fn viewport_resize_clamps() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 2000.0);
        s.scroll_to(0.0, 1400.0);

        // Make viewport taller → max scroll shrinks.
        s.set_viewport_size(800.0, 1800.0);
        assert_eq!(s.offset_y, 200.0); // 2000 - 1800
    }

    #[test]
    fn content_fits_viewport() {
        let s = ScrollState::new(800.0, 600.0);
        assert!(!s.can_scroll_down());
        assert!(!s.can_scroll_up());
        assert!(!s.can_scroll_right());
        assert!(!s.can_scroll_left());
    }

    #[test]
    fn horizontal_scroll() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(2000.0, 600.0);
        assert!(s.can_scroll_right());

        s.scroll_by(300.0, 0.0);
        assert_eq!(s.offset_x, 300.0);
        assert!(s.can_scroll_left());
        assert!(s.can_scroll_right());

        s.scroll_by(99999.0, 0.0);
        assert_eq!(s.offset_x, 1200.0); // 2000 - 800
        assert!(!s.can_scroll_right());
    }
}
