// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `<video>` and `<audio>` element model.
//!
//! Implements the HTMLMediaElement state machine: readiness states,
//! play/pause, seek, volume, playback rate, and media events.

use crate::sync::MediaClock;

/// Media element type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Video,
    Audio,
}

/// Network state of the media resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    /// No source set.
    Empty,
    /// Idle — source set but not actively loading.
    Idle,
    /// Actively loading/buffering.
    Loading,
    /// Resource could not be fetched.
    NoSource,
}

/// Ready state — how much data is available for playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadyState {
    /// No data at all.
    HaveNothing,
    /// Metadata (duration, dimensions) available.
    HaveMetadata,
    /// Data for the current position only (may stall).
    HaveCurrentData,
    /// Enough data for a few frames ahead.
    HaveFutureData,
    /// Enough data to play through to the end without stalling.
    HaveEnoughData,
}

/// Error that may occur during media loading/playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaError {
    /// An unknown error.
    Aborted,
    /// Network error during fetch.
    Network,
    /// Decode error — corrupt or unsupported codec.
    Decode,
    /// Source not supported.
    SrcNotSupported,
}

/// Event types fired by the media element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaEvent {
    Play,
    Pause,
    TimeUpdate,
    Ended,
    CanPlay,
    CanPlayThrough,
    Seeking,
    Seeked,
    VolumeChange,
    RateChange,
    DurationChange,
    LoadStart,
    LoadedMetadata,
    LoadedData,
    Waiting,
    Error,
}

/// Model for `<video>` and `<audio>` HTML elements.
///
/// This struct does **not** contain decoded frames or audio buffers;
/// it only tracks the *state machine* that the media pipeline drives.
#[derive(Debug)]
pub struct MediaElement {
    media_type: MediaType,
    src: String,
    clock: MediaClock,
    duration_ms: i64,
    volume: f64,
    muted: bool,
    playback_rate: f64,
    network_state: NetworkState,
    ready_state: ReadyState,
    error: Option<MediaError>,
    ended: bool,
    seeking: bool,
    events: Vec<MediaEvent>,
    /// Video dimensions (0 for audio)
    video_width: u32,
    video_height: u32,
}

impl MediaElement {
    /// Create a new media element with the given type and source URL.
    #[must_use]
    pub fn new(media_type: MediaType, src: &str) -> Self {
        Self {
            media_type,
            src: src.to_string(),
            clock: MediaClock::new(),
            duration_ms: 0,
            volume: 1.0,
            muted: false,
            playback_rate: 1.0,
            network_state: NetworkState::Empty,
            ready_state: ReadyState::HaveNothing,
            error: None,
            ended: false,
            seeking: false,
            events: Vec::new(),
            video_width: 0,
            video_height: 0,
        }
    }

    // ── Properties ─────────────────────────────────────────

    /// The source URL.
    #[must_use]
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Set a new source URL. Resets the element state.
    pub fn set_src(&mut self, src: &str) {
        self.src = src.to_string();
        self.clock = MediaClock::new();
        self.duration_ms = 0;
        self.network_state = NetworkState::Loading;
        self.ready_state = ReadyState::HaveNothing;
        self.error = None;
        self.ended = false;
        self.seeking = false;
        self.push_event(MediaEvent::LoadStart);
    }

    /// Current playback time in seconds.
    #[must_use]
    pub fn current_time(&self) -> f64 {
        self.clock.current_media_ms() as f64 / 1000.0
    }

    /// Duration in seconds. Returns 0.0 if unknown.
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.duration_ms as f64 / 1000.0
    }

    /// Is playback paused?
    #[must_use]
    pub fn paused(&self) -> bool {
        self.clock.is_paused()
    }

    /// Volume (0.0 – 1.0).
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Set volume (clamped to 0.0 – 1.0).
    pub fn set_volume(&mut self, v: f64) {
        self.volume = v.clamp(0.0, 1.0);
        self.push_event(MediaEvent::VolumeChange);
    }

    /// Is audio muted?
    #[must_use]
    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Set muted state.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        self.push_event(MediaEvent::VolumeChange);
    }

    /// Playback rate (1.0 = normal).
    #[must_use]
    pub fn playback_rate(&self) -> f64 {
        self.playback_rate
    }

    /// Set playback rate.
    pub fn set_playback_rate(&mut self, rate: f64) {
        self.playback_rate = rate.clamp(0.25, 4.0);
        self.clock.set_rate(self.playback_rate);
        self.push_event(MediaEvent::RateChange);
    }

    /// Network state.
    #[must_use]
    pub fn network_state(&self) -> NetworkState {
        self.network_state
    }

    /// Ready state.
    #[must_use]
    pub fn ready_state(&self) -> ReadyState {
        self.ready_state
    }

    /// Current error, if any.
    #[must_use]
    pub fn error(&self) -> Option<MediaError> {
        self.error
    }

    /// Has playback ended?
    #[must_use]
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Is the element currently seeking?
    #[must_use]
    pub fn seeking(&self) -> bool {
        self.seeking
    }

    /// Media type (video or audio).
    #[must_use]
    pub fn media_type(&self) -> MediaType {
        self.media_type
    }

    /// Video width (0 for audio elements).
    #[must_use]
    pub fn video_width(&self) -> u32 {
        self.video_width
    }

    /// Video height (0 for audio elements).
    #[must_use]
    pub fn video_height(&self) -> u32 {
        self.video_height
    }

    // ── Playback Control ───────────────────────────────────

    /// Start or resume playback.
    pub fn play(&mut self) {
        if self.ended {
            self.clock.seek(0);
            self.ended = false;
        }
        self.clock.play();
        self.push_event(MediaEvent::Play);
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.clock.pause();
        self.push_event(MediaEvent::Pause);
    }

    /// Seek to a time in seconds.
    pub fn seek(&mut self, time_secs: f64) {
        let ms = (time_secs * 1000.0) as i64;
        let clamped = ms.clamp(0, self.duration_ms.max(0));
        self.seeking = true;
        self.push_event(MediaEvent::Seeking);
        self.clock.seek(clamped);
        self.ended = false;
        self.seeking = false;
        self.push_event(MediaEvent::Seeked);
    }

    // ── Pipeline Callbacks ─────────────────────────────────

    /// Called by the media pipeline when metadata is available.
    pub fn on_metadata_loaded(&mut self, duration_ms: i64, width: u32, height: u32) {
        self.duration_ms = duration_ms;
        self.video_width = width;
        self.video_height = height;
        self.ready_state = ReadyState::HaveMetadata;
        self.network_state = NetworkState::Idle;
        self.push_event(MediaEvent::DurationChange);
        self.push_event(MediaEvent::LoadedMetadata);
    }

    /// Called when enough data is available to begin playback.
    pub fn on_can_play(&mut self) {
        if self.ready_state < ReadyState::HaveFutureData {
            self.ready_state = ReadyState::HaveFutureData;
        }
        self.push_event(MediaEvent::CanPlay);
    }

    /// Called when enough data is buffered for uninterrupted playback.
    pub fn on_can_play_through(&mut self) {
        self.ready_state = ReadyState::HaveEnoughData;
        self.push_event(MediaEvent::CanPlayThrough);
    }

    /// Called when playback reaches the end of the media.
    pub fn on_ended(&mut self) {
        self.ended = true;
        self.clock.pause();
        self.push_event(MediaEvent::Ended);
    }

    /// Called when the pipeline encounters an error.
    pub fn on_error(&mut self, error: MediaError) {
        self.error = Some(error);
        self.clock.pause();
        self.network_state = NetworkState::NoSource;
        self.push_event(MediaEvent::Error);
    }

    /// Called when buffering stalls playback.
    pub fn on_waiting(&mut self) {
        self.ready_state = ReadyState::HaveCurrentData;
        self.push_event(MediaEvent::Waiting);
    }

    /// Update the clock from the audio position (A/V sync).
    pub fn sync_to_audio(&mut self, audio_ms: i64) {
        self.clock.sync_to_audio(audio_ms);
    }

    /// Get a reference to the media clock (for video frame sync decisions).
    #[must_use]
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    // ── Events ─────────────────────────────────────────────

    /// Drain all pending events.
    pub fn drain_events(&mut self) -> Vec<MediaEvent> {
        std::mem::take(&mut self.events)
    }

    /// Push a time-update event (called periodically during playback).
    pub fn tick_time_update(&mut self) {
        if !self.paused() && !self.ended() {
            self.push_event(MediaEvent::TimeUpdate);
            // Check if we've reached the end
            if self.duration_ms > 0 && self.clock.current_media_ms() >= self.duration_ms {
                self.on_ended();
            }
        }
    }

    fn push_event(&mut self, event: MediaEvent) {
        self.events.push(event);
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_element_defaults() {
        let el = MediaElement::new(MediaType::Video, "test.mp4");
        assert_eq!(el.src(), "test.mp4");
        assert!(el.paused());
        assert_eq!(el.current_time(), 0.0);
        assert_eq!(el.duration(), 0.0);
        assert!((el.volume() - 1.0).abs() < f64::EPSILON);
        assert!(!el.muted());
        assert!((el.playback_rate() - 1.0).abs() < f64::EPSILON);
        assert_eq!(el.network_state(), NetworkState::Empty);
        assert_eq!(el.ready_state(), ReadyState::HaveNothing);
        assert_eq!(el.error(), None);
        assert!(!el.ended());
        assert!(!el.seeking());
        assert_eq!(el.media_type(), MediaType::Video);
    }

    #[test]
    fn test_play_pause() {
        let mut el = MediaElement::new(MediaType::Audio, "song.mp3");
        el.play();
        assert!(!el.paused());
        el.pause();
        assert!(el.paused());

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::Play));
        assert!(events.contains(&MediaEvent::Pause));
    }

    #[test]
    fn test_volume_clamp() {
        let mut el = MediaElement::new(MediaType::Audio, "a.mp3");
        el.set_volume(1.5);
        assert!((el.volume() - 1.0).abs() < f64::EPSILON);
        el.set_volume(-0.5);
        assert!(el.volume().abs() < f64::EPSILON);
    }

    #[test]
    fn test_playback_rate() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.set_playback_rate(2.0);
        assert!((el.playback_rate() - 2.0).abs() < f64::EPSILON);

        // Clamp at max
        el.set_playback_rate(10.0);
        assert!((el.playback_rate() - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metadata_loaded() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.on_metadata_loaded(60_000, 1920, 1080);
        assert_eq!(el.duration(), 60.0);
        assert_eq!(el.video_width(), 1920);
        assert_eq!(el.video_height(), 1080);
        assert_eq!(el.ready_state(), ReadyState::HaveMetadata);

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::DurationChange));
        assert!(events.contains(&MediaEvent::LoadedMetadata));
    }

    #[test]
    fn test_seek() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.on_metadata_loaded(10_000, 0, 0);
        el.seek(5.0);
        assert!((el.current_time() - 5.0).abs() < 0.1);

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::Seeking));
        assert!(events.contains(&MediaEvent::Seeked));
    }

    #[test]
    fn test_seek_clamps_to_duration() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.on_metadata_loaded(10_000, 0, 0);
        el.seek(999.0);
        assert!((el.current_time() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_error_handling() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.on_error(MediaError::Network);
        assert_eq!(el.error(), Some(MediaError::Network));
        assert!(el.paused());
        assert_eq!(el.network_state(), NetworkState::NoSource);
    }

    #[test]
    fn test_ended_state() {
        let mut el = MediaElement::new(MediaType::Audio, "a.mp3");
        el.on_ended();
        assert!(el.ended());
        assert!(el.paused());

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::Ended));
    }

    #[test]
    fn test_play_after_ended_restarts() {
        let mut el = MediaElement::new(MediaType::Audio, "a.mp3");
        el.on_metadata_loaded(5_000, 0, 0);
        el.on_ended();
        assert!(el.ended());

        el.play();
        assert!(!el.ended());
        assert!(!el.paused());
        assert!(el.current_time() < 0.1);
    }

    #[test]
    fn test_set_src_resets_state() {
        let mut el = MediaElement::new(MediaType::Video, "old.mp4");
        el.on_metadata_loaded(60_000, 1920, 1080);
        el.play();

        el.set_src("new.mp4");
        assert_eq!(el.src(), "new.mp4");
        assert_eq!(el.duration(), 0.0);
        assert_eq!(el.ready_state(), ReadyState::HaveNothing);
        assert_eq!(el.network_state(), NetworkState::Loading);
    }

    #[test]
    fn test_can_play_events() {
        let mut el = MediaElement::new(MediaType::Video, "v.mp4");
        el.on_can_play();
        assert!(el.ready_state() >= ReadyState::HaveFutureData);

        el.on_can_play_through();
        assert_eq!(el.ready_state(), ReadyState::HaveEnoughData);

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::CanPlay));
        assert!(events.contains(&MediaEvent::CanPlayThrough));
    }

    #[test]
    fn test_muted() {
        let mut el = MediaElement::new(MediaType::Audio, "a.mp3");
        assert!(!el.muted());
        el.set_muted(true);
        assert!(el.muted());

        let events = el.drain_events();
        assert!(events.contains(&MediaEvent::VolumeChange));
    }

    #[test]
    fn test_audio_element_zero_dimensions() {
        let el = MediaElement::new(MediaType::Audio, "a.mp3");
        assert_eq!(el.video_width(), 0);
        assert_eq!(el.video_height(), 0);
    }
}
