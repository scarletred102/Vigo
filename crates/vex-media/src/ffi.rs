// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! FFI declarations for the Zig media library (ffmpeg + audio_output).
//!
//! These mirror the C ABI exports from `zig/media/ffmpeg.zig` and
//! `zig/media/audio_output.zig`. The Zig functions load ffmpeg DLLs
//! at runtime, so no compile-time ffmpeg dependency is needed.
//!
//! # Usage
//!
//! Call [`is_ffmpeg_available`] first. If it returns `true`, the full
//! decode pipeline (`open_file` → `read_packet` → `decode_video` /
//! `decode_audio` → `close`) is available. If `false`, the browser
//! can still render pages — just without `<video>` / `<audio>` playback.

/// FFmpeg C ABI return codes.
pub const VEX_OK: i32 = 0;
pub const VEX_ERR_INVALID: i32 = -1;
pub const VEX_ERR_OOM: i32 = -2;
pub const VEX_ERR_NOT_AVAILABLE: i32 = -10;
pub const VEX_ERR_FORMAT: i32 = -11;
pub const VEX_ERR_CODEC: i32 = -12;
pub const VEX_ERR_EOF: i32 = -13;
pub const VEX_ERR_DECODE: i32 = -14;
pub const VEX_ERR_IO: i32 = -15;

/// Audio output error codes.
pub const VEX_ERR_AUDIO_INIT: i32 = -30;
pub const VEX_ERR_AUDIO_WRITE: i32 = -31;
pub const VEX_ERR_AUDIO_DEVICE: i32 = -32;

/// Pixel format enum matching Zig's `PixelFormat`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Yuv420p = 0,
    Rgb24 = 1,
    Rgba32 = 2,
    Nv12 = 3,
    Bgra32 = 4,
    Unknown = 255,
}

/// Decoded video frame (mirrors Zig `VideoFrame`).
#[repr(C)]
#[derive(Debug)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: *mut u8,
    pub pixel_len: u32,
    pub format: PixelFormat,
    pub _pad: [u8; 3],
    pub pts_ms: i64,
}

impl Default for VideoFrame {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: std::ptr::null_mut(),
            pixel_len: 0,
            format: PixelFormat::Unknown,
            _pad: [0; 3],
            pts_ms: 0,
        }
    }
}

/// Decoded audio frame (mirrors Zig `AudioFrame`).
#[repr(C)]
#[derive(Debug)]
pub struct AudioFrame {
    pub samples: *mut f32,
    pub sample_count: u32,
    pub channels: u32,
    pub sample_rate: u32,
    pub _pad: u32,
    pub pts_ms: i64,
}

impl Default for AudioFrame {
    fn default() -> Self {
        Self {
            samples: std::ptr::null_mut(),
            sample_count: 0,
            channels: 0,
            sample_rate: 0,
            _pad: 0,
            pts_ms: 0,
        }
    }
}

/// Demuxed media packet (mirrors Zig `MediaPacket`).
#[repr(C)]
#[derive(Debug)]
pub struct MediaPacket {
    pub data: *mut u8,
    pub size: u32,
    pub stream_index: i32,
    pub pts_ms: i64,
    pub is_video: u8,
    pub is_audio: u8,
    pub _pad: [u8; 6],
}

impl Default for MediaPacket {
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            size: 0,
            stream_index: -1,
            pts_ms: 0,
            is_video: 0,
            is_audio: 0,
            _pad: [0; 6],
        }
    }
}

/// Media file info (mirrors Zig `MediaInfo`).
#[repr(C)]
#[derive(Debug, Default)]
pub struct MediaInfo {
    pub duration_ms: i64,
    pub has_video: u8,
    pub has_audio: u8,
    pub video_width: u32,
    pub video_height: u32,
    pub audio_sample_rate: u32,
    pub audio_channels: u32,
    pub _pad: [u8; 2],
}

// SAFETY: All these functions are implemented in Zig, compiled to a static
// library (`vex_media_zig`). They follow C ABI. The Zig side loads ffmpeg
// DLLs at runtime; if ffmpeg is not installed, `vex_media_open_file` returns
// `VEX_ERR_NOT_AVAILABLE` and `vex_media_ffmpeg_available` returns 0.
#[cfg(feature = "zig-media")]
unsafe extern "C" {
    // ── ffmpeg.zig ────────────────────────────────────────────────
    pub fn vex_media_ffmpeg_available() -> i32;
    pub fn vex_media_open_file(path: *const u8, out_ctx: *mut *mut c_void) -> i32;
    pub fn vex_media_read_packet(ctx: *mut c_void, out_pkt: *mut MediaPacket) -> i32;
    pub fn vex_media_decode_video(ctx: *mut c_void, out_frame: *mut VideoFrame) -> i32;
    pub fn vex_media_decode_audio(ctx: *mut c_void, out_frame: *mut AudioFrame) -> i32;
    pub fn vex_media_get_info(ctx: *mut c_void, out_info: *mut MediaInfo) -> i32;
    pub fn vex_media_close(ctx: *mut c_void) -> i32;
    pub fn vex_media_free_video_frame(frame: *mut VideoFrame);
    pub fn vex_media_free_audio_frame(frame: *mut AudioFrame);

    // ── audio_output.zig ─────────────────────────────────────────
    pub fn vex_audio_open(sample_rate: u32, channels: u32, out_handle: *mut *mut c_void) -> i32;
    pub fn vex_audio_write(handle: *mut c_void, samples: *const f32, count: u32) -> i32;
    pub fn vex_audio_close(handle: *mut c_void) -> i32;
    pub fn vex_audio_get_buffer_frames(handle: *mut c_void) -> u32;
    pub fn vex_audio_get_padding(handle: *mut c_void) -> u32;

    // ── root.zig ─────────────────────────────────────────────────
    pub fn vex_media_init() -> i32;
    pub fn vex_media_shutdown() -> i32;
}

/// Check whether ffmpeg runtime libraries are available.
///
/// Returns `true` if the Zig media library can load ffmpeg DLLs.
/// When the `zig-media` feature is not enabled, always returns `false`.
#[must_use]
pub fn is_ffmpeg_available() -> bool {
    #[cfg(feature = "zig-media")]
    {
        // SAFETY: This function only loads/unloads DLLs, no state is modified.
        unsafe { vex_media_ffmpeg_available() != 0 }
    }
    #[cfg(not(feature = "zig-media"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_frame_default() {
        let frame = VideoFrame::default();
        assert_eq!(frame.width, 0);
        assert_eq!(frame.height, 0);
        assert!(frame.pixels.is_null());
        assert_eq!(frame.format, PixelFormat::Unknown);
    }

    #[test]
    fn audio_frame_default() {
        let frame = AudioFrame::default();
        assert_eq!(frame.sample_count, 0);
        assert!(frame.samples.is_null());
    }

    #[test]
    fn media_packet_default() {
        let pkt = MediaPacket::default();
        assert_eq!(pkt.stream_index, -1);
        assert!(pkt.data.is_null());
    }

    #[test]
    fn media_info_default() {
        let info = MediaInfo::default();
        assert_eq!(info.duration_ms, 0);
        assert_eq!(info.has_video, 0);
        assert_eq!(info.has_audio, 0);
    }

    #[test]
    fn pixel_format_values() {
        assert_eq!(PixelFormat::Yuv420p as u8, 0);
        assert_eq!(PixelFormat::Rgba32 as u8, 2);
        assert_eq!(PixelFormat::Unknown as u8, 255);
    }

    #[test]
    fn ffmpeg_available_without_feature() {
        // Without zig-media feature, always returns false.
        #[cfg(not(feature = "zig-media"))]
        assert!(!is_ffmpeg_available());
    }
}
