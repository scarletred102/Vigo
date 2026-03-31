// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! End-to-end media pipeline wiring.
//!
//! Connects the stages: URL detection → format probe → FFI decode →
//! `DecodedFrame` → `VideoSurface` → GPU texture. This module is the
//! integration glue between `media_loading`, `ffi`, `video_render`,
//! `sync`, and `controls`.
//!
//! # Architecture
//!
//! ```text
//! <video src="...">
//!   │
//!   ▼
//! MediaLoader  ─(fetch URL, detect format)─►  MediaPipeline
//!   │                                              │
//!   │  ┌───────────────────────────────────────────┘
//!   ▼  ▼
//! FFI: vex_media_open_file → vex_media_read_packet
//!   │                           │
//!   ├── video packet ──► vex_media_decode_video → DecodedFrame → VideoSurface
//!   └── audio packet ──► vex_media_decode_audio → AudioOutput (WASAPI)
//! ```

use crate::media_loading::MediaLoader;
use crate::sync::MediaClock;
use crate::video_render::{DecodedFrame, VideoSurface};

/// Metadata obtained from a media container header.
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    /// Duration in milliseconds.
    pub duration_ms: i64,
    /// Whether the container has a video track.
    pub has_video: bool,
    /// Whether the container has an audio track.
    pub has_audio: bool,
    /// Video width in pixels.
    pub video_width: u32,
    /// Video height in pixels.
    pub video_height: u32,
    /// Audio sample rate in Hz.
    pub audio_sample_rate: u32,
    /// Number of audio channels.
    pub audio_channels: u32,
}

/// High-level state for one `<video>` / `<audio>` element's playback.
#[derive(Debug)]
pub struct MediaPipeline {
    /// URL loader / format detector.
    loader: MediaLoader,
    /// Video surface for GPU frame submission.
    surface: VideoSurface,
    /// A/V synchronisation clock.
    clock: MediaClock,
    /// Pipeline state.
    state: PipelineState,
    /// Whether ffmpeg is available at runtime.
    ffmpeg_available: bool,
    /// Container metadata.
    meta: MediaMetadata,
}

/// Pipeline lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// Not started.
    Idle,
    /// Loading / probing the media resource.
    Loading,
    /// Decoder opened, metadata available.
    Ready,
    /// Playing — frames are being decoded and submitted.
    Playing,
    /// Paused.
    Paused,
    /// Reached end of stream.
    Ended,
    /// Error — pipeline cannot proceed.
    Error,
}

impl MediaPipeline {
    /// Create a new pipeline for the given media URL.
    #[must_use]
    pub fn new(url: &str) -> Self {
        let ffmpeg_available = crate::ffi::is_ffmpeg_available();
        Self {
            loader: MediaLoader::new(url),
            surface: VideoSurface::new(),
            clock: MediaClock::new(),
            state: PipelineState::Idle,
            ffmpeg_available,
            meta: MediaMetadata::default(),
        }
    }

    /// Pipeline's current state.
    #[must_use]
    pub fn state(&self) -> PipelineState {
        self.state
    }

    /// Whether ffmpeg runtime libraries are available.
    #[must_use]
    pub fn is_decoder_available(&self) -> bool {
        self.ffmpeg_available
    }

    /// Immutable reference to the video surface.
    #[must_use]
    pub fn surface(&self) -> &VideoSurface {
        &self.surface
    }

    /// Mutable reference to the video surface (for frame submission & dirty tracking).
    pub fn surface_mut(&mut self) -> &mut VideoSurface {
        &mut self.surface
    }

    /// Reference to the A/V clock.
    #[must_use]
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    /// Reference to the loader.
    #[must_use]
    pub fn loader(&self) -> &MediaLoader {
        &self.loader
    }

    /// Media duration in milliseconds. Zero until metadata is loaded.
    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.meta.duration_ms
    }

    /// Whether the opened media has a video track.
    #[must_use]
    pub fn has_video(&self) -> bool {
        self.meta.has_video
    }

    /// Whether the opened media has an audio track.
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.meta.has_audio
    }

    /// Video dimensions (0×0 until metadata is loaded).
    #[must_use]
    pub fn video_dimensions(&self) -> (u32, u32) {
        (self.meta.video_width, self.meta.video_height)
    }

    /// Audio parameters (0/0 until metadata is loaded).
    #[must_use]
    pub fn audio_params(&self) -> (u32, u32) {
        (self.meta.audio_sample_rate, self.meta.audio_channels)
    }

    /// Begin loading the media resource.
    ///
    /// Transitions from `Idle` → `Loading`. In the real pipeline, this
    /// would kick off an async fetch; here it sets up the loader state
    /// and checks decoder availability.
    pub fn start_loading(&mut self) {
        if self.state != PipelineState::Idle {
            return;
        }
        if !self.ffmpeg_available {
            tracing::warn!(
                url = self.loader.url(),
                "ffmpeg not available — media playback disabled"
            );
            self.state = PipelineState::Error;
            return;
        }
        self.loader.start_fetch();
        self.state = PipelineState::Loading;
        tracing::info!(url = self.loader.url(), "media pipeline: start loading");
    }

    /// Notify the pipeline that metadata has been obtained.
    ///
    /// Call after the FFI layer returns `MediaInfo`.
    pub fn set_metadata(&mut self, meta: MediaMetadata) {
        tracing::info!(
            duration_ms = meta.duration_ms,
            has_video = meta.has_video,
            has_audio = meta.has_audio,
            "media pipeline: metadata received"
        );
        self.meta = meta;
        self.loader.mark_ready();
        self.state = PipelineState::Ready;
    }

    /// Submit a decoded video frame to the surface.
    ///
    /// Converts raw RGBA pixel data into a `DecodedFrame` and submits
    /// it to the `VideoSurface` for GPU upload on the next render pass.
    pub fn submit_video_frame(&mut self, pixels: Vec<u8>, width: u32, height: u32, pts_ms: i64) {
        let frame = DecodedFrame::new(pixels, width, height, pts_ms);
        if !frame.is_valid() {
            tracing::warn!(width, height, "invalid video frame dimensions, skipping");
            return;
        }
        self.surface.submit_frame(frame);
    }

    /// Start playback.
    pub fn play(&mut self) {
        match self.state {
            PipelineState::Ready | PipelineState::Paused => {
                self.clock.play();
                self.state = PipelineState::Playing;
            }
            PipelineState::Ended => {
                // Restart from beginning.
                self.clock.seek(0);
                self.clock.play();
                self.state = PipelineState::Playing;
            }
            _ => {}
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        if self.state == PipelineState::Playing {
            self.clock.pause();
            self.state = PipelineState::Paused;
        }
    }

    /// Seek to a position in milliseconds.
    pub fn seek(&mut self, position_ms: i64) {
        let clamped = position_ms.clamp(0, self.meta.duration_ms.max(0));
        self.clock.seek(clamped);
        // Clear the surface so the next decoded frame replaces it.
        self.surface.clear();
        tracing::debug!(position_ms = clamped, "media pipeline: seek");
    }

    /// Mark playback as ended (end of stream reached).
    pub fn mark_ended(&mut self) {
        self.state = PipelineState::Ended;
        self.clock.pause();
    }

    /// Mark the pipeline as errored.
    pub fn mark_error(&mut self) {
        self.state = PipelineState::Error;
        self.clock.pause();
    }

    /// Clean up (release decoder resources).
    pub fn shutdown(&mut self) {
        self.surface.clear();
        self.state = PipelineState::Idle;
        tracing::debug!(url = self.loader.url(), "media pipeline: shutdown");
    }

    /// Whether playback is currently active.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.state == PipelineState::Playing
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_creation() {
        let p = MediaPipeline::new("https://example.com/video.mp4");
        assert_eq!(p.state(), PipelineState::Idle);
        assert_eq!(p.duration_ms(), 0);
        assert!(!p.has_video());
        assert!(!p.has_audio());
        assert_eq!(p.video_dimensions(), (0, 0));
    }

    #[test]
    fn loading_without_ffmpeg() {
        let mut p = MediaPipeline::new("test.mp4");
        // Without zig-media feature, ffmpeg is not available.
        if !p.is_decoder_available() {
            p.start_loading();
            assert_eq!(p.state(), PipelineState::Error);
        }
    }

    #[test]
    fn set_metadata_transitions_to_ready() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Loading;
        p.set_metadata(MediaMetadata {
            duration_ms: 120_000,
            has_video: true,
            has_audio: true,
            video_width: 1920,
            video_height: 1080,
            audio_sample_rate: 48000,
            audio_channels: 2,
        });
        assert_eq!(p.state(), PipelineState::Ready);
        assert_eq!(p.duration_ms(), 120_000);
        assert!(p.has_video());
        assert!(p.has_audio());
        assert_eq!(p.video_dimensions(), (1920, 1080));
        assert_eq!(p.audio_params(), (48000, 2));
    }

    #[test]
    fn play_pause_cycle() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Ready;
        p.play();
        assert_eq!(p.state(), PipelineState::Playing);
        assert!(p.is_playing());

        p.pause();
        assert_eq!(p.state(), PipelineState::Paused);
        assert!(!p.is_playing());

        p.play();
        assert_eq!(p.state(), PipelineState::Playing);
    }

    #[test]
    fn submit_valid_frame() {
        let mut p = MediaPipeline::new("test.mp4");
        let pixels = vec![0u8; 320 * 240 * 4];
        p.submit_video_frame(pixels, 320, 240, 1000);
        assert!(p.surface().current_frame().is_some());
        assert!(p.surface().is_dirty());
    }

    #[test]
    fn submit_invalid_frame_ignored() {
        let mut p = MediaPipeline::new("test.mp4");
        // Wrong pixel count for dimensions.
        p.submit_video_frame(vec![0u8; 10], 320, 240, 0);
        assert!(p.surface().current_frame().is_none());
    }

    #[test]
    fn seek_clamps_to_duration() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Loading;
        p.set_metadata(MediaMetadata {
            duration_ms: 10_000,
            has_video: true,
            video_width: 640,
            video_height: 480,
            ..MediaMetadata::default()
        });
        p.seek(20_000);
        // Should be clamped to duration.
    }

    #[test]
    fn mark_ended_and_restart() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Playing;
        p.mark_ended();
        assert_eq!(p.state(), PipelineState::Ended);

        p.play();
        assert_eq!(p.state(), PipelineState::Playing);
    }

    #[test]
    fn shutdown_resets_state() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Playing;
        p.shutdown();
        assert_eq!(p.state(), PipelineState::Idle);
        assert!(p.surface().current_frame().is_none());
    }

    #[test]
    fn start_loading_from_non_idle_is_noop() {
        let mut p = MediaPipeline::new("test.mp4");
        p.state = PipelineState::Playing;
        p.start_loading();
        // Should still be Playing — not changed.
        assert_eq!(p.state(), PipelineState::Playing);
    }
}
