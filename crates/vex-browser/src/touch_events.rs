// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Touch Events API.
//!
//! Provides touch input handling for touchscreen devices, following
//! the W3C Touch Events specification.

use std::time::Instant;

// ── Types ────────────────────────────────────────────────────────────────────

/// A single touch point.
#[derive(Debug, Clone)]
pub struct Touch {
    /// Unique identifier for this touch.
    pub identifier: u32,
    /// X coordinate in the viewport.
    pub client_x: f64,
    /// Y coordinate in the viewport.
    pub client_y: f64,
    /// X coordinate relative to the screen.
    pub screen_x: f64,
    /// Y coordinate relative to the screen.
    pub screen_y: f64,
    /// X coordinate relative to the page.
    pub page_x: f64,
    /// Y coordinate relative to the page.
    pub page_y: f64,
    /// Contact area width.
    pub radius_x: f64,
    /// Contact area height.
    pub radius_y: f64,
    /// Rotation angle in degrees.
    pub rotation_angle: f64,
    /// Pressure (0.0 to 1.0).
    pub force: f64,
}

impl Touch {
    pub fn new(identifier: u32, client_x: f64, client_y: f64) -> Self {
        Self {
            identifier,
            client_x,
            client_y,
            screen_x: client_x,
            screen_y: client_y,
            page_x: client_x,
            page_y: client_y,
            radius_x: 0.0,
            radius_y: 0.0,
            rotation_angle: 0.0,
            force: 0.0,
        }
    }
}

/// A list of touch points.
#[derive(Debug, Clone, Default)]
pub struct TouchList {
    pub touches: Vec<Touch>,
}

impl TouchList {
    pub fn new() -> Self {
        Self {
            touches: Vec::new(),
        }
    }

    pub fn length(&self) -> usize {
        self.touches.len()
    }

    pub fn item(&self, index: usize) -> Option<&Touch> {
        self.touches.get(index)
    }

    pub fn add(&mut self, touch: Touch) {
        self.touches.push(touch);
    }

    pub fn remove_by_id(&mut self, id: u32) -> bool {
        if let Some(pos) = self.touches.iter().position(|t| t.identifier == id) {
            self.touches.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn find_by_id(&self, id: u32) -> Option<&Touch> {
        self.touches.iter().find(|t| t.identifier == id)
    }
}

/// Touch event type.
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

/// A touch event.
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub event_type: TouchEventType,
    /// All active touches on the screen.
    pub touches: TouchList,
    /// Touches that started on the target element.
    pub target_touches: TouchList,
    /// Touches that changed in this event.
    pub changed_touches: TouchList,
    /// Whether Alt key is pressed.
    pub alt_key: bool,
    /// Whether Ctrl key is pressed.
    pub ctrl_key: bool,
    /// Whether Meta key is pressed.
    pub meta_key: bool,
    /// Whether Shift key is pressed.
    pub shift_key: bool,
    pub timestamp: Instant,
    pub default_prevented: bool,
}

impl TouchEvent {
    pub fn new(event_type: TouchEventType) -> Self {
        Self {
            event_type,
            touches: TouchList::new(),
            target_touches: TouchList::new(),
            changed_touches: TouchList::new(),
            alt_key: false,
            ctrl_key: false,
            meta_key: false,
            shift_key: false,
            timestamp: Instant::now(),
            default_prevented: false,
        }
    }

    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }
}

/// Gesture recognizer for common touch patterns.
#[derive(Debug)]
pub struct GestureRecognizer {
    /// Active touch points for gesture detection.
    start_touches: Vec<(u32, f64, f64, Instant)>,
    /// Tap threshold (max duration in ms).
    tap_threshold_ms: u128,
    /// Swipe threshold (min distance).
    swipe_threshold_px: f64,
}

/// Recognized gesture.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    Tap { x: f64, y: f64 },
    DoubleTap { x: f64, y: f64 },
    LongPress { x: f64, y: f64 },
    Swipe { direction: SwipeDirection, distance: f64 },
    Pinch { scale: f64 },
}

/// Swipe direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            start_touches: Vec::new(),
            tap_threshold_ms: 300,
            swipe_threshold_px: 50.0,
        }
    }

    /// Record a touch start.
    pub fn on_touch_start(&mut self, touch: &Touch) {
        self.start_touches
            .push((touch.identifier, touch.client_x, touch.client_y, Instant::now()));
    }

    /// Process a touch end and detect gestures.
    pub fn on_touch_end(&mut self, touch: &Touch) -> Option<Gesture> {
        let pos = self
            .start_touches
            .iter()
            .position(|(id, _, _, _)| *id == touch.identifier)?;
        let (_, start_x, start_y, start_time) = self.start_touches.remove(pos);

        let dx = touch.client_x - start_x;
        let dy = touch.client_y - start_y;
        let distance = (dx * dx + dy * dy).sqrt();
        let duration = start_time.elapsed().as_millis();

        if distance < self.swipe_threshold_px && duration < self.tap_threshold_ms {
            return Some(Gesture::Tap {
                x: touch.client_x,
                y: touch.client_y,
            });
        }

        if distance >= self.swipe_threshold_px {
            let direction = if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    SwipeDirection::Right
                } else {
                    SwipeDirection::Left
                }
            } else if dy > 0.0 {
                SwipeDirection::Down
            } else {
                SwipeDirection::Up
            };
            return Some(Gesture::Swipe {
                direction,
                distance,
            });
        }

        None
    }

    /// Clear all tracked touches.
    pub fn reset(&mut self) {
        self.start_touches.clear();
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_new() {
        let t = Touch::new(1, 100.0, 200.0);
        assert_eq!(t.identifier, 1);
        assert_eq!(t.client_x, 100.0);
        assert_eq!(t.client_y, 200.0);
        assert_eq!(t.force, 0.0);
    }

    #[test]
    fn touch_list_operations() {
        let mut list = TouchList::new();
        assert_eq!(list.length(), 0);
        list.add(Touch::new(1, 10.0, 20.0));
        list.add(Touch::new(2, 30.0, 40.0));
        assert_eq!(list.length(), 2);
        assert_eq!(list.item(0).unwrap().identifier, 1);
        assert!(list.find_by_id(2).is_some());
        assert!(list.remove_by_id(1));
        assert_eq!(list.length(), 1);
    }

    #[test]
    fn touch_event_type_strings() {
        assert_eq!(TouchEventType::TouchStart.as_str(), "touchstart");
        assert_eq!(TouchEventType::TouchMove.as_str(), "touchmove");
        assert_eq!(TouchEventType::TouchEnd.as_str(), "touchend");
        assert_eq!(TouchEventType::TouchCancel.as_str(), "touchcancel");
    }

    #[test]
    fn touch_event_prevent_default() {
        let mut e = TouchEvent::new(TouchEventType::TouchStart);
        assert!(!e.default_prevented);
        e.prevent_default();
        assert!(e.default_prevented);
    }

    #[test]
    fn gesture_tap() {
        let mut gr = GestureRecognizer::new();
        let start = Touch::new(1, 100.0, 100.0);
        let end = Touch::new(1, 102.0, 101.0);
        gr.on_touch_start(&start);
        let gesture = gr.on_touch_end(&end);
        assert!(matches!(gesture, Some(Gesture::Tap { .. })));
    }

    #[test]
    fn gesture_swipe_right() {
        let mut gr = GestureRecognizer::new();
        let start = Touch::new(1, 100.0, 100.0);
        let end = Touch::new(1, 200.0, 105.0);
        gr.on_touch_start(&start);
        let gesture = gr.on_touch_end(&end);
        assert!(matches!(
            gesture,
            Some(Gesture::Swipe {
                direction: SwipeDirection::Right,
                ..
            })
        ));
    }

    #[test]
    fn gesture_swipe_up() {
        let mut gr = GestureRecognizer::new();
        let start = Touch::new(1, 100.0, 200.0);
        let end = Touch::new(1, 105.0, 100.0);
        gr.on_touch_start(&start);
        let gesture = gr.on_touch_end(&end);
        assert!(matches!(
            gesture,
            Some(Gesture::Swipe {
                direction: SwipeDirection::Up,
                ..
            })
        ));
    }

    #[test]
    fn gesture_reset() {
        let mut gr = GestureRecognizer::new();
        gr.on_touch_start(&Touch::new(1, 0.0, 0.0));
        gr.reset();
        let result = gr.on_touch_end(&Touch::new(1, 0.0, 0.0));
        assert!(result.is_none());
    }
}
