// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// Event types shared between Zig platform layer and Rust FFI.

/// Tag for the C-compatible event union.
pub const EventTag = enum(u8) {
    none = 0,
    window_close = 1,
    window_resize = 2,
    window_focus = 3,
    key_down = 4,
    key_up = 5,
    mouse_move = 6,
    mouse_button_down = 7,
    mouse_button_up = 8,
    mouse_scroll = 9,
};

/// C-compatible event struct passed across FFI boundary.
/// Laid out explicitly so Rust can read it without ambiguity.
pub const Event = extern struct {
    tag: EventTag = .none,
    _pad: [3]u8 = .{ 0, 0, 0 },

    // All payloads share one flat region.
    a: i32 = 0, // width  | keycode  | x  | focused(0/1)
    b: i32 = 0, // height | mods     | y  | —
    c: i32 = 0, // —      | —        | button | —
    // Scroll uses f32 reinterpreted as i32 bits.
    // Use helpers below.

    pub fn resize(w: u32, h: u32) Event {
        return .{ .tag = .window_resize, .a = @bitCast(w), .b = @bitCast(h) };
    }

    pub fn focus(focused: bool) Event {
        return .{ .tag = .window_focus, .a = @intFromBool(focused) };
    }

    pub fn keyDown(keycode: u32, mods: u32) Event {
        return .{ .tag = .key_down, .a = @bitCast(keycode), .b = @bitCast(mods) };
    }

    pub fn keyUp(keycode: u32, mods: u32) Event {
        return .{ .tag = .key_up, .a = @bitCast(keycode), .b = @bitCast(mods) };
    }

    pub fn mouseMove(x: i32, y: i32) Event {
        return .{ .tag = .mouse_move, .a = x, .b = y };
    }

    pub fn mouseButtonDown(button: u8, x: i32, y: i32) Event {
        return .{ .tag = .mouse_button_down, .a = x, .b = y, .c = button };
    }

    pub fn mouseButtonUp(button: u8, x: i32, y: i32) Event {
        return .{ .tag = .mouse_button_up, .a = x, .b = y, .c = button };
    }

    pub fn scroll(dx: f32, dy: f32) Event {
        return .{ .tag = .mouse_scroll, .a = @bitCast(dx), .b = @bitCast(dy) };
    }

    pub fn close() Event {
        return .{ .tag = .window_close };
    }
};

// ── Tests ─────────────────────────────────────────────────────────

const testing = @import("std").testing;

test "resize event" {
    const e = Event.resize(800, 600);
    try testing.expectEqual(EventTag.window_resize, e.tag);
    try testing.expectEqual(@as(u32, 800), @as(u32, @bitCast(e.a)));
    try testing.expectEqual(@as(u32, 600), @as(u32, @bitCast(e.b)));
}

test "close event" {
    const e = Event.close();
    try testing.expectEqual(EventTag.window_close, e.tag);
}

test "mouse_move event" {
    const e = Event.mouseMove(100, 200);
    try testing.expectEqual(EventTag.mouse_move, e.tag);
    try testing.expectEqual(@as(i32, 100), e.a);
    try testing.expectEqual(@as(i32, 200), e.b);
}
