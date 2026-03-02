// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Event bridge — connects JS `addEventListener` / `removeEventListener`
//! to the Rust DOM event dispatch system.
//!
//! ## Architecture
//!
//! - JS callbacks are stored in a [`JsCallbackMap`] keyed by a monotonic `u64`.
//! - `addEventListener` registers the callback in both the map and the DOM's
//!   [`EventListenerMap`].
//! - [`dispatch_js_event`] fires an event through the DOM dispatch algorithm,
//!   resolving each `callback_id` to a JS function and invoking it with an
//!   event proxy object.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use boa_engine::object::builtins::JsFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsObject, JsValue};
use vex_core::VexId;
use vex_dom::events::event_type::EventType;
use vex_dom::events::{dispatch_event, Event, EventListenerMap, EventPhase};

use crate::dom_bridge::SharedDocument;

/// Global counter for unique callback IDs.
static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(1);

/// Shared storage for JS event callbacks.
///
/// Clone-cheap — all clones point to the same inner `HashMap`.
pub type JsCallbackMap = Rc<RefCell<HashMap<u64, JsFunction>>>;

/// Create a new empty callback map.
pub fn new_callback_map() -> JsCallbackMap {
    Rc::new(RefCell::new(HashMap::new()))
}

/// Shared storage for the DOM event listener registrations.
pub type SharedListenerMap = Rc<RefCell<EventListenerMap>>;

/// Create a new empty listener map.
pub fn new_listener_map() -> SharedListenerMap {
    Rc::new(RefCell::new(EventListenerMap::new()))
}

// ── Callback Registry (for removeEventListener) ──────────────────────

/// Entry mapping a registered JS callback to its internal `callback_id`.
struct CallbackEntry {
    node_id: VexId,
    event_type: String,
    callback_id: u64,
    /// Stored for identity comparison in `removeEventListener`.
    js_object: JsObject,
    capture: bool,
}

/// Tracks which JS function objects correspond to which `callback_id`s,
/// enabling `removeEventListener` to find the correct listener to remove.
pub(crate) struct CallbackRegistry {
    entries: Vec<CallbackEntry>,
}

impl CallbackRegistry {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn add(&mut self, node_id: VexId, event_type: &str, id: u64, obj: JsObject, capture: bool) {
        self.entries.push(CallbackEntry {
            node_id,
            event_type: event_type.to_owned(),
            callback_id: id,
            js_object: obj,
            capture,
        });
    }

    fn find_and_remove(
        &mut self,
        node_id: VexId,
        event_type: &str,
        obj: &JsObject,
        capture: bool,
    ) -> Option<u64> {
        let pos = self.entries.iter().position(|e| {
            e.node_id == node_id
                && e.event_type == event_type
                && e.capture == capture
                && e.js_object == *obj
        })?;
        Some(self.entries.remove(pos).callback_id)
    }
}

/// Shared callback registry.
pub(crate) type SharedCallbackRegistry = Rc<RefCell<CallbackRegistry>>;

/// Create a new empty callback registry.
pub(crate) fn new_callback_registry() -> SharedCallbackRegistry {
    Rc::new(RefCell::new(CallbackRegistry::new()))
}

// ── Event Bridge ─────────────────────────────────────────────────────

/// Groups the three event-system handles that are threaded through element
/// and document proxies. Clone is cheap (three `Rc` bumps).
#[derive(Clone)]
pub struct EventBridge {
    /// Callback-id → `JsFunction` lookup used during dispatch.
    pub callbacks: JsCallbackMap,
    /// DOM-level listener registrations.
    pub listeners: SharedListenerMap,
    /// Reverse mapping for `removeEventListener`.
    pub(crate) registry: SharedCallbackRegistry,
}

impl EventBridge {
    /// Create a new event bridge with empty maps.
    pub fn new() -> Self {
        Self {
            callbacks: new_callback_map(),
            listeners: new_listener_map(),
            registry: new_callback_registry(),
        }
    }
}

impl Default for EventBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Register a JS callback for a DOM event, returning the `callback_id`.
pub fn register_listener(
    callbacks: &JsCallbackMap,
    listeners: &SharedListenerMap,
    node_id: VexId,
    event_type: EventType,
    callback: JsFunction,
    capture: bool,
) -> u64 {
    let id = NEXT_CALLBACK_ID.fetch_add(1, Ordering::Relaxed);
    callbacks.borrow_mut().insert(id, callback);
    listeners
        .borrow_mut()
        .add_listener(node_id, event_type, id, capture);
    id
}

/// Register a JS callback using the event bridge (also records in registry
/// for `removeEventListener` identity lookup).
pub fn register_listener_bridge(
    bridge: &EventBridge,
    node_id: VexId,
    event_type: EventType,
    callback: JsFunction,
    capture: bool,
) -> u64 {
    let js_obj: JsObject = callback.clone().into();
    let type_name = event_type.name().to_owned();
    let id = register_listener(
        &bridge.callbacks,
        &bridge.listeners,
        node_id,
        event_type,
        callback,
        capture,
    );
    bridge
        .registry
        .borrow_mut()
        .add(node_id, &type_name, id, js_obj, capture);
    id
}

/// Remove a previously registered event listener by callback_id.
pub fn unregister_listener(
    callbacks: &JsCallbackMap,
    listeners: &SharedListenerMap,
    node_id: VexId,
    event_type: &EventType,
    callback_id: u64,
    capture: bool,
) {
    callbacks.borrow_mut().remove(&callback_id);
    listeners
        .borrow_mut()
        .remove_listener(node_id, event_type, callback_id, capture);
}

/// Remove a listener by JS function identity (for `removeEventListener`).
///
/// Returns the callback_id if found and removed, otherwise `None`.
pub fn unregister_listener_bridge(
    bridge: &EventBridge,
    node_id: VexId,
    event_type: &str,
    js_obj: &JsObject,
    capture: bool,
) -> Option<u64> {
    let id = bridge
        .registry
        .borrow_mut()
        .find_and_remove(node_id, event_type, js_obj, capture)?;
    let et = parse_event_type(event_type);
    unregister_listener(
        &bridge.callbacks,
        &bridge.listeners,
        node_id,
        &et,
        id,
        capture,
    );
    Some(id)
}

/// Dispatch a DOM event, invoking JS callbacks along the propagation path.
///
/// Returns `true` if `preventDefault()` was called.
pub fn dispatch_js_event(
    doc: &SharedDocument,
    listeners: &SharedListenerMap,
    callbacks: &JsCallbackMap,
    event: &mut Event,
    context: &mut Context,
) -> bool {
    let doc_ref = doc.borrow();
    let arena = doc_ref.arena();
    let listener_map = listeners.borrow();

    dispatch_event(arena, &listener_map, event, |callback_id, evt| {
        let cbs = callbacks.borrow();
        if let Some(func) = cbs.get(&callback_id) {
            let event_proxy = build_event_proxy(evt, context);
            let _ = func.call(&JsValue::undefined(), &[event_proxy], context);
        }
    })
}

/// Build a JS event object from a Rust `Event`.
fn build_event_proxy(event: &Event, context: &mut Context) -> JsValue {
    let phase_str = match event.phase {
        EventPhase::None => "none",
        EventPhase::Capturing => "capturing",
        EventPhase::AtTarget => "at-target",
        EventPhase::Bubbling => "bubbling",
    };

    ObjectInitializer::new(context)
        .property(
            js_string!("type"),
            js_string!(event.event_type.name().to_string()),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("target"),
            JsValue::from(event.target.index() as i32),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("currentTarget"),
            event
                .current_target
                .map(|id| JsValue::from(id.index() as i32))
                .unwrap_or(JsValue::null()),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("eventPhase"),
            js_string!(phase_str.to_string()),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("bubbles"),
            JsValue::from(event.bubbles),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("cancelable"),
            JsValue::from(event.cancelable),
            Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("defaultPrevented"),
            JsValue::from(event.default_prevented()),
            Attribute::CONFIGURABLE,
        )
        .build()
        .into()
}

/// Parse an event type string (e.g., "click") into an `EventType`.
pub fn parse_event_type(name: &str) -> EventType {
    match name {
        "click" => EventType::Click,
        "mousedown" => EventType::MouseDown,
        "mouseup" => EventType::MouseUp,
        "mousemove" => EventType::MouseMove,
        "keydown" => EventType::KeyDown,
        "keyup" => EventType::KeyUp,
        "focus" => EventType::Focus,
        "blur" => EventType::Blur,
        "input" => EventType::Input,
        "change" => EventType::Change,
        "submit" => EventType::Submit,
        "load" => EventType::Load,
        "DOMContentLoaded" => EventType::DomContentLoaded,
        "scroll" => EventType::Scroll,
        "resize" => EventType::Resize,
        other => EventType::Custom(other.to_string()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::Source;
    use vex_dom::{Document, Namespace};

    fn setup() -> (Document, VexId) {
        let mut doc = Document::new();
        let root = doc.root();
        let div = doc.create_element("div", Namespace::Html);
        doc.append_child(root, div);
        (doc, div)
    }

    #[test]
    fn register_and_dispatch_fires_callback() {
        let (doc, div) = setup();
        let shared = crate::dom_bridge::shared_document(doc);
        let callbacks = new_callback_map();
        let listeners = new_listener_map();
        let mut ctx = Context::default();

        // Register a JS function that sets a global flag
        ctx.eval(Source::from_bytes("var fired = false;")).unwrap();
        let func_val = ctx
            .eval(Source::from_bytes("(function(e) { fired = true; })"))
            .unwrap();
        let func = JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();

        register_listener(&callbacks, &listeners, div, EventType::Click, func, false);

        let mut event = Event::new(EventType::Click, div);
        dispatch_js_event(&shared, &listeners, &callbacks, &mut event, &mut ctx);

        let fired = ctx.eval(Source::from_bytes("fired")).unwrap();
        assert!(fired.as_boolean().unwrap());
    }

    #[test]
    fn event_proxy_has_type_property() {
        let (doc, div) = setup();
        let shared = crate::dom_bridge::shared_document(doc);
        let callbacks = new_callback_map();
        let listeners = new_listener_map();
        let mut ctx = Context::default();

        ctx.eval(Source::from_bytes("var evtType = '';")).unwrap();
        let func_val = ctx
            .eval(Source::from_bytes("(function(e) { evtType = e.type; })"))
            .unwrap();
        let func = JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();

        register_listener(&callbacks, &listeners, div, EventType::Click, func, false);

        let mut event = Event::new(EventType::Click, div);
        dispatch_js_event(&shared, &listeners, &callbacks, &mut event, &mut ctx);

        let evt_type = ctx.eval(Source::from_bytes("evtType")).unwrap();
        assert_eq!(
            evt_type.as_string().unwrap().to_std_string_escaped(),
            "click"
        );
    }

    #[test]
    fn unregister_removes_callback() {
        let (doc, div) = setup();
        let shared = crate::dom_bridge::shared_document(doc);
        let callbacks = new_callback_map();
        let listeners = new_listener_map();
        let mut ctx = Context::default();

        ctx.eval(Source::from_bytes("var count = 0;")).unwrap();
        let func_val = ctx
            .eval(Source::from_bytes("(function(e) { count++; })"))
            .unwrap();
        let func = JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();

        let cb_id = register_listener(&callbacks, &listeners, div, EventType::Click, func, false);

        // Fire once
        let mut event = Event::new(EventType::Click, div);
        dispatch_js_event(&shared, &listeners, &callbacks, &mut event, &mut ctx);

        // Unregister
        unregister_listener(&callbacks, &listeners, div, &EventType::Click, cb_id, false);

        // Fire again — should NOT increment
        let mut event2 = Event::new(EventType::Click, div);
        dispatch_js_event(&shared, &listeners, &callbacks, &mut event2, &mut ctx);

        let count = ctx.eval(Source::from_bytes("count")).unwrap();
        assert_eq!(count.as_number().unwrap() as i32, 1);
    }

    #[test]
    fn parse_event_type_known() {
        assert_eq!(parse_event_type("click"), EventType::Click);
        assert_eq!(parse_event_type("load"), EventType::Load);
        assert_eq!(
            parse_event_type("DOMContentLoaded"),
            EventType::DomContentLoaded
        );
    }

    #[test]
    fn parse_event_type_custom() {
        let et = parse_event_type("myCustomEvent");
        assert!(matches!(et, EventType::Custom(ref s) if s == "myCustomEvent"));
    }

    #[test]
    fn event_bridge_register_and_unregister_by_identity() {
        let (doc, div) = setup();
        let shared = crate::dom_bridge::shared_document(doc);
        let bridge = EventBridge::new();
        let mut ctx = Context::default();

        ctx.eval(Source::from_bytes("var count = 0;")).unwrap();
        let func_val = ctx
            .eval(Source::from_bytes("(function(e) { count++; })"))
            .unwrap();
        let func = JsFunction::from_object(func_val.as_object().unwrap().clone()).unwrap();
        let js_obj: JsObject = func.clone().into();

        register_listener_bridge(&bridge, div, EventType::Click, func, false);

        // Fire once → count = 1
        let mut event = Event::new(EventType::Click, div);
        dispatch_js_event(
            &shared,
            &bridge.listeners,
            &bridge.callbacks,
            &mut event,
            &mut ctx,
        );

        // Unregister by object identity
        let removed = unregister_listener_bridge(&bridge, div, "click", &js_obj, false);
        assert!(removed.is_some());

        // Fire again → count still 1
        let mut event2 = Event::new(EventType::Click, div);
        dispatch_js_event(
            &shared,
            &bridge.listeners,
            &bridge.callbacks,
            &mut event2,
            &mut ctx,
        );

        let count = ctx.eval(Source::from_bytes("count")).unwrap();
        assert_eq!(count.as_number().unwrap() as i32, 1);
    }
}
