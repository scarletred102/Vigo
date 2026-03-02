// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! JavaScript runtime context wrapping the Boa engine.
//!
//! Provides [`JsRuntime`] — the primary entry point for executing JavaScript
//! within a browser document context.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use boa_engine::object::builtins::JsFunction;
use boa_engine::{js_string, Context, JsValue, Source};
use vex_core::{VexError, VexResult};

use crate::api::events::EventBridge;
use crate::browser_request::{new_request_queue, RequestQueue};
use crate::dom_bridge::SharedDocument;

/// A pending timer entry for the event-loop timer queue.
struct PendingTimer {
    id: u32,
    callback: JsFunction,
    interval: Option<Duration>,
}

/// Wraps a Boa [`Context`] with Vex-specific Web API registrations.
///
/// Each document gets its own `JsRuntime` — contexts are isolated.
pub struct JsRuntime {
    context: Context,
    timer_queue: BTreeMap<Instant, Vec<PendingTimer>>,
    /// Shared request queue for JS → browser communication.
    request_queue: RequestQueue,
    /// Event bridge (callbacks, listener map, registry).
    event_bridge: EventBridge,
    /// Shared tokio runtime handle for async operations (fetch, etc.).
    tokio_handle: tokio::runtime::Handle,
    /// Owned tokio runtime (kept alive for the handle).
    _tokio_runtime: tokio::runtime::Runtime,
}

impl JsRuntime {
    /// Create a new JS runtime with default built-ins (ES2024).
    ///
    /// Registers Web APIs: `console`, `setTimeout`/`setInterval`/`clearTimeout`/`clearInterval`.
    pub fn new() -> Self {
        Self::with_request_queue(new_request_queue())
    }

    /// Create a JS runtime with an externally-provided request queue.
    ///
    /// The browser loop clones the same `RequestQueue` and drains it each tick.
    pub fn with_request_queue(queue: RequestQueue) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for JS");
        let handle = rt.handle().clone();

        let mut context = Context::default();
        crate::api::console::register(&queue, &mut context);
        crate::api::timers::register(&mut context);
        crate::api::fetch::register_with_handle(&handle, &mut context);
        crate::api::window::register_with_queue(&queue, &mut context);
        Self {
            context,
            timer_queue: BTreeMap::new(),
            request_queue: queue,
            event_bridge: EventBridge::new(),
            tokio_handle: handle,
            _tokio_runtime: rt,
        }
    }

    /// Access the shared request queue.
    ///
    /// The browser loop drains this after each JS execution to process
    /// navigation requests, alerts, console logs, etc.
    pub fn request_queue(&self) -> &RequestQueue {
        &self.request_queue
    }

    /// Access the shared tokio runtime handle.
    pub fn tokio_handle(&self) -> &tokio::runtime::Handle {
        &self.tokio_handle
    }

    /// Access the event bridge (callbacks, listeners, registry).
    pub fn event_bridge(&self) -> &EventBridge {
        &self.event_bridge
    }

    /// Register the DOM `document` global with full event listener support.
    ///
    /// This must be called after the DOM is parsed (the `SharedDocument`
    /// must contain a real document). Elements returned by
    /// `getElementById`, `querySelector`, etc. will include
    /// `addEventListener` / `removeEventListener`.
    pub fn register_document(&mut self, doc: &SharedDocument) {
        crate::api::document::register_with_events(doc, &self.event_bridge, &mut self.context);
    }

    /// Dispatch a DOM event through the JS event system.
    ///
    /// Walks the DOM capture → target → bubble path, invoking registered
    /// JS callbacks. Returns `true` if `preventDefault()` was called.
    pub fn dispatch_dom_event(
        &mut self,
        doc: &SharedDocument,
        event: &mut vex_dom::events::Event,
    ) -> bool {
        crate::api::events::dispatch_js_event(
            doc,
            &self.event_bridge.listeners,
            &self.event_bridge.callbacks,
            event,
            &mut self.context,
        )
    }

    /// Fire the `DOMContentLoaded` lifecycle event.
    ///
    /// Call after the DOM tree is fully built and all blocking + deferred
    /// scripts have executed. Returns `true` if `preventDefault()` was called.
    pub fn fire_dom_content_loaded(&mut self, doc: &SharedDocument) -> bool {
        crate::lifecycle::fire_dom_content_loaded(
            doc,
            &self.event_bridge.listeners,
            &self.event_bridge.callbacks,
            &mut self.context,
        )
    }

    /// Fire the `load` lifecycle event.
    ///
    /// Call after all sub-resources (images, stylesheets, async scripts)
    /// have finished loading. Returns `true` if `preventDefault()` was called.
    pub fn fire_load(&mut self, doc: &SharedDocument) -> bool {
        crate::lifecycle::fire_load(
            doc,
            &self.event_bridge.listeners,
            &self.event_bridge.callbacks,
            &mut self.context,
        )
    }

    /// Execute a JavaScript source string for its side effects.
    ///
    /// Returns `Ok(())` on success, or `VexError::Js` on parse/runtime errors.
    /// Use [`eval`](Self::eval) if you need the expression result.
    pub fn execute(&mut self, script: &str) -> VexResult<()> {
        let source = Source::from_bytes(script);
        self.context
            .eval(source)
            .map_err(|e| VexError::Js(format!("{e}")))?;
        self.drain_js_timers();
        Ok(())
    }

    /// Execute JS and return the actual result value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use vex_js::JsRuntime;
    /// let mut rt = JsRuntime::new();
    /// let result = rt.eval("1 + 2").unwrap();
    /// assert_eq!(result.as_number().unwrap(), 3.0);
    /// ```
    pub fn eval(&mut self, script: &str) -> VexResult<JsValue> {
        let source = Source::from_bytes(script);
        let result = self
            .context
            .eval(source)
            .map_err(|e| VexError::Js(format!("{e}")))?;
        self.drain_js_timers();
        Ok(result)
    }

    /// Fire all expired timers. Call once per event-loop tick.
    ///
    /// Returns the number of callbacks invoked.
    pub fn run_pending_timers(&mut self) -> u32 {
        let now = Instant::now();
        let mut fired = 0u32;
        let mut reschedule: Vec<(Instant, PendingTimer)> = Vec::new();

        let expired_keys: Vec<Instant> = self.timer_queue.range(..=now).map(|(k, _)| *k).collect();

        for key in expired_keys {
            if let Some(entries) = self.timer_queue.remove(&key) {
                for entry in entries {
                    let _ = entry
                        .callback
                        .call(&JsValue::undefined(), &[], &mut self.context);
                    fired += 1;

                    if let Some(interval) = entry.interval {
                        reschedule.push((
                            Instant::now() + interval,
                            PendingTimer {
                                id: entry.id,
                                callback: entry.callback,
                                interval: Some(interval),
                            },
                        ));
                    }
                }
            }
        }

        for (fire_at, entry) in reschedule {
            self.timer_queue.entry(fire_at).or_default().push(entry);
        }

        fired
    }

    /// Returns `true` if there are pending timers.
    pub fn has_pending_timers(&self) -> bool {
        !self.timer_queue.is_empty()
    }

    /// Cancel a timer by ID.
    pub fn clear_timer(&mut self, id: u32) {
        for entries in self.timer_queue.values_mut() {
            entries.retain(|e| e.id != id);
        }
        self.timer_queue.retain(|_, v| !v.is_empty());
    }

    /// Get a mutable reference to the underlying Boa context.
    ///
    /// Used by API registration modules to install global objects and functions.
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Get a shared reference to the underlying Boa context.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Drain timer descriptors from the JS-side `__vex_timers` array into
    /// the Rust-side `BTreeMap`.
    fn drain_js_timers(&mut self) {
        let global = self.context.global_object();
        let timers_key = js_string!("__vex_timers");

        let Ok(timers_val) = global.get(timers_key.clone(), &mut self.context) else {
            return;
        };
        if timers_val.is_undefined() || timers_val.is_null() {
            return;
        }
        let Ok(arr_obj) = timers_val.to_object(&mut self.context) else {
            return;
        };
        let Ok(len_val) = arr_obj.get(js_string!("length"), &mut self.context) else {
            return;
        };
        let Ok(len) = len_val.to_u32(&mut self.context) else {
            return;
        };

        for i in 0..len {
            let Ok(entry_val) = arr_obj.get(i, &mut self.context) else {
                continue;
            };
            let Ok(entry_obj) = entry_val.to_object(&mut self.context) else {
                continue;
            };

            // Read timer descriptor fields
            let Ok(id_val) = entry_obj.get(js_string!("id"), &mut self.context) else {
                continue;
            };
            let Ok(id) = id_val.to_u32(&mut self.context) else {
                continue;
            };

            let Ok(cb_val) = entry_obj.get(js_string!("callback"), &mut self.context) else {
                continue;
            };
            // Skip cancelled timers (callback set to undefined)
            if cb_val.is_undefined() {
                continue;
            }
            let Some(cb_obj) = cb_val.as_callable() else {
                continue;
            };

            let Ok(delay_val) = entry_obj.get(js_string!("delay"), &mut self.context) else {
                continue;
            };
            let Ok(delay_ms) = delay_val.to_u32(&mut self.context) else {
                continue;
            };

            let Ok(repeating_val) = entry_obj.get(js_string!("repeating"), &mut self.context)
            else {
                continue;
            };
            let repeating = repeating_val.to_boolean();

            let fire_at = Instant::now() + Duration::from_millis(u64::from(delay_ms));
            let Some(callback) = JsFunction::from_object(cb_obj.clone()) else {
                continue;
            };

            self.timer_queue
                .entry(fire_at)
                .or_default()
                .push(PendingTimer {
                    id,
                    callback,
                    interval: if repeating {
                        Some(Duration::from_millis(u64::from(delay_ms)))
                    } else {
                        None
                    },
                });
        }

        // Clear the JS-side array
        let empty_arr = boa_engine::object::builtins::JsArray::new(&mut self.context);
        let _ = global.set(
            timers_key,
            JsValue::from(empty_arr),
            false,
            &mut self.context,
        );
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_evaluation() {
        let mut rt = JsRuntime::new();
        let result = rt.eval("2 + 3 * 4").unwrap();
        assert_eq!(result.as_number().unwrap(), 14.0);
    }

    #[test]
    fn test_string_concatenation() {
        let mut rt = JsRuntime::new();
        let result = rt.eval("'hello' + ' ' + 'world'").unwrap();
        let s = result.as_string().unwrap();
        assert_eq!(s.to_std_string_escaped(), "hello world");
    }

    #[test]
    fn test_syntax_error_returns_err() {
        let mut rt = JsRuntime::new();
        let result = rt.eval("function(");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, VexError::Js(_)));
    }
}
