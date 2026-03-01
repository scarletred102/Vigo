// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `window` global object for the JS runtime.
//!
//! Provides `location`, `history`, `navigator`, `innerWidth`, `innerHeight`,
//! `alert()`, and `self` (alias for `window`).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsArgs, JsResult, JsValue, NativeFunction};

/// Default viewport width.
const DEFAULT_WIDTH: i32 = 1280;
/// Default viewport height.
const DEFAULT_HEIGHT: i32 = 720;

/// Register the `window` global object on the given Boa context.
pub fn register(context: &mut Context) {
    register_inner(None, context);
}

/// Register the `window` global with EME-enabled navigator.
pub fn register_with_eme(eme_state: &super::eme::EmeState, context: &mut Context) {
    register_inner(Some(eme_state), context);
}

fn register_inner(eme_state: Option<&super::eme::EmeState>, context: &mut Context) {
    let location = build_location(context);
    let history = build_history(context);
    let navigator = if let Some(eme) = eme_state {
        super::eme::build_navigator_with_eme(eme, context)
    } else {
        build_navigator(context)
    };

    // Clone navigator before it's consumed by ObjectInitializer.
    let navigator_clone = navigator.clone();

    let window = ObjectInitializer::new(context)
        .property(
            js_string!("location"),
            location,
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(js_string!("history"), history, Attribute::CONFIGURABLE)
        .property(js_string!("navigator"), navigator, Attribute::CONFIGURABLE)
        .property(
            js_string!("innerWidth"),
            JsValue::from(DEFAULT_WIDTH),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("innerHeight"),
            JsValue::from(DEFAULT_HEIGHT),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .function(
            NativeFunction::from_fn_ptr(alert_fn),
            js_string!("alert"),
            1,
        )
        .build();

    // Register `window` as a global property.
    if let Err(error) = context.register_global_property(
        js_string!("window"),
        window.clone(),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::window", "failed to register window global: {error}");
    }

    // `self` === `window` in browsers.
    if let Err(error) = context.register_global_property(
        js_string!("self"),
        window,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::window", "failed to register self global: {error}");
    }

    // Also register top-level `innerWidth` / `innerHeight` on the global
    // so `innerWidth` works without `window.` prefix.
    if let Err(error) = context.register_global_property(
        js_string!("innerWidth"),
        JsValue::from(DEFAULT_WIDTH),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::window", "failed to register innerWidth: {error}");
    }
    if let Err(error) = context.register_global_property(
        js_string!("innerHeight"),
        JsValue::from(DEFAULT_HEIGHT),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::window", "failed to register innerHeight: {error}");
    }

    // Top-level `alert()`.
    if let Err(error) = context.register_global_callable(
        js_string!("alert"),
        1,
        NativeFunction::from_fn_ptr(alert_fn),
    ) {
        tracing::error!(target: "vex_js::window", "failed to register alert: {error}");
    }

    // Top-level `navigator` so `navigator.userAgent` works without `window.` prefix.
    if let Err(error) = context.register_global_property(
        js_string!("navigator"),
        navigator_clone,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::window", "failed to register navigator: {error}");
    }
}

// ── location ──────────────────────────────────────────────────────────

fn build_location(context: &mut Context) -> JsValue {
    ObjectInitializer::new(context)
        .property(
            js_string!("href"),
            js_string!("about:blank"),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("origin"),
            js_string!("null"),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("protocol"),
            js_string!("about:"),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("pathname"),
            js_string!("blank"),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("search"),
            js_string!(""),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("hash"),
            js_string!(""),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .function(
            NativeFunction::from_fn_ptr(location_assign),
            js_string!("assign"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(location_reload),
            js_string!("reload"),
            0,
        )
        .build()
        .into()
}

/// `location.assign(url)` — stub: logs the navigation target.
fn location_assign(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url = args.get_or_undefined(0).to_string(context)?;
    tracing::info!("location.assign(\"{}\")", url.to_std_string_escaped());
    Ok(JsValue::undefined())
}

/// `location.reload()` — stub: logs the reload request.
fn location_reload(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    tracing::info!("location.reload()");
    Ok(JsValue::undefined())
}

// ── history ───────────────────────────────────────────────────────────

fn build_history(context: &mut Context) -> JsValue {
    ObjectInitializer::new(context)
        .property(
            js_string!("length"),
            JsValue::from(1),
            Attribute::CONFIGURABLE,
        )
        .function(
            NativeFunction::from_fn_ptr(history_back),
            js_string!("back"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(history_forward),
            js_string!("forward"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(history_push_state),
            js_string!("pushState"),
            3,
        )
        .build()
        .into()
}

fn history_back(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    tracing::info!("history.back()");
    Ok(JsValue::undefined())
}

fn history_forward(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    tracing::info!("history.forward()");
    Ok(JsValue::undefined())
}

fn history_push_state(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let _state = args.get_or_undefined(0);
    let _title = args.get_or_undefined(1);
    let url = args.get_or_undefined(2);
    if !url.is_undefined() && !url.is_null() {
        let url_str = url.to_string(context)?;
        tracing::info!(
            "history.pushState(url=\"{}\")",
            url_str.to_std_string_escaped()
        );
    } else {
        tracing::info!("history.pushState(url=null)");
    }
    Ok(JsValue::undefined())
}

// ── navigator ─────────────────────────────────────────────────────────

fn build_navigator(context: &mut Context) -> JsValue {
    let ua = js_string!("Vex/0.1 (Vigo Browser Engine)");
    ObjectInitializer::new(context)
        .property(js_string!("userAgent"), ua.clone(), Attribute::CONFIGURABLE)
        .property(
            js_string!("appName"),
            js_string!("Vex"),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("platform"),
            platform_string(),
            Attribute::CONFIGURABLE,
        )
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
        .build()
        .into()
}

fn platform_string() -> JsValue {
    if cfg!(target_os = "windows") {
        JsValue::from(js_string!("Win32"))
    } else if cfg!(target_os = "macos") {
        JsValue::from(js_string!("MacIntel"))
    } else {
        JsValue::from(js_string!("Linux x86_64"))
    }
}

// ── alert ─────────────────────────────────────────────────────────────

fn alert_fn(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    if args.is_empty() {
        tracing::info!("[alert]");
    } else {
        let msg = args.get_or_undefined(0).to_string(context)?;
        tracing::info!("[alert] {}", msg.to_std_string_escaped());
    }
    Ok(JsValue::undefined())
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> Context {
        let mut ctx = Context::default();
        register(&mut ctx);
        ctx
    }

    #[test]
    fn window_inner_dimensions() {
        let mut ctx = make_ctx();
        let w = ctx
            .eval(boa_engine::Source::from_bytes("window.innerWidth"))
            .unwrap();
        assert_eq!(w.as_number().unwrap() as i32, DEFAULT_WIDTH);

        let h = ctx
            .eval(boa_engine::Source::from_bytes("window.innerHeight"))
            .unwrap();
        assert_eq!(h.as_number().unwrap() as i32, DEFAULT_HEIGHT);
    }

    #[test]
    fn bare_inner_width() {
        let mut ctx = make_ctx();
        let w = ctx
            .eval(boa_engine::Source::from_bytes("innerWidth"))
            .unwrap();
        assert_eq!(w.as_number().unwrap() as i32, DEFAULT_WIDTH);
    }

    #[test]
    fn location_href_default() {
        let mut ctx = make_ctx();
        let href = ctx
            .eval(boa_engine::Source::from_bytes("window.location.href"))
            .unwrap();
        let s = href.as_string().unwrap().to_std_string_escaped();
        assert_eq!(s, "about:blank");
    }

    #[test]
    fn navigator_user_agent() {
        let mut ctx = make_ctx();
        let ua = ctx
            .eval(boa_engine::Source::from_bytes("window.navigator.userAgent"))
            .unwrap();
        let s = ua.as_string().unwrap().to_std_string_escaped();
        assert!(s.contains("Vex"));
    }

    #[test]
    fn history_length() {
        let mut ctx = make_ctx();
        let len = ctx
            .eval(boa_engine::Source::from_bytes("window.history.length"))
            .unwrap();
        assert_eq!(len.as_number().unwrap() as i32, 1);
    }

    #[test]
    fn self_equals_window() {
        let mut ctx = make_ctx();
        let result = ctx
            .eval(boa_engine::Source::from_bytes("self === window"))
            .unwrap();
        assert!(result.as_boolean().unwrap());
    }

    #[test]
    fn location_assign_no_panic() {
        let mut ctx = make_ctx();
        let result = ctx.eval(boa_engine::Source::from_bytes(
            "window.location.assign('https://example.com')",
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn history_push_state_no_panic() {
        let mut ctx = make_ctx();
        let result = ctx.eval(boa_engine::Source::from_bytes(
            "window.history.pushState({}, '', '/new-page')",
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn alert_no_panic() {
        let mut ctx = make_ctx();
        let result = ctx.eval(boa_engine::Source::from_bytes("alert('hello')"));
        assert!(result.is_ok());
    }

    #[test]
    fn typeof_window_is_object() {
        let mut ctx = make_ctx();
        let result = ctx
            .eval(boa_engine::Source::from_bytes("typeof window"))
            .unwrap();
        assert_eq!(
            result.as_string().unwrap().to_std_string_escaped(),
            "object"
        );
    }
}
