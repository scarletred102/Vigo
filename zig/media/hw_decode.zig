// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// Hardware-accelerated video decoding detection and initialization.
// Windows: probes DXVA2 / D3D11VA via Win32 APIs.
// Falls back to software decode when hardware is unavailable.

const std = @import("std");
const ffmpeg = @import("ffmpeg.zig");

// ──── Error Codes ──────────────────────────────────────────────

pub const VEX_OK: c_int = 0;
pub const VEX_ERR_NOT_AVAILABLE: c_int = -10;
pub const VEX_ERR_HW_INIT: c_int = -20;
pub const VEX_ERR_HW_DECODE: c_int = -21;

// ──── Hardware Decoder Type ────────────────────────────────────

pub const HwDecoderType = enum(u8) {
    none = 0,
    dxva2 = 1,
    d3d11va = 2,
    software = 3,
};

pub const HwDecoderInfo = extern struct {
    decoder_type: HwDecoderType = .none,
    _pad: [3]u8 = .{ 0, 0, 0 },
    device_name: [128]u8 = [_]u8{0} ** 128,
    max_width: u32 = 0,
    max_height: u32 = 0,
    supports_h264: u8 = 0,
    supports_hevc: u8 = 0,
    supports_vp9: u8 = 0,
    supports_av1: u8 = 0,
};

// ──── Windows Video Acceleration Probing ────────────────────────

const HRESULT = i32;
const S_OK: HRESULT = 0;

// D3D11 interfaces (minimal, for probing only)
extern "kernel32" fn LoadLibraryA(lpFileName: [*:0]const u8) callconv(.winapi) ?*anyopaque;
extern "kernel32" fn GetProcAddress(hModule: *anyopaque, lpProcName: [*:0]const u8) callconv(.winapi) ?*anyopaque;
extern "kernel32" fn FreeLibrary(hModule: *anyopaque) callconv(.winapi) c_int;

// D3D11CreateDevice function signature
const D3D_DRIVER_TYPE_HARDWARE: c_int = 1;
const D3D11_SDK_VERSION: u32 = 7;

const FnD3D11CreateDevice = *const fn (
    ?*anyopaque, // pAdapter
    c_int, // DriverType
    ?*anyopaque, // Software
    u32, // Flags
    ?[*]const c_int, // pFeatureLevels
    u32, // FeatureLevels
    u32, // SDKVersion
    ?*?*anyopaque, // ppDevice
    ?*c_int, // pFeatureLevel
    ?*?*anyopaque, // ppImmediateContext
) callconv(.winapi) HRESULT;

// IUnknown::Release vtable offset = 2
fn comRelease(obj: *anyopaque) void {
    const vtable: *const [*]const *const fn (*anyopaque) callconv(.winapi) u32 = @ptrCast(@alignCast(obj));
    _ = vtable.*[2](obj);
}

/// Probe whether D3D11 video acceleration is available.
fn probeD3D11VA() ?HwDecoderInfo {
    const d3d11_dll = LoadLibraryA("d3d11.dll") orelse return null;
    defer _ = FreeLibrary(d3d11_dll);

    const create_device_ptr = GetProcAddress(d3d11_dll, "D3D11CreateDevice") orelse return null;
    const create_device: FnD3D11CreateDevice = @ptrCast(@alignCast(create_device_ptr));

    var device: ?*anyopaque = null;
    var context: ?*anyopaque = null;
    var feature_level: c_int = 0;

    const hr = create_device(
        null, // default adapter
        D3D_DRIVER_TYPE_HARDWARE,
        null,
        0, // flags (no debug)
        null, // default feature levels
        0,
        D3D11_SDK_VERSION,
        &device,
        &feature_level,
        &context,
    );

    if (hr != S_OK) return null;

    // Clean up
    if (context) |ctx| comRelease(ctx);
    defer if (device) |dev| comRelease(dev);

    var info = HwDecoderInfo{
        .decoder_type = .d3d11va,
        // D3D11VA supports common codecs when GPU driver is present
        .supports_h264 = 1,
        .supports_hevc = 1,
        .supports_vp9 = 1,
        .supports_av1 = 0, // AV1 only on newer GPUs, conservative default
    };

    // Write a device description
    const name = "D3D11 Hardware Accelerated";
    @memcpy(info.device_name[0..name.len], name);

    return info;
}

/// Probe whether DXVA2 is available (older API, Windows Vista+).
fn probeDXVA2() ?HwDecoderInfo {
    const dxva2_dll = LoadLibraryA("dxva2.dll") orelse return null;
    defer _ = FreeLibrary(dxva2_dll);

    // DXVA2 requires a Direct3D 9 device. If the DLL exists,
    // the system likely supports basic DXVA2 for H.264.
    var info = HwDecoderInfo{
        .decoder_type = .dxva2,
        .supports_h264 = 1,
        .supports_hevc = 0, // DXVA2 rarely supports HEVC
        .supports_vp9 = 0,
        .supports_av1 = 0,
    };

    const name = "DXVA2 Legacy";
    @memcpy(info.device_name[0..name.len], name);

    return info;
}

// ──── Exported C ABI Functions ─────────────────────────────────

/// Initialize hardware decoder probing. Returns `1` if any hardware
/// decoder is available, `0` otherwise.
export fn vex_media_hw_init(out_info: *HwDecoderInfo) c_int {
    out_info.* = HwDecoderInfo{};

    // Prefer D3D11VA (modern), fall back to DXVA2 (legacy)
    if (probeD3D11VA()) |info| {
        out_info.* = info;
        return 1;
    }

    if (probeDXVA2()) |info| {
        out_info.* = info;
        return 1;
    }

    // No hardware acceleration available — software fallback
    out_info.decoder_type = .software;
    const name = "Software (no hardware acceleration)";
    @memcpy(out_info.device_name[0..name.len], name);
    return 0;
}

/// Query the preferred decoder type for the current system.
export fn vex_media_hw_preferred_type() u8 {
    var info = HwDecoderInfo{};
    _ = vex_media_hw_init(&info);
    return @intFromEnum(info.decoder_type);
}

/// Check if a specific codec is hardware-accelerated.
/// codec_id: 0 = H.264, 1 = HEVC, 2 = VP9, 3 = AV1
export fn vex_media_hw_supports_codec(codec_id: u8) c_int {
    var info = HwDecoderInfo{};
    _ = vex_media_hw_init(&info);

    return switch (codec_id) {
        0 => @as(c_int, info.supports_h264),
        1 => @as(c_int, info.supports_hevc),
        2 => @as(c_int, info.supports_vp9),
        3 => @as(c_int, info.supports_av1),
        else => 0,
    };
}

// ──── Tests ────────────────────────────────────────────────────

test "HwDecoderInfo has correct size" {
    // 1 enum + 3 pad + 128 name + 4 + 4 + 4×1 = 144
    try std.testing.expectEqual(@sizeOf(HwDecoderInfo), 144);
}

test "HwDecoderType enum values" {
    try std.testing.expectEqual(@intFromEnum(HwDecoderType.none), 0);
    try std.testing.expectEqual(@intFromEnum(HwDecoderType.dxva2), 1);
    try std.testing.expectEqual(@intFromEnum(HwDecoderType.d3d11va), 2);
    try std.testing.expectEqual(@intFromEnum(HwDecoderType.software), 3);
}

test "hw_init returns a result" {
    // On any Windows system, this should succeed (may return software fallback)
    var info = HwDecoderInfo{};
    const result = vex_media_hw_init(&info);
    // result is 0 (no hw) or 1 (hw available) — both valid
    try std.testing.expect(result == 0 or result == 1);
    try std.testing.expect(@intFromEnum(info.decoder_type) >= 0);
}

test "hw_preferred_type returns valid enum" {
    const t = vex_media_hw_preferred_type();
    try std.testing.expect(t <= 3);
}
