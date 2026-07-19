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

    /// Target offset for smooth scrolling.
    pub target_x: f32,
    pub target_y: f32,
    /// Current kinetic velocity (pixels / second).
    pub velocity_x: f32,
    pub velocity_y: f32,
    /// Whether smooth scrolling animation is enabled.
    pub smooth_enabled: bool,
    /// Friction coefficient used by fling / kinetic decay.
    pub friction: f32,
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
            target_x: 0.0,
            target_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            smooth_enabled: true,
            friction: 8.0,
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
        self.target_x = self.offset_x;
        self.target_y = self.offset_y;
    }

    /// Scroll to an absolute position, clamped to content bounds.
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.offset_x = x;
        self.offset_y = y;
        self.clamp();
        self.target_x = self.offset_x;
        self.target_y = self.offset_y;
    }

    /// Enable or disable smooth scrolling animation.
    pub fn set_smooth_enabled(&mut self, enabled: bool) {
        self.smooth_enabled = enabled;
        if !enabled {
            self.target_x = self.offset_x;
            self.target_y = self.offset_y;
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
        }
    }

    /// Smooth-scroll by a delta (accumulates into target offset).
    pub fn scroll_by_smooth(&mut self, dx: f32, dy: f32) {
        if !self.smooth_enabled {
            self.scroll_by(dx, dy);
            return;
        }
        self.target_x = (self.target_x + dx).clamp(0.0, self.max_scroll_x());
        self.target_y = (self.target_y + dy).clamp(0.0, self.max_scroll_y());
    }

    /// Start kinetic scrolling with an initial velocity.
    pub fn fling(&mut self, vx: f32, vy: f32) {
        self.velocity_x = vx;
        self.velocity_y = vy;
    }

    /// Advance smooth/kinetic scroll animation by `dt_seconds`.
    pub fn tick(&mut self, dt_seconds: f32) {
        if dt_seconds <= 0.0 {
            return;
        }

        // Kinetic contribution.
        if self.velocity_x.abs() > 0.1 || self.velocity_y.abs() > 0.1 {
            self.target_x =
                (self.target_x + self.velocity_x * dt_seconds).clamp(0.0, self.max_scroll_x());
            self.target_y =
                (self.target_y + self.velocity_y * dt_seconds).clamp(0.0, self.max_scroll_y());

            let decay = (-self.friction * dt_seconds).exp();
            self.velocity_x *= decay;
            self.velocity_y *= decay;
        }

        if !self.smooth_enabled {
            self.offset_x = self.target_x;
            self.offset_y = self.target_y;
            self.clamp();
            return;
        }

        // Critically-damped approach to target.
        let lerp = (1.0 - (-20.0 * dt_seconds).exp()).clamp(0.0, 1.0);
        self.offset_x += (self.target_x - self.offset_x) * lerp;
        self.offset_y += (self.target_y - self.offset_y) * lerp;
        self.clamp();
    }

    /// Whether smooth/kinetic animation is still active.
    pub fn is_animating(&self) -> bool {
        (self.offset_x - self.target_x).abs() > 0.1
            || (self.offset_y - self.target_y).abs() > 0.1
            || self.velocity_x.abs() > 0.1
            || self.velocity_y.abs() > 0.1
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
        self.target_x = self.target_x.clamp(0.0, self.max_scroll_x());
        self.target_y = self.target_y.clamp(0.0, self.max_scroll_y());
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

    #[test]
    fn smooth_scroll_moves_towards_target() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 2000.0);
        s.scroll_by_smooth(0.0, 500.0);
        assert_eq!(s.target_y, 500.0);

        s.tick(0.016);
        assert!(s.offset_y > 0.0);
        assert!(s.offset_y < 500.0);
    }

    #[test]
    fn fling_adds_momentum() {
        let mut s = ScrollState::new(800.0, 600.0);
        s.set_content_size(800.0, 3000.0);
        s.fling(0.0, 1500.0);
        s.tick(0.1);

        assert!(s.target_y > 0.0);
        assert!(s.velocity_y < 1500.0);
    }
}
