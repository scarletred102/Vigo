// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Encrypted Media Extensions (EME) API detection.
//!
//! When JavaScript calls `navigator.requestMediaKeySystemAccess(keySystem, configs)`,
//! this module intercepts the call, records which DRM system was requested, and
//! sets a flag so the browser shell can activate the WebView2 DRM fallback.
//!
//! We intentionally *reject* the EME request (return a rejected Promise) because
//! Vex does not implement CDM natively. The flag is the trigger for fallback.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsArgs, JsValue, NativeFunction};

/// Known DRM key systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySystem {
    Widevine,
    FairPlay,
    PlayReady,
    ClearKey,
    Other(String),
}

impl KeySystem {
    /// Parse a key system string (e.g., `com.widevine.alpha`).
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "com.widevine.alpha" => Self::Widevine,
            "com.apple.fps" | "com.apple.fps.1_0" | "com.apple.fps.2_0" => Self::FairPlay,
            "com.microsoft.playready" | "com.microsoft.playready.recommendation" => Self::PlayReady,
            "org.w3.clearkey" => Self::ClearKey,
            other => Self::Other(other.to_string()),
        }
    }

    /// Key system identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Widevine => "com.widevine.alpha",
            Self::FairPlay => "com.apple.fps",
            Self::PlayReady => "com.microsoft.playready",
            Self::ClearKey => "org.w3.clearkey",
            Self::Other(s) => s,
        }
    }
}

/// Shared EME detection state.
///
/// When `drm_requested` is set, the browser shell should activate
/// the WebView2 DRM fallback for the current page.
#[derive(Debug, Clone)]
pub struct EmeState {
    inner: Rc<RefCell<EmeStateInner>>,
}

#[derive(Debug)]
struct EmeStateInner {
    drm_requested: bool,
    key_systems: Vec<KeySystem>,
}

impl EmeState {
    /// Create a new EME state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(EmeStateInner {
                drm_requested: false,
                key_systems: Vec::new(),
            })),
        }
    }

    /// Whether any DRM key system was requested.
    #[must_use]
    pub fn drm_requested(&self) -> bool {
        self.inner.borrow().drm_requested
    }

    /// The key systems that were requested.
    #[must_use]
    pub fn requested_key_systems(&self) -> Vec<KeySystem> {
        self.inner.borrow().key_systems.clone()
    }

    /// Record a DRM request.
    pub fn record_request(&self, key_system: KeySystem) {
        let mut inner = self.inner.borrow_mut();
        inner.drm_requested = true;
        if !inner.key_systems.contains(&key_system) {
            inner.key_systems.push(key_system);
        }
    }

    /// Reset the state (e.g., on navigation).
    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.drm_requested = false;
        inner.key_systems.clear();
    }
}

impl Default for EmeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a `navigator` object that includes `requestMediaKeySystemAccess`.
///
/// This returns a `JsValue` suitable for use as `window.navigator`.
/// Call this from `window.rs` instead of the plain `build_navigator`.
pub fn build_navigator_with_eme(eme_state: &EmeState, context: &mut Context) -> JsValue {
    let state = eme_state.clone();

    // Build the requestMediaKeySystemAccess function.
    // SAFETY: EmeState uses Rc<RefCell<_>> which is !Send, but the JS
    // runtime is single-threaded — the closure only runs on the creating thread.
    let request_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let key_system_str = args
                .get_or_undefined(0)
                .to_string(ctx)?
                .to_std_string_escaped();

            let key_system = KeySystem::parse(&key_system_str);
            tracing::info!(
                "EME: navigator.requestMediaKeySystemAccess(\"{}\") — DRM detected",
                key_system.as_str()
            );
            state.record_request(key_system);

            // Return a rejected promise — we don't implement CDM natively.
            // Sites will see this rejection and may show a fallback message,
            // but the browser shell intercepts via `drm_requested` flag.
            let error_msg =
                format!("NotSupportedError: Key system '{key_system_str}' not supported by Vex");
            Err(boa_engine::JsNativeError::typ()
                .with_message(error_msg)
                .into())
        })
    };

    let ua = js_string!("Vex/0.1 (Vigo Browser Engine)");
    let platform = if cfg!(target_os = "windows") {
        JsValue::from(js_string!("Win32"))
    } else if cfg!(target_os = "macos") {
        JsValue::from(js_string!("MacIntel"))
    } else {
        JsValue::from(js_string!("Linux x86_64"))
    };

    ObjectInitializer::new(context)
        .property(js_string!("userAgent"), ua, Attribute::CONFIGURABLE)
        .property(
            js_string!("appName"),
            js_string!("Vex"),
            Attribute::CONFIGURABLE,
        )
        .property(js_string!("platform"), platform, Attribute::CONFIGURABLE)
        .property(
            js_string!("language"),
            js_string!("en-US"),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("cookieEnabled"),
            JsValue::from(true),
            Attribute::CONFIGURABLE,
        )
        .function(request_fn, js_string!("requestMediaKeySystemAccess"), 2)
        .build()
        .into()
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::window;

    fn make_ctx_with_eme() -> (Context, EmeState) {
        let mut ctx = Context::default();
        let eme = EmeState::new();
        window::register_with_eme(&eme, &mut ctx);
        (ctx, eme)
    }

    #[test]
    fn test_key_system_parse() {
        assert_eq!(KeySystem::parse("com.widevine.alpha"), KeySystem::Widevine);
        assert_eq!(KeySystem::parse("com.apple.fps"), KeySystem::FairPlay);
        assert_eq!(
            KeySystem::parse("com.microsoft.playready"),
            KeySystem::PlayReady
        );
        assert_eq!(KeySystem::parse("org.w3.clearkey"), KeySystem::ClearKey);
        assert!(matches!(
            KeySystem::parse("unknown.drm"),
            KeySystem::Other(_)
        ));
    }

    #[test]
    fn test_eme_state_initially_clean() {
        let state = EmeState::new();
        assert!(!state.drm_requested());
        assert!(state.requested_key_systems().is_empty());
    }

    #[test]
    fn test_eme_state_records_request() {
        let state = EmeState::new();
        state.record_request(KeySystem::Widevine);
        assert!(state.drm_requested());
        assert_eq!(state.requested_key_systems(), vec![KeySystem::Widevine]);
    }

    #[test]
    fn test_eme_state_dedup() {
        let state = EmeState::new();
        state.record_request(KeySystem::Widevine);
        state.record_request(KeySystem::Widevine);
        assert_eq!(state.requested_key_systems().len(), 1);
    }

    #[test]
    fn test_eme_state_reset() {
        let state = EmeState::new();
        state.record_request(KeySystem::Widevine);
        state.reset();
        assert!(!state.drm_requested());
        assert!(state.requested_key_systems().is_empty());
    }

    #[test]
    fn test_eme_intercept_sets_flag() {
        let (mut ctx, eme) = make_ctx_with_eme();
        assert!(!eme.drm_requested());

        // Calling requestMediaKeySystemAccess should throw (rejected)
        // but also set the DRM flag.
        let result = ctx.eval(boa_engine::Source::from_bytes(
            "try { \
                navigator.requestMediaKeySystemAccess('com.widevine.alpha', []); \
            } catch (e) { \
                e.message; \
            }",
        ));
        assert!(result.is_ok());
        assert!(eme.drm_requested());
        assert_eq!(eme.requested_key_systems(), vec![KeySystem::Widevine]);
    }

    #[test]
    fn test_eme_intercept_playready() {
        let (mut ctx, eme) = make_ctx_with_eme();
        let _ = ctx.eval(boa_engine::Source::from_bytes(
            "try { \
                navigator.requestMediaKeySystemAccess('com.microsoft.playready', []); \
            } catch (e) {}",
        ));
        assert!(eme.drm_requested());
        assert_eq!(eme.requested_key_systems(), vec![KeySystem::PlayReady]);
    }

    #[test]
    fn test_navigator_still_has_user_agent() {
        let (mut ctx, _eme) = make_ctx_with_eme();
        let ua = ctx
            .eval(boa_engine::Source::from_bytes("navigator.userAgent"))
            .unwrap();
        let s = ua.as_string().unwrap().to_std_string_escaped();
        assert!(s.contains("Vex"));
    }
}
