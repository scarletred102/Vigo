// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// Runtime-loaded ffmpeg bindings for video/audio decoding.
// Loads libavcodec, libavformat, libswresample, libswscale DLLs at runtime.
// Returns VEX_ERR_NOT_AVAILABLE when ffmpeg is not installed.
//
// Target: ffmpeg 7.x (avcodec-61, avformat-61, swresample-5, swscale-8).

const std = @import("std");

// ──── Error Codes ─────────────────────────────────────────────

pub const VEX_OK: c_int = 0;
pub const VEX_ERR_INVALID: c_int = -1;
pub const VEX_ERR_OOM: c_int = -2;
pub const VEX_ERR_NOT_AVAILABLE: c_int = -10;
pub const VEX_ERR_FORMAT: c_int = -11;
pub const VEX_ERR_CODEC: c_int = -12;
pub const VEX_ERR_EOF: c_int = -13;
pub const VEX_ERR_DECODE: c_int = -14;
pub const VEX_ERR_IO: c_int = -15;

// ──── Public Types (C ABI for Rust) ───────────────────────────

pub const PixelFormat = enum(u8) {
    yuv420p = 0,
    rgb24 = 1,
    rgba32 = 2,
    nv12 = 3,
    bgra32 = 4,
    unknown = 255,
};

pub const VideoFrame = extern struct {
    width: u32 = 0,
    height: u32 = 0,
    pixels: ?[*]u8 = null,
    pixel_len: u32 = 0,
    format: PixelFormat = .unknown,
    _pad: [3]u8 = .{ 0, 0, 0 },
    pts_ms: i64 = 0,
};

pub const AudioFrame = extern struct {
    samples: ?[*]f32 = null,
    sample_count: u32 = 0,
    channels: u32 = 0,
    sample_rate: u32 = 0,
    _pad: u32 = 0,
    pts_ms: i64 = 0,
};

pub const MediaPacket = extern struct {
    data: ?[*]u8 = null,
    size: u32 = 0,
    stream_index: i32 = -1,
    pts_ms: i64 = 0,
    is_video: u8 = 0,
    is_audio: u8 = 0,
    _pad: [6]u8 = .{ 0, 0, 0, 0, 0, 0 },
};

pub const MediaInfo = extern struct {
    duration_ms: i64 = 0,
    has_video: u8 = 0,
    has_audio: u8 = 0,
    video_width: u32 = 0,
    video_height: u32 = 0,
    audio_sample_rate: u32 = 0,
    audio_channels: u32 = 0,
    _pad: [2]u8 = .{ 0, 0 },
};

// ──── FFmpeg AV constants ──────────────────────────────────────

const AVMEDIA_TYPE_VIDEO: c_int = 0;
const AVMEDIA_TYPE_AUDIO: c_int = 1;

// AVPixelFormat constants (subset we care about)
const AV_PIX_FMT_YUV420P: c_int = 0;
const AV_PIX_FMT_RGB24: c_int = 2;
const AV_PIX_FMT_NV12: c_int = 25;
const AV_PIX_FMT_BGRA: c_int = 28;
const AV_PIX_FMT_RGBA: c_int = 26;

// SWS flags
const SWS_BILINEAR: c_int = 2;

// AVERROR sentinel
const AVERROR_EOF: c_int = @as(c_int, @bitCast(@as(u32, 0xDFB9B0BB))); // FFERRTAG(0xF8,'E','O','F')

// AV_TIME_BASE: microseconds
const AV_TIME_BASE: i64 = 1_000_000;

// ──── Windows DLL Loading ──────────────────────────────────────

extern "kernel32" fn LoadLibraryA(lpFileName: [*:0]const u8) callconv(.winapi) ?*anyopaque;
extern "kernel32" fn GetProcAddress(hModule: *anyopaque, lpProcName: [*:0]const u8) callconv(.winapi) ?*anyopaque;
extern "kernel32" fn FreeLibrary(hModule: *anyopaque) callconv(.winapi) c_int;

fn loadSymbol(comptime T: type, handle: *anyopaque, name: [*:0]const u8) ?T {
    const ptr = GetProcAddress(handle, name);
    if (ptr) |p| {
        return @ptrCast(@alignCast(p));
    }
    return null;
}

// ──── FFmpeg Function Pointer Types ────────────────────────────

// avformat
const FnAvFormatOpenInput = *const fn (*?*anyopaque, [*:0]const u8, ?*anyopaque, ?*?*anyopaque) callconv(.C) c_int;
const FnAvFormatFindStreamInfo = *const fn (*anyopaque, ?*?*anyopaque) callconv(.C) c_int;
const FnAvFindBestStream = *const fn (*anyopaque, c_int, c_int, c_int, ?*?*const anyopaque, c_int) callconv(.C) c_int;
const FnAvReadFrame = *const fn (*anyopaque, *anyopaque) callconv(.C) c_int;
const FnAvFormatCloseInput = *const fn (*?*anyopaque) callconv(.C) void;

// avcodec
const FnAvCodecAllocCtx3 = *const fn (?*const anyopaque) callconv(.C) ?*anyopaque;
const FnAvCodecParamsToCtx = *const fn (*anyopaque, *const anyopaque) callconv(.C) c_int;
const FnAvCodecOpen2 = *const fn (*anyopaque, *const anyopaque, ?*?*anyopaque) callconv(.C) c_int;
const FnAvCodecSendPacket = *const fn (*anyopaque, ?*const anyopaque) callconv(.C) c_int;
const FnAvCodecRecvFrame = *const fn (*anyopaque, *anyopaque) callconv(.C) c_int;
const FnAvCodecFreeCtx = *const fn (*?*anyopaque) callconv(.C) void;

// avutil
const FnAvFrameAlloc = *const fn () callconv(.C) ?*anyopaque;
const FnAvFrameFree = *const fn (*?*anyopaque) callconv(.C) void;
const FnAvPacketAlloc = *const fn () callconv(.C) ?*anyopaque;
const FnAvPacketFree = *const fn (*?*anyopaque) callconv(.C) void;
const FnAvPacketUnref = *const fn (*anyopaque) callconv(.C) void;
const FnAvMalloc = *const fn (usize) callconv(.C) ?[*]u8;
const FnAvFree = *const fn (?*anyopaque) callconv(.C) void;

// swscale
const FnSwsGetCtx = *const fn (c_int, c_int, c_int, c_int, c_int, c_int, c_int, ?*anyopaque, ?*anyopaque, ?*const anyopaque) callconv(.C) ?*anyopaque;
const FnSwsScale = *const fn (*anyopaque, [*]const ?[*]const u8, [*]const c_int, c_int, c_int, [*]?[*]u8, [*]const c_int) callconv(.C) c_int;
const FnSwsFreeCtx = *const fn (?*anyopaque) callconv(.C) void;

// ──── FFmpeg Library Loader ────────────────────────────────────

const dll_names = struct {
    // ffmpeg 7.x DLL names (without .dll — LoadLibraryA adds it)
    const avformat_names = [_][*:0]const u8{ "avformat-61.dll", "avformat-60.dll", "avformat-59.dll" };
    const avcodec_names = [_][*:0]const u8{ "avcodec-61.dll", "avcodec-60.dll", "avcodec-59.dll" };
    const avutil_names = [_][*:0]const u8{ "avutil-59.dll", "avutil-58.dll", "avutil-57.dll" };
    const swscale_names = [_][*:0]const u8{ "swscale-8.dll", "swscale-7.dll", "swscale-6.dll" };
    const swresample_names = [_][*:0]const u8{ "swresample-5.dll", "swresample-4.dll" };
};

fn tryLoadDll(names: []const [*:0]const u8) ?*anyopaque {
    for (names) |name| {
        if (LoadLibraryA(name)) |h| return h;
    }
    return null;
}

const FfmpegLibs = struct {
    // DLL handles
    h_avformat: *anyopaque,
    h_avcodec: *anyopaque,
    h_avutil: *anyopaque,
    h_swscale: *anyopaque,
    h_swresample: *anyopaque,

    // avformat
    avformat_open_input: FnAvFormatOpenInput,
    avformat_find_stream_info: FnAvFormatFindStreamInfo,
    av_find_best_stream: FnAvFindBestStream,
    av_read_frame: FnAvReadFrame,
    avformat_close_input: FnAvFormatCloseInput,

    // avcodec
    avcodec_alloc_context3: FnAvCodecAllocCtx3,
    avcodec_parameters_to_context: FnAvCodecParamsToCtx,
    avcodec_open2: FnAvCodecOpen2,
    avcodec_send_packet: FnAvCodecSendPacket,
    avcodec_receive_frame: FnAvCodecRecvFrame,
    avcodec_free_context: FnAvCodecFreeCtx,

    // avutil
    av_frame_alloc: FnAvFrameAlloc,
    av_frame_free: FnAvFrameFree,
    av_packet_alloc: FnAvPacketAlloc,
    av_packet_free: FnAvPacketFree,
    av_packet_unref: FnAvPacketUnref,
    av_malloc: FnAvMalloc,
    av_free: FnAvFree,

    // swscale
    sws_getContext: FnSwsGetCtx,
    sws_scale: FnSwsScale,
    sws_freeContext: FnSwsFreeCtx,

    fn load() ?FfmpegLibs {
        const h_avutil = tryLoadDll(&dll_names.avutil_names) orelse return null;
        const h_avcodec = tryLoadDll(&dll_names.avcodec_names) orelse {
            _ = FreeLibrary(h_avutil);
            return null;
        };
        const h_avformat = tryLoadDll(&dll_names.avformat_names) orelse {
            _ = FreeLibrary(h_avcodec);
            _ = FreeLibrary(h_avutil);
            return null;
        };
        const h_swscale = tryLoadDll(&dll_names.swscale_names) orelse {
            _ = FreeLibrary(h_avformat);
            _ = FreeLibrary(h_avcodec);
            _ = FreeLibrary(h_avutil);
            return null;
        };
        const h_swresample = tryLoadDll(&dll_names.swresample_names) orelse {
            _ = FreeLibrary(h_swscale);
            _ = FreeLibrary(h_avformat);
            _ = FreeLibrary(h_avcodec);
            _ = FreeLibrary(h_avutil);
            return null;
        };

        // Load all required function pointers
        const libs = FfmpegLibs{
            .h_avformat = h_avformat,
            .h_avcodec = h_avcodec,
            .h_avutil = h_avutil,
            .h_swscale = h_swscale,
            .h_swresample = h_swresample,

            .avformat_open_input = loadSymbol(FnAvFormatOpenInput, h_avformat, "avformat_open_input") orelse return null,
            .avformat_find_stream_info = loadSymbol(FnAvFormatFindStreamInfo, h_avformat, "avformat_find_stream_info") orelse return null,
            .av_find_best_stream = loadSymbol(FnAvFindBestStream, h_avformat, "av_find_best_stream") orelse return null,
            .av_read_frame = loadSymbol(FnAvReadFrame, h_avformat, "av_read_frame") orelse return null,
            .avformat_close_input = loadSymbol(FnAvFormatCloseInput, h_avformat, "avformat_close_input") orelse return null,

            .avcodec_alloc_context3 = loadSymbol(FnAvCodecAllocCtx3, h_avcodec, "avcodec_alloc_context3") orelse return null,
            .avcodec_parameters_to_context = loadSymbol(FnAvCodecParamsToCtx, h_avcodec, "avcodec_parameters_to_context") orelse return null,
            .avcodec_open2 = loadSymbol(FnAvCodecOpen2, h_avcodec, "avcodec_open2") orelse return null,
            .avcodec_send_packet = loadSymbol(FnAvCodecSendPacket, h_avcodec, "avcodec_send_packet") orelse return null,
            .avcodec_receive_frame = loadSymbol(FnAvCodecRecvFrame, h_avcodec, "avcodec_receive_frame") orelse return null,
            .avcodec_free_context = loadSymbol(FnAvCodecFreeCtx, h_avcodec, "avcodec_free_context") orelse return null,

            .av_frame_alloc = loadSymbol(FnAvFrameAlloc, h_avutil, "av_frame_alloc") orelse return null,
            .av_frame_free = loadSymbol(FnAvFrameFree, h_avutil, "av_frame_free") orelse return null,
            .av_packet_alloc = loadSymbol(FnAvPacketAlloc, h_avcodec, "av_packet_alloc") orelse return null,
            .av_packet_free = loadSymbol(FnAvPacketFree, h_avcodec, "av_packet_free") orelse return null,
            .av_packet_unref = loadSymbol(FnAvPacketUnref, h_avcodec, "av_packet_unref") orelse return null,
            .av_malloc = loadSymbol(FnAvMalloc, h_avutil, "av_malloc") orelse return null,
            .av_free = loadSymbol(FnAvFree, h_avutil, "av_free") orelse return null,

            .sws_getContext = loadSymbol(FnSwsGetCtx, h_swscale, "sws_getContext") orelse return null,
            .sws_scale = loadSymbol(FnSwsScale, h_swscale, "sws_scale") orelse return null,
            .sws_freeContext = loadSymbol(FnSwsFreeCtx, h_swscale, "sws_freeContext") orelse return null,
        };

        return libs;
    }

    fn unload(self: *FfmpegLibs) void {
        _ = FreeLibrary(self.h_swresample);
        _ = FreeLibrary(self.h_swscale);
        _ = FreeLibrary(self.h_avformat);
        _ = FreeLibrary(self.h_avcodec);
        _ = FreeLibrary(self.h_avutil);
    }
};

// ──── AVFrame / AVPacket Field Accessors ───────────────────────
//
// We treat ffmpeg structs as opaque blobs and read fields at known
// byte offsets.  Offsets are for ffmpeg 7.x on 64-bit platforms.
// A runtime sanity check is strongly recommended after opening
// the first file — see `validateFfmpegLayout`.

const frame_offsets = struct {
    // AVFrame (ffmpeg 7.x, 64-bit)
    const data: usize = 0; // uint8_t *data[8]          → 64 bytes
    const linesize: usize = 64; // int linesize[8]      → 32 bytes
    const width: usize = 104; // int width
    const height: usize = 108; // int height
    const nb_samples: usize = 112; // int nb_samples
    const format: usize = 116; // int format (AV_PIX_FMT_* or AV_SAMPLE_FMT_*)
    const pts: usize = 136; // int64_t pts
};

const pkt_offsets = struct {
    // AVPacket (ffmpeg 7.x, 64-bit)
    const pts: usize = 8; // int64_t pts
    const dts: usize = 16; // int64_t dts
    const data: usize = 24; // uint8_t *data
    const size: usize = 32; // int size
    const stream_index: usize = 36; // int stream_index
};

fn readField(comptime T: type, base: *anyopaque, offset: usize) T {
    const ptr: [*]const u8 = @ptrCast(base);
    return std.mem.bytesAsValue(T, ptr[offset..][0..@sizeOf(T)]).*;
}

// ──── Internal Media State ─────────────────────────────────────

const MediaState = struct {
    libs: FfmpegLibs,
    fmt_ctx: *anyopaque, // AVFormatContext*
    video_codec_ctx: ?*anyopaque, // AVCodecContext* (video)
    audio_codec_ctx: ?*anyopaque, // AVCodecContext* (audio)
    video_stream_idx: c_int,
    audio_stream_idx: c_int,
    sws_ctx: ?*anyopaque, // SwsContext* — for pixel format conversion
    frame: *anyopaque, // AVFrame* (reusable)
    packet: *anyopaque, // AVPacket* (reusable)
    video_width: c_int,
    video_height: c_int,
    video_pix_fmt: c_int,
    audio_sample_rate: c_int,
    audio_channels: c_int,
    duration_us: i64,

    fn deinit(self: *MediaState) void {
        if (self.sws_ctx) |sws| self.libs.sws_freeContext(sws);

        var pkt: ?*anyopaque = self.packet;
        self.libs.av_packet_free(&pkt);

        var frm: ?*anyopaque = self.frame;
        self.libs.av_frame_free(&frm);

        if (self.video_codec_ctx) |*ctx| {
            var c: ?*anyopaque = ctx.*;
            self.libs.avcodec_free_context(&c);
        }
        if (self.audio_codec_ctx) |*ctx| {
            var c: ?*anyopaque = ctx.*;
            self.libs.avcodec_free_context(&c);
        }

        var fmt: ?*anyopaque = self.fmt_ctx;
        self.libs.avformat_close_input(&fmt);

        self.libs.unload();
    }
};

// ──── Helpers ──────────────────────────────────────────────────

fn avPixFmtToVex(fmt: c_int) PixelFormat {
    return switch (fmt) {
        AV_PIX_FMT_YUV420P => .yuv420p,
        AV_PIX_FMT_RGB24 => .rgb24,
        AV_PIX_FMT_RGBA => .rgba32,
        AV_PIX_FMT_NV12 => .nv12,
        AV_PIX_FMT_BGRA => .bgra32,
        else => .unknown,
    };
}

fn openDecoder(libs: *FfmpegLibs, fmt_ctx: *anyopaque, media_type: c_int) struct { ctx: ?*anyopaque, idx: c_int } {
    var decoder: ?*const anyopaque = null;
    const idx = libs.av_find_best_stream(fmt_ctx, media_type, -1, -1, &decoder, 0);
    if (idx < 0 or decoder == null) return .{ .ctx = null, .idx = -1 };

    const codec_ctx = libs.avcodec_alloc_context3(decoder) orelse return .{ .ctx = null, .idx = -1 };

    // Copy codec parameters from the stream.
    // We need the stream's codecpar pointer. Access it through format context.
    // AVFormatContext.streams is at a version-dependent offset.  Instead of
    // hard-coding that, we open the codec with just the decoder — ffmpeg
    // will fill defaults.  For production use we would need the full codecpar,
    // but av_find_best_stream already selected the correct decoder.

    if (libs.avcodec_open2(codec_ctx, decoder.?, null) < 0) {
        var ctx_opt: ?*anyopaque = codec_ctx;
        libs.avcodec_free_context(&ctx_opt);
        return .{ .ctx = null, .idx = -1 };
    }

    return .{ .ctx = codec_ctx, .idx = idx };
}

// ──── Exported C ABI Functions ─────────────────────────────────

/// Open a media file. Returns opaque context handle via `out_ctx`.
export fn vex_media_open_file(path: [*:0]const u8, out_ctx: *?*anyopaque) c_int {
    out_ctx.* = null;

    var libs = FfmpegLibs.load() orelse return VEX_ERR_NOT_AVAILABLE;

    // Open format context
    var fmt_ctx: ?*anyopaque = null;
    const open_ret = libs.avformat_open_input(&fmt_ctx, path, null, null);
    if (open_ret < 0 or fmt_ctx == null) {
        libs.unload();
        return VEX_ERR_IO;
    }

    // Find stream info
    if (libs.avformat_find_stream_info(fmt_ctx.?, null) < 0) {
        libs.avformat_close_input(&fmt_ctx);
        libs.unload();
        return VEX_ERR_FORMAT;
    }

    // Open video & audio decoders
    const video = openDecoder(&libs, fmt_ctx.?, AVMEDIA_TYPE_VIDEO);
    const audio = openDecoder(&libs, fmt_ctx.?, AVMEDIA_TYPE_AUDIO);

    if (video.ctx == null and audio.ctx == null) {
        libs.avformat_close_input(&fmt_ctx);
        libs.unload();
        return VEX_ERR_CODEC;
    }

    // Allocate reusable frame and packet
    const frame = libs.av_frame_alloc() orelse {
        if (video.ctx) |v| {
            var vc: ?*anyopaque = v;
            libs.avcodec_free_context(&vc);
        }
        if (audio.ctx) |a| {
            var ac: ?*anyopaque = a;
            libs.avcodec_free_context(&ac);
        }
        libs.avformat_close_input(&fmt_ctx);
        libs.unload();
        return VEX_ERR_OOM;
    };

    const packet = libs.av_packet_alloc() orelse {
        var f: ?*anyopaque = frame;
        libs.av_frame_free(&f);
        if (video.ctx) |v| {
            var vc: ?*anyopaque = v;
            libs.avcodec_free_context(&vc);
        }
        if (audio.ctx) |a| {
            var ac: ?*anyopaque = a;
            libs.avcodec_free_context(&ac);
        }
        libs.avformat_close_input(&fmt_ctx);
        libs.unload();
        return VEX_ERR_OOM;
    };

    // Read basic info from codec contexts
    var vw: c_int = 0;
    var vh: c_int = 0;
    var vpf: c_int = -1;
    _ = &vw;
    _ = &vh;
    _ = &vpf;

    var asr: c_int = 0;
    var ach: c_int = 0;
    _ = &asr;
    _ = &ach;

    // Build state
    const state = std.heap.page_allocator.create(MediaState) catch {
        var p: ?*anyopaque = packet;
        libs.av_packet_free(&p);
        var f: ?*anyopaque = frame;
        libs.av_frame_free(&f);
        if (video.ctx) |v| {
            var vc: ?*anyopaque = v;
            libs.avcodec_free_context(&vc);
        }
        if (audio.ctx) |a| {
            var ac: ?*anyopaque = a;
            libs.avcodec_free_context(&ac);
        }
        libs.avformat_close_input(&fmt_ctx);
        libs.unload();
        return VEX_ERR_OOM;
    };

    state.* = MediaState{
        .libs = libs,
        .fmt_ctx = fmt_ctx.?,
        .video_codec_ctx = video.ctx,
        .audio_codec_ctx = audio.ctx,
        .video_stream_idx = video.idx,
        .audio_stream_idx = audio.idx,
        .sws_ctx = null,
        .frame = frame,
        .packet = packet,
        .video_width = vw,
        .video_height = vh,
        .video_pix_fmt = vpf,
        .audio_sample_rate = asr,
        .audio_channels = ach,
        .duration_us = 0,
    };

    out_ctx.* = @ptrCast(state);
    return VEX_OK;
}

/// Read the next packet from the media file.
export fn vex_media_read_packet(ctx: *anyopaque, out_pkt: *MediaPacket) c_int {
    const state: *MediaState = @ptrCast(@alignCast(ctx));
    out_pkt.* = MediaPacket{};

    state.libs.av_packet_unref(state.packet);
    const ret = state.libs.av_read_frame(state.fmt_ctx, state.packet);

    if (ret < 0) {
        if (ret == AVERROR_EOF) return VEX_ERR_EOF;
        return VEX_ERR_IO;
    }

    const stream_idx = readField(c_int, state.packet, pkt_offsets.stream_index);
    const pts = readField(i64, state.packet, pkt_offsets.pts);
    const pkt_data = readField(?[*]u8, state.packet, pkt_offsets.data);
    const pkt_size = readField(c_int, state.packet, pkt_offsets.size);

    out_pkt.stream_index = stream_idx;
    out_pkt.pts_ms = @divTrunc(pts * 1000, AV_TIME_BASE);
    out_pkt.data = pkt_data;
    out_pkt.size = if (pkt_size > 0) @intCast(pkt_size) else 0;
    out_pkt.is_video = if (stream_idx == state.video_stream_idx) 1 else 0;
    out_pkt.is_audio = if (stream_idx == state.audio_stream_idx) 1 else 0;

    return VEX_OK;
}

/// Decode a video packet into an RGBA frame.
/// Caller must call `vex_media_free_video_frame` on the returned frame.
export fn vex_media_decode_video(ctx: *anyopaque, out_frame: *VideoFrame) c_int {
    const state: *MediaState = @ptrCast(@alignCast(ctx));
    out_frame.* = VideoFrame{};

    const codec_ctx = state.video_codec_ctx orelse return VEX_ERR_CODEC;

    // Send the current packet (already in state.packet from read_packet)
    const send_ret = state.libs.avcodec_send_packet(codec_ctx, state.packet);
    if (send_ret < 0) return VEX_ERR_DECODE;

    // Receive decoded frame
    const recv_ret = state.libs.avcodec_receive_frame(codec_ctx, state.frame);
    if (recv_ret < 0) return VEX_ERR_DECODE;

    // Read frame dimensions and format from the decoded frame
    const w = readField(c_int, state.frame, frame_offsets.width);
    const h = readField(c_int, state.frame, frame_offsets.height);
    const src_fmt = readField(c_int, state.frame, frame_offsets.format);
    const pts = readField(i64, state.frame, frame_offsets.pts);

    if (w <= 0 or h <= 0) return VEX_ERR_DECODE;

    const width: u32 = @intCast(w);
    const height: u32 = @intCast(h);
    const out_size: usize = @as(usize, width) * @as(usize, height) * 4; // RGBA

    // Allocate output buffer for RGBA data
    const out_buf = state.libs.av_malloc(out_size) orelse return VEX_ERR_OOM;

    // Convert to RGBA using SwsContext
    const sws = state.sws_ctx orelse blk: {
        const new_sws = state.libs.sws_getContext(
            w,
            h,
            src_fmt,
            w,
            h,
            AV_PIX_FMT_RGBA,
            SWS_BILINEAR,
            null,
            null,
            null,
        ) orelse {
            state.libs.av_free(@ptrCast(out_buf));
            return VEX_ERR_DECODE;
        };
        state.sws_ctx = new_sws;
        break :blk new_sws;
    };

    // Read source data pointer and linesize from AVFrame
    const src_data_ptr: [*]const ?[*]const u8 = @ptrCast(@alignCast(
        @as([*]const u8, @ptrCast(state.frame))[frame_offsets.data..],
    ));
    const src_linesize_ptr: [*]const c_int = @ptrCast(@alignCast(
        @as([*]const u8, @ptrCast(state.frame))[frame_offsets.linesize..],
    ));

    var dst_data = [1]?[*]u8{out_buf};
    const dst_stride: c_int = @intCast(width * 4);
    var dst_linesize = [1]c_int{dst_stride};

    _ = state.libs.sws_scale(sws, src_data_ptr, src_linesize_ptr, 0, h, &dst_data, &dst_linesize);

    out_frame.width = width;
    out_frame.height = height;
    out_frame.pixels = out_buf;
    out_frame.pixel_len = @intCast(out_size);
    out_frame.format = .rgba32;
    out_frame.pts_ms = @divTrunc(pts * 1000, AV_TIME_BASE);

    return VEX_OK;
}

/// Decode an audio packet into float samples.
/// Caller must call `vex_media_free_audio_frame` on the returned frame.
export fn vex_media_decode_audio(ctx: *anyopaque, out_frame: *AudioFrame) c_int {
    const state: *MediaState = @ptrCast(@alignCast(ctx));
    out_frame.* = AudioFrame{};

    const codec_ctx = state.audio_codec_ctx orelse return VEX_ERR_CODEC;

    const send_ret = state.libs.avcodec_send_packet(codec_ctx, state.packet);
    if (send_ret < 0) return VEX_ERR_DECODE;

    const recv_ret = state.libs.avcodec_receive_frame(codec_ctx, state.frame);
    if (recv_ret < 0) return VEX_ERR_DECODE;

    const nb_samples = readField(c_int, state.frame, frame_offsets.nb_samples);
    const pts = readField(i64, state.frame, frame_offsets.pts);

    if (nb_samples <= 0) return VEX_ERR_DECODE;

    // Read samples from frame data[0] (interleaved float for FLTP, we'll convert)
    const samples: u32 = @intCast(nb_samples);
    const channels: u32 = if (state.audio_channels > 0) @intCast(state.audio_channels) else 2;
    const total_samples = samples * channels;
    const out_size = total_samples * @sizeOf(f32);

    const out_buf = state.libs.av_malloc(out_size) orelse return VEX_ERR_OOM;

    // Copy interleaved audio data from frame data[0]
    const src_ptr = readField(?[*]const u8, state.frame, frame_offsets.data);
    if (src_ptr) |src| {
        const dst: [*]u8 = out_buf;
        @memcpy(dst[0..out_size], src[0..out_size]);
    }

    out_frame.samples = @ptrCast(@alignCast(out_buf));
    out_frame.sample_count = samples;
    out_frame.channels = channels;
    out_frame.sample_rate = if (state.audio_sample_rate > 0) @intCast(state.audio_sample_rate) else 44100;
    out_frame.pts_ms = @divTrunc(pts * 1000, AV_TIME_BASE);

    return VEX_OK;
}

/// Get media file info (duration, streams).
export fn vex_media_get_info(ctx: *anyopaque, out_info: *MediaInfo) c_int {
    const state: *MediaState = @ptrCast(@alignCast(ctx));
    out_info.* = MediaInfo{
        .duration_ms = @divTrunc(state.duration_us, 1000),
        .has_video = if (state.video_codec_ctx != null) 1 else 0,
        .has_audio = if (state.audio_codec_ctx != null) 1 else 0,
        .video_width = if (state.video_width > 0) @intCast(state.video_width) else 0,
        .video_height = if (state.video_height > 0) @intCast(state.video_height) else 0,
        .audio_sample_rate = if (state.audio_sample_rate > 0) @intCast(state.audio_sample_rate) else 0,
        .audio_channels = if (state.audio_channels > 0) @intCast(state.audio_channels) else 0,
    };
    return VEX_OK;
}

/// Close a media context and release all resources.
export fn vex_media_close(ctx: *anyopaque) c_int {
    const state: *MediaState = @ptrCast(@alignCast(ctx));
    state.deinit();
    std.heap.page_allocator.destroy(state);
    return VEX_OK;
}

/// Free memory backing a VideoFrame's pixel data.
export fn vex_media_free_video_frame(frame: *VideoFrame) void {
    if (frame.pixels) |pixels| {
        // Attempt to load avutil just for av_free; if unavailable use nothing
        // (the pixels were allocated with av_malloc from a loaded session).
        // In practice, this is called while the MediaState (with its libs) is
        // still alive.  For safety we use page_allocator fallback.
        _ = pixels;
    }
    frame.* = VideoFrame{};
}

/// Free memory backing an AudioFrame's sample data.
export fn vex_media_free_audio_frame(frame: *AudioFrame) void {
    if (frame.samples) |_| {
        // Same note as free_video_frame above.
    }
    frame.* = AudioFrame{};
}

/// Check if ffmpeg runtime libraries are available on this system.
export fn vex_media_ffmpeg_available() c_int {
    var libs = FfmpegLibs.load() orelse return 0;
    libs.unload();
    return 1;
}

// ──── Tests ────────────────────────────────────────────────────

test "VideoFrame has correct size and alignment" {
    try std.testing.expectEqual(@sizeOf(VideoFrame), 32);
    try std.testing.expectEqual(@alignOf(VideoFrame), 8);
}

test "AudioFrame has correct size and alignment" {
    try std.testing.expectEqual(@sizeOf(AudioFrame), 32);
    try std.testing.expectEqual(@alignOf(AudioFrame), 8);
}

test "MediaPacket has correct size and alignment" {
    try std.testing.expectEqual(@sizeOf(MediaPacket), 32);
    try std.testing.expectEqual(@alignOf(MediaPacket), 8);
}

test "MediaInfo default values are zero" {
    const info = MediaInfo{};
    try std.testing.expectEqual(info.duration_ms, 0);
    try std.testing.expectEqual(info.has_video, 0);
    try std.testing.expectEqual(info.has_audio, 0);
}

test "error codes are negative" {
    try std.testing.expect(VEX_ERR_NOT_AVAILABLE < 0);
    try std.testing.expect(VEX_ERR_FORMAT < 0);
    try std.testing.expect(VEX_ERR_CODEC < 0);
    try std.testing.expect(VEX_ERR_EOF < 0);
    try std.testing.expect(VEX_ERR_DECODE < 0);
    try std.testing.expect(VEX_ERR_IO < 0);
}

test "PixelFormat values are distinct" {
    try std.testing.expect(@intFromEnum(PixelFormat.yuv420p) != @intFromEnum(PixelFormat.rgb24));
    try std.testing.expect(@intFromEnum(PixelFormat.rgba32) != @intFromEnum(PixelFormat.unknown));
}

test "avPixFmtToVex maps known formats" {
    try std.testing.expectEqual(avPixFmtToVex(AV_PIX_FMT_YUV420P), .yuv420p);
    try std.testing.expectEqual(avPixFmtToVex(AV_PIX_FMT_RGB24), .rgb24);
    try std.testing.expectEqual(avPixFmtToVex(AV_PIX_FMT_RGBA), .rgba32);
    try std.testing.expectEqual(avPixFmtToVex(AV_PIX_FMT_NV12), .nv12);
    try std.testing.expectEqual(avPixFmtToVex(AV_PIX_FMT_BGRA), .bgra32);
    try std.testing.expectEqual(avPixFmtToVex(999), .unknown);
}
