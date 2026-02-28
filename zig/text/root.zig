// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_text — SIMD text rasterization, HarfBuzz/FreeType FFI.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the text subsystem.
export fn vex_text_init() VexResult {
    return VEX_OK;
}

/// Shut down the text subsystem.
export fn vex_text_shutdown() VexResult {
    return VEX_OK;
}

test "text_init returns OK" {
    const result = vex_text_init();
    try std.testing.expectEqual(VEX_OK, result);
}

test "text_shutdown returns OK" {
    const result = vex_text_shutdown();
    try std.testing.expectEqual(VEX_OK, result);
}
