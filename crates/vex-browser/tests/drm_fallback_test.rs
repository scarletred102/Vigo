// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Integration test: DRM detection → WebView2 fallback pipeline.
//!
//! Tests the full flow from EME detection in JS through to WebView2
//! overlay activation. Marked `#[ignore]` because it requires a
//! windowed environment.

use vex_browser::drm_overlay::OverlayManager;
use vex_browser::webview_fallback::{WebViewConfig, WebViewFallback, WebViewState};
use vex_core::geometry::Rect;
use vex_js::api::eme::{EmeState, KeySystem};

/// Simulates the DRM fallback pipeline without actual network/GPU.
///
/// Flow:
/// 1. JS calls `navigator.requestMediaKeySystemAccess("com.widevine.alpha")`
/// 2. Our EME intercept sets `drm_requested = true`
/// 3. Browser shell checks `eme_state.drm_requested()`
/// 4. Activates WebView2 fallback at the video element's position
/// 5. WebView2 navigates to the page URL
/// 6. On navigation away, WebView2 is destroyed
#[test]
fn test_drm_fallback_pipeline() {
    // Step 1-2: Simulate JS EME detection
    let eme_state = EmeState::new();
    assert!(!eme_state.drm_requested());

    // Simulate a requestMediaKeySystemAccess call
    eme_state.record_request(KeySystem::Widevine);
    assert!(eme_state.drm_requested());
    assert_eq!(eme_state.requested_key_systems(), vec![KeySystem::Widevine]);

    // Step 3: Browser shell detects DRM request
    let page_url = "https://www.netflix.com/watch/80100172";
    let video_rect = Rect::new(0.0, 120.0, 1280.0, 720.0);

    // Step 4: Activate overlay
    let mut overlay = OverlayManager::new();
    overlay.activate_for_drm(page_url, video_rect);

    // Platform-dependent: Windows → Active, others → Failed
    let fallback_state = overlay.fallback().state();
    if fallback_state == WebViewState::Active {
        // Step 5: Verify overlay is rendering
        assert!(overlay.is_rendering());

        // Verify positioning (chrome offset 80px)
        let screen = overlay.screen_rect().unwrap();
        assert!((screen.origin.x - 0.0).abs() < 0.1);
        // Y = 120 (layout) + 80 (chrome) = 200
        assert!((screen.origin.y - 200.0).abs() < 0.1);
        assert!((screen.size.width - 1280.0).abs() < 0.1);
        assert!((screen.size.height - 720.0).abs() < 0.1);

        // Simulate scrolling
        overlay.on_scroll(50.0);
        let scrolled = overlay.screen_rect().unwrap();
        // Y = 120 - 50 + 80 = 150
        assert!((scrolled.origin.y - 150.0).abs() < 0.1);

        // Simulate tab switch
        overlay.on_tab_switch(false);
        assert!(!overlay.is_rendering());
        overlay.on_tab_switch(true);
        assert!(overlay.is_rendering());
    }

    // Step 6: Navigation away — destroy WebView2
    overlay.deactivate();
    assert!(!overlay.is_rendering());

    // EME state would be reset on navigation
    eme_state.reset();
    assert!(!eme_state.drm_requested());
}

/// Test WebView2 availability check.
#[test]
fn test_webview2_runtime_check() {
    let available = WebViewFallback::is_runtime_available();
    // On Windows: true (stub), on other platforms: false
    if cfg!(target_os = "windows") {
        assert!(available);
    } else {
        assert!(!available);
    }
}

/// Test WebView2 configuration.
#[test]
fn test_webview2_config() {
    let config = WebViewConfig::new(
        "https://www.netflix.com/watch/12345",
        Rect::new(0.0, 0.0, 800.0, 450.0),
    );
    assert_eq!(config.url, "https://www.netflix.com/watch/12345");
    assert!(config.sync_cookies);
    assert!(config.user_agent.is_none());
}

/// Test multiple DRM key systems.
#[test]
fn test_multiple_key_systems() {
    let eme = EmeState::new();
    eme.record_request(KeySystem::Widevine);
    eme.record_request(KeySystem::PlayReady);
    assert_eq!(eme.requested_key_systems().len(), 2);
}
