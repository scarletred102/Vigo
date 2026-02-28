// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! FFI declarations matching the Zig platform C ABI exports.

#[allow(dead_code)] // All variants are used at the FFI boundary.
/// C-compatible event tag (mirrors Zig `EventTag`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTag {
    None = 0,
    WindowClose = 1,
    WindowResize = 2,
    WindowFocus = 3,
    KeyDown = 4,
    KeyUp = 5,
    MouseMove = 6,
    MouseButtonDown = 7,
    MouseButtonUp = 8,
    MouseScroll = 9,
}

/// C-compatible event struct (mirrors Zig `Event`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EventC {
    pub tag: EventTag,
    pub _pad: [u8; 3],
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

impl Default for EventC {
    fn default() -> Self {
        Self {
            tag: EventTag::None,
            _pad: [0; 3],
            a: 0,
            b: 0,
            c: 0,
        }
    }
}

/// C-compatible window config (mirrors Zig `WindowConfig`).
#[repr(C)]
pub struct WindowConfigC {
    pub title: *const u16,
    pub width: u32,
    pub height: u32,
    pub resizable: u8,
}

/// Raw window handle returned by the platform layer.
#[repr(C)]
pub struct RawHandleC {
    pub hwnd: *mut std::ffi::c_void,
    pub hinstance: *mut std::ffi::c_void,
}

impl Default for RawHandleC {
    fn default() -> Self {
        Self {
            hwnd: std::ptr::null_mut(),
            hinstance: std::ptr::null_mut(),
        }
    }
}

// SAFETY: All these functions are implemented in Zig, compiled to a static
// library. They follow C ABI and are safe to call with valid arguments.
unsafe extern "C" {
    pub fn vex_platform_create_window(config: *const WindowConfigC) -> *mut std::ffi::c_void;
    pub fn vex_platform_destroy_window(handle: *mut std::ffi::c_void);
    pub fn vex_platform_poll_event(out: *mut EventC) -> bool;
    pub fn vex_platform_get_dpi(handle: *mut std::ffi::c_void) -> f32;
    pub fn vex_platform_get_raw_handle(handle: *mut std::ffi::c_void, out: *mut RawHandleC);
}
