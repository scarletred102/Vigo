// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `fetch()` global function — wraps `vex_net::HttpClient` for JavaScript.
//!
//! Returns a JS `Promise` that resolves with a `Response` object containing
//! `.status`, `.ok`, `.text()`, `.json()`, and `.headers`.
//!
//! The network call blocks on a short-lived tokio runtime so the Promise
//! resolves synchronously in this initial implementation.

use boa_engine::object::builtins::JsPromise;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

/// Register the global `fetch(url)` function.
pub fn register(context: &mut Context) {
    context
        .register_global_callable(
            js_string!("fetch"),
            1,
            NativeFunction::from_fn_ptr(fetch_fn),
        )
        .expect("register fetch");
}

/// `fetch(url)` → `Promise<Response>`
fn fetch_fn(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url_str = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("fetch requires a URL argument"))?
        .to_string(context)?
        .to_std_string_escaped();

    let fetch_result = perform_fetch(&url_str);

    match fetch_result {
        Ok(result) => {
            let response_obj =
                build_response_object(result.status, &result.body, &result.headers, context);
            let promise = JsPromise::resolve(response_obj, context);
            Ok(JsValue::from(promise))
        }
        Err(err_msg) => {
            let error = JsNativeError::typ().with_message(err_msg);
            let promise = JsPromise::reject(error, context);
            Ok(JsValue::from(promise))
        }
    }
}

/// Result of a synchronous HTTP fetch.
struct FetchResult {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}

/// Perform the actual HTTP fetch synchronously.
fn perform_fetch(url: &str) -> Result<FetchResult, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to create runtime: {e}"))?;

    rt.block_on(async {
        let client = vex_net::HttpClient::new().map_err(|e| format!("client error: {e}"))?;

        let request = vex_net::Request::get(url).map_err(|e| format!("invalid URL: {e}"))?;

        let response: vex_net::Response = client
            .fetch(request)
            .await
            .map_err(|e| format!("fetch failed: {e}"))?;

        let status = response.status;
        let headers: Vec<(String, String)> = response
            .headers
            .iter()
            .map(|(k, v): (&String, &String)| (k.clone(), v.clone()))
            .collect();
        let body = response.text().unwrap_or("").to_owned();

        Ok(FetchResult {
            status,
            body,
            headers,
        })
    })
}

/// Build a JS Response-like object: `{ status, ok, headers, text(), json() }`.
fn build_response_object(
    status: u16,
    body: &str,
    headers: &[(String, String)],
    context: &mut Context,
) -> JsValue {
    let body_for_text = body.to_owned();
    let body_for_json = body.to_owned();

    // Build headers object
    let headers_obj = {
        let mut builder = ObjectInitializer::new(context);
        for (key, value) in headers {
            builder.property(
                js_string!(key.as_str()),
                JsValue::from(js_string!(value.as_str())),
                Attribute::all(),
            );
        }
        builder.build()
    };

    // text() — returns the body as a resolved Promise<string>
    // SAFETY: from_closure captures a non-Copy String. Safe because the
    // String is moved into the closure and JS is single-threaded.
    let text_fn = unsafe {
        NativeFunction::from_closure(move |_, _, context| {
            let val = JsValue::from(js_string!(body_for_text.as_str()));
            let promise = JsPromise::resolve(val, context);
            Ok(JsValue::from(promise))
        })
    };

    // json() — parses body as JSON and returns a resolved Promise
    // SAFETY: same reasoning as text_fn above.
    let json_fn = unsafe {
        NativeFunction::from_closure(move |_, _, context| {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&body_for_json);
            match parsed {
                Ok(json_val) => {
                    let js_val = JsValue::from_json(&json_val, context).map_err(|e| {
                        JsNativeError::typ().with_message(format!("JSON conversion error: {e}"))
                    })?;
                    let promise = JsPromise::resolve(js_val, context);
                    Ok(JsValue::from(promise))
                }
                Err(e) => {
                    let error =
                        JsNativeError::syntax().with_message(format!("JSON parse error: {e}"));
                    let promise = JsPromise::reject(error, context);
                    Ok(JsValue::from(promise))
                }
            }
        })
    };

    let mut builder = ObjectInitializer::new(context);
    builder.property(
        js_string!("status"),
        JsValue::from(status),
        Attribute::READONLY,
    );
    builder.property(
        js_string!("ok"),
        JsValue::from((200..300).contains(&status)),
        Attribute::READONLY,
    );
    builder.property(
        js_string!("headers"),
        JsValue::from(headers_obj),
        Attribute::READONLY,
    );
    builder.function(text_fn, js_string!("text"), 0);
    builder.function(json_fn, js_string!("json"), 0);
    JsValue::from(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsRuntime;

    #[test]
    fn test_fetch_is_function() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval("typeof fetch").unwrap();
        assert_eq!(
            result.as_string().unwrap().to_std_string_escaped(),
            "function"
        );
    }

    #[test]
    #[ignore] // Requires network
    fn test_fetch_returns_response() {
        let mut rt = JsRuntime::new();
        register(rt.context_mut());

        let result = rt.eval(
            r#"
            var p = fetch('https://httpbin.org/get');
            typeof p
            "#,
        );
        assert!(result.is_ok());
    }
}
