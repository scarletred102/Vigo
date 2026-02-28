// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// vex_alloc — Custom allocators: arena, pool, frame, stats.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the allocator subsystem.
export fn vex_alloc_init() callconv(.C) VexResult {
    return VEX_OK;
}

/// Shut down the allocator subsystem.
export fn vex_alloc_shutdown() callconv(.C) VexResult {
    return VEX_OK;
}

test "alloc_init returns OK" {
    const result = vex_alloc_init();
    try std.testing.expectEqual(VEX_OK, result);
}

test "alloc_shutdown returns OK" {
    const result = vex_alloc_shutdown();
    try std.testing.expectEqual(VEX_OK, result);
}
