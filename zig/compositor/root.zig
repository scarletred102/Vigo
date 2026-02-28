// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_compositor — GPU composition, display list rasterization.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the compositor.
export fn vex_compositor_init() VexResult {
    return VEX_OK;
}

/// Shut down the compositor.
export fn vex_compositor_shutdown() VexResult {
    return VEX_OK;
}

test "compositor_init returns OK" {
    const result = vex_compositor_init();
    try std.testing.expectEqual(VEX_OK, result);
}

test "compositor_shutdown returns OK" {
    const result = vex_compositor_shutdown();
    try std.testing.expectEqual(VEX_OK, result);
}
