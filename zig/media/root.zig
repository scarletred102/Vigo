// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// vex_media_zig — Media codec FFI (ffmpeg/dav1d), hardware decode.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the media subsystem.
export fn vex_media_init() VexResult {
    return VEX_OK;
}

/// Shut down the media subsystem.
export fn vex_media_shutdown() VexResult {
    return VEX_OK;
}

test "media_init returns OK" {
    const result = vex_media_init();
    try std.testing.expectEqual(VEX_OK, result);
}

test "media_shutdown returns OK" {
    const result = vex_media_shutdown();
    try std.testing.expectEqual(VEX_OK, result);
}
