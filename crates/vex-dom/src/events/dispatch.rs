// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Event dispatch algorithm — capture → target → bubble.

use crate::arena::NodeArena;

use super::event::{Event, EventPhase};
use super::listeners::EventListenerMap;

/// Dispatch an event through the DOM tree.
///
/// Returns `true` if `prevent_default()` was called.
///
/// Algorithm:
/// 1. Build the path from target to root (ancestors list).
/// 2. **Capture phase**: root → target, fire `capture: true` listeners.
/// 3. **At-target phase**: fire all listeners on the target.
/// 4. **Bubbling phase** (if `event.bubbles`): target → root, fire `capture: false` listeners.
///
/// The `callback` function is called for each listener that fires. In Phase 7,
/// this will resolve callback_id to a JS function. For now, it receives
/// `(callback_id, &Event)`.
pub fn dispatch_event<F>(
    arena: &NodeArena,
    listeners: &EventListenerMap,
    event: &mut Event,
    mut callback: F,
) -> bool
where
    F: FnMut(u64, &mut Event),
{
    // Build path: target → root
    let mut path = Vec::new();
    let mut current = Some(event.target);
    while let Some(id) = current {
        path.push(id);
        current = arena.get(id).parent;
    }

    // path[0] = target, path[last] = root
    // For capture: iterate root → (target's parent)
    // For at-target: path[0]
    // For bubbling: (target's parent) → root

    // ── Capture phase ────────────────────────────────────────────
    event.phase = EventPhase::Capturing;
    for &node_id in path.iter().rev().skip(1) {
        // Skip the target itself (it's handled at At-Target)
        if event.propagation_stopped {
            break;
        }
        event.current_target = Some(node_id);

        let node_listeners = listeners.get_listeners(node_id, &event.event_type);
        for listener in node_listeners {
            if event.immediate_propagation_stopped {
                break;
            }
            if listener.capture {
                callback(listener.callback_id, event);
            }
        }
    }

    // ── At-target phase ──────────────────────────────────────────
    if !event.propagation_stopped {
        event.phase = EventPhase::AtTarget;
        event.current_target = Some(event.target);

        let target_listeners = listeners.get_listeners(event.target, &event.event_type);
        for listener in target_listeners {
            if event.immediate_propagation_stopped {
                break;
            }
            // At target, fire ALL listeners (capture and bubble)
            callback(listener.callback_id, event);
        }
    }

    // ── Bubbling phase ───────────────────────────────────────────
    if event.bubbles && !event.propagation_stopped {
        event.phase = EventPhase::Bubbling;
        // Skip target (index 0), walk ancestors toward root
        for &node_id in path.iter().skip(1) {
            if event.propagation_stopped {
                break;
            }
            event.current_target = Some(node_id);

            let node_listeners = listeners.get_listeners(node_id, &event.event_type);
            for listener in node_listeners {
                if event.immediate_propagation_stopped {
                    break;
                }
                if !listener.capture {
                    callback(listener.callback_id, event);
                }
            }
        }
    }

    event.phase = EventPhase::None;
    event.default_prevented
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::NodeArena;
    use crate::node::{ElementData, Namespace, NodeData};
    use crate::tree;
    use super::super::event_type::EventType;
    use vex_core::VexId;

    /// Build a small tree: root → parent → child
    fn build_tree() -> (NodeArena, VexId, VexId, VexId) {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let parent = arena.alloc(NodeData::Element(ElementData {
            tag_name: "div".into(),
            namespace: Namespace::Html,
            attributes: vec![],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        let child = arena.alloc(NodeData::Element(ElementData {
            tag_name: "p".into(),
            namespace: Namespace::Html,
            attributes: vec![],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, root, parent);
        tree::append_child(&mut arena, parent, child);
        (arena, root, parent, child)
    }

    #[test]
    fn basic_dispatch() {
        let (arena, _root, _parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(child, EventType::Click, 1, false);

        let mut evt = Event::new(EventType::Click, child);
        let mut called = Vec::new();
        dispatch_event(&arena, &lm, &mut evt, |id, _| called.push(id));
        assert_eq!(called, vec![1]);
    }

    #[test]
    fn bubbling() {
        let (arena, _root, parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(child, EventType::Click, 1, false);
        lm.add_listener(parent, EventType::Click, 2, false);

        let mut evt = Event::new(EventType::Click, child);
        let mut called = Vec::new();
        dispatch_event(&arena, &lm, &mut evt, |id, _| called.push(id));
        // target first, then parent during bubbling
        assert_eq!(called, vec![1, 2]);
    }

    #[test]
    fn capture_phase() {
        let (arena, _root, parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(parent, EventType::Click, 1, true); // capture
        lm.add_listener(child, EventType::Click, 2, false);

        let mut evt = Event::new(EventType::Click, child);
        let mut called = Vec::new();
        dispatch_event(&arena, &lm, &mut evt, |id, _| called.push(id));
        // capture fires first (parent), then at-target (child)
        assert_eq!(called, vec![1, 2]);
    }

    #[test]
    fn stop_propagation() {
        let (arena, _root, parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(child, EventType::Click, 1, false);
        lm.add_listener(parent, EventType::Click, 2, false);

        let mut evt = Event::new(EventType::Click, child);
        let mut called = Vec::new();
        dispatch_event(&arena, &lm, &mut evt, |id, evt| {
            called.push(id);
            if id == 1 {
                evt.stop_propagation();
            }
        });
        // Only child fires, parent doesn't receive
        assert_eq!(called, vec![1]);
    }

    #[test]
    fn prevent_default() {
        let (arena, _root, _parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(child, EventType::Click, 1, false);

        let mut evt = Event::new(EventType::Click, child);
        let prevented = dispatch_event(&arena, &lm, &mut evt, |_, evt| {
            evt.prevent_default();
        });
        assert!(prevented);
    }

    #[test]
    fn non_bubbling_event() {
        let (arena, _root, parent, child) = build_tree();
        let mut lm = EventListenerMap::new();
        lm.add_listener(child, EventType::Focus, 1, false);
        lm.add_listener(parent, EventType::Focus, 2, false);

        let mut evt = Event::new(EventType::Focus, child);
        assert!(!evt.bubbles);
        let mut called = Vec::new();
        dispatch_event(&arena, &lm, &mut evt, |id, _| called.push(id));
        // Focus doesn't bubble — only target fires
        assert_eq!(called, vec![1]);
    }
}
