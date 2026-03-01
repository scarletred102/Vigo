// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Event listener storage.
//!
//! Maps nodes to their registered event listeners. Listener callbacks
//! are identified by an opaque `u64` id that will be resolved to actual
//! functions (e.g., JS closures) by the JavaScript engine in Phase 7.

use std::collections::HashMap;

use vex_core::VexId;

use super::event_type::EventType;

/// An event listener registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventListener {
    /// Opaque handle — will map to a JS callback in Phase 7.
    pub callback_id: u64,
    /// Whether this listener fires during the capture phase.
    pub capture: bool,
}

/// Storage for all event listeners in a document.
#[derive(Debug, Default)]
pub struct EventListenerMap {
    /// node id → event type → list of listeners
    map: HashMap<VexId, HashMap<EventType, Vec<EventListener>>>,
}

impl EventListenerMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a listener on a node for an event type.
    pub fn add_listener(
        &mut self,
        node: VexId,
        event_type: EventType,
        callback_id: u64,
        capture: bool,
    ) {
        let listeners = self
            .map
            .entry(node)
            .or_default()
            .entry(event_type)
            .or_default();

        // Avoid duplicates (same callback_id + capture combination)
        if !listeners
            .iter()
            .any(|l| l.callback_id == callback_id && l.capture == capture)
        {
            listeners.push(EventListener {
                callback_id,
                capture,
            });
        }
    }

    /// Remove a specific listener.
    pub fn remove_listener(
        &mut self,
        node: VexId,
        event_type: &EventType,
        callback_id: u64,
        capture: bool,
    ) {
        if let Some(node_map) = self.map.get_mut(&node) {
            if let Some(listeners) = node_map.get_mut(event_type) {
                listeners.retain(|l| !(l.callback_id == callback_id && l.capture == capture));
            }
        }
    }

    /// Get all listeners for a node and event type.
    pub fn get_listeners(&self, node: VexId, event_type: &EventType) -> &[EventListener] {
        self.map
            .get(&node)
            .and_then(|m| m.get(event_type))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Whether any listeners are registered for a node.
    pub fn has_listeners(&self, node: VexId) -> bool {
        self.map.get(&node).is_some_and(|m| !m.is_empty())
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn node(n: u32) -> VexId {
        VexId::new(n)
    }

    #[test]
    fn add_and_get_listener() {
        let mut map = EventListenerMap::new();
        map.add_listener(node(1), EventType::Click, 42, false);

        let listeners = map.get_listeners(node(1), &EventType::Click);
        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].callback_id, 42);
        assert!(!listeners[0].capture);
    }

    #[test]
    fn no_duplicate_listeners() {
        let mut map = EventListenerMap::new();
        map.add_listener(node(1), EventType::Click, 42, false);
        map.add_listener(node(1), EventType::Click, 42, false); // dup

        assert_eq!(map.get_listeners(node(1), &EventType::Click).len(), 1);
    }

    #[test]
    fn same_callback_different_capture() {
        let mut map = EventListenerMap::new();
        map.add_listener(node(1), EventType::Click, 42, false);
        map.add_listener(node(1), EventType::Click, 42, true);

        // Different capture modes → both kept
        assert_eq!(map.get_listeners(node(1), &EventType::Click).len(), 2);
    }

    #[test]
    fn remove_listener() {
        let mut map = EventListenerMap::new();
        map.add_listener(node(1), EventType::Click, 42, false);
        map.add_listener(node(1), EventType::Click, 43, false);

        map.remove_listener(node(1), &EventType::Click, 42, false);
        let listeners = map.get_listeners(node(1), &EventType::Click);
        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].callback_id, 43);
    }

    #[test]
    fn get_empty_listeners() {
        let map = EventListenerMap::new();
        assert!(map.get_listeners(node(1), &EventType::Click).is_empty());
    }

    #[test]
    fn multiple_event_types() {
        let mut map = EventListenerMap::new();
        map.add_listener(node(1), EventType::Click, 1, false);
        map.add_listener(node(1), EventType::KeyDown, 2, false);

        assert_eq!(map.get_listeners(node(1), &EventType::Click).len(), 1);
        assert_eq!(map.get_listeners(node(1), &EventType::KeyDown).len(), 1);
        assert!(map.get_listeners(node(1), &EventType::Focus).is_empty());
    }
}
