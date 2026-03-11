// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Pointer Events & Touch Events API.
//!
//! Unified input model for mouse, touch, and stylus.
//! Implements the W3C Pointer Events spec and basic Touch Events.

use std::collections::HashMap;

// ── Pointer Types ────────────────────────────────────────────────────────────

/// The type of pointer device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerType {
    Mouse,
    Touch,
    Pen,
}

impl PointerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Touch => "touch",
            Self::Pen => "pen",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "mouse" => Some(Self::Mouse),
            "touch" => Some(Self::Touch),
            "pen" => Some(Self::Pen),
            _ => None,
        }
    }
}

// ── PointerEvent ─────────────────────────────────────────────────────────────

/// A pointer event (W3C Pointer Events Level 2).
#[derive(Debug, Clone)]
pub struct PointerEvent {
    /// Unique pointer ID.
    pub pointer_id: i32,
    /// Width of the contact area.
    pub width: f64,
    /// Height of the contact area.
    pub height: f64,
    /// Pressure (0.0 to 1.0).
    pub pressure: f32,
    /// Tangential pressure (-1.0 to 1.0).
    pub tangential_pressure: f32,
    /// Tilt on X axis (-90 to 90 degrees).
    pub tilt_x: i32,
    /// Tilt on Y axis (-90 to 90 degrees).
    pub tilt_y: i32,
    /// Twist (0 to 359 degrees).
    pub twist: u32,
    /// Pointer type.
    pub pointer_type: PointerType,
    /// Whether this is the primary pointer.
    pub is_primary: bool,
    /// Client X coordinate.
    pub client_x: f64,
    /// Client Y coordinate.
    pub client_y: f64,
    /// The event type name.
    pub event_type: PointerEventType,
}

/// Pointer event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerEventType {
    PointerDown,
    PointerMove,
    PointerUp,
    PointerCancel,
    PointerOver,
    PointerOut,
    PointerEnter,
    PointerLeave,
    GotPointerCapture,
    LostPointerCapture,
}

impl PointerEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PointerDown => "pointerdown",
            Self::PointerMove => "pointermove",
            Self::PointerUp => "pointerup",
            Self::PointerCancel => "pointercancel",
            Self::PointerOver => "pointerover",
            Self::PointerOut => "pointerout",
            Self::PointerEnter => "pointerenter",
            Self::PointerLeave => "pointerleave",
            Self::GotPointerCapture => "gotpointercapture",
            Self::LostPointerCapture => "lostpointercapture",
        }
    }
}

impl PointerEvent {
    /// Create a basic mouse pointer event.
    pub fn mouse(event_type: PointerEventType, x: f64, y: f64) -> Self {
        Self {
            pointer_id: 1,
            width: 1.0,
            height: 1.0,
            pressure: if matches!(
                event_type,
                PointerEventType::PointerDown | PointerEventType::PointerMove
            ) {
                0.5
            } else {
                0.0
            },
            tangential_pressure: 0.0,
            tilt_x: 0,
            tilt_y: 0,
            twist: 0,
            pointer_type: PointerType::Mouse,
            is_primary: true,
            client_x: x,
            client_y: y,
            event_type,
        }
    }

    /// Create a touch pointer event.
    pub fn touch(pointer_id: i32, event_type: PointerEventType, x: f64, y: f64) -> Self {
        Self {
            pointer_id,
            width: 20.0,
            height: 20.0,
            pressure: if matches!(
                event_type,
                PointerEventType::PointerDown | PointerEventType::PointerMove
            ) {
                0.5
            } else {
                0.0
            },
            tangential_pressure: 0.0,
            tilt_x: 0,
            tilt_y: 0,
            twist: 0,
            pointer_type: PointerType::Touch,
            is_primary: pointer_id == 1,
            client_x: x,
            client_y: y,
            event_type,
        }
    }
}

// ── TouchEvent ───────────────────────────────────────────────────────────────

/// A single touch point.
#[derive(Debug, Clone)]
pub struct Touch {
    pub identifier: i32,
    pub client_x: f64,
    pub client_y: f64,
    pub page_x: f64,
    pub page_y: f64,
    pub screen_x: f64,
    pub screen_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub rotation_angle: f64,
    pub force: f64,
}

impl Touch {
    pub fn new(id: i32, x: f64, y: f64) -> Self {
        Self {
            identifier: id,
            client_x: x,
            client_y: y,
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            radius_x: 10.0,
            radius_y: 10.0,
            rotation_angle: 0.0,
            force: 0.5,
        }
    }
}

/// A touch event.
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub event_type: TouchEventType,
    pub touches: Vec<Touch>,
    pub target_touches: Vec<Touch>,
    pub changed_touches: Vec<Touch>,
}

/// Touch event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchEventType {
    TouchStart,
    TouchMove,
    TouchEnd,
    TouchCancel,
}

impl TouchEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TouchStart => "touchstart",
            Self::TouchMove => "touchmove",
            Self::TouchEnd => "touchend",
            Self::TouchCancel => "touchcancel",
        }
    }
}

// ── PointerCaptureManager ────────────────────────────────────────────────────

/// Manages pointer capture (setPointerCapture / releasePointerCapture).
#[derive(Debug, Default)]
pub struct PointerCaptureManager {
    /// Pointer ID → target element ID.
    captures: HashMap<i32, u64>,
    /// Active pointers.
    active_pointers: HashMap<i32, PointerType>,
}

impl PointerCaptureManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an active pointer.
    pub fn pointer_down(&mut self, pointer_id: i32, pointer_type: PointerType) {
        self.active_pointers.insert(pointer_id, pointer_type);
    }

    /// Remove an active pointer.
    pub fn pointer_up(&mut self, pointer_id: i32) {
        self.active_pointers.remove(&pointer_id);
        self.captures.remove(&pointer_id);
    }

    /// Set pointer capture on an element.
    pub fn set_capture(&mut self, pointer_id: i32, element_id: u64) -> bool {
        if self.active_pointers.contains_key(&pointer_id) {
            self.captures.insert(pointer_id, element_id);
            true
        } else {
            false
        }
    }

    /// Release pointer capture.
    pub fn release_capture(&mut self, pointer_id: i32) -> bool {
        self.captures.remove(&pointer_id).is_some()
    }

    /// Get the element that has capture for a pointer.
    pub fn get_capture_target(&self, pointer_id: i32) -> Option<u64> {
        self.captures.get(&pointer_id).copied()
    }

    /// Whether any pointer is captured.
    pub fn has_capture(&self, pointer_id: i32) -> bool {
        self.captures.contains_key(&pointer_id)
    }

    /// Number of active pointers.
    pub fn active_count(&self) -> usize {
        self.active_pointers.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_event() {
        let evt = PointerEvent::mouse(PointerEventType::PointerDown, 100.0, 200.0);
        assert_eq!(evt.pointer_type, PointerType::Mouse);
        assert_eq!(evt.client_x, 100.0);
        assert_eq!(evt.client_y, 200.0);
        assert!(evt.is_primary);
        assert!(evt.pressure > 0.0);
    }

    #[test]
    fn touch_event() {
        let evt = PointerEvent::touch(2, PointerEventType::PointerDown, 50.0, 60.0);
        assert_eq!(evt.pointer_type, PointerType::Touch);
        assert_eq!(evt.pointer_id, 2);
        assert!(!evt.is_primary); // Only ID 1 is primary
    }

    #[test]
    fn pointer_type_str() {
        assert_eq!(PointerType::Mouse.as_str(), "mouse");
        assert_eq!(PointerType::from_name("pen"), Some(PointerType::Pen));
        assert_eq!(PointerType::from_name("unknown"), None);
    }

    #[test]
    fn event_type_str() {
        assert_eq!(PointerEventType::PointerDown.as_str(), "pointerdown");
        assert_eq!(PointerEventType::LostPointerCapture.as_str(), "lostpointercapture");
    }

    #[test]
    fn touch_object() {
        let t = Touch::new(1, 100.0, 200.0);
        assert_eq!(t.identifier, 1);
        assert_eq!(t.client_x, 100.0);
        assert!(t.force > 0.0);
    }

    #[test]
    fn touch_event_type_str() {
        assert_eq!(TouchEventType::TouchStart.as_str(), "touchstart");
        assert_eq!(TouchEventType::TouchCancel.as_str(), "touchcancel");
    }

    #[test]
    fn pointer_capture_set_release() {
        let mut mgr = PointerCaptureManager::new();
        mgr.pointer_down(1, PointerType::Mouse);
        assert!(mgr.set_capture(1, 42));
        assert_eq!(mgr.get_capture_target(1), Some(42));
        assert!(mgr.release_capture(1));
        assert_eq!(mgr.get_capture_target(1), None);
    }

    #[test]
    fn capture_requires_active_pointer() {
        let mut mgr = PointerCaptureManager::new();
        assert!(!mgr.set_capture(1, 42)); // pointer not active
    }

    #[test]
    fn pointer_up_releases_capture() {
        let mut mgr = PointerCaptureManager::new();
        mgr.pointer_down(1, PointerType::Touch);
        mgr.set_capture(1, 10);
        mgr.pointer_up(1);
        assert!(!mgr.has_capture(1));
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn multiple_active_pointers() {
        let mut mgr = PointerCaptureManager::new();
        mgr.pointer_down(1, PointerType::Touch);
        mgr.pointer_down(2, PointerType::Touch);
        mgr.pointer_down(3, PointerType::Touch);
        assert_eq!(mgr.active_count(), 3);
    }

    #[test]
    fn pressure_varies_by_event_type() {
        let down = PointerEvent::mouse(PointerEventType::PointerDown, 0.0, 0.0);
        let up = PointerEvent::mouse(PointerEventType::PointerUp, 0.0, 0.0);
        assert!(down.pressure > 0.0);
        assert_eq!(up.pressure, 0.0);
    }
}
