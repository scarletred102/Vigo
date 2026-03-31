// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Picture-in-Picture (PiP) support.
//!
//! When PiP is activated, the video detaches from the tab layout and
//! renders in a separate always-on-top floating window. Mini controls
//! are shown. On PiP exit, the video re-attaches to the tab.

use vex_core::geometry::Rect;

/// PiP window position and size.
#[derive(Debug, Clone, Copy)]
pub struct PipGeometry {
    /// X position on screen.
    pub x: i32,
    /// Y position on screen.
    pub y: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Default for PipGeometry {
    fn default() -> Self {
        // Default: bottom-right corner, 320×180
        Self {
            x: -1, // sentinel for "auto-position"
            y: -1,
            width: 320,
            height: 180,
        }
    }
}

/// State of the Picture-in-Picture feature for a media element.
#[derive(Debug)]
pub struct PipState {
    /// Whether PiP is currently active.
    active: bool,
    /// PiP window geometry (position + size).
    geometry: PipGeometry,
    /// Original layout rect before entering PiP (for re-attachment).
    original_rect: Option<Rect>,
    /// Whether the user is dragging the PiP window.
    dragging: bool,
    /// Drag offset from window origin.
    drag_offset_x: i32,
    drag_offset_y: i32,
    /// Minimum PiP dimensions.
    min_width: u32,
    min_height: u32,
}

impl PipState {
    /// Create a new PiP state (inactive).
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: false,
            geometry: PipGeometry::default(),
            original_rect: None,
            dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            min_width: 200,
            min_height: 112,
        }
    }

    /// Whether PiP mode is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enter PiP mode. Saves the original layout rect.
    pub fn enter(&mut self, original_rect: Rect) {
        self.active = true;
        self.original_rect = Some(original_rect);

        // Auto-position if sentinel values
        if self.geometry.x < 0 || self.geometry.y < 0 {
            // Default to bottom-right area
            self.geometry.x = 100;
            self.geometry.y = 100;
        }
    }

    /// Exit PiP mode. Returns the original layout rect for re-attachment.
    pub fn exit(&mut self) -> Option<Rect> {
        self.active = false;
        self.dragging = false;
        self.original_rect.take()
    }

    /// Current PiP window geometry.
    #[must_use]
    pub fn geometry(&self) -> PipGeometry {
        self.geometry
    }

    /// Set the PiP window position.
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.geometry.x = x;
        self.geometry.y = y;
    }

    /// Resize the PiP window (clamped to minimum).
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.geometry.width = width.max(self.min_width);
        self.geometry.height = height.max(self.min_height);
    }

    /// Resize maintaining the given aspect ratio.
    pub fn resize_with_aspect(&mut self, width: u32, aspect: f32) {
        let w = width.max(self.min_width);
        let h = ((w as f32) / aspect) as u32;
        self.geometry.width = w;
        self.geometry.height = h.max(self.min_height);
    }

    /// Auto-position the PiP window in the bottom-right of the screen.
    pub fn auto_position(&mut self, screen_width: u32, screen_height: u32) {
        let margin = 20;
        self.geometry.x = (screen_width as i32) - (self.geometry.width as i32) - margin;
        self.geometry.y = (screen_height as i32) - (self.geometry.height as i32) - margin;
    }

    /// Get the PiP rect as a floating-point `Rect`.
    #[must_use]
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.geometry.x as f32,
            self.geometry.y as f32,
            self.geometry.width as f32,
            self.geometry.height as f32,
        )
    }

    // ── Drag support ───────────────────────────────────────

    /// Begin dragging the PiP window.
    pub fn start_drag(&mut self, mouse_x: i32, mouse_y: i32) {
        self.dragging = true;
        self.drag_offset_x = mouse_x - self.geometry.x;
        self.drag_offset_y = mouse_y - self.geometry.y;
    }

    /// Update dragging position.
    pub fn update_drag(&mut self, mouse_x: i32, mouse_y: i32) {
        if self.dragging {
            self.geometry.x = mouse_x - self.drag_offset_x;
            self.geometry.y = mouse_y - self.drag_offset_y;
        }
    }

    /// End dragging.
    pub fn end_drag(&mut self) {
        self.dragging = false;
    }

    /// Whether the PiP window is being dragged.
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Hit-test a point against the PiP window.
    #[must_use]
    pub fn hit_test(&self, x: i32, y: i32) -> bool {
        if !self.active {
            return false;
        }
        let g = &self.geometry;
        x >= g.x && x < g.x + g.width as i32 && y >= g.y && y < g.y + g.height as i32
    }
}

impl Default for PipState {
    fn default() -> Self {
        Self::new()
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_initially_inactive() {
        let pip = PipState::new();
        assert!(!pip.is_active());
    }

    #[test]
    fn test_enter_exit_pip() {
        let mut pip = PipState::new();
        let orig = Rect::new(100.0, 100.0, 640.0, 360.0);
        pip.enter(orig);
        assert!(pip.is_active());

        let returned = pip.exit();
        assert!(!pip.is_active());
        assert!(returned.is_some());
    }

    #[test]
    fn test_default_geometry() {
        let g = PipGeometry::default();
        assert_eq!(g.width, 320);
        assert_eq!(g.height, 180);
    }

    #[test]
    fn test_set_position() {
        let mut pip = PipState::new();
        pip.set_position(500, 300);
        assert_eq!(pip.geometry().x, 500);
        assert_eq!(pip.geometry().y, 300);
    }

    #[test]
    fn test_set_size_clamps_to_min() {
        let mut pip = PipState::new();
        pip.set_size(50, 30);
        assert!(pip.geometry().width >= 200);
        assert!(pip.geometry().height >= 112);
    }

    #[test]
    fn test_resize_with_aspect() {
        let mut pip = PipState::new();
        pip.resize_with_aspect(400, 16.0 / 9.0);
        assert_eq!(pip.geometry().width, 400);
        // height = 400 / 1.777... ≈ 225
        assert!((pip.geometry().height as f32 - 225.0).abs() < 5.0);
    }

    #[test]
    fn test_auto_position() {
        let mut pip = PipState::new();
        pip.auto_position(1920, 1080);
        assert!(pip.geometry().x > 0);
        assert!(pip.geometry().y > 0);
        // Should be near bottom-right
        assert!(pip.geometry().x > 1500);
        assert!(pip.geometry().y > 800);
    }

    #[test]
    fn test_hit_test() {
        let mut pip = PipState::new();
        pip.set_position(100, 100);
        pip.set_size(300, 200);

        // Not active — no hit
        assert!(!pip.hit_test(150, 150));

        pip.enter(Rect::new(0.0, 0.0, 640.0, 360.0));
        assert!(pip.hit_test(150, 150));
        assert!(!pip.hit_test(50, 50));
        assert!(!pip.hit_test(500, 500));
    }

    #[test]
    fn test_drag() {
        let mut pip = PipState::new();
        pip.enter(Rect::new(0.0, 0.0, 640.0, 360.0));
        pip.set_position(100, 100);

        pip.start_drag(150, 120); // mouse at (150, 120), window at (100, 100)
        assert!(pip.is_dragging());

        pip.update_drag(250, 220); // moved +100 in each direction
        assert_eq!(pip.geometry().x, 200);
        assert_eq!(pip.geometry().y, 200);

        pip.end_drag();
        assert!(!pip.is_dragging());
    }

    #[test]
    fn test_rect() {
        let mut pip = PipState::new();
        pip.set_position(50, 60);
        pip.set_size(300, 200);
        let r = pip.rect();
        assert!((r.origin.x - 50.0).abs() < 0.1);
        assert!((r.origin.y - 60.0).abs() < 0.1);
        assert!((r.size.width - 300.0).abs() < 0.1);
        assert!((r.size.height - 200.0).abs() < 0.1);
    }
}
