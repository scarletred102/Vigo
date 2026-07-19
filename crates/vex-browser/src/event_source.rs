// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Server-Sent Events (EventSource) API.
//!
//! Provides a client for receiving server-pushed events over HTTP.
//! The protocol follows the W3C Server-Sent Events specification.

use std::collections::HashMap;

// ── Types ────────────────────────────────────────────────────────────────────

/// EventSource ready state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EventSourceState {
    /// Connecting to the server.
    #[default]
    Connecting,
    /// Connection is open and receiving events.
    Open,
    /// Connection is closed.
    Closed,
}

impl EventSourceState {
    /// The numeric value per the spec.
    pub fn value(&self) -> u16 {
        match self {
            Self::Connecting => 0,
            Self::Open => 1,
            Self::Closed => 2,
        }
    }
}

/// A parsed SSE event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The event type (defaults to "message").
    pub event_type: String,
    /// The event data.
    pub data: String,
    /// The last event ID.
    pub last_event_id: String,
    /// Retry interval in milliseconds (if sent by server).
    pub retry: Option<u64>,
}

impl Default for SseEvent {
    fn default() -> Self {
        Self {
            event_type: "message".to_string(),
            data: String::new(),
            last_event_id: String::new(),
            retry: None,
        }
    }
}

/// EventSource configuration.
#[derive(Debug, Clone)]
pub struct EventSourceInit {
    /// The URL to connect to.
    pub url: String,
    /// Whether to include credentials (cookies, auth headers).
    pub with_credentials: bool,
}

/// Callback for SSE events.
pub type EventCallback = Box<dyn Fn(&SseEvent) + Send>;

/// Callback for open events.
pub type OpenCallback = Box<dyn Fn() + Send>;

/// Callback for error events.
pub type ErrorCallback = Box<dyn Fn(&str) + Send>;

/// EventSource — a client for server-sent events.
pub struct EventSource {
    url: String,
    with_credentials: bool,
    state: EventSourceState,
    last_event_id: String,
    retry_ms: u64,
    /// Event handlers: event_type → callback.
    handlers: HashMap<String, Vec<EventCallback>>,
    /// onopen handler.
    on_open: Option<OpenCallback>,
    /// onerror handler.
    on_error: Option<ErrorCallback>,
}

impl std::fmt::Debug for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSource")
            .field("url", &self.url)
            .field("state", &self.state)
            .field("last_event_id", &self.last_event_id)
            .field("retry_ms", &self.retry_ms)
            .finish()
    }
}

impl EventSource {
    /// Create a new EventSource.
    pub fn new(init: EventSourceInit) -> Self {
        Self {
            url: init.url,
            with_credentials: init.with_credentials,
            state: EventSourceState::Connecting,
            last_event_id: String::new(),
            retry_ms: 3000, // Default retry interval
            handlers: HashMap::new(),
            on_open: None,
            on_error: None,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn ready_state(&self) -> EventSourceState {
        self.state
    }

    pub fn with_credentials(&self) -> bool {
        self.with_credentials
    }

    pub fn last_event_id(&self) -> &str {
        &self.last_event_id
    }

    /// Register an event listener.
    pub fn add_event_listener(&mut self, event_type: &str, callback: EventCallback) {
        self.handlers
            .entry(event_type.to_string())
            .or_default()
            .push(callback);
    }

    /// Set the onopen handler.
    pub fn set_onopen(&mut self, callback: Box<dyn Fn() + Send>) {
        self.on_open = Some(callback);
    }

    /// Set the onerror handler.
    pub fn set_onerror(&mut self, callback: Box<dyn Fn(&str) + Send>) {
        self.on_error = Some(callback);
    }

    /// Close the connection.
    pub fn close(&mut self) {
        self.state = EventSourceState::Closed;
    }

    /// Simulate opening the connection (for testing/integration).
    pub fn simulate_open(&mut self) {
        self.state = EventSourceState::Open;
        if let Some(ref cb) = self.on_open {
            cb();
        }
    }

    /// Simulate receiving SSE data (a raw text chunk).
    pub fn simulate_receive(&mut self, chunk: &str) {
        let events = parse_sse_stream(chunk, &self.last_event_id);
        for event in events {
            // Update retry if server sent one
            if let Some(retry) = event.retry {
                self.retry_ms = retry;
            }

            // Update last event ID
            if !event.last_event_id.is_empty() {
                self.last_event_id = event.last_event_id.clone();
            }

            // Dispatch to type-specific handlers
            if let Some(handlers) = self.handlers.get(&event.event_type) {
                for handler in handlers {
                    handler(&event);
                }
            }

            // Also dispatch to "message" handlers if it's the default type
            if event.event_type == "message" {
                if let Some(handlers) = self.handlers.get("message") {
                    // Already dispatched above if event_type == "message"
                    let _ = handlers;
                }
            }
        }
    }

    /// Simulate an error.
    pub fn simulate_error(&mut self, message: &str) {
        self.state = EventSourceState::Connecting; // Will attempt reconnect
        if let Some(ref cb) = self.on_error {
            cb(message);
        }
    }

    /// Get the retry interval in milliseconds.
    pub fn retry_ms(&self) -> u64 {
        self.retry_ms
    }
}

/// Parse a raw SSE text stream into events.
///
/// Per the spec, events are separated by blank lines.
/// Fields: `event:`, `data:`, `id:`, `retry:`.
pub fn parse_sse_stream(text: &str, current_id: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current = SseEvent {
        last_event_id: current_id.to_string(),
        ..Default::default()
    };
    let mut has_data = false;

    for line in text.lines() {
        if line.is_empty() {
            // Empty line = dispatch event
            if has_data {
                // Remove trailing newline from data
                if current.data.ends_with('\n') {
                    current.data.pop();
                }
                events.push(current);
            }
            current = SseEvent {
                last_event_id: events
                    .last()
                    .map(|e: &SseEvent| e.last_event_id.clone())
                    .unwrap_or_else(|| current_id.to_string()),
                ..Default::default()
            };
            has_data = false;
            continue;
        }

        // Comment line
        if line.starts_with(':') {
            continue;
        }

        // Parse field
        let (field, value) = if let Some(colon_pos) = line.find(':') {
            let field = &line[..colon_pos];
            let value = line[colon_pos + 1..]
                .strip_prefix(' ')
                .unwrap_or(&line[colon_pos + 1..]);
            (field, value)
        } else {
            (line, "")
        };

        match field {
            "event" => current.event_type = value.to_string(),
            "data" => {
                if has_data {
                    current.data.push('\n');
                }
                current.data.push_str(value);
                has_data = true;
            }
            "id" => {
                if !value.contains('\0') {
                    current.last_event_id = value.to_string();
                }
            }
            "retry" => {
                if let Ok(ms) = value.parse::<u64>() {
                    current.retry = Some(ms);
                }
            }
            _ => {} // Unknown fields are ignored per spec
        }
    }

    // Handle case where stream doesn't end with blank line
    if has_data {
        if current.data.ends_with('\n') {
            current.data.pop();
        }
        events.push(current);
    }

    events
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn parse_simple_event() {
        let events = parse_sse_stream("data: hello\n\n", "");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "message");
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn parse_multiline_data() {
        let events = parse_sse_stream("data: line1\ndata: line2\n\n", "");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn parse_custom_event_type() {
        let events = parse_sse_stream("event: update\ndata: {\"key\":\"value\"}\n\n", "");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "update");
        assert_eq!(events[0].data, "{\"key\":\"value\"}");
    }

    #[test]
    fn parse_event_id() {
        let events = parse_sse_stream("id: 42\ndata: test\n\n", "");
        assert_eq!(events[0].last_event_id, "42");
    }

    #[test]
    fn parse_retry() {
        let events = parse_sse_stream("retry: 5000\ndata: test\n\n", "");
        assert_eq!(events[0].retry, Some(5000));
    }

    #[test]
    fn parse_comments_ignored() {
        let events = parse_sse_stream(": this is a comment\ndata: hello\n\n", "");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn parse_multiple_events() {
        let input = "data: first\n\ndata: second\n\n";
        let events = parse_sse_stream(input, "");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn parse_no_trailing_newline() {
        let events = parse_sse_stream("data: hello", "");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn event_source_state() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let es = EventSource::new(init);
        assert_eq!(es.ready_state(), EventSourceState::Connecting);
        assert_eq!(es.url(), "https://example.com/events");
    }

    #[test]
    fn event_source_open() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let mut es = EventSource::new(init);

        let opened = Arc::new(AtomicU32::new(0));
        let o = opened.clone();
        es.set_onopen(Box::new(move || {
            o.fetch_add(1, Ordering::Relaxed);
        }));

        es.simulate_open();
        assert_eq!(es.ready_state(), EventSourceState::Open);
        assert_eq!(opened.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn event_source_close() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let mut es = EventSource::new(init);
        es.simulate_open();
        es.close();
        assert_eq!(es.ready_state(), EventSourceState::Closed);
    }

    #[test]
    fn event_source_receive() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let mut es = EventSource::new(init);
        es.simulate_open();

        let received = Arc::new(AtomicU32::new(0));
        let r = received.clone();
        es.add_event_listener(
            "message",
            Box::new(move |_event| {
                r.fetch_add(1, Ordering::Relaxed);
            }),
        );

        es.simulate_receive("data: hello\n\n");
        assert_eq!(received.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn event_source_retry() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let mut es = EventSource::new(init);
        assert_eq!(es.retry_ms(), 3000); // Default

        es.simulate_receive("retry: 5000\ndata: test\n\n");
        assert_eq!(es.retry_ms(), 5000);
    }

    #[test]
    fn event_source_last_event_id() {
        let init = EventSourceInit {
            url: "https://example.com/events".to_string(),
            with_credentials: false,
        };
        let mut es = EventSource::new(init);
        es.simulate_receive("id: 42\ndata: test\n\n");
        assert_eq!(es.last_event_id(), "42");
    }

    #[test]
    fn event_source_state_values() {
        assert_eq!(EventSourceState::Connecting.value(), 0);
        assert_eq!(EventSourceState::Open.value(), 1);
        assert_eq!(EventSourceState::Closed.value(), 2);
    }
}
