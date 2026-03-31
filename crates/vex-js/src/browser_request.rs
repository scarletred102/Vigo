// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Browser request queue — messages from JavaScript to the browser loop.
//!
//! JS APIs (like `location.assign()`, `history.back()`, `alert()`) push
//! requests into a shared queue. The browser event loop drains the queue
//! each tick and dispatches to `TabManager` / navigation / UI.
//!
//! The queue uses `Rc<RefCell<Vec<...>>>` because Boa JS is single-threaded
//! and the browser loop runs on the same thread.

use std::cell::RefCell;
use std::rc::Rc;

/// A request from JavaScript to the browser shell.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserRequest {
    /// `location.assign(url)` — navigate to a new URL.
    Navigate(String),
    /// `location.reload()` — re-fetch the current page.
    Reload,
    /// `history.back()` — go to the previous history entry.
    Back,
    /// `history.forward()` — go to the next history entry.
    Forward,
    /// `history.pushState(state, title, url)` — add a history entry without navigation.
    PushState {
        /// Optional URL for the new history entry.
        url: Option<String>,
    },
    /// `alert(message)` — show a message to the user.
    Alert(String),
    /// `console.*()` entry for the DevTools console panel.
    ConsoleLog {
        /// Log level: "log", "info", "warn", "error", "debug".
        level: String,
        /// Formatted message text.
        message: String,
    },
    /// `vigo.runtime.sendMessage(...)` from an extension content script.
    ExtensionSendMessage {
        /// Sender extension ID (captured from the currently executing script).
        from_extension_id: String,
        /// Optional recipient extension ID (`None` means broadcast).
        target_extension_id: Option<String>,
        /// Serialized payload string (JSON when possible).
        payload: String,
    },
}

/// Shared queue of browser requests from JavaScript.
///
/// Clone this to share between JS closures and the browser loop.
pub type RequestQueue = Rc<RefCell<Vec<BrowserRequest>>>;

/// Create a new empty request queue.
pub fn new_request_queue() -> RequestQueue {
    Rc::new(RefCell::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_push_and_drain() {
        let queue = new_request_queue();
        queue
            .borrow_mut()
            .push(BrowserRequest::Navigate("https://example.com".into()));
        queue.borrow_mut().push(BrowserRequest::Reload);
        queue.borrow_mut().push(BrowserRequest::Back);

        let drained: Vec<BrowserRequest> = queue.borrow_mut().drain(..).collect();
        assert_eq!(drained.len(), 3);
        assert_eq!(
            drained[0],
            BrowserRequest::Navigate("https://example.com".into())
        );
        assert_eq!(drained[1], BrowserRequest::Reload);
        assert_eq!(drained[2], BrowserRequest::Back);
        assert!(queue.borrow().is_empty());
    }

    #[test]
    fn alert_request() {
        let queue = new_request_queue();
        queue
            .borrow_mut()
            .push(BrowserRequest::Alert("Hello!".into()));
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(reqs[0], BrowserRequest::Alert("Hello!".into()));
    }

    #[test]
    fn push_state_with_url() {
        let queue = new_request_queue();
        queue.borrow_mut().push(BrowserRequest::PushState {
            url: Some("/new-page".into()),
        });
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        assert_eq!(
            reqs[0],
            BrowserRequest::PushState {
                url: Some("/new-page".into())
            }
        );
    }

    #[test]
    fn console_log_request() {
        let queue = new_request_queue();
        queue.borrow_mut().push(BrowserRequest::ConsoleLog {
            level: "warn".into(),
            message: "test warning".into(),
        });
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        match &reqs[0] {
            BrowserRequest::ConsoleLog { level, message } => {
                assert_eq!(level, "warn");
                assert_eq!(message, "test warning");
            }
            _ => panic!("expected ConsoleLog"),
        }
    }

    #[test]
    fn extension_send_message_request() {
        let queue = new_request_queue();
        queue
            .borrow_mut()
            .push(BrowserRequest::ExtensionSendMessage {
                from_extension_id: "ext-a".into(),
                target_extension_id: Some("ext-b".into()),
                payload: "{\"kind\":\"ping\"}".into(),
            });
        let reqs: Vec<_> = queue.borrow_mut().drain(..).collect();
        match &reqs[0] {
            BrowserRequest::ExtensionSendMessage {
                from_extension_id,
                target_extension_id,
                payload,
            } => {
                assert_eq!(from_extension_id, "ext-a");
                assert_eq!(target_extension_id.as_deref(), Some("ext-b"));
                assert_eq!(payload, "{\"kind\":\"ping\"}");
            }
            _ => panic!("expected ExtensionSendMessage"),
        }
    }
}
