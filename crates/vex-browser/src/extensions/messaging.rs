// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension runtime messaging hub.
//!
//! Provides a browser-side message router for extension-to-extension
//! communication patterns similar to `chrome.runtime.sendMessage`.
//!
//! Current scope:
//! - direct message (`extension A` -> `extension B`)
//! - broadcast message (`extension A` -> all other registered extensions)
//! - FIFO per-recipient inbox

use std::collections::{HashMap, VecDeque};

/// Target for an extension runtime message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageTarget {
    /// Deliver to all registered extensions except the sender.
    Broadcast,
    /// Deliver to one specific extension ID.
    Extension(String),
}

/// A routed runtime message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMessage {
    /// Sender extension ID.
    pub from_extension_id: String,
    /// Recipient extension ID (none for broadcast copy before fanout).
    pub to_extension_id: Option<String>,
    /// Message payload (JSON string or opaque text).
    pub payload: String,
    /// Capture timestamp (unix epoch millis).
    pub unix_ms: u128,
}

impl RuntimeMessage {
    fn new(from_extension_id: &str, to_extension_id: Option<String>, payload: String) -> Self {
        let unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self {
            from_extension_id: from_extension_id.to_owned(),
            to_extension_id,
            payload,
            unix_ms,
        }
    }
}

/// Browser-side extension messaging router.
#[derive(Debug, Default)]
pub struct MessagingHub {
    inboxes: HashMap<String, VecDeque<RuntimeMessage>>,
}

impl MessagingHub {
    /// Create an empty messaging hub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an extension inbox if it doesn't exist.
    pub fn register_extension(&mut self, extension_id: &str) {
        self.inboxes.entry(extension_id.to_owned()).or_default();
    }

    /// Remove extension inbox and pending messages.
    pub fn unregister_extension(&mut self, extension_id: &str) {
        self.inboxes.remove(extension_id);
    }

    /// Whether an extension is currently registered.
    #[must_use]
    pub fn is_registered(&self, extension_id: &str) -> bool {
        self.inboxes.contains_key(extension_id)
    }

    /// Number of registered extension inboxes.
    #[must_use]
    pub fn registered_count(&self) -> usize {
        self.inboxes.len()
    }

    /// Send a message from one extension to a target.
    ///
    /// Returns number of recipients that received the message.
    pub fn send_message(
        &mut self,
        from_extension_id: &str,
        target: MessageTarget,
        payload: String,
    ) -> usize {
        if !self.is_registered(from_extension_id) {
            return 0;
        }

        match target {
            MessageTarget::Broadcast => {
                let recipients: Vec<String> = self
                    .inboxes
                    .keys()
                    .filter(|id| id.as_str() != from_extension_id)
                    .cloned()
                    .collect();

                for recipient in &recipients {
                    let msg = RuntimeMessage::new(
                        from_extension_id,
                        Some(recipient.clone()),
                        payload.clone(),
                    );
                    if let Some(queue) = self.inboxes.get_mut(recipient) {
                        queue.push_back(msg);
                    }
                }

                recipients.len()
            }
            MessageTarget::Extension(recipient) => {
                if recipient == from_extension_id {
                    return 0;
                }
                if let Some(queue) = self.inboxes.get_mut(&recipient) {
                    let msg =
                        RuntimeMessage::new(from_extension_id, Some(recipient.clone()), payload);
                    queue.push_back(msg);
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Receive the next pending message for an extension.
    pub fn poll_message(&mut self, extension_id: &str) -> Option<RuntimeMessage> {
        self.inboxes
            .get_mut(extension_id)
            .and_then(VecDeque::pop_front)
    }

    /// Number of pending messages for an extension inbox.
    #[must_use]
    pub fn pending_count(&self, extension_id: &str) -> usize {
        self.inboxes
            .get(extension_id)
            .map_or(0, std::collections::VecDeque::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_delivers_to_other_extensions() {
        let mut hub = MessagingHub::new();
        hub.register_extension("ext-a");
        hub.register_extension("ext-b");
        hub.register_extension("ext-c");

        let delivered = hub.send_message(
            "ext-a",
            MessageTarget::Broadcast,
            "{\"kind\":\"ping\"}".to_owned(),
        );

        assert_eq!(delivered, 2);
        assert_eq!(hub.pending_count("ext-a"), 0);
        assert_eq!(hub.pending_count("ext-b"), 1);
        assert_eq!(hub.pending_count("ext-c"), 1);
    }

    #[test]
    fn direct_message_delivers_single_recipient() {
        let mut hub = MessagingHub::new();
        hub.register_extension("sender");
        hub.register_extension("receiver");

        let delivered = hub.send_message(
            "sender",
            MessageTarget::Extension("receiver".to_owned()),
            "hello".to_owned(),
        );

        assert_eq!(delivered, 1);
        let msg = hub
            .poll_message("receiver")
            .expect("message should be queued");
        assert_eq!(msg.from_extension_id, "sender");
        assert_eq!(msg.to_extension_id, Some("receiver".to_owned()));
        assert_eq!(msg.payload, "hello");
    }

    #[test]
    fn sender_must_be_registered() {
        let mut hub = MessagingHub::new();
        hub.register_extension("receiver");

        let delivered = hub.send_message(
            "unknown",
            MessageTarget::Extension("receiver".to_owned()),
            "x".to_owned(),
        );

        assert_eq!(delivered, 0);
        assert_eq!(hub.pending_count("receiver"), 0);
    }

    #[test]
    fn unregister_drops_inbox_and_messages() {
        let mut hub = MessagingHub::new();
        hub.register_extension("a");
        hub.register_extension("b");

        let _ = hub.send_message(
            "a",
            MessageTarget::Extension("b".to_owned()),
            "queued".to_owned(),
        );
        assert_eq!(hub.pending_count("b"), 1);

        hub.unregister_extension("b");
        assert!(!hub.is_registered("b"));
        assert_eq!(hub.pending_count("b"), 0);
    }
}
