// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `window` global object for the JS runtime.
//!
//! Provides `location`, `history`, `navigator`, `innerWidth`, `innerHeight`,
//! `alert()`, and `self` (alias for `window`).

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsArgs, JsResult, JsValue, NativeFunction};

use crate::browser_request::{new_request_queue, BrowserRequest, RequestQueue};

/// Default viewport width.
const DEFAULT_WIDTH: i32 = 1280;
/// Default viewport height.
const DEFAULT_HEIGHT: i32 = 720;

/// Navigator configuration sourced from browser settings.
#[derive(Debug, Clone)]
pub struct NavigatorConfig {
    /// User-agent string.
    pub user_agent: String,
    /// Browser language (BCP 47 tag, e.g. "en-US").
    pub language: String,
    /// Whether cookies are enabled.
    pub cookie_enabled: bool,
}

impl Default for NavigatorConfig {
    fn default() -> Self {
        Self {
            user_agent: "Vex/0.1 (Vigo Browser Engine)".to_owned(),
            language: system_language(),
            cookie_enabled: true,
        }
    }
}

/// Register the `window` global object (no request queue — legacy path).
pub fn register(context: &mut Context) {
    let queue = new_request_queue();
    register_with_queue(&queue, context);
}

/// Register the `window` global with EME-enabled navigator.
pub fn register_with_eme(eme_state: &super::eme::EmeState, context: &mut Context) {
    let queue = new_request_queue();
    register_inner(
        Some(eme_state),
        &queue,
        &NavigatorConfig::default(),
        context,
    );
}

/// Register the `window` global with a shared browser request queue.
pub fn register_with_queue(queue: &RequestQueue, context: &mut Context) {
    register_inner(None, queue, &NavigatorConfig::default(), context);
}

/// Register the `window` global with a queue and custom navigator config.
pub fn register_with_config(
    queue: &RequestQueue,
    nav_config: &NavigatorConfig,
    context: &mut Context,
) {
    register_inner(None, queue, nav_config, context);
}

fn register_inner(
    eme_state: Option<&super::eme::EmeState>,
    queue: &RequestQueue,
    nav_config: &NavigatorConfig,
    context: &mut Context,
) {
    let location = build_location(queue, context);
    let history = build_history(queue, context);
    let navigator = if let Some(eme) = eme_state {
        super::eme::build_navigator_with_eme(eme, context)
    } else {
        build_navigator(nav_config, context)
    };

    // Clone navigator before it's consumed by ObjectInitializer.
    let navigator_clone = navigator.clone();

    let alert_queue = queue.clone();
    // SAFETY: Closure captures Rc<RefCell<>> and runs on the single JS thread.
    let alert_closure = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| alert_impl(&alert_queue, args, ctx))
    };
    let alert_closure_top = {
        let q = queue.clone();
        // SAFETY: Same single-thread guarantee.
        unsafe { NativeFunction::from_closure(move |_this, args, ctx| alert_impl(&q, args, ctx)) }
    };

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
        .function(alert_closure, js_string!("alert"), 1)
        .build();

    // Mirror selected global APIs onto window for browser-compat surface.
    let global = context.global_object();
    if let Ok(v) = global.get(js_string!("setTimeout"), context) {
        let _ = window.set(js_string!("setTimeout"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("setInterval"), context) {
        let _ = window.set(js_string!("setInterval"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("clearTimeout"), context) {
        let _ = window.set(js_string!("clearTimeout"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("clearInterval"), context) {
        let _ = window.set(js_string!("clearInterval"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("requestAnimationFrame"), context) {
        let _ = window.set(js_string!("requestAnimationFrame"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("cancelAnimationFrame"), context) {
        let _ = window.set(js_string!("cancelAnimationFrame"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("queueMicrotask"), context) {
        let _ = window.set(js_string!("queueMicrotask"), v, false, context);
    }
    if let Ok(v) = global.get(js_string!("fetch"), context) {
        let _ = window.set(js_string!("fetch"), v, false, context);
    }

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
    if let Err(error) = context.register_global_callable(js_string!("alert"), 1, alert_closure_top)
    {
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

/// Update all `location` properties on the global `window.location` object.
///
/// Call after navigation completes so JS sees the correct URL.
pub fn update_location(url_str: &str, context: &mut Context) {
    let global = context.global_object();

    // Parse URL components.
    let (protocol, hostname, port, pathname, search, hash, origin) =
        if let Ok(parsed) = url::Url::parse(url_str) {
            (
                format!("{}:", parsed.scheme()),
                parsed.host_str().unwrap_or("").to_owned(),
                parsed.port().map(|p| p.to_string()).unwrap_or_default(),
                parsed.path().to_owned(),
                parsed.query().map(|q| format!("?{q}")).unwrap_or_default(),
                parsed
                    .fragment()
                    .map(|f| format!("#{f}"))
                    .unwrap_or_default(),
                parsed.origin().ascii_serialization(),
            )
        } else {
            (
                "about:".to_owned(),
                String::new(),
                String::new(),
                url_str.to_owned(),
                String::new(),
                String::new(),
                "null".to_owned(),
            )
        };

    // Get the window.location object.
    let Ok(window_val) = global.get(js_string!("window"), context) else {
        return;
    };
    let Ok(window_obj) = window_val.to_object(context) else {
        return;
    };
    let Ok(location_val) = window_obj.get(js_string!("location"), context) else {
        return;
    };
    let Ok(location_obj) = location_val.to_object(context) else {
        return;
    };

    let _ = location_obj.set(
        js_string!("href"),
        JsValue::from(js_string!(url_str)),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("origin"),
        JsValue::from(js_string!(origin.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("protocol"),
        JsValue::from(js_string!(protocol.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("hostname"),
        JsValue::from(js_string!(hostname.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("port"),
        JsValue::from(js_string!(port.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("pathname"),
        JsValue::from(js_string!(pathname.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("search"),
        JsValue::from(js_string!(search.as_str())),
        false,
        context,
    );
    let _ = location_obj.set(
        js_string!("hash"),
        JsValue::from(js_string!(hash.as_str())),
        false,
        context,
    );

    tracing::debug!(target: "vex_js::window", url = url_str, "location updated");
}

// ── location ──────────────────────────────────────────────────────────

fn build_location(queue: &RequestQueue, context: &mut Context) -> JsValue {
    let assign_queue = queue.clone();
    // SAFETY: Closure captures Rc<RefCell<>> and runs on the single JS thread.
    let assign_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let url = args.get_or_undefined(0).to_string(ctx)?;
            let url_str = url.to_std_string_escaped();
            tracing::info!(target: "vex_js::window", url = url_str, "location.assign()");
            assign_queue
                .borrow_mut()
                .push(BrowserRequest::Navigate(url_str));
            Ok(JsValue::undefined())
        })
    };

    let reload_queue = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let reload_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            tracing::info!(target: "vex_js::window", "location.reload()");
            reload_queue.borrow_mut().push(BrowserRequest::Reload);
            Ok(JsValue::undefined())
        })
    };

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
            js_string!("hostname"),
            js_string!(""),
            Attribute::WRITABLE | Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("port"),
            js_string!(""),
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
        .function(assign_fn, js_string!("assign"), 1)
        .function(reload_fn, js_string!("reload"), 0)
        .build()
        .into()
}

// ── history ───────────────────────────────────────────────────────────

fn build_history(queue: &RequestQueue, context: &mut Context) -> JsValue {
    let back_queue = queue.clone();
    // SAFETY: Closure captures Rc<RefCell<>> and runs on the single JS thread.
    let back_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            tracing::info!(target: "vex_js::window", "history.back()");
            back_queue.borrow_mut().push(BrowserRequest::Back);
            Ok(JsValue::undefined())
        })
    };

    let fwd_queue = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let fwd_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            tracing::info!(target: "vex_js::window", "history.forward()");
            fwd_queue.borrow_mut().push(BrowserRequest::Forward);
            Ok(JsValue::undefined())
        })
    };

    let push_queue = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let push_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let url = args.get_or_undefined(2);
            let url_str = if !url.is_undefined() && !url.is_null() {
                let s = url.to_string(ctx)?;
                Some(s.to_std_string_escaped())
            } else {
                None
            };
            tracing::info!(target: "vex_js::window", url = ?url_str, "history.pushState()");
            push_queue
                .borrow_mut()
                .push(BrowserRequest::PushState { url: url_str });
            Ok(JsValue::undefined())
        })
    };

    ObjectInitializer::new(context)
        .property(
            js_string!("length"),
            JsValue::from(1),
            Attribute::CONFIGURABLE,
        )
        .function(back_fn, js_string!("back"), 0)
        .function(fwd_fn, js_string!("forward"), 0)
        .function(push_fn, js_string!("pushState"), 3)
        .build()
        .into()
}

// ── navigator ─────────────────────────────────────────────────────────

fn build_navigator(config: &NavigatorConfig, context: &mut Context) -> JsValue {
    let ua = js_string!(config.user_agent.as_str());
    let lang = js_string!(config.language.as_str());
    ObjectInitializer::new(context)
        .property(js_string!("userAgent"), ua, Attribute::CONFIGURABLE)
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
        .property(js_string!("language"), lang, Attribute::CONFIGURABLE)
        .property(
            js_string!("cookieEnabled"),
            JsValue::from(config.cookie_enabled),
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

/// Detect the system language (BCP 47 tag).
fn system_language() -> String {
    // On Windows, try to get the user's preferred language.
    #[cfg(target_os = "windows")]
    {
        // Use LOCALE_SNAME via GetUserDefaultLocaleName if available.
        std::env::var("LANG")
            .or_else(|_| std::env::var("LANGUAGE"))
            .unwrap_or_else(|_| "en-US".to_owned())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("LANG")
            .or_else(|_| std::env::var("LANGUAGE"))
            .map(|l| l.split('.').next().unwrap_or("en-US").replace('_', "-"))
            .unwrap_or_else(|_| "en-US".to_owned())
    }
}

// ── alert ─────────────────────────────────────────────────────────────

fn alert_impl(queue: &RequestQueue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let msg = if args.is_empty() {
        String::new()
    } else {
        args.get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped()
    };
    tracing::info!(target: "vex_js::window", message = msg, "alert()");
    queue.borrow_mut().push(BrowserRequest::Alert(msg));
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

    fn make_ctx_with_queue() -> (Context, RequestQueue) {
        let queue = new_request_queue();
        let mut ctx = Context::default();
        register_with_queue(&queue, &mut ctx);
        (ctx, queue)
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
    fn location_assign_pushes_navigate_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let result = ctx.eval(boa_engine::Source::from_bytes(
            "window.location.assign('https://example.com')",
        ));
        assert!(result.is_ok());
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs.len(), 1);
        assert_eq!(
            reqs[0],
            BrowserRequest::Navigate("https://example.com".into())
        );
    }

    #[test]
    fn location_reload_pushes_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let _ = ctx.eval(boa_engine::Source::from_bytes("window.location.reload()"));
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0], BrowserRequest::Reload);
    }

    #[test]
    fn history_back_pushes_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let _ = ctx.eval(boa_engine::Source::from_bytes("window.history.back()"));
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs[0], BrowserRequest::Back);
    }

    #[test]
    fn history_forward_pushes_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let _ = ctx.eval(boa_engine::Source::from_bytes("window.history.forward()"));
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs[0], BrowserRequest::Forward);
    }

    #[test]
    fn history_push_state_pushes_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let result = ctx.eval(boa_engine::Source::from_bytes(
            "window.history.pushState({}, '', '/new-page')",
        ));
        assert!(result.is_ok());
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(
            reqs[0],
            BrowserRequest::PushState {
                url: Some("/new-page".into())
            }
        );
    }

    #[test]
    fn alert_pushes_request() {
        let (mut ctx, queue) = make_ctx_with_queue();
        let result = ctx.eval(boa_engine::Source::from_bytes("alert('hello')"));
        assert!(result.is_ok());
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs[0], BrowserRequest::Alert("hello".into()));
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

    #[test]
    fn window_has_timer_and_fetch_methods() {
        let mut ctx = Context::default();
        super::super::timers::register(&mut ctx);
        super::super::fetch::register(&mut ctx);
        register(&mut ctx);

        let raf = ctx
            .eval(boa_engine::Source::from_bytes(
                "typeof window.requestAnimationFrame",
            ))
            .unwrap();
        assert_eq!(raf.as_string().unwrap().to_std_string_escaped(), "function");

        let caf = ctx
            .eval(boa_engine::Source::from_bytes(
                "typeof window.cancelAnimationFrame",
            ))
            .unwrap();
        assert_eq!(caf.as_string().unwrap().to_std_string_escaped(), "function");

        let fetch = ctx
            .eval(boa_engine::Source::from_bytes("typeof window.fetch"))
            .unwrap();
        assert_eq!(fetch.as_string().unwrap().to_std_string_escaped(), "function");

        let micro = ctx
            .eval(boa_engine::Source::from_bytes("typeof window.queueMicrotask"))
            .unwrap();
        assert_eq!(micro.as_string().unwrap().to_std_string_escaped(), "function");
    }

    #[test]
    fn update_location_sets_all_properties() {
        let mut ctx = make_ctx();
        update_location("https://example.com:8080/path?q=1#hash", &mut ctx);
        let href = ctx
            .eval(boa_engine::Source::from_bytes("window.location.href"))
            .unwrap();
        assert_eq!(
            href.as_string().unwrap().to_std_string_escaped(),
            "https://example.com:8080/path?q=1#hash"
        );
        let protocol = ctx
            .eval(boa_engine::Source::from_bytes("window.location.protocol"))
            .unwrap();
        assert_eq!(
            protocol.as_string().unwrap().to_std_string_escaped(),
            "https:"
        );
    }

    #[test]
    fn navigator_with_custom_config() {
        let queue = new_request_queue();
        let config = NavigatorConfig {
            user_agent: "TestAgent/1.0".into(),
            language: "fr-FR".into(),
            cookie_enabled: false,
        };
        let mut ctx = Context::default();
        register_with_config(&queue, &config, &mut ctx);
        let ua = ctx
            .eval(boa_engine::Source::from_bytes("navigator.userAgent"))
            .unwrap();
        assert_eq!(
            ua.as_string().unwrap().to_std_string_escaped(),
            "TestAgent/1.0"
        );
        let lang = ctx
            .eval(boa_engine::Source::from_bytes("navigator.language"))
            .unwrap();
        assert_eq!(lang.as_string().unwrap().to_std_string_escaped(), "fr-FR");
        let cookies = ctx
            .eval(boa_engine::Source::from_bytes("navigator.cookieEnabled"))
            .unwrap();
        assert!(!cookies.as_boolean().unwrap());
    }
}
