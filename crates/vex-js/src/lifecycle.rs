// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Page lifecycle events — `DOMContentLoaded` and `load`.
//!
//! ## Timing
//!
//! - **`DOMContentLoaded`**: Fired on the `document` node after the DOM is fully
//!   built and all blocking + deferred scripts have executed. Async scripts and
//!   sub-resources (images, stylesheets) may still be loading.
//!
//! - **`load`**: Fired on the `document` node after *all* resources have finished
//!   loading (images, stylesheets, async scripts, etc.).
//!
//! ## Usage
//!
//! ```ignore
//! use vex_js::lifecycle::fire_dom_content_loaded;
//!
//! // After DOM parse + defer scripts have run:
//! fire_dom_content_loaded(&shared_doc, &listeners, &callbacks, &mut context);
//!
//! // After all sub-resources loaded:
//! fire_load(&shared_doc, &listeners, &callbacks, &mut context);
//! ```

use boa_engine::Context;
use tracing::debug;
use vex_dom::events::{Event, EventType};

use crate::api::events::{dispatch_js_event, JsCallbackMap, SharedListenerMap};
use crate::dom_bridge::SharedDocument;

/// Fire the `DOMContentLoaded` event on the document root.
///
/// Should be called after the DOM tree is fully built and all blocking +
/// deferred scripts have executed.
///
/// Returns `true` if any listener called `preventDefault()`.
pub fn fire_dom_content_loaded(
    doc: &SharedDocument,
    listeners: &SharedListenerMap,
    callbacks: &JsCallbackMap,
    context: &mut Context,
) -> bool {
    let root = doc.borrow().root();
    debug!("firing DOMContentLoaded on node {}", root.index());

    let mut event = Event::new(EventType::DomContentLoaded, root);
    dispatch_js_event(doc, listeners, callbacks, &mut event, context)
}

/// Fire the `load` event on the document root.
///
/// Should be called after all sub-resources (images, stylesheets, async
/// scripts) have finished loading.
///
/// Returns `true` if any listener called `preventDefault()`.
pub fn fire_load(
    doc: &SharedDocument,
    listeners: &SharedListenerMap,
    callbacks: &JsCallbackMap,
    context: &mut Context,
) -> bool {
    let root = doc.borrow().root();
    debug!("firing load on node {}", root.index());

    let mut event = Event::new(EventType::Load, root);
    dispatch_js_event(doc, listeners, callbacks, &mut event, context)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::object::builtins::JsFunction;
    use boa_engine::Source;
    use vex_dom::events::EventType;
    use vex_dom::{Document, Namespace};

    use crate::api::events::{new_callback_map, new_listener_map, register_listener};

    fn setup() -> (SharedDocument, Context) {
        let mut doc = Document::new();
        let root = doc.root();
        let body = doc.create_element("body", Namespace::Html);
        doc.append_child(root, body);
        let shared = crate::dom_bridge::shared_document(doc);
        let ctx = Context::default();
        (shared, ctx)
    }

    #[test]
    fn dom_content_loaded_fires_listener() {
        let (shared, mut ctx) = setup();
        let callbacks = new_callback_map();
        let listeners = new_listener_map();

        // Register a DOMContentLoaded listener on the root
        let root = shared.borrow().root();
        ctx.eval(Source::from_bytes("var dcl_fired = false;"))
            .unwrap();
        let func_val = ctx
            .eval(Source::from_bytes(
                "(function(e) { dcl_fired = true; })",
            ))
            .unwrap();
        let func =
            JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();

        register_listener(
            &callbacks,
            &listeners,
            root,
            EventType::DomContentLoaded,
            func,
            false,
        );

        let prevented = fire_dom_content_loaded(&shared, &listeners, &callbacks, &mut ctx);
        assert!(!prevented);

        let val = ctx.eval(Source::from_bytes("dcl_fired")).unwrap();
        assert!(val.as_boolean().unwrap());
    }

    #[test]
    fn load_fires_listener() {
        let (shared, mut ctx) = setup();
        let callbacks = new_callback_map();
        let listeners = new_listener_map();

        let root = shared.borrow().root();
        ctx.eval(Source::from_bytes("var load_fired = false;"))
            .unwrap();
        let func_val = ctx
            .eval(Source::from_bytes("(function(e) { load_fired = true; })"))
            .unwrap();
        let func =
            JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();

        register_listener(
            &callbacks,
            &listeners,
            root,
            EventType::Load,
            func,
            false,
        );

        let prevented = fire_load(&shared, &listeners, &callbacks, &mut ctx);
        assert!(!prevented);

        let val = ctx.eval(Source::from_bytes("load_fired")).unwrap();
        assert!(val.as_boolean().unwrap());
    }

    #[test]
    fn dom_content_loaded_fires_before_load() {
        let (shared, mut ctx) = setup();
        let callbacks = new_callback_map();
        let listeners = new_listener_map();
        let root = shared.borrow().root();

        ctx.eval(Source::from_bytes("var order = [];")).unwrap();

        let dcl_func_val = ctx
            .eval(Source::from_bytes(
                "(function(e) { order.push('DOMContentLoaded'); })",
            ))
            .unwrap();
        let dcl_func =
            JsFunction::from_object(dcl_func_val.as_object().unwrap().clone()).unwrap();

        let load_func_val = ctx
            .eval(Source::from_bytes(
                "(function(e) { order.push('load'); })",
            ))
            .unwrap();
        let load_func =
            JsFunction::from_object(load_func_val.as_object().unwrap().clone()).unwrap();

        register_listener(
            &callbacks,
            &listeners,
            root,
            EventType::DomContentLoaded,
            dcl_func,
            false,
        );
        register_listener(
            &callbacks,
            &listeners,
            root,
            EventType::Load,
            load_func,
            false,
        );

        // Fire in correct order
        fire_dom_content_loaded(&shared, &listeners, &callbacks, &mut ctx);
        fire_load(&shared, &listeners, &callbacks, &mut ctx);

        let len = ctx
            .eval(Source::from_bytes("order.length"))
            .unwrap()
            .as_number()
            .unwrap() as i32;
        assert_eq!(len, 2);

        let first = ctx
            .eval(Source::from_bytes("order[0]"))
            .unwrap()
            .as_string()
            .unwrap()
            .to_std_string_escaped();
        assert_eq!(first, "DOMContentLoaded");

        let second = ctx
            .eval(Source::from_bytes("order[1]"))
            .unwrap()
            .as_string()
            .unwrap()
            .to_std_string_escaped();
        assert_eq!(second, "load");
    }

    #[test]
    fn no_listener_does_not_panic() {
        let (shared, mut ctx) = setup();
        let callbacks = new_callback_map();
        let listeners = new_listener_map();

        // Should not panic, just returns false
        let prevented = fire_dom_content_loaded(&shared, &listeners, &callbacks, &mut ctx);
        assert!(!prevented);

        let prevented = fire_load(&shared, &listeners, &callbacks, &mut ctx);
        assert!(!prevented);
    }
}
