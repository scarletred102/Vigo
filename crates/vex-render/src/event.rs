// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Input and window events.

/// A high-level event produced by the platform layer.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    WindowClose,
    WindowResize { width: u32, height: u32 },
    WindowFocus { focused: bool },
    KeyDown { keycode: u32, modifiers: u32 },
    KeyUp { keycode: u32, modifiers: u32 },
    MouseMove { x: i32, y: i32 },
    MouseButtonDown { button: MouseButton, x: i32, y: i32 },
    MouseButtonUp { button: MouseButton, x: i32, y: i32 },
    MouseScroll { dx: f32, dy: f32 },
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

impl MouseButton {
    pub(crate) fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::Left,
            1 => Self::Right,
            2 => Self::Middle,
            n => Self::Other(n),
        }
    }
}
