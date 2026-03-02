// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `console` global object — `console.log`, `console.warn`, `console.error`, etc.
//!
//! Each method formats its arguments and forwards to [`tracing`] at the
//! appropriate level.

use boa_engine::object::ObjectInitializer;
use boa_engine::{Context, JsValue, NativeFunction};

use crate::browser_request::{BrowserRequest, RequestQueue};

/// Format JS arguments into a single display string.
///
/// Mimics browser `console.log` formatting: values are space-separated,
/// strings are unquoted, other types use their default JS stringification.
fn format_args(args: &[JsValue], context: &mut Context) -> String {
    args.iter()
        .map(|v| {
            if v.is_undefined() {
                "undefined".to_owned()
            } else if v.is_null() {
                "null".to_owned()
            } else if let Some(s) = v.as_string() {
                s.to_std_string_escaped()
            } else if let Some(n) = v.as_number() {
                if n.fract() == 0.0 && n.abs() < (i64::MAX as f64) {
                    format!("{}", n as i64)
                } else {
                    format!("{n}")
                }
            } else if let Some(b) = v.as_boolean() {
                format!("{b}")
            } else {
                // Fallback: use JS toString()
                v.to_string(context)
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|_| "[object]".to_owned())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Helper that logs to `tracing` AND pushes a `ConsoleLog` to the request queue.
fn emit(queue: &RequestQueue, level: &str, msg: &str) {
    match level {
        "warn" => tracing::warn!(target: "vex_js::console", "{msg}"),
        "error" => tracing::error!(target: "vex_js::console", "{msg}"),
        "debug" => tracing::debug!(target: "vex_js::console", "{msg}"),
        _ => tracing::info!(target: "vex_js::console", "{msg}"),
    }
    queue.borrow_mut().push(BrowserRequest::ConsoleLog {
        level: level.to_owned(),
        message: msg.to_owned(),
    });
}

/// Register the `console` global object on the given context.
///
/// Installs: `console.log`, `console.warn`, `console.error`,
/// `console.info`, `console.debug`.
///
/// Each method pushes a [`BrowserRequest::ConsoleLog`] to the shared
/// request queue so DevTools can display entries.
pub fn register(queue: &RequestQueue, context: &mut Context) {
    let q_log = queue.clone();
    // SAFETY: Closure captures Rc<RefCell<>> and runs on the single JS thread.
    let log_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let msg = format_args(args, ctx);
            emit(&q_log, "log", &msg);
            Ok(JsValue::undefined())
        })
    };

    let q_warn = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let warn_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let msg = format_args(args, ctx);
            emit(&q_warn, "warn", &msg);
            Ok(JsValue::undefined())
        })
    };

    let q_err = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let error_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let msg = format_args(args, ctx);
            emit(&q_err, "error", &msg);
            Ok(JsValue::undefined())
        })
    };

    let q_info = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let info_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let msg = format_args(args, ctx);
            emit(&q_info, "info", &msg);
            Ok(JsValue::undefined())
        })
    };

    let q_debug = queue.clone();
    // SAFETY: Same single-thread guarantee.
    let debug_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let msg = format_args(args, ctx);
            emit(&q_debug, "debug", &msg);
            Ok(JsValue::undefined())
        })
    };

    let console = ObjectInitializer::new(context)
        .function(log_fn, js_string!("log"), 0)
        .function(warn_fn, js_string!("warn"), 0)
        .function(error_fn, js_string!("error"), 0)
        .function(info_fn, js_string!("info"), 0)
        .function(debug_fn, js_string!("debug"), 0)
        .build();

    if let Err(error) = context.register_global_property(
        js_string!("console"),
        console,
        boa_engine::property::Attribute::all(),
    ) {
        tracing::error!(target: "vex_js::console", "failed to register console global: {error}");
    }
}

// Bring the js_string! macro into scope.
use boa_engine::js_string;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_request::new_request_queue;
    use crate::JsRuntime;

    #[test]
    fn test_console_log_formats_string() {
        let mut rt = JsRuntime::new();
        // Should not panic — the call goes to tracing which is a no-op without a subscriber
        let result = rt.execute("console.log('hello', 'world')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_log_multiple_arg_types() {
        let mut rt = JsRuntime::new();
        let result = rt.execute("console.log(42, true, null, undefined, 'text')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_log_non_string_args() {
        let mut rt = JsRuntime::new();
        // Objects, arrays
        let result = rt.execute("console.log({a: 1}, [1,2,3])");
        assert!(result.is_ok());
    }

    #[test]
    fn console_log_pushes_to_queue() {
        let queue = new_request_queue();
        let mut rt = JsRuntime::with_request_queue(queue.clone());
        rt.execute("console.log('hello', 42)").unwrap();
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        // Find the ConsoleLog entry (there may also be other entries from window init)
        let console_entries: Vec<_> = reqs
            .into_iter()
            .filter(|r| matches!(r, BrowserRequest::ConsoleLog { .. }))
            .collect();
        assert!(!console_entries.is_empty());
        if let BrowserRequest::ConsoleLog { level, message } = &console_entries[0] {
            assert_eq!(level, "log");
            assert!(message.contains("hello"));
        }
    }

    #[test]
    fn console_error_pushes_error_level() {
        let queue = new_request_queue();
        let mut rt = JsRuntime::with_request_queue(queue.clone());
        rt.execute("console.error('oops')").unwrap();
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        let console_entries: Vec<_> = reqs
            .into_iter()
            .filter(|r| matches!(r, BrowserRequest::ConsoleLog { .. }))
            .collect();
        assert!(!console_entries.is_empty());
        if let BrowserRequest::ConsoleLog { level, message } = &console_entries[0] {
            assert_eq!(level, "error");
            assert!(message.contains("oops"));
        }
    }

    #[test]
    fn console_warn_pushes_warn_level() {
        let queue = new_request_queue();
        let mut rt = JsRuntime::with_request_queue(queue.clone());
        rt.execute("console.warn('caution')").unwrap();
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        let console_entries: Vec<_> = reqs
            .into_iter()
            .filter(|r| matches!(r, BrowserRequest::ConsoleLog { .. }))
            .collect();
        assert!(!console_entries.is_empty());
        if let BrowserRequest::ConsoleLog { level, .. } = &console_entries[0] {
            assert_eq!(level, "warn");
        }
    }
}
