//! Browser-chrome bridge for APIs that need an embedder decision.
//!
//! JavaScript runs on the same thread as the browser shell, so native dialogs
//! and clipboard access cannot synchronously block that thread. This bridge
//! returns real JavaScript promises and keeps their resolvers in the page
//! realm until browser chrome completes the request.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction, Source};

use crate::browser_request::{BrowserDialogKind, BrowserRequest, RequestQueue};

#[derive(Debug, Default)]
pub struct BrowserPromiseBroker {
    next_id: u64,
    pending: HashSet<u64>,
}

impl BrowserPromiseBroker {
    fn allocate(&mut self) -> u64 {
        self.next_id = self.next_id.saturating_add(1);
        self.pending.insert(self.next_id);
        self.next_id
    }

    pub fn settle(&mut self, id: u64) -> bool {
        self.pending.remove(&id)
    }

    pub fn cancel_all(&mut self) -> Vec<u64> {
        self.pending.drain().collect()
    }
}

pub type SharedBrowserPromiseBroker = Rc<RefCell<BrowserPromiseBroker>>;

pub fn new_browser_promise_broker() -> SharedBrowserPromiseBroker {
    Rc::new(RefCell::new(BrowserPromiseBroker::default()))
}

/// Values returned to JavaScript when a browser request completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserPromiseResult {
    Undefined,
    Bool(bool),
    String(String),
    Null,
    Error(String),
}

/// Install async dialog and clipboard APIs into an already-created window.
pub fn register(queue: &RequestQueue, broker: &SharedBrowserPromiseBroker, context: &mut Context) {
    install_promise_runtime(context);
    register_dialog_api(queue, broker, context);
    register_clipboard_api(queue, broker, context);

    // The standard synchronous dialog APIs cannot yield the native browser
    // event loop in this single-threaded architecture. Vigo deliberately
    // exposes promise-returning versions so pages can use `await confirm()` /
    // `await prompt()` without receiving fabricated answers.
    if let Err(error) = context.eval(Source::from_bytes(
        "window.alert = globalThis.__vigoAlert; window.confirm = globalThis.__vigoConfirm; \
         window.prompt = globalThis.__vigoPrompt; alert = window.alert; confirm = window.confirm; \
         prompt = window.prompt; window.navigator.clipboard = globalThis.__vigoClipboard;",
    )) {
        tracing::warn!(target: "vex_js::browser_bridge", error = %error, "failed to expose browser bridge APIs");
    }
}

/// Resolve or reject an outstanding page promise.
pub fn settle(
    broker: &SharedBrowserPromiseBroker,
    id: u64,
    result: BrowserPromiseResult,
    context: &mut Context,
) -> bool {
    if !broker.borrow_mut().settle(id) {
        return false;
    }

    let (ok, value) = match result {
        BrowserPromiseResult::Undefined => (true, "undefined".to_owned()),
        BrowserPromiseResult::Bool(value) => (true, value.to_string()),
        BrowserPromiseResult::String(value) => (
            true,
            serde_json::to_string(&value).expect("strings always serialize"),
        ),
        BrowserPromiseResult::Null => (true, "null".to_owned()),
        BrowserPromiseResult::Error(message) => (
            false,
            serde_json::to_string(&message).expect("strings always serialize"),
        ),
    };
    let source = format!("globalThis.__vigoBrowserSettle({id}, {ok}, {value});");
    if let Err(error) = context.eval(Source::from_bytes(source.as_bytes())) {
        tracing::warn!(target: "vex_js::browser_bridge", id, error = %error, "failed to settle browser promise");
        return false;
    }
    // Promise reactions are scheduled as Boa jobs, not as Vex's custom
    // `queueMicrotask` callbacks. Run them immediately so the continuation
    // can enqueue its next browser request during this browser-loop tick.
    context.run_jobs();
    true
}

pub fn cancel_all(
    broker: &SharedBrowserPromiseBroker,
    context: &mut Context,
    message: &str,
) -> usize {
    let ids = broker.borrow_mut().cancel_all();
    let count = ids.len();
    for id in ids {
        let _ = settle_untracked(id, BrowserPromiseResult::Error(message.to_owned()), context);
    }
    count
}

fn settle_untracked(id: u64, result: BrowserPromiseResult, context: &mut Context) -> bool {
    let (ok, value) = match result {
        BrowserPromiseResult::Undefined => (true, "undefined".to_owned()),
        BrowserPromiseResult::Bool(value) => (true, value.to_string()),
        BrowserPromiseResult::String(value) => (
            true,
            serde_json::to_string(&value).expect("strings always serialize"),
        ),
        BrowserPromiseResult::Null => (true, "null".to_owned()),
        BrowserPromiseResult::Error(message) => (
            false,
            serde_json::to_string(&message).expect("strings always serialize"),
        ),
    };
    context
        .eval(Source::from_bytes(
            format!("globalThis.__vigoBrowserSettle({id}, {ok}, {value});").as_bytes(),
        ))
        .is_ok()
}

fn install_promise_runtime(context: &mut Context) {
    if let Err(error) = context.eval(Source::from_bytes(
        "(() => {\n\
           const pending = Object.create(null);\n\
           globalThis.__vigoBrowserMakePending = (id) => new Promise((resolve, reject) => {\n\
             pending[id] = { resolve, reject };\n\
           });\n\
           globalThis.__vigoBrowserSettle = (id, ok, value) => {\n\
             const entry = pending[id];\n\
             if (!entry) return false;\n\
             delete pending[id];\n\
             if (ok) entry.resolve(value); else entry.reject(new Error(value));\n\
             return true;\n\
           };\n\
         })();",
    )) {
        tracing::warn!(target: "vex_js::browser_bridge", error = %error, "failed to initialize browser promise runtime");
    }
}

fn new_pending_promise(
    broker: &SharedBrowserPromiseBroker,
    context: &mut Context,
) -> JsResult<(u64, JsValue)> {
    let id = broker.borrow_mut().allocate();
    let promise = context
        .eval(Source::from_bytes(
            format!("globalThis.__vigoBrowserMakePending({id})").as_bytes(),
        ))
        .map_err(|error| {
            JsNativeError::error()
                .with_message(format!("failed to create browser promise: {error}"))
        })?;
    Ok((id, promise))
}

fn register_dialog_api(
    queue: &RequestQueue,
    broker: &SharedBrowserPromiseBroker,
    context: &mut Context,
) {
    let mut make_dialog =
        |name: &'static str, make_kind: fn(String, Option<String>) -> BrowserDialogKind| {
            let queue = queue.clone();
            let broker = broker.clone();
            // SAFETY: all captured state is single-threaded JS runtime state.
            let callback = unsafe {
                NativeFunction::from_closure(move |_this, args, context| {
                    let message = args
                        .first()
                        .map(|value| value.to_string(context).map(|s| s.to_std_string_escaped()))
                        .transpose()?
                        .unwrap_or_default();
                    let default = args
                        .get(1)
                        .filter(|value| !value.is_undefined())
                        .map(|value| value.to_string(context).map(|s| s.to_std_string_escaped()))
                        .transpose()?;
                    let (id, promise) = new_pending_promise(&broker, context)?;
                    queue.borrow_mut().push(BrowserRequest::Dialog {
                        id,
                        kind: make_kind(message, default),
                    });
                    Ok(promise)
                })
            };
            if let Err(error) = context.register_global_callable(js_string!(name), 1, callback) {
                tracing::warn!(target: "vex_js::browser_bridge", name, error = %error, "failed to register dialog API");
            }
        };

    make_dialog("__vigoAlert", |message, _| {
        BrowserDialogKind::Alert(message)
    });
    make_dialog("__vigoConfirm", |message, _| {
        BrowserDialogKind::Confirm(message)
    });
    make_dialog("__vigoPrompt", |message, default| {
        BrowserDialogKind::Prompt { message, default }
    });
}

fn register_clipboard_api(
    queue: &RequestQueue,
    broker: &SharedBrowserPromiseBroker,
    context: &mut Context,
) {
    let read_queue = queue.clone();
    let read_broker = broker.clone();
    let read = unsafe {
        NativeFunction::from_closure(move |_this, _args, context| {
            let (id, promise) = new_pending_promise(&read_broker, context)?;
            read_queue
                .borrow_mut()
                .push(BrowserRequest::ClipboardRead { id });
            Ok(promise)
        })
    };
    let write_queue = queue.clone();
    let write_broker = broker.clone();
    let write = unsafe {
        NativeFunction::from_closure(move |_this, args, context| {
            let text = args
                .first()
                .unwrap_or(&JsValue::undefined())
                .to_string(context)?
                .to_std_string_escaped();
            let (id, promise) = new_pending_promise(&write_broker, context)?;
            write_queue
                .borrow_mut()
                .push(BrowserRequest::ClipboardWrite { id, text });
            Ok(promise)
        })
    };

    let clipboard = boa_engine::object::ObjectInitializer::new(context)
        .function(read, js_string!("readText"), 0)
        .function(write, js_string!("writeText"), 1)
        .build();
    if let Err(error) = context.register_global_property(
        js_string!("__vigoClipboard"),
        clipboard,
        boa_engine::property::Attribute::CONFIGURABLE,
    ) {
        tracing::warn!(target: "vex_js::browser_bridge", error = %error, "failed to register clipboard API");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_request_queue;

    #[test]
    fn confirm_returns_pending_promise_and_resumes_after_response() {
        let queue = new_request_queue();
        let broker = new_browser_promise_broker();
        let mut context = Context::default();
        crate::api::window::register_with_queue(&queue, &mut context);
        register(&queue, &broker, &mut context);

        context
            .eval(Source::from_bytes(
                "let result; confirm('continue?').then(v => result = v);",
            ))
            .expect("dialog request should execute");
        let requests: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert!(
            matches!(requests.as_slice(), [BrowserRequest::Dialog { kind: BrowserDialogKind::Confirm(message), .. }] if message == "continue?")
        );
        let id = match &requests[0] {
            BrowserRequest::Dialog { id, .. } => *id,
            _ => unreachable!(),
        };
        assert!(settle(
            &broker,
            id,
            BrowserPromiseResult::Bool(true),
            &mut context
        ));
        context.run_jobs();
        let value = context.eval(Source::from_bytes("result")).unwrap();
        assert!(value.as_boolean().unwrap());
    }

    #[test]
    fn prompt_and_clipboard_requests_keep_their_result_channels_separate() {
        let queue = new_request_queue();
        let broker = new_browser_promise_broker();
        let mut context = Context::default();
        crate::api::window::register_with_queue(&queue, &mut context);
        register(&queue, &broker, &mut context);

        context
            .eval(Source::from_bytes(
                "let answer; let copied; prompt('name', 'Vigo').then(v => answer = v); navigator.clipboard.readText().then(v => copied = v);",
            ))
            .expect("bridge requests should execute");
        let requests: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(requests.len(), 2);
        let prompt_id = match &requests[0] {
            BrowserRequest::Dialog {
                id,
                kind:
                    BrowserDialogKind::Prompt {
                        message,
                        default: Some(default),
                    },
            } => {
                assert_eq!(message, "name");
                assert_eq!(default, "Vigo");
                *id
            }
            _ => panic!("expected prompt request"),
        };
        let clipboard_id = match &requests[1] {
            BrowserRequest::ClipboardRead { id } => *id,
            _ => panic!("expected clipboard request"),
        };
        assert!(settle(
            &broker,
            prompt_id,
            BrowserPromiseResult::String("Ada".to_owned()),
            &mut context,
        ));
        assert!(settle(
            &broker,
            clipboard_id,
            BrowserPromiseResult::String("copied text".to_owned()),
            &mut context,
        ));
        context.run_jobs();
        let values = context
            .eval(Source::from_bytes("[answer, copied].join('|')"))
            .unwrap();
        assert_eq!(
            values.as_string().unwrap().to_std_string_escaped(),
            "Ada|copied text"
        );
    }
}
