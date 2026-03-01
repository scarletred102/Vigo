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

use super::element::build_element_proxy;
use crate::dom_bridge::SharedDocument;

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
}
