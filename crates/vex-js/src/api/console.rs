// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `console` global object — `console.log`, `console.warn`, `console.error`, etc.
//!
//! Each method formats its arguments and forwards to [`tracing`] at the
//! appropriate level.

use boa_engine::object::ObjectInitializer;
use boa_engine::{Context, JsValue, NativeFunction};

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

/// Register the `console` global object on the given context.
///
/// Installs: `console.log`, `console.warn`, `console.error`,
/// `console.info`, `console.debug`.
pub fn register(context: &mut Context) {
    let console = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(console_log),
            js_string!("log"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(console_warn),
            js_string!("warn"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(console_error),
            js_string!("error"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(console_info),
            js_string!("info"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(console_debug),
            js_string!("debug"),
            0,
        )
        .build();

    if let Err(error) = context.register_global_property(
        js_string!("console"),
        console,
        boa_engine::property::Attribute::all(),
    ) {
        tracing::error!(target: "vex_js::console", "failed to register console global: {error}");
    }
}

/// `console.log(...args)` → `tracing::info!`
fn console_log(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let msg = format_args(args, context);
    tracing::info!(target: "vex_js::console", "{msg}");
    Ok(JsValue::undefined())
}

/// `console.warn(...args)` → `tracing::warn!`
fn console_warn(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let msg = format_args(args, context);
    tracing::warn!(target: "vex_js::console", "{msg}");
    Ok(JsValue::undefined())
}

/// `console.error(...args)` → `tracing::error!`
fn console_error(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let msg = format_args(args, context);
    tracing::error!(target: "vex_js::console", "{msg}");
    Ok(JsValue::undefined())
}

/// `console.info(...args)` → `tracing::info!`
fn console_info(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let msg = format_args(args, context);
    tracing::info!(target: "vex_js::console", "{msg}");
    Ok(JsValue::undefined())
}

/// `console.debug(...args)` → `tracing::debug!`
fn console_debug(
    _: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> boa_engine::JsResult<JsValue> {
    let msg = format_args(args, context);
    tracing::debug!(target: "vex_js::console", "{msg}");
    Ok(JsValue::undefined())
}

// Bring the js_string! macro into scope.
use boa_engine::js_string;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsRuntime;

    #[test]
    fn test_console_log_formats_string() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        // Should not panic — the call goes to tracing which is a no-op without a subscriber
        let result = rt.execute("console.log('hello', 'world')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_log_multiple_arg_types() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.execute("console.log(42, true, null, undefined, 'text')");
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_log_non_string_args() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        // Objects, arrays
        let result = rt.execute("console.log({a: 1}, [1,2,3])");
        assert!(result.is_ok());
    }
}
