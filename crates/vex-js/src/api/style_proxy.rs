// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `element.style` proxy — read/write inline CSS via the `style` attribute.
//!
//! Getting `el.style.color` reads the `color` value from the inline
//! `style="..."` attribute. Setting it modifies the attribute in-place.
//!
//! ## Design
//!
//! We build a plain JS object with getter/setter closures for common
//! CSS properties. Each closure reads/writes `style` attribute on the
//! DOM element via `SharedDocument`.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::{Attribute, PropertyDescriptor};
use boa_engine::{js_string, Context, JsValue, NativeFunction};
use vex_core::VexId;

use crate::dom_bridge::SharedDocument;

/// Known CSS properties exposed on `element.style`.
///
/// Each entry is `(js_property_name, css_property_name)`.
const STYLE_PROPERTIES: &[(&str, &str)] = &[
    ("color", "color"),
    ("backgroundColor", "background-color"),
    ("background", "background"),
    ("fontSize", "font-size"),
    ("fontFamily", "font-family"),
    ("fontWeight", "font-weight"),
    ("fontStyle", "font-style"),
    ("margin", "margin"),
    ("marginTop", "margin-top"),
    ("marginRight", "margin-right"),
    ("marginBottom", "margin-bottom"),
    ("marginLeft", "margin-left"),
    ("padding", "padding"),
    ("paddingTop", "padding-top"),
    ("paddingRight", "padding-right"),
    ("paddingBottom", "padding-bottom"),
    ("paddingLeft", "padding-left"),
    ("width", "width"),
    ("height", "height"),
    ("display", "display"),
    ("position", "position"),
    ("top", "top"),
    ("right", "right"),
    ("bottom", "bottom"),
    ("left", "left"),
    ("border", "border"),
    ("borderRadius", "border-radius"),
    ("opacity", "opacity"),
    ("overflow", "overflow"),
    ("textAlign", "text-align"),
    ("textDecoration", "text-decoration"),
    ("lineHeight", "line-height"),
    ("visibility", "visibility"),
    ("zIndex", "z-index"),
    ("cursor", "cursor"),
    ("boxSizing", "box-sizing"),
    ("flexDirection", "flex-direction"),
    ("justifyContent", "justify-content"),
    ("alignItems", "align-items"),
    ("flexWrap", "flex-wrap"),
    ("flexGrow", "flex-grow"),
    ("flexShrink", "flex-shrink"),
];

/// Build a style proxy object for the given element.
///
/// The proxy has `cssText` for raw style access plus named getters/setters
/// for the common properties listed in [`STYLE_PROPERTIES`].
pub fn build_style_proxy(node_id: VexId, doc: &SharedDocument, context: &mut Context) -> JsValue {
    let mut builder = ObjectInitializer::new(context);

    // cssText — current raw style string
    let css_text = {
        let doc_ref = doc.borrow();
        vex_dom::attributes::get_attribute(doc_ref.arena(), node_id, "style")
            .unwrap_or("")
            .to_string()
    };
    builder.property(
        js_string!("cssText"),
        js_string!(css_text),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // setProperty(name, value) — generic setter
    let doc_sp = doc.clone();
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let set_property = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let prop = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let val = args
                .get(1)
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            set_style_property(&doc_sp, node_id, &prop, &val);
            Ok(JsValue::undefined())
        })
    };
    builder.function(set_property, js_string!("setProperty"), 2);

    // getPropertyValue(name) — generic getter
    let doc_gp = doc.clone();
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let get_property_value = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let prop = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let val = get_style_property(&doc_gp, node_id, &prop);
            Ok(JsValue::from(js_string!(val)))
        })
    };
    builder.function(get_property_value, js_string!("getPropertyValue"), 1);

    // removeProperty(name)
    let doc_rp = doc.clone();
    // SAFETY: The closure captures only Rc<RefCell<Document>> handles and is
    // invoked by Boa on the same thread as the owning JS context.
    let remove_property = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let prop = args
                .first()
                .map(|v| v.to_string(ctx))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let old = get_style_property(&doc_rp, node_id, &prop);
            set_style_property(&doc_rp, node_id, &prop, "");
            Ok(JsValue::from(js_string!(old)))
        })
    };
    builder.function(remove_property, js_string!("removeProperty"), 1);

    let obj = builder.build();

    // Named property accessors — live getter/setter closures that read/write
    // the DOM `style` attribute on every access.
    for &(js_name, css_name) in STYLE_PROPERTIES {
        let getter_doc = doc.clone();
        let css_get = css_name.to_owned();
        // SAFETY: Closure captures Rc<RefCell<Document>> and a String.
        // Invoked by Boa on the same single JS thread.
        let getter = unsafe {
            NativeFunction::from_closure(move |_this, _args, _ctx| {
                let val = get_style_property(&getter_doc, node_id, &css_get);
                Ok(JsValue::from(js_string!(val)))
            })
        };

        let setter_doc = doc.clone();
        let css_set = css_name.to_owned();
        // SAFETY: Same single-thread guarantee.
        let setter = unsafe {
            NativeFunction::from_closure(move |_this, args, ctx| {
                let val = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                set_style_property(&setter_doc, node_id, &css_set, &val);
                Ok(JsValue::undefined())
            })
        };

        let get_fn = getter.to_js_function(context.realm());
        let set_fn = setter.to_js_function(context.realm());

        let _ = obj.define_property_or_throw(
            js_string!(js_name),
            PropertyDescriptor::builder()
                .get(get_fn)
                .set(set_fn)
                .enumerable(true)
                .configurable(true)
                .build(),
            context,
        );
    }

    obj.into()
}

/// Read a single CSS property from the element's `style` attribute.
fn get_style_property(doc: &SharedDocument, node_id: VexId, property: &str) -> String {
    let doc_ref = doc.borrow();
    let style_str =
        vex_dom::attributes::get_attribute(doc_ref.arena(), node_id, "style").unwrap_or("");
    parse_inline_value(style_str, property)
}

/// Write a single CSS property into the element's `style` attribute.
///
/// If `value` is empty, removes the property.
fn set_style_property(doc: &SharedDocument, node_id: VexId, property: &str, value: &str) {
    let mut doc_mut = doc.borrow_mut();
    let style_str = vex_dom::attributes::get_attribute(doc_mut.arena(), node_id, "style")
        .unwrap_or("")
        .to_string();

    let new_style = update_inline_property(&style_str, property, value);
    vex_dom::attributes::set_attribute(doc_mut.arena_mut(), node_id, "style", &new_style);
}

/// Parse a CSS property value from an inline `style` string.
///
/// Example: `parse_inline_value("color: red; font-size: 14px", "color")` → `"red"`
fn parse_inline_value(style: &str, property: &str) -> String {
    for decl in style.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some((name, value)) = decl.split_once(':') {
            if name.trim().eq_ignore_ascii_case(property) {
                return value.trim().to_string();
            }
        }
    }
    String::new()
}

/// Update (or remove) a property in an inline style string.
fn update_inline_property(style: &str, property: &str, value: &str) -> String {
    let mut found = false;
    let mut parts: Vec<String> = Vec::new();

    for decl in style.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some((name, _)) = decl.split_once(':') {
            if name.trim().eq_ignore_ascii_case(property) {
                found = true;
                if !value.is_empty() {
                    parts.push(format!("{}: {}", property, value));
                }
                continue;
            }
        }
        parts.push(decl.to_string());
    }

    if !found && !value.is_empty() {
        parts.push(format!("{}: {}", property, value));
    }

    parts.join("; ")
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inline_value_basic() {
        assert_eq!(
            parse_inline_value("color: red; font-size: 14px", "color"),
            "red"
        );
        assert_eq!(
            parse_inline_value("color: red; font-size: 14px", "font-size"),
            "14px"
        );
        assert_eq!(
            parse_inline_value("color: red; font-size: 14px", "display"),
            ""
        );
    }

    #[test]
    fn update_inline_property_add() {
        let result = update_inline_property("", "color", "blue");
        assert_eq!(result, "color: blue");
    }

    #[test]
    fn update_inline_property_modify() {
        let result = update_inline_property("color: red; font-size: 14px", "color", "blue");
        assert!(result.contains("color: blue"));
        assert!(result.contains("font-size: 14px"));
    }

    #[test]
    fn update_inline_property_remove() {
        let result = update_inline_property("color: red; font-size: 14px", "color", "");
        assert!(!result.contains("color"));
        assert!(result.contains("font-size: 14px"));
    }

    #[test]
    fn style_proxy_reads_inline_style() {
        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);
        vex_dom::attributes::set_attribute(
            doc.arena_mut(),
            div,
            "style",
            "color: red; font-size: 16px",
        );

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();

        let proxy = build_style_proxy(div, &shared, &mut ctx);
        let obj = proxy.as_object().unwrap();

        let color = obj.get(js_string!("color"), &mut ctx).unwrap();
        assert_eq!(color.as_string().unwrap().to_std_string_escaped(), "red");

        let fs = obj.get(js_string!("fontSize"), &mut ctx).unwrap();
        assert_eq!(fs.as_string().unwrap().to_std_string_escaped(), "16px");
    }

    #[test]
    fn style_set_property_modifies_dom() {
        use boa_engine::Source;

        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "id", "target");
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "style", "color: red");

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        crate::api::document::register(&shared, &mut ctx);

        // Use setProperty on the style proxy
        ctx.eval(Source::from_bytes(
            "var el = document.getElementById('target'); \
             el.style.setProperty('color', 'blue');",
        ))
        .unwrap();

        let doc_ref = shared.borrow();
        let style = vex_dom::attributes::get_attribute(doc_ref.arena(), div, "style").unwrap();
        assert!(
            style.contains("blue"),
            "style should contain blue: got {style}"
        );
    }

    #[test]
    fn style_get_property_value() {
        use boa_engine::Source;

        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "id", "target");
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "style", "color: green");

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        crate::api::document::register(&shared, &mut ctx);

        let result = ctx
            .eval(Source::from_bytes(
                "document.getElementById('target').style.getPropertyValue('color')",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "green");
    }

    #[test]
    fn style_named_setter_updates_dom() {
        use boa_engine::Source;

        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "id", "target");
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "style", "color: red");

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        crate::api::document::register(&shared, &mut ctx);

        // Write via named property setter: el.style.color = 'blue'
        ctx.eval(Source::from_bytes(
            "var el = document.getElementById('target'); \
             el.style.color = 'blue';",
        ))
        .unwrap();

        let doc_ref = shared.borrow();
        let style = vex_dom::attributes::get_attribute(doc_ref.arena(), div, "style").unwrap();
        assert!(
            style.contains("blue"),
            "style should contain blue after named setter: got {style}"
        );
    }

    #[test]
    fn style_named_getter_reflects_changes() {
        use boa_engine::Source;

        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "id", "target");
        vex_dom::attributes::set_attribute(doc.arena_mut(), div, "style", "color: red");

        let shared = crate::dom_bridge::shared_document(doc);
        let mut ctx = Context::default();
        crate::api::document::register(&shared, &mut ctx);

        // Change via setProperty, read via named getter.
        let result = ctx
            .eval(Source::from_bytes(
                "var el = document.getElementById('target'); \
                 el.style.setProperty('color', 'green'); \
                 el.style.color",
            ))
            .unwrap();
        assert_eq!(result.as_string().unwrap().to_std_string_escaped(), "green");
    }
}
