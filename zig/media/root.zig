// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_media_zig — Media codec FFI (ffmpeg/dav1d), hardware decode, audio output.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");

pub const ffmpeg = @import("ffmpeg.zig");
pub const hw_decode = @import("hw_decode.zig");
pub const audio_output = @import("audio_output.zig");

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
