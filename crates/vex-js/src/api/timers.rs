// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Timer API — `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`.
//!
//! JS-side, these functions store timer descriptors in a hidden
//! `__vex_timers` array. The Rust-side [`JsRuntime`](crate::JsRuntime)
//! drains that array into a `BTreeMap` and fires callbacks when their
//! deadlines expire.

use boa_engine::{js_string, Context, JsResult, JsValue, NativeFunction};

/// Register `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`
/// as global functions on the context.
///
/// The caller must pump
/// [`JsRuntime::run_pending_timers`](crate::JsRuntime::run_pending_timers)
/// each frame to fire expired callbacks.
pub fn register(context: &mut Context) {
    if let Err(error) = context.register_global_callable(
        js_string!("setTimeout"),
        2,
        NativeFunction::from_fn_ptr(set_timeout),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register setTimeout: {error}");
    }

    if let Err(error) = context.register_global_callable(
        js_string!("setInterval"),
        2,
        NativeFunction::from_fn_ptr(set_interval),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register setInterval: {error}");
    }

    if let Err(error) = context.register_global_callable(
        js_string!("clearTimeout"),
        1,
        NativeFunction::from_fn_ptr(clear_timeout),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register clearTimeout: {error}");
    }

    if let Err(error) = context.register_global_callable(
        js_string!("clearInterval"),
        1,
        NativeFunction::from_fn_ptr(clear_interval),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register clearInterval: {error}");
    }

    if let Err(error) = context.register_global_callable(
        js_string!("requestAnimationFrame"),
        1,
        NativeFunction::from_fn_ptr(request_animation_frame),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register requestAnimationFrame: {error}");
    }

    if let Err(error) = context.register_global_callable(
        js_string!("cancelAnimationFrame"),
        1,
        NativeFunction::from_fn_ptr(cancel_animation_frame),
    ) {
        tracing::error!(target: "vex_js::timers", "failed to register cancelAnimationFrame: {error}");
    }
}

// ---------------------------------------------------------------------------
// Native function implementations
// ---------------------------------------------------------------------------
// These are stubs that store timer metadata. The actual timer queue interaction
// happens through JsRuntime which owns both the Context and TimerQueue.
// The native functions store the callback as a property on a global __timers
// object, and JsRuntime.run_pending_timers() processes them.
// ---------------------------------------------------------------------------

/// Global counter for timer IDs shared across a single-threaded runtime.
static NEXT_TIMER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn set_timeout(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let id = schedule_timer(args, context, false, None, false)?;
    Ok(JsValue::from(id))
}

fn set_interval(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let id = schedule_timer(args, context, true, None, false)?;
    Ok(JsValue::from(id))
}

fn request_animation_frame(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    // rAF uses next-frame scheduling (~60Hz) and callback(timestamp).
    let id = schedule_timer(args, context, false, Some(16), true)?;
    Ok(JsValue::from(id))
}

fn cancel_animation_frame(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    clear_timeout(&JsValue::undefined(), args, context)
}

fn clear_timeout(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    if let Some(id_val) = args.first() {
        let id = id_val.to_u32(context)?;
        clear_timer_by_id(id, context);
    }
    Ok(JsValue::undefined())
}

fn clear_interval(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // clearInterval and clearTimeout are interchangeable per spec
    clear_timeout(&JsValue::undefined(), args, context)
}

/// Shared timer scheduling logic.
///
/// Stores the callback and metadata in a global `__vex_timers` object on the
/// JS context so that `JsRuntime::run_pending_timers()` can retrieve and
/// invoke them.
fn schedule_timer(
    args: &[JsValue],
    context: &mut Context,
    repeating: bool,
    default_delay_ms: Option<u32>,
    raf: bool,
) -> JsResult<u32> {
    let callback = args
        .first()
        .and_then(|v| v.as_callable())
        .ok_or_else(|| {
            boa_engine::JsNativeError::typ().with_message("first argument must be a function")
        })?
        .clone();

    let delay_ms = match default_delay_ms {
        Some(v) => v,
        None => args
            .get(1)
            .map(|v| v.to_u32(context))
            .transpose()?
            .unwrap_or(0),
    };

    let id = NEXT_TIMER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Store timer info on context-local __vex_timers
    let timers_key = js_string!("__vex_timers");
    let global = context.global_object();

    // Ensure __vex_timers array exists
    let timers_val = global.get(timers_key.clone(), context)?;
    let timers_array = if timers_val.is_undefined() || timers_val.is_null() {
        let arr = boa_engine::object::builtins::JsArray::new(context);
        global.set(
            timers_key.clone(),
            JsValue::from(arr.clone()),
            false,
            context,
        )?;
        arr
    } else {
        boa_engine::object::builtins::JsArray::from_object(timers_val.to_object(context)?)?
    };

    // Create timer descriptor: { id, callback, delay, repeating }
    let descriptor = boa_engine::object::ObjectInitializer::new(context)
        .property(
            js_string!("id"),
            JsValue::from(id),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("callback"),
            JsValue::from(callback),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("delay"),
            JsValue::from(delay_ms),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("repeating"),
            JsValue::from(repeating),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("scheduled"),
            JsValue::from(false),
            boa_engine::property::Attribute::all(),
        )
        .property(
            js_string!("raf"),
            JsValue::from(raf),
            boa_engine::property::Attribute::all(),
        )
        .build();

    timers_array.push(descriptor, context)?;

    Ok(id)
}

/// Remove a timer from the `__vex_timers` array by ID.
fn clear_timer_by_id(id: u32, context: &mut Context) {
    let global = context.global_object();
    let timers_key = js_string!("__vex_timers");

    let Ok(timers_val) = global.get(timers_key, context) else {
        return;
    };
    if timers_val.is_undefined() || timers_val.is_null() {
        return;
    }
    let Ok(arr) = timers_val.to_object(context) else {
        return;
    };
    let Ok(len_val) = arr.get(js_string!("length"), context) else {
        return;
    };
    let Ok(len) = len_val.to_u32(context) else {
        return;
    };

    for i in 0..len {
        let Ok(entry_val) = arr.get(i, context) else {
            continue;
        };
        let Ok(entry_obj) = entry_val.to_object(context) else {
            continue;
        };
        let Ok(entry_id_val) = entry_obj.get(js_string!("id"), context) else {
            continue;
        };
        let Ok(entry_id) = entry_id_val.to_u32(context) else {
            continue;
        };
        if entry_id == id {
            // Mark as cancelled by setting callback to undefined
            let _ = entry_obj.set(js_string!("callback"), JsValue::undefined(), false, context);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsRuntime;

    #[test]
    fn test_set_timeout_returns_id() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval("setTimeout(function() {}, 100)").unwrap();
        let id = result.as_number().unwrap();
        assert!(id > 0.0, "timer ID should be positive");
    }

    #[test]
    fn test_clear_timeout_does_not_panic() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        // Clear a timer — should not error
        let result = rt.eval("var id = setTimeout(function() {}, 100); clearTimeout(id);");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_interval_returns_id() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval("setInterval(function() {}, 50)").unwrap();
        let id = result.as_number().unwrap();
        assert!(id > 0.0, "timer ID should be positive");
    }

    #[test]
    fn request_animation_frame_returns_id() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval("requestAnimationFrame(function(ts) {})").unwrap();
        let id = result.as_number().unwrap();
        assert!(id > 0.0, "raf ID should be positive");
    }

    #[test]
    fn cancel_animation_frame_does_not_panic() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval(
            "var id = requestAnimationFrame(function(ts) {}); cancelAnimationFrame(id);",
        );
        assert!(result.is_ok());
    }
}
