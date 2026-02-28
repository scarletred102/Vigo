// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// vex_compositor — GPU composition, display list rasterization.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the compositor.
export fn vex_compositor_init() callconv(.C) VexResult {
    return VEX_OK;
}

/// Shut down the compositor.
export fn vex_compositor_shutdown() callconv(.C) VexResult {
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
