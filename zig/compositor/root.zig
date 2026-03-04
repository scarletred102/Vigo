// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// vex_compositor — GPU composition, display list rasterization.
// Exports C ABI for Rust FFI consumption.
//
// STATUS: DEPRECATED — Composition is handled entirely by the Rust-side wgpu
// renderer (crates/vex-render/src/renderer.rs). This module is retained for
// backward compatibility and Zig test coverage. No new functionality should
// be added here.
//
// The wgpu renderer provides:
//   - Rect pipeline (instanced, SDF rounded corners, border-radius)
//   - Text pipeline (glyph atlas, cosmic-text)
//   - Image pipeline (image atlas, 4096×4096 RGBA)
//   - Opacity stack (PushOpacity/PopOpacity)
//   - Clip stack (PushClip/PopClip)
//   - Scroll offset via camera uniform
//
// If a Zig-side compositor is needed in the future (e.g., for offscreen
// layer compositing or hardware video overlay blending), this module can
// be revived with the appropriate display list rasterization logic.

const std = @import("std");

pub const VexResult = c_int;
const VEX_OK: VexResult = 0;

/// Initialize the compositor (no-op — deprecated).
export fn vex_compositor_init() VexResult {
    return VEX_OK;
}

/// Shut down the compositor (no-op — deprecated).
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
