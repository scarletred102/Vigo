// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// vex_platform — Native windowing, event loop, DPI, raw handles.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

/// Result code: 0 = success, negative = error.
pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

// ── C ABI Exports ─────────────────────────────────────────────────

/// Initialize the platform layer. Call once at startup.
export fn vex_platform_init() callconv(.C) VexResult {
    return VEX_OK;
}

/// Shut down the platform layer. Call once at exit.
export fn vex_platform_shutdown() callconv(.C) VexResult {
    return VEX_OK;
}

// ── Tests ─────────────────────────────────────────────────────────

test "platform_init returns OK" {
    const result = vex_platform_init();
    try std.testing.expectEqual(VEX_OK, result);
}

test "platform_shutdown returns OK" {
    const result = vex_platform_shutdown();
    try std.testing.expectEqual(VEX_OK, result);
}
