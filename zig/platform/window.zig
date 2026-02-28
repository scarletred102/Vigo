// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// Win32 windowing — create window, poll events, DPI, raw handle.

const std = @import("std");
const event = @import("event.zig");
const Event = event.Event;

const windows = std.os.windows;

// ── Win32 type aliases ────────────────────────────────────────────

const HWND = windows.HWND;
const HINSTANCE = windows.HINSTANCE;
const LRESULT = windows.LRESULT;
const WPARAM = windows.WPARAM;
const LPARAM = windows.LPARAM;
const UINT = c_uint;
const BOOL = windows.BOOL;
const RECT = windows.RECT;

const HDC = *opaque {};
const HBRUSH = *opaque {};
const HCURSOR = *opaque {};
const HICON = *opaque {};
const HMENU = *opaque {};
const PAINTSTRUCT = extern struct {
    hdc: ?HDC = null,
    fErase: BOOL = 0,
    rcPaint: RECT = .{ .left = 0, .top = 0, .right = 0, .bottom = 0 },
    fRestore: BOOL = 0,
    fIncUpdate: BOOL = 0,
    rgbReserved: [32]u8 = std.mem.zeroes([32]u8),
};

// ── Win32 constants ───────────────────────────────────────────────

const WM_CLOSE: UINT = 0x0010;
const WM_DESTROY: UINT = 0x0002;
const WM_SIZE: UINT = 0x0005;
const WM_SETFOCUS: UINT = 0x0007;
const WM_KILLFOCUS: UINT = 0x0008;
const WM_KEYDOWN: UINT = 0x0100;
const WM_KEYUP: UINT = 0x0101;
const WM_MOUSEMOVE: UINT = 0x0200;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_LBUTTONUP: UINT = 0x0202;
const WM_RBUTTONDOWN: UINT = 0x0204;
const WM_RBUTTONUP: UINT = 0x0205;
const WM_MBUTTONDOWN: UINT = 0x0207;
const WM_MBUTTONUP: UINT = 0x0208;
const WM_MOUSEWHEEL: UINT = 0x020A;
const WM_PAINT: UINT = 0x000F;

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const WS_VISIBLE: u32 = 0x10000000;
const CW_USEDEFAULT: i32 = @bitCast(@as(u32, 0x80000000));
const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;
const COLOR_WINDOW: c_int = 5;
const IDC_ARROW: [*:0]align(1) const u16 = @ptrFromInt(32512);
const PM_REMOVE: UINT = 0x0001;
const WHEEL_DELTA: i16 = 120;

// ── Win32 externs ─────────────────────────────────────────────────

const WNDCLASSEXW = extern struct {
    cbSize: UINT = @sizeOf(WNDCLASSEXW),
    style: UINT = 0,
    lpfnWndProc: ?*const fn (HWND, UINT, WPARAM, LPARAM) callconv(.winapi) LRESULT = null,
    cbClsExtra: c_int = 0,
    cbWndExtra: c_int = 0,
    hInstance: ?HINSTANCE = null,
    hIcon: ?HICON = null,
    hCursor: ?HCURSOR = null,
    hbrBackground: ?HBRUSH = null,
    lpszMenuName: ?[*:0]const u16 = null,
    lpszClassName: ?[*:0]const u16 = null,
    hIconSm: ?HICON = null,
};

const MSG = extern struct {
    hwnd: ?HWND = null,
    message: UINT = 0,
    wParam: WPARAM = 0,
    lParam: LPARAM = 0,
    time: u32 = 0,
    pt_x: i32 = 0,
    pt_y: i32 = 0,
};

extern "user32" fn RegisterClassExW(*const WNDCLASSEXW) callconv(.winapi) u16;
extern "user32" fn CreateWindowExW(
    u32,
    [*:0]const u16,
    [*:0]const u16,
    u32,
    c_int,
    c_int,
    c_int,
    c_int,
    ?HWND,
    ?HMENU,
    ?HINSTANCE,
    ?*anyopaque,
) callconv(.winapi) ?HWND;
extern "user32" fn DestroyWindow(HWND) callconv(.winapi) BOOL;
extern "user32" fn DefWindowProcW(HWND, UINT, WPARAM, LPARAM) callconv(.winapi) LRESULT;
extern "user32" fn PeekMessageW(*MSG, ?HWND, UINT, UINT, UINT) callconv(.winapi) BOOL;
extern "user32" fn TranslateMessage(*const MSG) callconv(.winapi) BOOL;
extern "user32" fn DispatchMessageW(*const MSG) callconv(.winapi) LRESULT;
extern "user32" fn PostQuitMessage(c_int) callconv(.winapi) void;
extern "user32" fn GetClientRect(HWND, *RECT) callconv(.winapi) BOOL;
extern "user32" fn LoadCursorW(?HINSTANCE, [*:0]align(1) const u16) callconv(.winapi) ?HCURSOR;
extern "user32" fn BeginPaint(HWND, *PAINTSTRUCT) callconv(.winapi) ?HDC;
extern "user32" fn EndPaint(HWND, *const PAINTSTRUCT) callconv(.winapi) BOOL;
extern "user32" fn GetDpiForWindow(HWND) callconv(.winapi) UINT;
extern "kernel32" fn GetModuleHandleW(?[*:0]const u16) callconv(.winapi) ?HINSTANCE;

// ── Thread-local event queue ──────────────────────────────────────

const MAX_EVENTS = 64;
var event_ring: [MAX_EVENTS]Event = @splat(Event{});
var ring_head: usize = 0;
var ring_tail: usize = 0;

fn pushEvent(e: Event) void {
    event_ring[ring_head] = e;
    ring_head = (ring_head + 1) % MAX_EVENTS;
}

fn popEvent() ?Event {
    if (ring_tail == ring_head) return null;
    const e = event_ring[ring_tail];
    ring_tail = (ring_tail + 1) % MAX_EVENTS;
    return e;
}

// ── WndProc ───────────────────────────────────────────────────────

fn wndProc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) callconv(.winapi) LRESULT {
    switch (msg) {
        WM_CLOSE => {
            pushEvent(Event.close());
            return 0;
        },
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        },
        WM_SIZE => {
            const w: u32 = @truncate(@as(usize, @bitCast(lp)) & 0xFFFF);
            const h: u32 = @truncate((@as(usize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.resize(w, h));
            return 0;
        },
        WM_SETFOCUS => {
            pushEvent(Event.focus(true));
            return 0;
        },
        WM_KILLFOCUS => {
            pushEvent(Event.focus(false));
            return 0;
        },
        WM_KEYDOWN => {
            pushEvent(Event.keyDown(@truncate(@as(usize, @bitCast(wp))), 0));
            return 0;
        },
        WM_KEYUP => {
            pushEvent(Event.keyUp(@truncate(@as(usize, @bitCast(wp))), 0));
            return 0;
        },
        WM_MOUSEMOVE => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseMove(x, y));
            return 0;
        },
        WM_LBUTTONDOWN => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonDown(0, x, y));
            return 0;
        },
        WM_LBUTTONUP => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonUp(0, x, y));
            return 0;
        },
        WM_RBUTTONDOWN => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonDown(1, x, y));
            return 0;
        },
        WM_RBUTTONUP => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonUp(1, x, y));
            return 0;
        },
        WM_MBUTTONDOWN => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonDown(2, x, y));
            return 0;
        },
        WM_MBUTTONUP => {
            const x: i32 = @truncate(@as(isize, @bitCast(lp)) & 0xFFFF);
            const y: i32 = @truncate((@as(isize, @bitCast(lp)) >> 16) & 0xFFFF);
            pushEvent(Event.mouseButtonUp(2, x, y));
            return 0;
        },
        WM_MOUSEWHEEL => {
            const delta: i16 = @truncate(@as(isize, @bitCast(wp)) >> 16);
            const dy: f32 = @as(f32, @floatFromInt(delta)) / @as(f32, @floatFromInt(WHEEL_DELTA));
            pushEvent(Event.scroll(0, dy));
            return 0;
        },
        WM_PAINT => {
            var ps = PAINTSTRUCT{};
            _ = BeginPaint(hwnd, &ps);
            _ = EndPaint(hwnd, &ps);
            return 0;
        },
        else => return DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ── Public API ────────────────────────────────────────────────────

pub const WindowConfig = extern struct {
    title: [*:0]const u16,
    width: u32,
    height: u32,
    resizable: u8, // 1 = yes
};

pub const RawHandle = extern struct {
    hwnd: ?*anyopaque = null,
    hinstance: ?*anyopaque = null,
};

const CLASS_NAME = std.unicode.utf8ToUtf16LeStringLiteral("VexWindow");

pub fn createWindow(config: *const WindowConfig) ?HWND {
    const hinstance = GetModuleHandleW(null);

    const wc = WNDCLASSEXW{
        .style = CS_HREDRAW | CS_VREDRAW,
        .lpfnWndProc = wndProc,
        .hInstance = hinstance,
        .hCursor = LoadCursorW(null, IDC_ARROW),
        .hbrBackground = @ptrFromInt(@as(usize, @intCast(COLOR_WINDOW + 1))),
        .lpszClassName = CLASS_NAME,
    };
    _ = RegisterClassExW(&wc);

    return CreateWindowExW(
        0,
        CLASS_NAME,
        config.title,
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        @bitCast(config.width),
        @bitCast(config.height),
        null,
        null,
        hinstance,
        null,
    );
}

pub fn destroyWindow(hwnd: HWND) void {
    _ = DestroyWindow(hwnd);
}

/// Pump Win32 messages and write the next event into `out`.
/// Returns true if an event was available.
pub fn pollEvent(out: *Event) bool {
    // Drain Win32 message queue into our ring buffer.
    var msg = MSG{};
    while (PeekMessageW(&msg, null, 0, 0, PM_REMOVE) != 0) {
        _ = TranslateMessage(&msg);
        _ = DispatchMessageW(&msg);
    }

    if (popEvent()) |e| {
        out.* = e;
        return true;
    }
    return false;
}

pub fn getDpiScale(hwnd: HWND) f32 {
    const dpi = GetDpiForWindow(hwnd);
    return if (dpi > 0) @as(f32, @floatFromInt(dpi)) / 96.0 else 1.0;
}

pub fn getRawHandle(hwnd: HWND) RawHandle {
    return .{
        .hwnd = @ptrCast(hwnd),
        .hinstance = @ptrCast(GetModuleHandleW(null)),
    };
}
