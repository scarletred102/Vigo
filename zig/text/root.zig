// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_text — SIMD text rasterization, HarfBuzz/FreeType FFI.
// Exports C ABI for Rust FFI consumption.
//
// STATUS: DEPRECATED — Text shaping and glyph rasterization is handled
// entirely by cosmic-text 0.12 on the Rust side
// (crates/vex-render/src/glyph_atlas.rs). The glyph atlas uses a 2048×2048
// R8Unorm texture with shelf packing and LRU eviction.
//
// cosmic-text provides:
//   - System font discovery (fontdb)
//   - HarfBuzz shaping (built-in)
//   - SwashCache for glyph rasterization (get_image_uncached)
//   - Line breaking and bidi support
//
// If a Zig-side text renderer is needed in the future (e.g., for SIMD-
// accelerated subpixel rasterization or ClearType-style rendering), this
// module can be revived with FreeType/HarfBuzz bindings.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the text subsystem (no-op — deprecated).
export fn vex_text_init() VexResult {
    return VEX_OK;
}

/// Shut down the text subsystem (no-op — deprecated).
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
