// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `document` global object for the JS runtime.
//!
//! Provides `getElementById`, `querySelector`, `querySelectorAll`,
//! `createElement`, `createTextNode`, and `body` / `documentElement`.
//! Each element-returning method gives back a JS proxy object wrapping a `VexId`.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction};
use vex_dom::Namespace;

use super::element::{
    build_element_proxy, build_element_proxy_with_events_rooted,
};
use super::events::EventBridge;
use crate::dom_bridge::SharedDocument;
use crate::GcRootSet;

/// Register the `document` global on the given Boa context.
///
/// The `doc` is the shared DOM document that JS methods operate on.
pub fn register(doc: &SharedDocument, context: &mut Context) {
    let doc_clone = doc.clone();
    let doc_get_el = doc.clone();
    let doc_qs = doc.clone();
    let doc_qsa = doc.clone();
    let doc_ce = doc.clone();
    let doc_ctn = doc.clone();
    let doc_body = doc.clone();

    // --- getElementById ---
    // SAFETY: SharedDocument (Rc<RefCell<Document>>) is !Send but the JS
    // runtime is single-threaded, so this closure will only be called on the
    // same thread that created it.
    let get_element_by_id = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let id_str = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("getElementById requires an argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_get_el.borrow();
            match doc_ref.get_element_by_id(&id_str) {
                Some(vex_id) => {
                    drop(doc_ref);
                    Ok(build_element_proxy(vex_id, &doc_get_el, ctx))
                }
                None => Ok(JsValue::null()),
            }
        })
    };

    // --- querySelector ---
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let query_selector = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let sel = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("querySelector requires a selector")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_qs.borrow();
            match doc_ref.query_selector(&sel) {
                Ok(Some(vex_id)) => {
                    drop(doc_ref);
                    Ok(build_element_proxy(vex_id, &doc_qs, ctx))
                }
                Ok(None) => Ok(JsValue::null()),
                Err(e) => Err(JsNativeError::syntax().with_message(e).into()),
            }
        })
    };

    // --- querySelectorAll ---
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let query_selector_all = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let sel = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("querySelectorAll requires a selector")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_qsa.borrow();
            match doc_ref.query_selector_all(&sel) {
                Ok(ids) => {
                    drop(doc_ref);
                    let arr = boa_engine::object::builtins::JsArray::new(ctx);
                    for vex_id in ids {
                        let proxy = build_element_proxy(vex_id, &doc_qsa, ctx);
                        arr.push(proxy, ctx)?;
                    }
                    Ok(JsValue::from(arr))
                }
                Err(e) => Err(JsNativeError::syntax().with_message(e).into()),
            }
        })
    };

    // --- createElement ---
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let create_element = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let tag = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("createElement requires a tag name")
                })?
                .to_string(ctx)?
                .to_std_string_escaped()
                .to_ascii_lowercase();

            let vex_id = doc_ce.borrow_mut().create_element(&tag, Namespace::Html);
            Ok(build_element_proxy(vex_id, &doc_ce, ctx))
        })
    };

    // --- createTextNode ---
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let create_text_node = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let text = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("createTextNode requires text"))?
                .to_string(ctx)?
                .to_std_string_escaped();

            let vex_id = doc_ctn.borrow_mut().create_text(&text);
            Ok(build_element_proxy(vex_id, &doc_ctn, ctx))
        })
    };

    // --- body (getter) ---
    let body_val = {
        let doc_ref = doc_body.borrow();
        let body_id = doc_ref.get_elements_by_tag_name("body").into_iter().next();
        match body_id {
            Some(id) => {
                drop(doc_ref);
                build_element_proxy(id, &doc_body, context)
            }
            None => JsValue::null(),
        }
    };

    // --- documentElement (getter) ---
    let doc_el_val = {
        let doc_ref = doc_clone.borrow();
        match doc_ref.root_element() {
            Some(id) => {
                drop(doc_ref);
                build_element_proxy(id, &doc_clone, context)
            }
            None => JsValue::null(),
        }
    };

    let document = ObjectInitializer::new(context)
        .property(js_string!("body"), body_val, Attribute::CONFIGURABLE)
        .property(
            js_string!("documentElement"),
            doc_el_val,
            Attribute::CONFIGURABLE,
        )
        .function(get_element_by_id, js_string!("getElementById"), 1)
        .function(query_selector, js_string!("querySelector"), 1)
        .function(query_selector_all, js_string!("querySelectorAll"), 1)
        .function(create_element, js_string!("createElement"), 1)
        .function(create_text_node, js_string!("createTextNode"), 1)
        .build();

    if let Err(error) = context.register_global_property(
        js_string!("document"),
        document,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::document", "failed to register document global: {error}");
    }
}

/// Register the `document` global with event listener support.
///
/// Like [`register`] but element-returning methods return proxies with
/// `addEventListener` / `removeEventListener`, and the document object
/// itself also gets those methods.
pub fn register_with_events(
    doc: &SharedDocument,
    bridge: &EventBridge,
    roots: &GcRootSet,
    context: &mut Context,
) {
    let doc_clone = doc.clone();
    let doc_get_el = doc.clone();
    let doc_qs = doc.clone();
    let doc_qsa = doc.clone();
    let doc_ce = doc.clone();
    let doc_ctn = doc.clone();
    let doc_body = doc.clone();

    let br_get_el = bridge.clone();
    let br_qs = bridge.clone();
    let br_qsa = bridge.clone();
    let br_ce = bridge.clone();
    let br_ctn = bridge.clone();

    let roots_get_el = roots.clone();
    let roots_qs = roots.clone();
    let roots_qsa = roots.clone();
    let roots_ce = roots.clone();
    let roots_ctn = roots.clone();

    // --- getElementById ---
    // SAFETY: Rc handles; single-threaded JS.
    let get_element_by_id = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let id_str = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("getElementById requires an argument")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_get_el.borrow();
            match doc_ref.get_element_by_id(&id_str) {
                Some(vex_id) => {
                    drop(doc_ref);
                    Ok(build_element_proxy_with_events_rooted(
                        vex_id,
                        &doc_get_el,
                        &br_get_el,
                        &roots_get_el,
                        ctx,
                    ))
                }
                None => Ok(JsValue::null()),
            }
        })
    };

    // --- querySelector ---
    // SAFETY: Rc handles; single-threaded JS.
    let query_selector = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let sel = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("querySelector requires a selector")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_qs.borrow();
            match doc_ref.query_selector(&sel) {
                Ok(Some(vex_id)) => {
                    drop(doc_ref);
                    Ok(build_element_proxy_with_events_rooted(
                        vex_id,
                        &doc_qs,
                        &br_qs,
                        &roots_qs,
                        ctx,
                    ))
                }
                Ok(None) => Ok(JsValue::null()),
                Err(e) => Err(JsNativeError::syntax().with_message(e).into()),
            }
        })
    };

    // --- querySelectorAll ---
    // SAFETY: Rc handles; single-threaded JS.
    let query_selector_all = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let sel = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("querySelectorAll requires a selector")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_qsa.borrow();
            match doc_ref.query_selector_all(&sel) {
                Ok(ids) => {
                    drop(doc_ref);
                    let arr = boa_engine::object::builtins::JsArray::new(ctx);
                    for vex_id in ids {
                        let proxy = build_element_proxy_with_events_rooted(
                            vex_id,
                            &doc_qsa,
                            &br_qsa,
                            &roots_qsa,
                            ctx,
                        );
                        arr.push(proxy, ctx)?;
                    }
                    Ok(JsValue::from(arr))
                }
                Err(e) => Err(JsNativeError::syntax().with_message(e).into()),
            }
        })
    };

    // --- createElement ---
    // SAFETY: Rc handles; single-threaded JS.
    let create_element = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let tag = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("createElement requires a tag name")
                })?
                .to_string(ctx)?
                .to_std_string_escaped()
                .to_ascii_lowercase();

            let vex_id = doc_ce.borrow_mut().create_element(&tag, Namespace::Html);
            Ok(build_element_proxy_with_events_rooted(
                vex_id,
                &doc_ce,
                &br_ce,
                &roots_ce,
                ctx,
            ))
        })
    };

    // --- createTextNode ---
    // SAFETY: Rc handles; single-threaded JS.
    let create_text_node = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let text = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("createTextNode requires text"))?
                .to_string(ctx)?
                .to_std_string_escaped();

            let vex_id = doc_ctn.borrow_mut().create_text(&text);
            Ok(build_element_proxy_with_events_rooted(
                vex_id,
                &doc_ctn,
                &br_ctn,
                &roots_ctn,
                ctx,
            ))
        })
    };

    // --- body (getter) ---
    let body_val = {
        let doc_ref = doc_body.borrow();
        let body_id = doc_ref.get_elements_by_tag_name("body").into_iter().next();
        match body_id {
            Some(id) => {
                drop(doc_ref);
                build_element_proxy_with_events_rooted(id, &doc_body, bridge, roots, context)
            }
            None => JsValue::null(),
        }
    };

    // --- documentElement (getter) ---
    let doc_el_val = {
        let doc_ref = doc_clone.borrow();
        match doc_ref.root_element() {
            Some(id) => {
                drop(doc_ref);
                build_element_proxy_with_events_rooted(id, &doc_clone, bridge, roots, context)
            }
            None => JsValue::null(),
        }
    };

    // --- document.addEventListener / removeEventListener ---
    let bridge_add = bridge.clone();
    let doc_add = doc.clone();
    // SAFETY: Rc handles; single-threaded JS.
    let doc_add_event_listener = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            // Document events target the root node.
            let root = doc_add.borrow().root();

            let type_str = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("addEventListener requires event type")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let cb_val = args.get(1).ok_or_else(|| {
                JsNativeError::typ().with_message("addEventListener requires a callback")
            })?;
            let cb_obj = cb_val
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;
            let func = boa_engine::object::builtins::JsFunction::from_object(cb_obj.clone())
                .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;

            let capture = args.get(2).map(|v| v.to_boolean()).unwrap_or(false);
            let event_type = super::events::parse_event_type(&type_str);
            super::events::register_listener_bridge(&bridge_add, root, event_type, func, capture);

            Ok(JsValue::undefined())
        })
    };

    let bridge_remove = bridge.clone();
    let doc_remove = doc.clone();
    // SAFETY: Rc handles; single-threaded JS.
    let doc_remove_event_listener = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let root = doc_remove.borrow().root();

            let type_str = args
                .first()
                .ok_or_else(|| {
                    JsNativeError::typ().with_message("removeEventListener requires event type")
                })?
                .to_string(ctx)?
                .to_std_string_escaped();

            let cb_val = args.get(1).ok_or_else(|| {
                JsNativeError::typ().with_message("removeEventListener requires a callback")
            })?;
            let cb_obj = cb_val
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;

            let capture = args.get(2).map(|v| v.to_boolean()).unwrap_or(false);
            super::events::unregister_listener_bridge(
                &bridge_remove,
                root,
                &type_str,
                cb_obj,
                capture,
            );

            Ok(JsValue::undefined())
        })
    };

    let document = ObjectInitializer::new(context)
        .property(js_string!("body"), body_val, Attribute::CONFIGURABLE)
        .property(
            js_string!("documentElement"),
            doc_el_val,
            Attribute::CONFIGURABLE,
        )
        .function(get_element_by_id, js_string!("getElementById"), 1)
        .function(query_selector, js_string!("querySelector"), 1)
        .function(query_selector_all, js_string!("querySelectorAll"), 1)
        .function(create_element, js_string!("createElement"), 1)
        .function(create_text_node, js_string!("createTextNode"), 1)
        .function(doc_add_event_listener, js_string!("addEventListener"), 2)
        .function(
            doc_remove_event_listener,
            js_string!("removeEventListener"),
            2,
        )
        .build();

    if let Err(error) = context.register_global_property(
        js_string!("document"),
        document,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::error!(target: "vex_js::document", "failed to register document global: {error}");
    }
}

// ── document.cookie (getter/setter) ───────────────────────────────────

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::property::PropertyDescriptor;
use vex_storage::{CookieStore, PersistentCookie};

/// Shared handle to the persistent cookie store.
pub type SharedCookieStore = Rc<RefCell<CookieStore>>;

/// Create a shared in-memory cookie store for testing.
pub fn shared_cookie_store_in_memory() -> SharedCookieStore {
    Rc::new(RefCell::new(
        CookieStore::open_in_memory().expect("in-memory cookie store"),
    ))
}

/// Attach a `cookie` getter/setter property to the existing `document` global.
///
/// - **Getter**: reads non-HttpOnly cookies for `domain`/`path`,
///   returns `"name=value; name2=value2"`.
/// - **Setter**: parses `"name=value"` and stores via `CookieStore::save`.
pub fn register_document_cookie(
    store: &SharedCookieStore,
    domain: &str,
    path: &str,
    context: &mut Context,
) {
    let global = context.global_object();
    let Ok(doc_val) = global.get(js_string!("document"), context) else {
        tracing::warn!(target: "vex_js::document", "cannot attach cookie: document not found");
        return;
    };
    let Ok(doc_obj) = doc_val.to_object(context) else {
        return;
    };

    let getter_store = store.clone();
    let getter_domain = domain.to_owned();
    let getter_path = path.to_owned();
    // SAFETY: Closure captures Rc<RefCell<>> and Strings; single JS thread.
    let getter = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            let store_ref = getter_store.borrow();
            let cookies = store_ref
                .load(&getter_domain, &getter_path)
                .unwrap_or_default();
            // Filter out HttpOnly cookies — JS must not see them.
            let pairs: Vec<String> = cookies
                .iter()
                .filter(|c| !c.http_only)
                .map(|c| format!("{}={}", c.name, c.value))
                .collect();
            Ok(JsValue::from(js_string!(pairs.join("; "))))
        })
    };

    let setter_store = store.clone();
    let setter_domain = domain.to_owned();
    let setter_path = path.to_owned();
    // SAFETY: Same single-thread guarantee.
    let setter = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let raw = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            if let Some(cookie) = parse_set_cookie(&raw, &setter_domain, &setter_path) {
                if let Err(e) = setter_store.borrow().save(&cookie) {
                    tracing::warn!(target: "vex_js::document", error = %e, "cookie save failed");
                }
            }
            Ok(JsValue::undefined())
        })
    };

    let get_fn = getter.to_js_function(context.realm());
    let set_fn = setter.to_js_function(context.realm());

    let _ = doc_obj.define_property_or_throw(
        js_string!("cookie"),
        PropertyDescriptor::builder()
            .get(get_fn)
            .set(set_fn)
            .enumerable(true)
            .configurable(true)
            .build(),
        context,
    );
}

/// Parse a simple `document.cookie = "name=value; path=/; secure"` string.
fn parse_set_cookie(
    raw: &str,
    default_domain: &str,
    default_path: &str,
) -> Option<PersistentCookie> {
    let parts: Vec<&str> = raw.split(';').collect();
    let name_value = parts.first()?;
    let (name, value) = name_value.split_once('=')?;
    let name = name.trim();
    let value = value.trim();

    if name.is_empty() {
        return None;
    }

    let mut cookie = PersistentCookie {
        domain: default_domain.to_owned(),
        path: default_path.to_owned(),
        name: name.to_owned(),
        value: value.to_owned(),
        secure: false,
        http_only: false,
        same_site: "Lax".to_owned(),
        expires: None,
    };

    for attr in parts.iter().skip(1) {
        let attr = attr.trim();
        let lower = attr.to_ascii_lowercase();
        if lower == "secure" {
            cookie.secure = true;
        } else if lower == "httponly" {
            cookie.http_only = true;
        } else if let Some(val) = lower.strip_prefix("path=") {
            cookie.path = val.trim().to_owned();
        } else if let Some(val) = lower.strip_prefix("domain=") {
            cookie.domain = val.trim().to_owned();
        } else if let Some(val) = lower.strip_prefix("samesite=") {
            cookie.same_site = val.trim().to_owned();
        }
        // max-age / expires parsing omitted for simplicity
    }

    Some(cookie)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;
    use vex_dom::Document;

    fn setup() -> (Context, SharedDocument) {
        let mut doc = Document::new();
        let root = doc.root();

        let html = doc.create_element("html", Namespace::Html);
        doc.append_child(root, html);

        let body = doc.create_element("body", Namespace::Html);
        doc.append_child(html, body);

        let div = doc.create_element("div", Namespace::Html);
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "id", "main");
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "class", "container");
        doc.append_child(body, div);

        let p = doc.create_element("p", Namespace::Html);
        vex_dom::attributes::set_attribute(doc.arena_mut(), p, "class", "text");
        doc.append_child(div, p);

        let text = doc.create_text("Hello, world!");
        doc.append_child(p, text);

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        register(&shared, &mut ctx);
        (ctx, shared)
    }

    #[test]
    fn get_element_by_id_found() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.getElementById('main')"))
            .unwrap();
        let obj = result.as_object().unwrap();
        let tag = obj.get(js_string!("tagName"), &mut ctx).unwrap();
        assert_eq!(tag.as_string().unwrap().to_std_string_escaped(), "DIV");
    }

    #[test]
    fn get_element_by_id_not_found() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.getElementById('nope')"))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn query_selector_finds_element() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.querySelector('.text')"))
            .unwrap();
        let obj = result.as_object().unwrap();
        let tag = obj.get(js_string!("tagName"), &mut ctx).unwrap();
        assert_eq!(tag.as_string().unwrap().to_std_string_escaped(), "P");
    }

    #[test]
    fn query_selector_all_returns_array() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.querySelectorAll('div, p').length",
            ))
            .unwrap();
        // div#main + p.text = 2
        assert_eq!(result.as_number().unwrap() as i32, 2);
    }

    #[test]
    fn create_element_returns_proxy() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.createElement('span').tagName"))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "SPAN");
    }

    #[test]
    fn create_text_node_returns_proxy() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.createTextNode('hi').nodeType"))
            .unwrap();
        assert_eq!(result.as_number().unwrap() as i32, 3);
    }

    #[test]
    fn body_is_available() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.body.tagName"))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "BODY");
    }

    #[test]
    fn document_element_is_html() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes("document.documentElement.tagName"))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "HTML");
    }

    #[test]
    fn proxy_has_vex_id() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('main').__vex_id !== undefined",
            ))
            .unwrap();
        assert!(result.as_boolean().unwrap());
    }

    #[test]
    fn create_element_allocates_in_arena() {
        let (mut ctx, doc) = setup();
        // Count nodes before
        let before = doc.borrow().arena().len();
        ctx.eval(Source::from_bytes("document.createElement('section')"))
            .unwrap();
        // Creating an element allocates a new node in the arena
        let after = doc.borrow().arena().len();
        assert_eq!(after, before + 1);
    }

    // ── Cookie tests ──────────────────────────────────────────────────

    #[test]
    fn parse_set_cookie_basic() {
        let c = parse_set_cookie("session_id=abc123", "example.com", "/").unwrap();
        assert_eq!(c.name, "session_id");
        assert_eq!(c.value, "abc123");
        assert_eq!(c.domain, "example.com");
        assert!(!c.secure);
        assert!(!c.http_only);
    }

    #[test]
    fn parse_set_cookie_with_attributes() {
        let c = parse_set_cookie("tok=val; Secure; HttpOnly; Path=/api", "a.com", "/").unwrap();
        assert_eq!(c.name, "tok");
        assert_eq!(c.value, "val");
        assert!(c.secure);
        assert!(c.http_only);
        assert_eq!(c.path, "/api");
    }

    #[test]
    fn document_cookie_roundtrip() {
        let (mut ctx, _doc) = setup();
        let store = shared_cookie_store_in_memory();
        register_document_cookie(&store, "example.com", "/", &mut ctx);

        // Set a cookie
        ctx.eval(Source::from_bytes("document.cookie = 'user=bob'"))
            .unwrap();

        // Read it back
        let result = ctx.eval(Source::from_bytes("document.cookie")).unwrap();
        let s = result.as_string().unwrap().to_std_string_escaped();
        assert!(s.contains("user=bob"), "expected user=bob, got: {s}");
    }

    #[test]
    fn document_cookie_hides_httponly() {
        let (mut ctx, _doc) = setup();
        let store = shared_cookie_store_in_memory();

        // Directly insert an HttpOnly cookie
        store
            .borrow()
            .save(&PersistentCookie {
                domain: "example.com".into(),
                path: "/".into(),
                name: "secret".into(),
                value: "hidden".into(),
                secure: false,
                http_only: true,
                same_site: "Lax".into(),
                expires: None,
            })
            .unwrap();

        register_document_cookie(&store, "example.com", "/", &mut ctx);

        let result = ctx.eval(Source::from_bytes("document.cookie")).unwrap();
        let s = result.as_string().unwrap().to_std_string_escaped();
        assert!(
            !s.contains("secret"),
            "HttpOnly cookies must not be visible in JS: {s}"
        );
    }
}
