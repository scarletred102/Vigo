// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Full element proxy for JS — wraps a `VexId` and delegates DOM operations
//! to the shared `Document` via `SharedDocument`.
//!
//! Provides:
//! - Properties: `tagName`, `id`, `className`, `nodeType`, `textContent`,
//!   `innerHTML`, `parentElement`, `children`, `__vex_id`
//! - Methods: `getAttribute(name)`, `setAttribute(name, value)`,
//!   `removeAttribute(name)`, `appendChild(child)`, `removeChild(child)`,
//!   `insertBefore(newNode, refNode)`

use boa_engine::object::builtins::JsArray;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};
use vex_core::VexId;
use vex_dom::{attributes, NodeData};

use crate::dom_bridge::SharedDocument;

/// Build a rich element proxy object for the given `VexId`.
///
/// Properties are populated as a snapshot on creation. Mutation methods
/// (`setAttribute`, `appendChild`, etc.) round-trip through the real DOM.
pub fn build_element_proxy(
    id: VexId,
    doc: &SharedDocument,
    context: &mut Context,
) -> JsValue {
    // Pre-build the style proxy before ObjectInitializer takes the &mut Context.
    let style_proxy = {
        let doc_ref = doc.borrow();
        let is_element = matches!(&doc_ref.arena().get(id).data, NodeData::Element(_));
        drop(doc_ref);
        if is_element {
            Some(super::style_proxy::build_style_proxy(id, doc, context))
        } else {
            None
        }
    };

    let mut builder = ObjectInitializer::new(context);

    // Internal id for round-tripping back to Rust.
    builder.property(
        js_string!("__vex_id"),
        JsValue::from(id.index() as i32),
        Attribute::CONFIGURABLE,
    );

    // Read-only snapshot properties from the DOM.
    {
        let doc_ref = doc.borrow();
        let arena = doc_ref.arena();
        let node = arena.get(id);

        match &node.data {
            NodeData::Element(el) => {
                builder.property(
                    js_string!("tagName"),
                    js_string!(el.tag_name.to_ascii_uppercase()),
                    Attribute::CONFIGURABLE,
                );
                builder.property(
                    js_string!("nodeType"),
                    JsValue::from(1),
                    Attribute::CONFIGURABLE,
                );

                let id_attr = el
                    .attributes
                    .iter()
                    .find(|a| a.name == "id")
                    .map(|a| JsValue::from(js_string!(a.value.clone())))
                    .unwrap_or(JsValue::from(js_string!("")));
                builder.property(
                    js_string!("id"),
                    id_attr,
                    Attribute::WRITABLE | Attribute::CONFIGURABLE,
                );

                let class_attr = el
                    .attributes
                    .iter()
                    .find(|a| a.name == "class")
                    .map(|a| JsValue::from(js_string!(a.value.clone())))
                    .unwrap_or(JsValue::from(js_string!("")));
                builder.property(
                    js_string!("className"),
                    class_attr,
                    Attribute::WRITABLE | Attribute::CONFIGURABLE,
                );
            }
            NodeData::Text(t) => {
                builder.property(
                    js_string!("nodeType"),
                    JsValue::from(3),
                    Attribute::CONFIGURABLE,
                );
                builder.property(
                    js_string!("textContent"),
                    js_string!(t.clone()),
                    Attribute::WRITABLE | Attribute::CONFIGURABLE,
                );
            }
            NodeData::Comment(_) => {
                builder.property(
                    js_string!("nodeType"),
                    JsValue::from(8),
                    Attribute::CONFIGURABLE,
                );
            }
            _ => {
                builder.property(
                    js_string!("nodeType"),
                    JsValue::from(9),
                    Attribute::CONFIGURABLE,
                );
            }
        }

        // textContent for elements
        if matches!(&node.data, NodeData::Element(_)) {
            let tc = doc_ref.text_content(id);
            builder.property(
                js_string!("textContent"),
                js_string!(tc),
                Attribute::WRITABLE | Attribute::CONFIGURABLE,
            );
        }

        // innerHTML
        if matches!(&node.data, NodeData::Element(_)) {
            let html = inner_html(arena, id);
            builder.property(
                js_string!("innerHTML"),
                js_string!(html),
                Attribute::WRITABLE | Attribute::CONFIGURABLE,
            );
        }

        // parentElement — not attached as a live proxy now; left for the event bridge phase.
    }

    // Attach the pre-built style proxy for element nodes.
    if let Some(style) = style_proxy {
        builder.property(
            js_string!("style"),
            style,
            Attribute::CONFIGURABLE,
        );
    }

    // ── Mutation methods (capture SharedDocument) ─────────────────

    // getAttribute(name) -> string | null
    let doc_ga = doc.clone();
    let get_attribute = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let node_id = extract_vex_id(this, ctx)?;
            let name = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("getAttribute requires a name"))?
                .to_string(ctx)?
                .to_std_string_escaped();

            let doc_ref = doc_ga.borrow();
            match attributes::get_attribute(doc_ref.arena(), node_id, &name) {
                Some(v) => Ok(JsValue::from(js_string!(v.to_string()))),
                None => Ok(JsValue::null()),
            }
        })
    };
    builder.function(get_attribute, js_string!("getAttribute"), 1);

    // setAttribute(name, value)
    let doc_sa = doc.clone();
    let set_attribute = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let node_id = extract_vex_id(this, ctx)?;
            let name = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("setAttribute requires a name"))?
                .to_string(ctx)?
                .to_std_string_escaped();
            let value = args
                .get(1)
                .ok_or_else(|| JsNativeError::typ().with_message("setAttribute requires a value"))?
                .to_string(ctx)?
                .to_std_string_escaped();

            attributes::set_attribute(doc_sa.borrow_mut().arena_mut(), node_id, &name, &value);
            Ok(JsValue::undefined())
        })
    };
    builder.function(set_attribute, js_string!("setAttribute"), 2);

    // removeAttribute(name)
    let doc_ra = doc.clone();
    let remove_attribute = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let node_id = extract_vex_id(this, ctx)?;
            let name = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("removeAttribute requires a name"))?
                .to_string(ctx)?
                .to_std_string_escaped();

            attributes::remove_attribute(doc_ra.borrow_mut().arena_mut(), node_id, &name);
            Ok(JsValue::undefined())
        })
    };
    builder.function(remove_attribute, js_string!("removeAttribute"), 1);

    // appendChild(child) — child proxy must have __vex_id
    let doc_ac = doc.clone();
    let append_child = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let parent_id = extract_vex_id(this, ctx)?;
            let child_val = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("appendChild requires a child node"))?;
            let child_id = extract_vex_id(child_val, ctx)?;

            doc_ac.borrow_mut().append_child(parent_id, child_id);
            Ok(child_val.clone())
        })
    };
    builder.function(append_child, js_string!("appendChild"), 1);

    // removeChild(child)
    let doc_rc = doc.clone();
    let remove_child = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let parent_id = extract_vex_id(this, ctx)?;
            let child_val = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("removeChild requires a child node"))?;
            let child_id = extract_vex_id(child_val, ctx)?;

            doc_rc.borrow_mut().remove_child(parent_id, child_id);
            Ok(child_val.clone())
        })
    };
    builder.function(remove_child, js_string!("removeChild"), 1);

    // insertBefore(newNode, refNode)
    let doc_ib = doc.clone();
    let insert_before = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let parent_id = extract_vex_id(this, ctx)?;
            let new_val = args
                .first()
                .ok_or_else(|| JsNativeError::typ().with_message("insertBefore requires newNode"))?;
            let new_id = extract_vex_id(new_val, ctx)?;

            let ref_val = args.get(1);
            if let Some(rv) = ref_val {
                if !rv.is_null() && !rv.is_undefined() {
                    let ref_id = extract_vex_id(rv, ctx)?;
                    doc_ib.borrow_mut().insert_before(parent_id, new_id, ref_id);
                    return Ok(new_val.clone());
                }
            }
            // If refNode is null/undefined, behaves like appendChild.
            doc_ib.borrow_mut().append_child(parent_id, new_id);
            Ok(new_val.clone())
        })
    };
    builder.function(insert_before, js_string!("insertBefore"), 2);

    // children (getter — returns array of child element proxies)
    let doc_ch = doc.clone();
    let get_children = unsafe {
        NativeFunction::from_closure(move |this, _args, ctx| {
            let node_id = extract_vex_id(this, ctx)?;

            // Collect child element VexIds first (borrow scope).
            let child_ids: Vec<VexId> = {
                let doc_ref = doc_ch.borrow();
                let arena = doc_ref.arena();
                let mut ids = Vec::new();
                let mut child = arena.get(node_id).first_child;
                while let Some(cid) = child {
                    if matches!(&arena.get(cid).data, NodeData::Element(_)) {
                        ids.push(cid);
                    }
                    child = arena.get(cid).next_sibling;
                }
                ids
            };

            // Build proxies outside the borrow.
            let arr = JsArray::new(ctx);
            for cid in child_ids {
                let proxy = build_element_proxy(cid, &doc_ch, ctx);
                arr.push(proxy, ctx)?;
            }
            Ok(JsValue::from(arr))
        })
    };
    builder.function(get_children, js_string!("getChildren"), 0);

    builder.build().into()
}

/// Extract `VexId` from a JS object proxy carrying `__vex_id`.
fn extract_vex_id(val: &JsValue, context: &mut Context) -> JsResult<VexId> {
    let obj = val
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("expected a DOM element proxy"))?;
    let raw = obj
        .get(js_string!("__vex_id"), context)?
        .to_i32(context)?;
    Ok(VexId::new(raw as u32))
}

/// Serialize child nodes of an element to an HTML string (for `innerHTML`).
fn inner_html(arena: &vex_dom::NodeArena, id: VexId) -> String {
    let mut out = String::new();
    let mut child = arena.get(id).first_child;
    while let Some(cid) = child {
        out.push_str(&vex_dom::serialize::serialize(arena, cid));
        child = arena.get(cid).next_sibling;
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;
    use vex_dom::{Document, Namespace};

    fn setup() -> (Context, SharedDocument) {
        let mut doc = Document::new();
        let root = doc.root();

        let html = doc.create_element("html", Namespace::Html);
        doc.append_child(root, html);

        let body = doc.create_element("body", Namespace::Html);
        doc.append_child(html, body);

        let div = doc.create_element("div", Namespace::Html);
        attributes::set_attribute(doc.arena_mut(), div, "id", "main");
        attributes::set_attribute(doc.arena_mut(), div, "class", "container");
        doc.append_child(body, div);

        let p = doc.create_element("p", Namespace::Html);
        attributes::set_attribute(doc.arena_mut(), p, "data-info", "test");
        doc.append_child(div, p);

        let text = doc.create_text("Hello");
        doc.append_child(p, text);

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        crate::api::document::register(&shared, &mut ctx);
        (ctx, shared)
    }

    #[test]
    fn get_attribute_returns_value() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('main').getAttribute('class')",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "container");
    }

    #[test]
    fn get_attribute_returns_null_for_missing() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('main').getAttribute('nonexistent')",
            ))
            .unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn set_attribute_modifies_dom() {
        let (mut ctx, doc) = setup();
        ctx.eval(Source::from_bytes(
            "document.getElementById('main').setAttribute('data-x', '42')",
        ))
        .unwrap();

        let doc_ref = doc.borrow();
        let main_id = doc_ref.get_element_by_id("main").unwrap();
        let val = attributes::get_attribute(doc_ref.arena(), main_id, "data-x");
        assert_eq!(val, Some("42"));
    }

    #[test]
    fn remove_attribute_removes_from_dom() {
        let (mut ctx, doc) = setup();
        ctx.eval(Source::from_bytes(
            "document.getElementById('main').removeAttribute('class')",
        ))
        .unwrap();

        let doc_ref = doc.borrow();
        let main_id = doc_ref.get_element_by_id("main").unwrap();
        assert!(!attributes::has_attribute(doc_ref.arena(), main_id, "class"));
    }

    #[test]
    fn append_child_adds_to_tree() {
        let (mut ctx, doc) = setup();
        ctx.eval(Source::from_bytes(
            "var span = document.createElement('span'); \
             document.getElementById('main').appendChild(span);",
        ))
        .unwrap();

        let doc_ref = doc.borrow();
        let spans = doc_ref.get_elements_by_tag_name("span");
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn remove_child_detaches_from_tree() {
        let (mut ctx, doc) = setup();
        // First verify p is in the tree
        assert_eq!(doc.borrow().get_elements_by_tag_name("p").len(), 1);

        ctx.eval(Source::from_bytes(
            "var main = document.getElementById('main'); \
             var p = document.querySelector('p'); \
             main.removeChild(p);",
        ))
        .unwrap();

        // p should no longer be reachable from root
        assert_eq!(doc.borrow().get_elements_by_tag_name("p").len(), 0);
    }

    #[test]
    fn text_content_reads_text() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.querySelector('p').textContent",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "Hello");
    }

    #[test]
    fn inner_html_reads_children() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('main').innerHTML",
            ))
            .unwrap();
        let html = result.as_string().unwrap().to_std_string_escaped();
        assert!(html.contains("<p"), "innerHTML should contain <p: got {html}");
        assert!(html.contains("Hello"), "innerHTML should contain text");
    }

    #[test]
    fn element_has_tag_name_uppercase() {
        let (mut ctx, _doc) = setup();
        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('main').tagName",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "DIV");
    }

    #[test]
    fn insert_before_inserts_node() {
        let (mut ctx, doc) = setup();
        ctx.eval(Source::from_bytes(
            "var main = document.getElementById('main'); \
             var span = document.createElement('span'); \
             var p = document.querySelector('p'); \
             main.insertBefore(span, p);",
        ))
        .unwrap();

        // span should now be a child of main, before p
        let doc_ref = doc.borrow();
        let main_id = doc_ref.get_element_by_id("main").unwrap();
        let first_child = doc_ref.arena().get(main_id).first_child.unwrap();
        if let NodeData::Element(el) = &doc_ref.arena().get(first_child).data {
            assert_eq!(el.tag_name, "span");
        } else {
            panic!("first child should be span element");
        }
    }
}
