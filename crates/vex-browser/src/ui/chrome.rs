// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Chrome layout model — defines the fixed regions of the browser chrome.

use vex_core::geometry::Rect;

/// Heights (in logical pixels) for each chrome region.
pub const TAB_BAR_HEIGHT: f32 = 36.0;
pub const NAV_BAR_HEIGHT: f32 = 40.0;
pub const BOOKMARK_BAR_HEIGHT: f32 = 28.0;
pub const FIND_BAR_HEIGHT: f32 = 36.0;

/// Accent line between chrome and content.
pub const ACCENT_LINE_HEIGHT: f32 = 1.0;

/// Total fixed chrome height (tab bar + nav bar + accent line).
pub fn chrome_height(show_bookmarks: bool) -> f32 {
    let mut h = TAB_BAR_HEIGHT + NAV_BAR_HEIGHT + ACCENT_LINE_HEIGHT;
    if show_bookmarks {
        h += BOOKMARK_BAR_HEIGHT;
    }
    h
}

/// Layout regions computed for a given viewport size.
#[derive(Debug, Clone)]
pub struct ChromeLayout {
    /// Tab bar region.
    pub tab_bar: Rect,
    /// Navigation bar region.
    pub nav_bar: Rect,
    /// Bookmark bar region (zero-height if hidden).
    pub bookmark_bar: Rect,
    /// Accent line below chrome.
    pub accent_line: Rect,
    /// Content area (remainder of the viewport).
    pub content_area: Rect,
    /// Find-in-page bar (overlays top of content area when visible).
    pub find_bar: Option<Rect>,
}

impl ChromeLayout {
    /// Compute the chrome layout for the given viewport dimensions.
    pub fn compute(vp_w: f32, vp_h: f32, show_bookmarks: bool, show_find: bool) -> Self {
        let mut y = 0.0;

        let tab_bar = Rect::new(0.0, y, vp_w, TAB_BAR_HEIGHT);
        y += TAB_BAR_HEIGHT;

        let nav_bar = Rect::new(0.0, y, vp_w, NAV_BAR_HEIGHT);
        y += NAV_BAR_HEIGHT;

        let bookmark_bar = if show_bookmarks {
            let r = Rect::new(0.0, y, vp_w, BOOKMARK_BAR_HEIGHT);
            y += BOOKMARK_BAR_HEIGHT;
            r
        } else {
            Rect::new(0.0, y, vp_w, 0.0)
        };

        let accent_line = Rect::new(0.0, y, vp_w, ACCENT_LINE_HEIGHT);
        y += ACCENT_LINE_HEIGHT;

        let find_bar = if show_find {
            Some(Rect::new(0.0, y, vp_w, FIND_BAR_HEIGHT))
        } else {
            None
        };

        let content_area = Rect::new(0.0, y, vp_w, (vp_h - y).max(0.0));

        Self {
            tab_bar,
            nav_bar,
            bookmark_bar,
            accent_line,
            content_area,
            find_bar,
        }
    }

    /// Total chrome height above the content area.
    pub fn chrome_height(&self) -> f32 {
        self.content_area.origin.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_layout_without_bookmarks() {
        let layout = ChromeLayout::compute(1280.0, 720.0, false, false);
        assert_eq!(layout.tab_bar.size.height, TAB_BAR_HEIGHT);
        assert_eq!(layout.nav_bar.size.height, NAV_BAR_HEIGHT);
        assert_eq!(layout.bookmark_bar.size.height, 0.0);
        assert!(layout.find_bar.is_none());
        assert!(layout.content_area.size.height > 0.0);
    }

    #[test]
    fn chrome_layout_with_bookmarks() {
        let layout = ChromeLayout::compute(1280.0, 720.0, true, false);
        assert_eq!(layout.bookmark_bar.size.height, BOOKMARK_BAR_HEIGHT);
        let expected_h = TAB_BAR_HEIGHT + NAV_BAR_HEIGHT + BOOKMARK_BAR_HEIGHT + ACCENT_LINE_HEIGHT;
        assert!((layout.chrome_height() - expected_h).abs() < 0.1);
    }

    #[test]
    fn chrome_layout_with_find_bar() {
        let layout = ChromeLayout::compute(1280.0, 720.0, false, true);
        assert!(layout.find_bar.is_some());
        let fb = layout.find_bar.unwrap();
        assert_eq!(fb.size.height, FIND_BAR_HEIGHT);
    }

    #[test]
    fn content_area_fills_remaining() {
        let layout = ChromeLayout::compute(1280.0, 720.0, false, false);
        let expected = 720.0 - TAB_BAR_HEIGHT - NAV_BAR_HEIGHT - ACCENT_LINE_HEIGHT;
        assert!((layout.content_area.size.height - expected).abs() < 0.1);
    }
}
