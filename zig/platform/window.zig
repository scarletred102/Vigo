// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// Win32 windowing — create window, poll events, DPI, raw handle.

const std = @import("std");
const event = @import("event.zig");
const Event = event.Event;

const windows = std.os.windows;

// ── Win32 type aliases ────────────────────────────────────────────

const HWND = windows.HWND;
const HINSTANCE = windows.HINSTANCE;
// Zig 0.16 removed these two legacy aliases from `std.os.windows`.
// Their Win32 definitions are pointer-sized signed/unsigned integers, which
// have remained available as `LONG_PTR` and `ULONG_PTR` across supported Zig
// versions.
const LRESULT = windows.LONG_PTR;
const WPARAM = windows.ULONG_PTR;
const LPARAM = windows.LPARAM;
const UINT = c_uint;
// Keep these ABI-only C definitions local. Zig 0.16 replaced the old integer
// `BOOL` and stopped exporting `RECT`, while the Win32 ABI itself is unchanged.
const BOOL = c_int;
const RECT = extern struct {
    left: i32 = 0,
    top: i32 = 0,
    right: i32 = 0,
    bottom: i32 = 0,
};

const HDC = *opaque {};
const HBRUSH = *opaque {};
const HCURSOR = *opaque {};
const HICON = *opaque {};
const HMENU = *opaque {};
const HMONITOR = *opaque {};
const PAINTSTRUCT = extern struct {
    hdc: ?HDC = null,
    fErase: BOOL = 0,
    rcPaint: RECT = .{ .left = 0, .top = 0, .right = 0, .bottom = 0 },
    fRestore: BOOL = 0,
    fIncUpdate: BOOL = 0,
    rgbReserved: [32]u8 = std.mem.zeroes([32]u8),
};
const MONITORINFO = extern struct {
    cbSize: u32 = @sizeOf(MONITORINFO),
    rcMonitor: RECT = .{},
    rcWork: RECT = .{},
    dwFlags: u32 = 0,
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
const WS_POPUP: u32 = 0x80000000;
const CW_USEDEFAULT: i32 = @bitCast(@as(u32, 0x80000000));
const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;
const COLOR_WINDOW: c_int = 5;
const IDC_ARROW: [*:0]align(1) const u16 = @ptrFromInt(32512);
const PM_REMOVE: UINT = 0x0001;
const WHEEL_DELTA: i16 = 120;
const GWL_STYLE: c_int = -16;
const MONITOR_DEFAULTTONEAREST: u32 = 2;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOOWNERZORDER: u32 = 0x0200;
const SWP_FRAMECHANGED: u32 = 0x0020;

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
extern "user32" fn GetKeyState(c_int) callconv(.winapi) i16;
extern "user32" fn SetWindowTextW(HWND, [*:0]const u16) callconv(.winapi) BOOL;
extern "user32" fn GetWindowLongPtrW(HWND, c_int) callconv(.winapi) windows.LONG_PTR;
extern "user32" fn SetWindowLongPtrW(HWND, c_int, windows.LONG_PTR) callconv(.winapi) windows.LONG_PTR;
extern "user32" fn GetWindowRect(HWND, *RECT) callconv(.winapi) BOOL;
extern "user32" fn SetWindowPos(HWND, ?HWND, c_int, c_int, c_int, c_int, u32) callconv(.winapi) BOOL;
extern "user32" fn MonitorFromWindow(HWND, u32) callconv(.winapi) ?HMONITOR;
extern "user32" fn GetMonitorInfoW(HMONITOR, *MONITORINFO) callconv(.winapi) BOOL;
extern "kernel32" fn GetModuleHandleW(?[*:0]const u16) callconv(.winapi) ?HINSTANCE;

// ── Virtual key codes for modifier detection ──────────────────────

const VK_SHIFT: c_int = 0x10;
const VK_CONTROL: c_int = 0x11;
const VK_MENU: c_int = 0x12; // Alt

/// Build a modifier bitmask from the current keyboard state.
/// Bit 0 = Ctrl, Bit 1 = Shift, Bit 2 = Alt.
fn getModifiers() u32 {
    var mods: u32 = 0;
    if (GetKeyState(VK_CONTROL) < 0) mods |= 0x01;
    if (GetKeyState(VK_SHIFT) < 0) mods |= 0x02;
    if (GetKeyState(VK_MENU) < 0) mods |= 0x04;
    return mods;
}

// ── Thread-local event queue ──────────────────────────────────────

const MAX_EVENTS = 64;
var event_ring: [MAX_EVENTS]Event = @splat(Event{});
var ring_head: usize = 0;
var ring_tail: usize = 0;
var fullscreen_active = false;
var fullscreen_style: windows.LONG_PTR = 0;
var fullscreen_rect = RECT{};

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
            pushEvent(Event.keyDown(@truncate(@as(usize, @bitCast(wp))), getModifiers()));
            return 0;
        },
        WM_KEYUP => {
            pushEvent(Event.keyUp(@truncate(@as(usize, @bitCast(wp))), getModifiers()));
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

/// Update the window title bar text.
///
/// `title` is a null-terminated UTF-8 string from Rust.
/// We convert to wide UTF-16 for Win32's `SetWindowTextW`.
pub fn setTitle(hwnd: HWND, title: [*:0]const u8) void {
    // Convert UTF-8 to UTF-16 with a fixed stack buffer (512 wide chars).
    var buf: [512]u16 = undefined;
    var i: usize = 0;
    var cursor: usize = 0;
    while (title[cursor] != 0 and i < buf.len - 1) {
        const byte = title[cursor];
        if (byte < 0x80) {
            buf[i] = byte;
            i += 1;
            cursor += 1;
        } else {
            // Multi-byte UTF-8: replace with '?' for simplicity.
            // Full UTF-8 → UTF-16 is handled in Rust-side before calling
            // platform APIs if needed.
            buf[i] = '?';
            i += 1;
            cursor += 1;
            // Skip continuation bytes.
            while (title[cursor] != 0 and (title[cursor] & 0xC0) == 0x80) {
                cursor += 1;
            }
        }
    }
    buf[i] = 0;
    const wide_ptr: [*:0]const u16 = @ptrCast(&buf);
    _ = SetWindowTextW(hwnd, wide_ptr);
}

/// Toggle borderless fullscreen for the primary browser window.
/// The previous window rectangle and style are retained so F11 restores the
/// exact placement the user had before entering fullscreen.
pub fn setFullscreen(hwnd: HWND, is_enabled: bool) bool {
    if (is_enabled == fullscreen_active) {
        return true;
    }

    if (is_enabled) {
        var monitor_info = MONITORINFO{};
        const monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) orelse return false;
        if (GetWindowRect(hwnd, &fullscreen_rect) == 0 or GetMonitorInfoW(monitor, &monitor_info) == 0) {
            return false;
        }
        fullscreen_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        _ = SetWindowLongPtrW(hwnd, GWL_STYLE, @intCast(WS_POPUP | WS_VISIBLE));
        if (SetWindowPos(
            hwnd,
            null,
            monitor_info.rcMonitor.left,
            monitor_info.rcMonitor.top,
            monitor_info.rcMonitor.right - monitor_info.rcMonitor.left,
            monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top,
            SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
        ) == 0) return false;
        fullscreen_active = true;
        return true;
    }

    _ = SetWindowLongPtrW(hwnd, GWL_STYLE, fullscreen_style);
    if (SetWindowPos(
        hwnd,
        null,
        fullscreen_rect.left,
        fullscreen_rect.top,
        fullscreen_rect.right - fullscreen_rect.left,
        fullscreen_rect.bottom - fullscreen_rect.top,
        SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
    ) == 0) return false;
    fullscreen_active = false;
    return true;
}
