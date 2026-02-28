// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Safe wrapper around the Zig platform windowing layer.

use std::num::NonZeroIsize;
use std::sync::atomic::{AtomicU32, Ordering};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use vex_core::{VexError, VexResult};

use crate::event::{Event, MouseButton};
use crate::platform_ffi::{self, EventC, EventTag, RawHandleC, WindowConfigC};

/// A native window managed by the Zig platform layer.
pub struct Window {
    handle: *mut std::ffi::c_void,
    width: AtomicU32,
    height: AtomicU32,
}

// SAFETY: Window is only accessed from the main thread.
// The raw pointer is to a Zig-managed struct that lives for the
// window's lifetime.  wgpu requires Send+Sync for surface creation.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    /// Create a new native window.
    pub fn new(title: &str, width: u32, height: u32) -> VexResult<Self> {
        // Encode title as null-terminated UTF-16 for Win32.
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let config = WindowConfigC {
            title: title_wide.as_ptr(),
            width,
            height,
            resizable: 1,
        };

        // SAFETY: We pass a valid config struct. The Zig function either
        // returns a valid handle or null.
        let handle = unsafe { platform_ffi::vex_platform_create_window(&config) };
        if handle.is_null() {
            return Err(VexError::Platform("failed to create window".into()));
        }

        Ok(Self {
            handle,
            width: AtomicU32::new(width),
            height: AtomicU32::new(height),
        })
    }

    /// Poll the next platform event. Returns `None` if the queue is empty.
    pub fn poll_event(&self) -> Option<Event> {
        let mut raw = EventC::default();

        // SAFETY: We pass a valid pointer to EventC.
        let has_event = unsafe { platform_ffi::vex_platform_poll_event(&mut raw) };
        if !has_event {
            return None;
        }

        Some(match raw.tag {
            EventTag::WindowClose => Event::WindowClose,
            EventTag::WindowResize => {
                let w = raw.a as u32;
                let h = raw.b as u32;
                self.width.store(w, Ordering::Relaxed);
                self.height.store(h, Ordering::Relaxed);
                Event::WindowResize {
                    width: w,
                    height: h,
                }
            }
            EventTag::WindowFocus => Event::WindowFocus {
                focused: raw.a != 0,
            },
            EventTag::KeyDown => Event::KeyDown {
                keycode: raw.a as u32,
                modifiers: raw.b as u32,
            },
            EventTag::KeyUp => Event::KeyUp {
                keycode: raw.a as u32,
                modifiers: raw.b as u32,
            },
            EventTag::MouseMove => Event::MouseMove {
                x: raw.a,
                y: raw.b,
            },
            EventTag::MouseButtonDown => Event::MouseButtonDown {
                button: MouseButton::from_raw(raw.c as u8),
                x: raw.a,
                y: raw.b,
            },
            EventTag::MouseButtonUp => Event::MouseButtonUp {
                button: MouseButton::from_raw(raw.c as u8),
                x: raw.a,
                y: raw.b,
            },
            EventTag::MouseScroll => Event::MouseScroll {
                dx: f32::from_bits(raw.a as u32),
                dy: f32::from_bits(raw.b as u32),
            },
            EventTag::None => return None,
        })
    }

    /// Current DPI scale factor.
    pub fn dpi_scale(&self) -> f32 {
        // SAFETY: valid handle.
        unsafe { platform_ffi::vex_platform_get_dpi(self.handle) }
    }

    /// Raw platform handle (HWND + HINSTANCE on Windows).
    pub fn raw_handle(&self) -> RawHandleC {
        let mut raw = RawHandleC::default();
        // SAFETY: valid handle + valid out pointer.
        unsafe { platform_ffi::vex_platform_get_raw_handle(self.handle, &mut raw) };
        raw
    }

    pub fn width(&self) -> u32 {
        self.width.load(Ordering::Relaxed)
    }

    pub fn height(&self) -> u32 {
        self.height.load(Ordering::Relaxed)
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: We own the handle and only destroy it once.
        unsafe { platform_ffi::vex_platform_destroy_window(self.handle) };
    }
}

// ── raw-window-handle integration ────────────────────────────────────

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = self.raw_handle();
        let hwnd = raw.hwnd as isize;
        let hwnd_nz = NonZeroIsize::new(hwnd).expect("HWND must not be null");
        let mut win32 = Win32WindowHandle::new(hwnd_nz);
        let hinstance = raw.hinstance as isize;
        if let Some(nz) = NonZeroIsize::new(hinstance) {
            win32.hinstance = Some(nz);
        }
        // SAFETY: The HWND is valid for the lifetime of `&self`.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(win32)) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows display handle is a zero-sized marker.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}
