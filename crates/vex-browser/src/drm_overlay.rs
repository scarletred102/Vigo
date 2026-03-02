// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Seamless WebView2 overlay positioning.
//!
//! Keeps the WebView2 fallback window exactly aligned with the
//! `<video>` element's position on screen, accounting for:
//! - Page scroll offset
//! - Tab bar / browser chrome offset
//! - Tab switching (hide/show)
//! - Window resize
//! - Navigation away (destroy WebView2)

use vex_core::geometry::Rect;

use crate::ui::chrome;
use crate::webview_fallback::{WebViewConfig, WebViewFallback};

/// Default chrome height offset (tab bar + nav bar + accent line, no bookmarks).
fn default_chrome_height() -> f32 {
    chrome::chrome_height(false)
}

/// Manages the seamless overlay of a WebView2 over the page content.
#[derive(Debug)]
pub struct OverlayManager {
    /// The WebView2 fallback instance.
    fallback: WebViewFallback,
    /// The video element's layout rect within the document.
    video_layout_rect: Option<Rect>,
    /// Current page scroll offset (Y).
    scroll_y: f32,
    /// Browser chrome offset (tab bar + address bar height).
    chrome_offset_y: f32,
    /// Whether the owning tab is the active (visible) tab.
    tab_active: bool,
}

impl OverlayManager {
    /// Create a new overlay manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fallback: WebViewFallback::new(),
            video_layout_rect: None,
            scroll_y: 0.0,
            chrome_offset_y: default_chrome_height(),
            tab_active: true,
        }
    }

    /// Access the underlying WebView2 fallback.
    #[must_use]
    pub fn fallback(&self) -> &WebViewFallback {
        &self.fallback
    }

    /// Mutable access to the fallback.
    pub fn fallback_mut(&mut self) -> &mut WebViewFallback {
        &mut self.fallback
    }

    /// Activate the WebView2 overlay for a DRM-protected video.
    pub fn activate_for_drm(&mut self, url: &str, video_rect: Rect) {
        self.video_layout_rect = Some(video_rect);
        let screen_rect = self.compute_screen_rect(video_rect);
        let config = WebViewConfig::new(url, screen_rect);
        self.fallback.activate(config);
    }

    /// Deactivate and destroy the overlay (e.g., on navigation away).
    pub fn deactivate(&mut self) {
        self.fallback.deactivate();
        self.video_layout_rect = None;
    }

    /// Update the page scroll offset.
    pub fn on_scroll(&mut self, scroll_y: f32) {
        self.scroll_y = scroll_y;
        self.reposition();
    }

    /// Notify that the video element's layout rect changed (e.g., resize).
    pub fn on_layout_change(&mut self, video_rect: Rect) {
        self.video_layout_rect = Some(video_rect);
        self.reposition();
    }

    /// Notify that the owning tab became active/inactive.
    pub fn on_tab_switch(&mut self, active: bool) {
        self.tab_active = active;
        self.fallback.set_tab_visible(active);
    }

    /// Set the browser chrome height offset.
    pub fn set_chrome_offset(&mut self, offset_y: f32) {
        self.chrome_offset_y = offset_y;
        self.reposition();
    }

    /// Whether the overlay is currently rendering.
    #[must_use]
    pub fn is_rendering(&self) -> bool {
        self.fallback.should_render()
    }

    /// Get the overlay's current screen-space rect.
    #[must_use]
    pub fn screen_rect(&self) -> Option<Rect> {
        let layout_rect = self.video_layout_rect?;
        Some(self.compute_screen_rect(layout_rect))
    }

    /// Whether the overlay is visible on screen (not scrolled out of view).
    #[must_use]
    pub fn is_on_screen(&self, viewport_height: f32) -> bool {
        if let Some(rect) = self.screen_rect() {
            let bottom = rect.origin.y + rect.size.height;
            let top = rect.origin.y;
            // Visible if any part is within the viewport
            bottom > self.chrome_offset_y && top < viewport_height
        } else {
            false
        }
    }

    // ── Internal ───────────────────────────────────────────

    /// Compute the screen-space rect from document-space layout rect.
    fn compute_screen_rect(&self, layout_rect: Rect) -> Rect {
        Rect::new(
            layout_rect.origin.x,
            layout_rect.origin.y - self.scroll_y + self.chrome_offset_y,
            layout_rect.size.width,
            layout_rect.size.height,
        )
    }

    /// Reposition the WebView2 to match current scroll/layout.
    fn reposition(&mut self) {
        if let Some(layout_rect) = self.video_layout_rect {
            let screen_rect = self.compute_screen_rect(layout_rect);
            self.fallback.update_rect(screen_rect);
            self.fallback.set_scroll_offset(self.scroll_y);
        }
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webview_fallback::WebViewState;

    #[test]
    fn test_overlay_initially_inactive() {
        let mgr = OverlayManager::new();
        assert!(!mgr.is_rendering());
        assert!(mgr.screen_rect().is_none());
    }

    #[test]
    fn test_activate_for_drm() {
        let mut mgr = OverlayManager::new();
        let video_rect = Rect::new(0.0, 200.0, 800.0, 450.0);
        mgr.activate_for_drm("https://netflix.com/watch/123", video_rect);

        // On Windows: should be rendering; on other platforms: failed
        if mgr.fallback().state() == WebViewState::Active {
            assert!(mgr.is_rendering());
            let screen = mgr.screen_rect().unwrap();
            // Y = 200 (layout) - 0 (scroll) + 77 (chrome) = 277
            let expected_y = 200.0 + default_chrome_height();
            assert!((screen.origin.y - expected_y).abs() < 0.1);
        }
    }

    #[test]
    fn test_scroll_repositions() {
        let mut mgr = OverlayManager::new();
        let video_rect = Rect::new(0.0, 500.0, 800.0, 450.0);
        mgr.activate_for_drm("https://example.com", video_rect);

        mgr.on_scroll(200.0);
        let screen = mgr.screen_rect().unwrap();
        // Y = 500 - 200 + chrome_height
        let expected_y = 500.0 - 200.0 + default_chrome_height();
        assert!((screen.origin.y - expected_y).abs() < 0.1);
    }

    #[test]
    fn test_tab_switch_hides() {
        let mut mgr = OverlayManager::new();
        mgr.activate_for_drm("https://example.com", Rect::new(0.0, 0.0, 800.0, 450.0));

        if mgr.fallback().state() == WebViewState::Active {
            assert!(mgr.is_rendering());
            mgr.on_tab_switch(false);
            assert!(!mgr.is_rendering());
            mgr.on_tab_switch(true);
            assert!(mgr.is_rendering());
        }
    }

    #[test]
    fn test_deactivate_clears() {
        let mut mgr = OverlayManager::new();
        mgr.activate_for_drm("https://example.com", Rect::new(0.0, 0.0, 800.0, 450.0));
        mgr.deactivate();
        assert!(!mgr.is_rendering());
        assert!(mgr.screen_rect().is_none());
    }

    #[test]
    fn test_is_on_screen() {
        let mut mgr = OverlayManager::new();
        let video_rect = Rect::new(0.0, 100.0, 800.0, 450.0);
        mgr.activate_for_drm("https://example.com", video_rect);

        // With default chrome offset, video is visible on screen
        assert!(mgr.is_on_screen(720.0));

        // Scroll the video way off screen
        mgr.on_scroll(2000.0);
        // Screen Y = 100 - 2000 + 80 = -1820, bottom = -1820 + 450 = -1370
        assert!(!mgr.is_on_screen(720.0));
    }

    #[test]
    fn test_layout_change() {
        let mut mgr = OverlayManager::new();
        mgr.activate_for_drm("https://example.com", Rect::new(0.0, 100.0, 800.0, 450.0));
        mgr.on_layout_change(Rect::new(10.0, 200.0, 640.0, 360.0));
        let screen = mgr.screen_rect().unwrap();
        assert!((screen.origin.x - 10.0).abs() < 0.1);
        assert!((screen.size.width - 640.0).abs() < 0.1);
    }
}
