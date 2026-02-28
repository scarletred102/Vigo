// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM event types — the kinds of events that can be dispatched.

use std::fmt;

/// The type of a DOM event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Click,
    MouseDown,
    MouseUp,
    MouseMove,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
    Input,
    Change,
    Submit,
    Load,
    DomContentLoaded,
    Scroll,
    Resize,
    /// A custom event type.
    Custom(String),
}

impl EventType {
    /// The JS-compatible event name string.
    pub fn name(&self) -> &str {
        match self {
            Self::Click => "click",
            Self::MouseDown => "mousedown",
            Self::MouseUp => "mouseup",
            Self::MouseMove => "mousemove",
            Self::KeyDown => "keydown",
            Self::KeyUp => "keyup",
            Self::Focus => "focus",
            Self::Blur => "blur",
            Self::Input => "input",
            Self::Change => "change",
            Self::Submit => "submit",
            Self::Load => "load",
            Self::DomContentLoaded => "DOMContentLoaded",
            Self::Scroll => "scroll",
            Self::Resize => "resize",
            Self::Custom(name) => name,
        }
    }

    /// Whether this event type bubbles by default.
    pub fn bubbles(&self) -> bool {
        matches!(
            self,
            Self::Click
                | Self::MouseDown
                | Self::MouseUp
                | Self::MouseMove
                | Self::KeyDown
                | Self::KeyUp
                | Self::Input
                | Self::Change
                | Self::Submit
                | Self::Scroll
        )
    }

    /// Whether this event type is cancelable by default.
    pub fn cancelable(&self) -> bool {
        matches!(
            self,
            Self::Click
                | Self::MouseDown
                | Self::MouseUp
                | Self::KeyDown
                | Self::KeyUp
                | Self::Submit
        )
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names() {
        assert_eq!(EventType::Click.name(), "click");
        assert_eq!(EventType::DomContentLoaded.name(), "DOMContentLoaded");
        assert_eq!(EventType::Custom("myevent".into()).name(), "myevent");
    }

    #[test]
    fn bubbling_events() {
        assert!(EventType::Click.bubbles());
        assert!(!EventType::Focus.bubbles());
        assert!(!EventType::Load.bubbles());
    }

    #[test]
    fn cancelable_events() {
        assert!(EventType::Click.cancelable());
        assert!(EventType::Submit.cancelable());
        assert!(!EventType::Scroll.cancelable());
    }
}
