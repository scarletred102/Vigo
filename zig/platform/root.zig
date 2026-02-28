// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_platform — Native windowing, event loop, DPI, raw handles.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");
pub const event = @import("event.zig");
pub const window = @import("window.zig");

const Event = event.Event;
const WindowConfig = window.WindowConfig;
const RawHandle = window.RawHandle;

// ── C ABI Exports ─────────────────────────────────────────────────

export fn vex_platform_create_window(config: *const WindowConfig) ?*anyopaque {
    const hwnd = window.createWindow(config) orelse return null;
    return @ptrCast(hwnd);
}

export fn vex_platform_destroy_window(handle: *anyopaque) void {
    window.destroyWindow(@ptrCast(@alignCast(handle)));
}

export fn vex_platform_poll_event(out: *Event) bool {
    return window.pollEvent(out);
}

export fn vex_platform_get_dpi(handle: *anyopaque) f32 {
    return window.getDpiScale(@ptrCast(@alignCast(handle)));
}

export fn vex_platform_get_raw_handle(handle: *anyopaque, out: *RawHandle) void {
    out.* = window.getRawHandle(@ptrCast(@alignCast(handle)));
}

// ── Tests ─────────────────────────────────────────────────────────

test "event module" {
    _ = event;
}

test "window module compiles" {
    _ = window;
}
