// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Minimal platform smoke binary.
//! Builds against the platform module and opens a native test window.

const std = @import("std");
const event = @import("event.zig");
const window = @import("window.zig");

pub fn main() !void {
    const title = std.unicode.utf8ToUtf16LeStringLiteral("Vex Platform Test Window");

    const config = window.WindowConfig{
        .title = title,
        .width = 960,
        .height = 540,
        .resizable = 1,
    };

    const hwnd = window.createWindow(&config) orelse return error.CreateWindowFailed;
    defer window.destroyWindow(hwnd);

    var event_buf: event.Event = .{};
    while (true) {
        while (window.pollEvent(&event_buf)) {
            if (event_buf.tag == .window_close) {
                return;
            }
        }
        std.Thread.sleep(16 * std.time.ns_per_ms);
    }
}
