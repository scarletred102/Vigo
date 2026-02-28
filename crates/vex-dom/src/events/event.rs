// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM event object.

use vex_core::VexId;

use super::event_type::EventType;

/// The current phase of event dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// Not currently dispatching.
    None,
    /// Travelling from root to target.
    Capturing,
    /// At the target element.
    AtTarget,
    /// Travelling from target back to root.
    Bubbling,
}

/// A DOM event to be dispatched through the tree.
#[derive(Debug)]
pub struct Event {
    /// The event type (e.g., click, keydown).
    pub event_type: EventType,
    /// The target node for this event.
    pub target: VexId,
    /// The node currently processing the event (set during dispatch).
    pub current_target: Option<VexId>,
    /// Current dispatch phase.
    pub phase: EventPhase,
    /// Whether this event bubbles up the tree.
    pub bubbles: bool,
    /// Whether this event is cancelable.
    pub cancelable: bool,
    /// Set by `prevent_default()`.
    pub(crate) default_prevented: bool,
    /// Set by `stop_propagation()`.
    pub(crate) propagation_stopped: bool,
    /// Set by `stop_immediate_propagation()`.
    pub(crate) immediate_propagation_stopped: bool,
}

impl Event {
    /// Create a new event targeting the given node.
    pub fn new(event_type: EventType, target: VexId) -> Self {
        let bubbles = event_type.bubbles();
        let cancelable = event_type.cancelable();
        Self {
            event_type,
            target,
            current_target: None,
            phase: EventPhase::None,
            bubbles,
            cancelable,
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
        }
    }

    /// Cancel the default action for this event.
    pub fn prevent_default(&mut self) {
        if self.cancelable {
            self.default_prevented = true;
        }
    }

    /// Stop the event from propagating further.
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Stop the event from propagating further AND prevent other
    /// listeners on the current target from being called.
    pub fn stop_immediate_propagation(&mut self) {
        self.propagation_stopped = true;
        self.immediate_propagation_stopped = true;
    }

    /// Whether `prevent_default()` was called.
    pub fn default_prevented(&self) -> bool {
        self.default_prevented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_target() -> VexId {
        VexId::new(1)
    }

    #[test]
    fn new_event_defaults() {
        let evt = Event::new(EventType::Click, dummy_target());
        assert!(evt.bubbles);
        assert!(evt.cancelable);
        assert!(!evt.default_prevented);
        assert!(!evt.propagation_stopped);
    }

    #[test]
    fn prevent_default_works() {
        let mut evt = Event::new(EventType::Click, dummy_target());
        evt.prevent_default();
        assert!(evt.default_prevented());
    }

    #[test]
    fn prevent_default_non_cancelable() {
        let mut evt = Event::new(EventType::Load, dummy_target());
        assert!(!evt.cancelable);
        evt.prevent_default();
        assert!(!evt.default_prevented()); // can't prevent non-cancelable
    }

    #[test]
    fn stop_propagation_works() {
        let mut evt = Event::new(EventType::Click, dummy_target());
        evt.stop_propagation();
        assert!(evt.propagation_stopped);
    }

    #[test]
    fn stop_immediate_propagation() {
        let mut evt = Event::new(EventType::Click, dummy_target());
        evt.stop_immediate_propagation();
        assert!(evt.propagation_stopped);
        assert!(evt.immediate_propagation_stopped);
    }
}
