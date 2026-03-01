// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! WebView2 DRM fallback for Windows.
//!
//! When EME (Encrypted Media Extensions) is detected on a page, this
//! module manages a WebView2 controller that handles DRM-protected
//! content. The WebView2 instance is sized and positioned to match
//! the `<video>` element's layout rect, creating a seamless overlay
//! within the Vigo browser window.
//!
//! On non-Windows platforms, this module provides a no-op stub.

use vex_core::geometry::Rect;

/// State of the WebView2 DRM fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebViewState {
    /// No WebView2 active — normal Vex rendering.
    Inactive,
    /// WebView2 is initialising (loading the WebView2 runtime).
    Initializing,
    /// WebView2 is active and displaying DRM content.
    Active,
    /// WebView2 failed to initialise (runtime not installed, etc.).
    Failed,
}

/// Configuration for the WebView2 fallback.
#[derive(Debug, Clone)]
pub struct WebViewConfig {
    /// The URL to navigate the WebView2 to.
    pub url: String,
    /// The layout rect where the WebView2 should be positioned.
    pub rect: Rect,
    /// Whether to sync cookies from Vigo's jar to WebView2.
    pub sync_cookies: bool,
    /// User-Agent string to use in WebView2.
    pub user_agent: Option<String>,
}

impl WebViewConfig {
    /// Create a new config with default settings.
    #[must_use]
    pub fn new(url: &str, rect: Rect) -> Self {
        Self {
            url: url.to_string(),
            rect,
            sync_cookies: true,
            user_agent: None,
        }
    }
}

/// Cookie to synchronise from Vigo's jar to WebView2.
#[derive(Debug, Clone)]
pub struct CookieSync {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Domain.
    pub domain: String,
    /// Path.
    pub path: String,
    /// Secure flag.
    pub secure: bool,
    /// HttpOnly flag.
    pub http_only: bool,
}

/// Manages the WebView2 DRM fallback lifecycle.
///
/// On Windows, this would use the `webview2-com` crate or raw COM
/// interop to create an `ICoreWebView2Controller` parented to the
/// Vigo HWND. For the initial implementation, this provides the
/// state machine and interface — the COM integration is a separate task.
#[derive(Debug)]
pub struct WebViewFallback {
    /// Current state.
    state: WebViewState,
    /// Active configuration (set when entering fallback mode).
    config: Option<WebViewConfig>,
    /// Cookies pending sync to the WebView2 instance.
    pending_cookies: Vec<CookieSync>,
    /// Whether the tab that owns this is visible.
    tab_visible: bool,
    /// Scroll offset to apply to WebView2 positioning.
    scroll_offset_y: f32,
}

impl WebViewFallback {
    /// Create a new inactive fallback.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: WebViewState::Inactive,
            config: None,
            pending_cookies: Vec::new(),
            tab_visible: true,
            scroll_offset_y: 0.0,
        }
    }

    /// Current state.
    #[must_use]
    pub fn state(&self) -> WebViewState {
        self.state
    }

    /// Whether the WebView2 is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == WebViewState::Active
    }

    /// Activate the WebView2 fallback with the given configuration.
    ///
    /// In a full implementation, this would:
    /// 1. Check if WebView2 runtime is installed
    /// 2. Create `ICoreWebView2Environment`
    /// 3. Create `ICoreWebView2Controller` parented to HWND
    /// 4. Size the controller to the video rect
    /// 5. Navigate to the URL
    /// 6. Sync cookies
    pub fn activate(&mut self, config: WebViewConfig) {
        tracing::info!("WebView2 fallback: activating for URL '{}'", config.url);
        self.config = Some(config);

        if cfg!(target_os = "windows") {
            self.state = WebViewState::Initializing;
            // In production: spawn COM initialisation here.
            // For now, transition directly to Active (stub).
            self.state = WebViewState::Active;
        } else {
            tracing::warn!("WebView2 fallback: not available on this platform");
            self.state = WebViewState::Failed;
        }
    }

    /// Deactivate the WebView2 and return to native Vex rendering.
    pub fn deactivate(&mut self) {
        if self.state != WebViewState::Inactive {
            tracing::info!("WebView2 fallback: deactivating");
        }
        self.state = WebViewState::Inactive;
        self.config = None;
        self.pending_cookies.clear();
    }

    /// Update the video element's layout rect (e.g., after resize).
    pub fn update_rect(&mut self, rect: Rect) {
        if let Some(config) = self.config.as_mut() {
            config.rect = rect;
        }
        // In production: resize the ICoreWebView2Controller bounds.
    }

    /// Update scroll offset (for repositioning the overlay).
    pub fn set_scroll_offset(&mut self, offset_y: f32) {
        self.scroll_offset_y = offset_y;
    }

    /// Get the effective screen rect accounting for scroll offset.
    #[must_use]
    pub fn effective_rect(&self) -> Option<Rect> {
        self.config.as_ref().map(|c| {
            Rect::new(
                c.rect.origin.x,
                c.rect.origin.y - self.scroll_offset_y,
                c.rect.size.width,
                c.rect.size.height,
            )
        })
    }

    /// Notify that the owning tab's visibility changed.
    pub fn set_tab_visible(&mut self, visible: bool) {
        self.tab_visible = visible;
        // In production: show/hide the WebView2 controller
        if visible {
            tracing::debug!("WebView2: tab became visible");
        } else {
            tracing::debug!("WebView2: tab hidden");
        }
    }

    /// Whether the WebView2 should be rendered (active + tab visible).
    #[must_use]
    pub fn should_render(&self) -> bool {
        self.state == WebViewState::Active && self.tab_visible
    }

    /// Queue a cookie for sync to the WebView2 instance.
    pub fn queue_cookie(&mut self, cookie: CookieSync) {
        self.pending_cookies.push(cookie);
    }

    /// Take pending cookies (for actual sync to WebView2 COM API).
    pub fn take_pending_cookies(&mut self) -> Vec<CookieSync> {
        std::mem::take(&mut self.pending_cookies)
    }

    /// Get the current URL being displayed.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.config.as_ref().map(|c| c.url.as_str())
    }

    /// Check if the WebView2 runtime is available on this system.
    ///
    /// On Windows, this checks for the WebView2 Evergreen Runtime.
    /// On other platforms, always returns `false`.
    #[must_use]
    pub fn is_runtime_available() -> bool {
        if cfg!(target_os = "windows") {
            // In production: check registry or call
            // GetAvailableCoreWebView2BrowserVersionString
            true // stub: assume available on Windows
        } else {
            false
        }
    }
}

impl Default for WebViewFallback {
    fn default() -> Self {
        Self::new()
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> WebViewConfig {
        WebViewConfig::new(
            "https://www.netflix.com/watch/12345",
            Rect::new(0.0, 100.0, 800.0, 450.0),
        )
    }

    #[test]
    fn test_initially_inactive() {
        let wv = WebViewFallback::new();
        assert_eq!(wv.state(), WebViewState::Inactive);
        assert!(!wv.is_active());
    }

    #[test]
    fn test_activate_deactivate() {
        let mut wv = WebViewFallback::new();
        wv.activate(test_config());
        // On Windows this is Active; on other platforms it's Failed
        assert!(wv.state() == WebViewState::Active || wv.state() == WebViewState::Failed);

        wv.deactivate();
        assert_eq!(wv.state(), WebViewState::Inactive);
    }

    #[test]
    fn test_update_rect() {
        let mut wv = WebViewFallback::new();
        wv.activate(test_config());
        wv.update_rect(Rect::new(10.0, 20.0, 640.0, 360.0));
        // Just verify no panic
    }

    #[test]
    fn test_effective_rect_with_scroll() {
        let mut wv = WebViewFallback::new();
        wv.activate(WebViewConfig::new(
            "https://example.com",
            Rect::new(0.0, 200.0, 800.0, 450.0),
        ));
        wv.set_scroll_offset(50.0);
        let r = wv.effective_rect().unwrap();
        assert!((r.origin.y - 150.0).abs() < 0.1); // 200 - 50 scroll
    }

    #[test]
    fn test_tab_visibility() {
        let mut wv = WebViewFallback::new();
        wv.activate(test_config());
        if wv.is_active() {
            assert!(wv.should_render());
            wv.set_tab_visible(false);
            assert!(!wv.should_render());
            wv.set_tab_visible(true);
            assert!(wv.should_render());
        }
    }

    #[test]
    fn test_cookie_sync() {
        let mut wv = WebViewFallback::new();
        wv.queue_cookie(CookieSync {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: ".netflix.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
        });
        let cookies = wv.take_pending_cookies();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "session");

        // After take, queue is empty
        assert!(wv.take_pending_cookies().is_empty());
    }

    #[test]
    fn test_url_access() {
        let wv = WebViewFallback::new();
        assert!(wv.url().is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = WebViewConfig::new("https://example.com", Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(config.sync_cookies);
        assert!(config.user_agent.is_none());
    }
}
