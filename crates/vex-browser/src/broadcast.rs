// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! BroadcastChannel API.
//!
//! Enables simple message passing between browsing contexts
//! (tabs, windows, iframes) on the same origin.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ── ChannelId ────────────────────────────────────────────────────────────────

/// Unique identifier for a BroadcastChannel endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelEndpointId(u64);

static NEXT_ENDPOINT_ID: AtomicU64 = AtomicU64::new(1);

impl ChannelEndpointId {
    pub fn new() -> Self {
        Self(NEXT_ENDPOINT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Create from a raw value (for tests).
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

impl Default for ChannelEndpointId {
    fn default() -> Self {
        Self::new()
    }
}

// ── BroadcastMessage ─────────────────────────────────────────────────────────

/// A message sent through a broadcast channel.
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    /// The message data (structured clone, represented as JSON string).
    pub data: String,
    /// The origin that sent the message.
    pub origin: String,
    /// The endpoint that sent this message.
    pub sender: ChannelEndpointId,
}

impl BroadcastMessage {
    pub fn new(data: &str, origin: &str, sender: ChannelEndpointId) -> Self {
        Self {
            data: data.to_string(),
            origin: origin.to_string(),
            sender,
        }
    }
}

// ── BroadcastChannel ─────────────────────────────────────────────────────────

/// A single BroadcastChannel endpoint.
#[derive(Debug)]
pub struct BroadcastChannel {
    /// Unique endpoint ID.
    pub id: ChannelEndpointId,
    /// Channel name.
    pub name: String,
    /// Origin of this endpoint.
    pub origin: String,
    /// Whether the channel is open.
    pub closed: bool,
    /// Incoming message buffer.
    pub inbox: Vec<BroadcastMessage>,
}

impl BroadcastChannel {
    pub fn new(name: &str, origin: &str) -> Self {
        Self {
            id: ChannelEndpointId::new(),
            name: name.to_string(),
            origin: origin.to_string(),
            closed: false,
            inbox: Vec::new(),
        }
    }

    /// Close the channel.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Receive the next pending message.
    pub fn receive(&mut self) -> Option<BroadcastMessage> {
        if self.inbox.is_empty() {
            None
        } else {
            Some(self.inbox.remove(0))
        }
    }

    /// Number of pending messages.
    pub fn pending_count(&self) -> usize {
        self.inbox.len()
    }
}

// ── BroadcastHub ─────────────────────────────────────────────────────────────

/// Central hub that routes messages between BroadcastChannel endpoints.
#[derive(Debug, Default)]
pub struct BroadcastHub {
    /// Channels grouped by name. Each name maps to a list of endpoint IDs.
    channels: HashMap<String, Vec<ChannelEndpointId>>,
    /// All endpoints by ID.
    endpoints: HashMap<ChannelEndpointId, BroadcastChannel>,
}

impl BroadcastHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new channel endpoint.
    pub fn register(&mut self, channel: BroadcastChannel) -> ChannelEndpointId {
        let id = channel.id;
        let name = channel.name.clone();
        self.channels.entry(name).or_default().push(id);
        self.endpoints.insert(id, channel);
        id
    }

    /// Create and register a new channel endpoint.
    pub fn create_channel(&mut self, name: &str, origin: &str) -> ChannelEndpointId {
        let chan = BroadcastChannel::new(name, origin);
        self.register(chan)
    }

    /// Post a message on a channel (from a specific endpoint).
    /// Message is delivered to all *other* endpoints with the same name and origin.
    pub fn post_message(&mut self, sender_id: ChannelEndpointId, data: &str) -> usize {
        let (name, origin) = {
            let sender = match self.endpoints.get(&sender_id) {
                Some(ep) if !ep.closed => ep,
                _ => return 0,
            };
            (sender.name.clone(), sender.origin.clone())
        };

        let msg = BroadcastMessage::new(data, &origin, sender_id);

        let recipients = match self.channels.get(&name) {
            Some(ids) => ids.clone(),
            None => return 0,
        };

        let mut delivered = 0;
        for id in &recipients {
            if *id == sender_id {
                continue; // Don't deliver to sender
            }
            if let Some(ep) = self.endpoints.get_mut(id) {
                if !ep.closed && ep.origin == origin {
                    ep.inbox.push(msg.clone());
                    delivered += 1;
                }
            }
        }
        delivered
    }

    /// Close a specific endpoint.
    pub fn close(&mut self, id: ChannelEndpointId) {
        if let Some(ep) = self.endpoints.get_mut(&id) {
            ep.close();
        }
    }

    /// Remove closed endpoints from the hub.
    pub fn cleanup(&mut self) {
        let closed_ids: Vec<ChannelEndpointId> = self
            .endpoints
            .iter()
            .filter(|(_, ep)| ep.closed)
            .map(|(id, _)| *id)
            .collect();

        for id in &closed_ids {
            if let Some(ep) = self.endpoints.remove(id) {
                if let Some(ids) = self.channels.get_mut(&ep.name) {
                    ids.retain(|i| i != id);
                    if ids.is_empty() {
                        self.channels.remove(&ep.name);
                    }
                }
            }
        }
    }

    /// Get an endpoint by ID.
    pub fn get(&self, id: ChannelEndpointId) -> Option<&BroadcastChannel> {
        self.endpoints.get(&id)
    }

    /// Get a mutable endpoint by ID.
    pub fn get_mut(&mut self, id: ChannelEndpointId) -> Option<&mut BroadcastChannel> {
        self.endpoints.get_mut(&id)
    }

    /// Number of channel names.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Number of endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";

    #[test]
    fn create_channel() {
        let mut hub = BroadcastHub::new();
        let id = hub.create_channel("test", ORIGIN);
        assert!(hub.get(id).is_some());
        assert_eq!(hub.get(id).unwrap().name, "test");
    }

    #[test]
    fn post_message_delivered() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("chat", ORIGIN);
        let id2 = hub.create_channel("chat", ORIGIN);

        let delivered = hub.post_message(id1, r#"{"text":"hello"}"#);
        assert_eq!(delivered, 1);

        let msg = hub.get_mut(id2).unwrap().receive().unwrap();
        assert_eq!(msg.data, r#"{"text":"hello"}"#);
        assert_eq!(msg.origin, ORIGIN);
    }

    #[test]
    fn sender_does_not_receive() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch", ORIGIN);
        hub.post_message(id1, "hello");
        assert_eq!(hub.get(id1).unwrap().pending_count(), 0);
    }

    #[test]
    fn different_names_isolated() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch-a", ORIGIN);
        let id2 = hub.create_channel("ch-b", ORIGIN);

        hub.post_message(id1, "for-a");
        assert_eq!(hub.get(id2).unwrap().pending_count(), 0);
    }

    #[test]
    fn different_origins_isolated() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("shared", ORIGIN);
        let id2 = hub.create_channel("shared", "https://other.com");

        hub.post_message(id1, "data");
        assert_eq!(hub.get(id2).unwrap().pending_count(), 0);
    }

    #[test]
    fn multi_recipients() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("sync", ORIGIN);
        let id2 = hub.create_channel("sync", ORIGIN);
        let id3 = hub.create_channel("sync", ORIGIN);

        let delivered = hub.post_message(id1, "update");
        assert_eq!(delivered, 2);
        assert_eq!(hub.get(id2).unwrap().pending_count(), 1);
        assert_eq!(hub.get(id3).unwrap().pending_count(), 1);
    }

    #[test]
    fn closed_channel_no_receive() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch", ORIGIN);
        let id2 = hub.create_channel("ch", ORIGIN);

        hub.close(id2);
        let delivered = hub.post_message(id1, "hello");
        assert_eq!(delivered, 0);
    }

    #[test]
    fn closed_channel_no_send() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch", ORIGIN);
        let _id2 = hub.create_channel("ch", ORIGIN);

        hub.close(id1);
        let delivered = hub.post_message(id1, "hello");
        assert_eq!(delivered, 0);
    }

    #[test]
    fn cleanup() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch", ORIGIN);
        let _id2 = hub.create_channel("ch", ORIGIN);

        hub.close(id1);
        assert_eq!(hub.endpoint_count(), 2);
        hub.cleanup();
        assert_eq!(hub.endpoint_count(), 1);
    }

    #[test]
    fn channel_count() {
        let mut hub = BroadcastHub::new();
        hub.create_channel("ch-a", ORIGIN);
        hub.create_channel("ch-b", ORIGIN);
        hub.create_channel("ch-a", ORIGIN); // Same name

        assert_eq!(hub.channel_count(), 2);
        assert_eq!(hub.endpoint_count(), 3);
    }

    #[test]
    fn message_ordering() {
        let mut hub = BroadcastHub::new();
        let id1 = hub.create_channel("ch", ORIGIN);
        let id2 = hub.create_channel("ch", ORIGIN);

        hub.post_message(id1, "first");
        hub.post_message(id1, "second");
        hub.post_message(id1, "third");

        let ep = hub.get_mut(id2).unwrap();
        assert_eq!(ep.receive().unwrap().data, "first");
        assert_eq!(ep.receive().unwrap().data, "second");
        assert_eq!(ep.receive().unwrap().data, "third");
    }
}
