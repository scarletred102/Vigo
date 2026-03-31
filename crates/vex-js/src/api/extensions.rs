// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension runtime API (`vigo.runtime`) for content scripts.
//!
//! Current surface:
//! - `vigo.runtime.onMessage.addListener(callback)`
//! - `vigo.runtime.sendMessage(message)` (broadcast)
//! - `vigo.runtime.sendMessage(targetExtensionId, message)` (direct)

use boa_engine::object::builtins::{JsArray, JsFunction};
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsValue, NativeFunction, Source};

use crate::browser_request::{BrowserRequest, RequestQueue};

const RUNTIME_BOOTSTRAP: &str = r#"
(() => {
  if (!globalThis.__vigoRuntimeState) {
    globalThis.__vigoRuntimeState = { listeners: [] };
  } else if (!Array.isArray(globalThis.__vigoRuntimeState.listeners)) {
    globalThis.__vigoRuntimeState.listeners = [];
  }

  globalThis.__vigoDispatchMessage = function(targetExtensionId, payloadText, senderExtensionId) {
    const state = globalThis.__vigoRuntimeState || { listeners: [] };
    const listeners = Array.isArray(state.listeners) ? state.listeners : [];
    let payload = payloadText;
    if (typeof payloadText === "string") {
      try {
        payload = JSON.parse(payloadText);
      } catch (_) {}
    }
    const sender = { id: senderExtensionId || "" };
    for (let i = 0; i < listeners.length; i += 1) {
      const entry = listeners[i];
      if (!entry || entry.extensionId !== targetExtensionId || typeof entry.callback !== "function") {
        continue;
      }
      try {
        entry.callback(payload, sender);
      } catch (err) {
        try { console.error("vigo.runtime.onMessage listener error", err); } catch (_) {}
      }
    }
  };
})();
"#;

/// Register `vigo.runtime` on the JS global.
pub fn register_with_queue(queue: &RequestQueue, context: &mut Context) {
    if let Err(e) = context.eval(Source::from_bytes(RUNTIME_BOOTSTRAP)) {
        tracing::warn!(target: "vex_js::extensions", "runtime bootstrap failed: {e}");
    }

    let on_message_add = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let cb_val = args.first().ok_or_else(|| {
                JsNativeError::typ()
                    .with_message("vigo.runtime.onMessage.addListener requires callback")
            })?;
            let cb_obj = cb_val
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;
            let _func = JsFunction::from_object(cb_obj.clone())
                .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;

            let extension_id = current_extension_id(this, ctx).ok_or_else(|| {
                JsNativeError::typ().with_message(
                    "vigo.runtime.onMessage.addListener must run inside an extension context",
                )
            })?;

            let global = ctx.global_object();
            let state_val = global.get(js_string!("__vigoRuntimeState"), ctx)?;
            let state_obj = state_val.to_object(ctx)?;
            let listeners_val = state_obj.get(js_string!("listeners"), ctx)?;
            let listeners_obj = listeners_val.to_object(ctx)?;
            let listeners = JsArray::from_object(listeners_obj).map_err(|_| {
                JsNativeError::typ().with_message("runtime listeners must be an array")
            })?;

            let entry = ObjectInitializer::new(ctx)
                .property(
                    js_string!("extensionId"),
                    JsValue::from(js_string!(extension_id.as_str())),
                    Attribute::all(),
                )
                .property(js_string!("callback"), cb_val.clone(), Attribute::all())
                .build();
            listeners.push(entry, ctx)?;
            Ok(JsValue::undefined())
        })
    };

    let q_send = queue.clone();
    let send_message = unsafe {
        NativeFunction::from_closure(move |this, args, ctx| {
            let from_extension_id = current_extension_id(this, ctx).ok_or_else(|| {
                JsNativeError::typ()
                    .with_message("vigo.runtime.sendMessage must run inside an extension context")
            })?;

            let (target_extension_id, payload_value) = if args.len() >= 2 {
                let target = args[0]
                    .to_string(ctx)?
                    .to_std_string_escaped()
                    .trim()
                    .to_owned();
                if target.is_empty() {
                    (
                        None,
                        args.get(1).cloned().unwrap_or_else(JsValue::undefined),
                    )
                } else {
                    (
                        Some(target),
                        args.get(1).cloned().unwrap_or_else(JsValue::undefined),
                    )
                }
            } else {
                (
                    None,
                    args.first().cloned().unwrap_or_else(JsValue::undefined),
                )
            };

            let payload = serialize_payload(&payload_value, ctx);

            q_send
                .borrow_mut()
                .push(BrowserRequest::ExtensionSendMessage {
                    from_extension_id,
                    target_extension_id,
                    payload,
                });

            Ok(JsValue::undefined())
        })
    };

    let on_message = ObjectInitializer::new(context)
        .function(on_message_add, js_string!("addListener"), 1)
        .build();
    let runtime = ObjectInitializer::new(context)
        .property(js_string!("onMessage"), on_message, Attribute::all())
        .function(send_message, js_string!("sendMessage"), 2)
        .build();
    let vigo = ObjectInitializer::new(context)
        .property(js_string!("runtime"), runtime, Attribute::all())
        .build();

    if let Err(error) = context.register_global_property(
        js_string!("vigo"),
        vigo,
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    ) {
        tracing::warn!(target: "vex_js::extensions", "failed to register vigo global: {error}");
    }
}

/// Set current extension context for subsequent script execution.
pub fn set_current_extension(extension_id: Option<&str>, context: &mut Context) {
    let value = extension_id
        .map(|id| JsValue::from(js_string!(id)))
        .unwrap_or_else(JsValue::undefined);
    let global = context.global_object();
    let _ = global.set(
        js_string!("__vigo_current_extension"),
        value,
        false,
        context,
    );
}

/// Dispatch a runtime message to listeners of a specific extension.
pub fn dispatch_runtime_message(
    context: &mut Context,
    target_extension_id: &str,
    from_extension_id: &str,
    payload: &str,
) -> Result<(), String> {
    let global = context.global_object();
    let dispatcher = global
        .get(js_string!("__vigoDispatchMessage"), context)
        .map_err(|e| format!("failed to read runtime dispatcher: {e}"))?;
    let dispatcher_obj = dispatcher
        .as_object()
        .ok_or_else(|| "runtime dispatcher is not callable".to_owned())?;
    let dispatcher_fn = JsFunction::from_object(dispatcher_obj.clone())
        .ok_or_else(|| "runtime dispatcher is not a function".to_owned())?;
    dispatcher_fn
        .call(
            &JsValue::undefined(),
            &[
                JsValue::from(js_string!(target_extension_id)),
                JsValue::from(js_string!(payload)),
                JsValue::from(js_string!(from_extension_id)),
            ],
            context,
        )
        .map_err(|e| format!("runtime dispatch failed: {e}"))?;
    Ok(())
}

fn current_extension_id(this: &JsValue, context: &mut Context) -> Option<String> {
    if let Some(this_obj) = this.as_object() {
        if let Ok(value) = this_obj.get(js_string!("__vigoExtensionId"), context) {
            if !(value.is_undefined() || value.is_null()) {
                if let Ok(s) = value.to_string(context) {
                    let ext = s.to_std_string_escaped();
                    if !ext.trim().is_empty() {
                        return Some(ext);
                    }
                }
            }
        }
    }

    let global = context.global_object();
    let value = global
        .get(js_string!("__vigo_current_extension"), context)
        .ok()?;
    if value.is_undefined() || value.is_null() {
        return None;
    }
    let s = value.to_string(context).ok()?.to_std_string_escaped();
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn serialize_payload(value: &JsValue, context: &mut Context) -> String {
    if value.is_undefined() || value.is_null() {
        return "null".to_owned();
    }
    if let Some(s) = value.as_string() {
        return s.to_std_string_escaped();
    }

    let global = context.global_object();
    if let Ok(json_val) = global.get(js_string!("JSON"), context) {
        if let Ok(json_obj) = json_val.to_object(context) {
            if let Ok(stringify_val) = json_obj.get(js_string!("stringify"), context) {
                if let Some(func) = stringify_val.as_callable() {
                    if let Ok(serialized) =
                        func.call(&JsValue::undefined(), &[value.clone()], context)
                    {
                        if let Some(s) = serialized.as_string() {
                            return s.to_std_string_escaped();
                        }
                    }
                }
            }
        }
    }

    value
        .to_string(context)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|_| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{dispatch_runtime_message, set_current_extension};
    use crate::browser_request::BrowserRequest;
    use crate::JsRuntime;

    #[test]
    fn send_message_enqueues_extension_request() {
        let mut rt = JsRuntime::new();
        set_current_extension(Some("ext-a"), rt.context_mut());
        rt.execute("vigo.runtime.sendMessage({ kind: 'ping' })")
            .expect("sendMessage should run");
        let reqs: Vec<_> = rt.request_queue().borrow_mut().drain(..).collect();
        let ext_req = reqs
            .into_iter()
            .find(|r| matches!(r, BrowserRequest::ExtensionSendMessage { .. }))
            .expect("extension request should be present");
        match ext_req {
            BrowserRequest::ExtensionSendMessage {
                from_extension_id,
                target_extension_id,
                payload,
            } => {
                assert_eq!(from_extension_id, "ext-a");
                assert!(target_extension_id.is_none());
                assert!(payload.contains("\"kind\":\"ping\""));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn dispatch_runtime_message_invokes_matching_listener() {
        let mut rt = JsRuntime::new();
        set_current_extension(Some("ext-a"), rt.context_mut());
        rt.execute(
            "globalThis.__received = '';
             vigo.runtime.onMessage.addListener((msg, sender) => {
               globalThis.__received = msg.action + ':' + sender.id;
             });",
        )
        .expect("listener registration should succeed");

        dispatch_runtime_message(
            rt.context_mut(),
            "ext-a",
            "ext-b",
            "{\"action\":\"toggle\"}",
        )
        .expect("dispatch should succeed");

        let value = rt
            .eval("globalThis.__received")
            .expect("readback should succeed")
            .to_string(rt.context_mut())
            .expect("string conversion should succeed")
            .to_std_string_escaped();
        assert_eq!(value, "toggle:ext-b");
    }
}
