// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// WASAPI audio output for Windows.
// Opens the default audio render device, writes PCM float samples.
// Sample rate conversion handled by caller (vex-media/sync.rs).

const std = @import("std");

// ──── Error Codes ──────────────────────────────────────────────

pub const VEX_OK: c_int = 0;
pub const VEX_ERR_INVALID: c_int = -1;
pub const VEX_ERR_OOM: c_int = -2;
pub const VEX_ERR_AUDIO_INIT: c_int = -30;
pub const VEX_ERR_AUDIO_WRITE: c_int = -31;
pub const VEX_ERR_AUDIO_DEVICE: c_int = -32;

// ──── Win32 / COM Definitions (minimal WASAPI subset) ──────────

const HRESULT = i32;
const S_OK: HRESULT = 0;
const COINIT_MULTITHREADED: u32 = 0;
const CLSCTX_ALL: u32 = 0x17; // CLSCTX_INPROC_SERVER | ...

extern "ole32" fn CoInitializeEx(pvReserved: ?*anyopaque, dwCoInit: u32) callconv(.winapi) HRESULT;
extern "ole32" fn CoUninitialize() callconv(.winapi) void;
extern "ole32" fn CoCreateInstance(
    rclsid: *const GUID,
    pUnkOuter: ?*anyopaque,
    dwClsContext: u32,
    riid: *const GUID,
    ppv: *?*anyopaque,
) callconv(.winapi) HRESULT;

const GUID = extern struct {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [8]u8,
};

// MMDeviceEnumerator CLSID: BCDE0395-E52F-467C-8E3D-C4579291692E
const CLSID_MMDeviceEnumerator = GUID{
    .data1 = 0xBCDE0395,
    .data2 = 0xE52F,
    .data3 = 0x467C,
    .data4 = .{ 0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E },
};

// IMMDeviceEnumerator IID: A95664D2-9614-4F35-A746-DE8DB63617E6
const IID_IMMDeviceEnumerator = GUID{
    .data1 = 0xA95664D2,
    .data2 = 0x9614,
    .data3 = 0x4F35,
    .data4 = .{ 0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6 },
};

// IAudioClient IID: 1CB9AD4C-DBFA-4c32-B178-C2F568A703B2
const IID_IAudioClient = GUID{
    .data1 = 0x1CB9AD4C,
    .data2 = 0xDBFA,
    .data3 = 0x4c32,
    .data4 = .{ 0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03, 0xB2 },
};

// IAudioRenderClient IID: F294ACFC-3146-4483-A7BF-ADDCA7C260E2
const IID_IAudioRenderClient = GUID{
    .data1 = 0xF294ACFC,
    .data2 = 0x3146,
    .data3 = 0x4483,
    .data4 = .{ 0xA7, 0xBF, 0xAD, 0xDC, 0xA7, 0xC2, 0x60, 0xE2 },
};

// eRender = 0, eConsole = 0
const E_RENDER: u32 = 0;
const E_CONSOLE: u32 = 0;

// AUDCLNT_SHAREMODE_SHARED = 0
const AUDCLNT_SHAREMODE_SHARED: u32 = 0;

// REFERENCE_TIME: 100-nanosecond units
const REFTIMES_PER_SEC: i64 = 10_000_000;

// WAVEFORMATEX
const WAVEFORMATEX = extern struct {
    wFormatTag: u16,
    nChannels: u16,
    nSamplesPerSec: u32,
    nAvgBytesPerSec: u32,
    nBlockAlign: u16,
    wBitsPerSample: u16,
    cbSize: u16,
};

const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;

// COM vtable helpers (IUnknown at indices 0,1,2)
fn comRelease(obj: *anyopaque) void {
    const vt: *const [*]const *const fn (*anyopaque) callconv(.winapi) u32 = @ptrCast(@alignCast(obj));
    _ = vt.*[2](obj);
}

// ──── Audio Handle State ───────────────────────────────────────

const AudioState = struct {
    enumerator: *anyopaque, // IMMDeviceEnumerator*
    device: *anyopaque, // IMMDevice*
    audio_client: *anyopaque, // IAudioClient*
    render_client: *anyopaque, // IAudioRenderClient*
    buffer_frames: u32,
    sample_rate: u32,
    channels: u32,
    com_initialized: bool,

    fn deinit(self: *AudioState) void {
        comRelease(self.render_client);
        comRelease(self.audio_client);
        comRelease(self.device);
        comRelease(self.enumerator);
        if (self.com_initialized) {
            CoUninitialize();
        }
    }
};

// ──── IMMDeviceEnumerator vtable (indices beyond IUnknown) ─────
// Index 3: GetDefaultAudioEndpoint(dataFlow, role, ppDevice)

fn enumGetDefaultEndpoint(enumerator: *anyopaque, data_flow: u32, role: u32, out_device: *?*anyopaque) HRESULT {
    const vt: *const [*]const *const fn (*anyopaque, u32, u32, *?*anyopaque) callconv(.winapi) HRESULT = @ptrCast(@alignCast(enumerator));
    return vt.*[3](enumerator, data_flow, role, out_device);
}

// IMMDevice vtable: index 3 = Activate(iid, clsCtx, activationParams, ppInterface)
fn deviceActivate(device: *anyopaque, iid: *const GUID, cls_ctx: u32, params: ?*anyopaque, out: *?*anyopaque) HRESULT {
    const FnType = *const fn (*anyopaque, *const GUID, u32, ?*anyopaque, *?*anyopaque) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(device));
    return vt.*[3](device, iid, cls_ctx, params, out);
}

// IAudioClient vtable:
// 3: Initialize, 4: GetBufferSize, 5: GetStreamLatency,
// 6: GetCurrentPadding, 7: IsFormatSupported, 8: GetMixFormat,
// 9: GetDevicePeriod, 10: Start, 11: Stop, 12: Reset, 13: SetEventHandle

fn acInitialize(ac: *anyopaque, share_mode: u32, stream_flags: u32, buffer_dur: i64, period: i64, format: *const WAVEFORMATEX, session_guid: ?*const GUID) HRESULT {
    const FnType = *const fn (*anyopaque, u32, u32, i64, i64, *const WAVEFORMATEX, ?*const GUID) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(ac));
    return vt.*[3](ac, share_mode, stream_flags, buffer_dur, period, format, session_guid);
}

fn acGetBufferSize(ac: *anyopaque, out_frames: *u32) HRESULT {
    const FnType = *const fn (*anyopaque, *u32) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(ac));
    return vt.*[4](ac, out_frames);
}

fn acGetCurrentPadding(ac: *anyopaque, out_padding: *u32) HRESULT {
    const FnType = *const fn (*anyopaque, *u32) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(ac));
    return vt.*[6](ac, out_padding);
}

fn acStart(ac: *anyopaque) HRESULT {
    const FnType = *const fn (*anyopaque) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(ac));
    return vt.*[10](ac);
}

fn acStop(ac: *anyopaque) HRESULT {
    const FnType = *const fn (*anyopaque) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(ac));
    return vt.*[11](ac);
}

// IAudioRenderClient vtable:
// 3: GetBuffer, 4: ReleaseBuffer

fn rcGetBuffer(rc: *anyopaque, num_frames: u32, out_data: *?[*]u8) HRESULT {
    const FnType = *const fn (*anyopaque, u32, *?[*]u8) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(rc));
    return vt.*[3](rc, num_frames, out_data);
}

fn rcReleaseBuffer(rc: *anyopaque, num_frames: u32, flags: u32) HRESULT {
    const FnType = *const fn (*anyopaque, u32, u32) callconv(.winapi) HRESULT;
    const vt: *const [*]const FnType = @ptrCast(@alignCast(rc));
    return vt.*[4](rc, num_frames, flags);
}

// ──── Exported C ABI Functions ─────────────────────────────────

/// Open the default audio render device.
/// Returns opaque handle via `out_handle`, or error code.
export fn vex_audio_open(sample_rate: u32, channels: u32, out_handle: *?*anyopaque) c_int {
    out_handle.* = null;

    // Initialize COM
    const com_hr = CoInitializeEx(null, COINIT_MULTITHREADED);
    // S_OK or S_FALSE (already initialized) are both fine
    const com_init = (com_hr == S_OK) or (com_hr == 1); // S_FALSE = 1

    // Create device enumerator
    var enumerator: ?*anyopaque = null;
    if (CoCreateInstance(&CLSID_MMDeviceEnumerator, null, CLSCTX_ALL, &IID_IMMDeviceEnumerator, &enumerator) != S_OK) {
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_INIT;
    }

    // Get default render endpoint
    var device: ?*anyopaque = null;
    if (enumGetDefaultEndpoint(enumerator.?, E_RENDER, E_CONSOLE, &device) != S_OK) {
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_DEVICE;
    }

    // Activate IAudioClient
    var audio_client: ?*anyopaque = null;
    if (deviceActivate(device.?, &IID_IAudioClient, CLSCTX_ALL, null, &audio_client) != S_OK) {
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_DEVICE;
    }

    // Configure wave format: IEEE float, requested sample rate & channels
    const block_align: u16 = @intCast(channels * @sizeOf(f32));
    const wave_fmt = WAVEFORMATEX{
        .wFormatTag = WAVE_FORMAT_IEEE_FLOAT,
        .nChannels = @intCast(channels),
        .nSamplesPerSec = sample_rate,
        .nAvgBytesPerSec = sample_rate * @as(u32, block_align),
        .nBlockAlign = block_align,
        .wBitsPerSample = 32,
        .cbSize = 0,
    };

    // Request 100ms buffer
    const buffer_duration: i64 = REFTIMES_PER_SEC / 10;

    if (acInitialize(audio_client.?, AUDCLNT_SHAREMODE_SHARED, 0, buffer_duration, 0, &wave_fmt, null) != S_OK) {
        comRelease(audio_client.?);
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_INIT;
    }

    // Get buffer size
    var buffer_frames: u32 = 0;
    if (acGetBufferSize(audio_client.?, &buffer_frames) != S_OK) {
        comRelease(audio_client.?);
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_INIT;
    }

    // Get render client
    var render_client: ?*anyopaque = null;
    if (deviceActivate(audio_client.?, &IID_IAudioRenderClient, CLSCTX_ALL, null, &render_client) != S_OK) {
        // Try alternate: through IAudioClient::GetService
        comRelease(audio_client.?);
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_INIT;
    }

    // Start playback
    if (acStart(audio_client.?) != S_OK) {
        comRelease(render_client.?);
        comRelease(audio_client.?);
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_AUDIO_INIT;
    }

    // Allocate state
    const state = std.heap.page_allocator.create(AudioState) catch {
        _ = acStop(audio_client.?);
        comRelease(render_client.?);
        comRelease(audio_client.?);
        comRelease(device.?);
        comRelease(enumerator.?);
        if (com_init) CoUninitialize();
        return VEX_ERR_OOM;
    };

    state.* = AudioState{
        .enumerator = enumerator.?,
        .device = device.?,
        .audio_client = audio_client.?,
        .render_client = render_client.?,
        .buffer_frames = buffer_frames,
        .sample_rate = sample_rate,
        .channels = channels,
        .com_initialized = com_init,
    };

    out_handle.* = @ptrCast(state);
    return VEX_OK;
}

/// Write interleaved float samples to the audio device.
/// `samples` points to `count` interleaved float samples (count = frames × channels).
export fn vex_audio_write(handle: *anyopaque, samples: [*]const f32, count: u32) c_int {
    const state: *AudioState = @ptrCast(@alignCast(handle));
    const frames = count / state.channels;
    if (frames == 0) return VEX_OK;

    // Get available buffer space
    var padding: u32 = 0;
    if (acGetCurrentPadding(state.audio_client, &padding) != S_OK) return VEX_ERR_AUDIO_WRITE;

    const available = state.buffer_frames -| padding;
    const to_write = @min(frames, available);
    if (to_write == 0) return VEX_OK;

    // Get buffer from render client
    var buf_ptr: ?[*]u8 = null;
    if (rcGetBuffer(state.render_client, to_write, &buf_ptr) != S_OK) return VEX_ERR_AUDIO_WRITE;

    const dst = buf_ptr orelse return VEX_ERR_AUDIO_WRITE;
    const byte_count = to_write * state.channels * @sizeOf(f32);
    const src_bytes: [*]const u8 = @ptrCast(samples);
    @memcpy(dst[0..byte_count], src_bytes[0..byte_count]);

    if (rcReleaseBuffer(state.render_client, to_write, 0) != S_OK) return VEX_ERR_AUDIO_WRITE;

    return VEX_OK;
}

/// Close the audio device and release all resources.
export fn vex_audio_close(handle: *anyopaque) c_int {
    const state: *AudioState = @ptrCast(@alignCast(handle));
    _ = acStop(state.audio_client);
    state.deinit();
    std.heap.page_allocator.destroy(state);
    return VEX_OK;
}

/// Get the buffer size in frames for the opened audio device.
export fn vex_audio_get_buffer_frames(handle: *anyopaque) u32 {
    const state: *AudioState = @ptrCast(@alignCast(handle));
    return state.buffer_frames;
}

/// Get current audio playback position in frames (padding = buffered frames).
export fn vex_audio_get_padding(handle: *anyopaque) u32 {
    const state: *AudioState = @ptrCast(@alignCast(handle));
    var padding: u32 = 0;
    _ = acGetCurrentPadding(state.audio_client, &padding);
    return padding;
}

// ──── Tests ────────────────────────────────────────────────────

test "WAVEFORMATEX has correct size" {
    try std.testing.expectEqual(@sizeOf(WAVEFORMATEX), 18);
}

test "GUID has correct size" {
    try std.testing.expectEqual(@sizeOf(GUID), 16);
}

test "AudioState has reasonable size" {
    // 8 pointers + 3 u32 + 1 bool = ~80 bytes (with alignment)
    try std.testing.expect(@sizeOf(AudioState) > 0);
    try std.testing.expect(@sizeOf(AudioState) <= 128);
}

test "error codes are distinct" {
    try std.testing.expect(VEX_ERR_AUDIO_INIT != VEX_ERR_AUDIO_WRITE);
    try std.testing.expect(VEX_ERR_AUDIO_WRITE != VEX_ERR_AUDIO_DEVICE);
    try std.testing.expect(VEX_ERR_AUDIO_INIT != VEX_ERR_AUDIO_DEVICE);
}
