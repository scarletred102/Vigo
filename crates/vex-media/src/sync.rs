// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Audio-video synchronisation using a clock-based approach.
//!
//! The audio stream drives the master clock. Video frames are presented
//! at their PTS relative to the audio clock. When video falls behind,
//! frames are skipped. When video is ahead, presentation waits.
//! Target: <20 ms audio-video drift.

use std::time::{Duration, Instant};

/// Decision returned by the sync engine for each video frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    /// Present the frame now — it is on time.
    Present,
    /// The frame is late — skip it and decode the next one.
    Skip,
    /// The frame is early — wait before presenting.
    Wait(Duration),
}

/// Master clock driven by the audio playback position.
///
/// The clock tracks the relationship between wall-clock time and media
/// time so that video frames can be synchronised to audio output.
#[derive(Debug)]
pub struct MediaClock {
    /// Wall-clock instant when the clock was last synced (audio callback).
    wall_anchor: Instant,
    /// Media timestamp (ms) corresponding to `wall_anchor`.
    media_anchor_ms: i64,
    /// Playback rate multiplier (1.0 = normal speed).
    rate: f64,
    /// Whether playback is paused.
    paused: bool,
    /// Media time at which playback was paused (ms).
    pause_media_ms: i64,
}

impl MediaClock {
    /// Create a new clock starting at media time 0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            wall_anchor: Instant::now(),
            media_anchor_ms: 0,
            rate: 1.0,
            paused: true,
            pause_media_ms: 0,
        }
    }

    /// Start or resume playback.
    pub fn play(&mut self) {
        if !self.paused {
            return;
        }
        self.wall_anchor = Instant::now();
        self.media_anchor_ms = self.pause_media_ms;
        self.paused = false;
    }

    /// Pause playback. Freezes the media time.
    pub fn pause(&mut self) {
        if self.paused {
            return;
        }
        self.pause_media_ms = self.current_media_ms();
        self.paused = true;
    }

    /// Seek to a specific media time (milliseconds).
    pub fn seek(&mut self, media_ms: i64) {
        if self.paused {
            self.pause_media_ms = media_ms;
        } else {
            self.wall_anchor = Instant::now();
            self.media_anchor_ms = media_ms;
        }
    }

    /// Update the clock from the audio playback position.
    ///
    /// Called whenever the audio subsystem reports its current position.
    /// This re-anchors the clock to keep audio and video in sync.
    pub fn sync_to_audio(&mut self, audio_media_ms: i64) {
        if self.paused {
            self.pause_media_ms = audio_media_ms;
        } else {
            self.wall_anchor = Instant::now();
            self.media_anchor_ms = audio_media_ms;
        }
    }

    /// Set the playback rate (e.g. 1.0 = normal, 2.0 = double speed).
    pub fn set_rate(&mut self, rate: f64) {
        if !self.paused {
            // Re-anchor before changing rate
            let now_ms = self.current_media_ms();
            self.wall_anchor = Instant::now();
            self.media_anchor_ms = now_ms;
        }
        self.rate = rate;
    }

    /// Get the current media time in milliseconds.
    #[must_use]
    pub fn current_media_ms(&self) -> i64 {
        if self.paused {
            return self.pause_media_ms;
        }
        let elapsed = self.wall_anchor.elapsed();
        let elapsed_ms = (elapsed.as_secs_f64() * self.rate * 1000.0) as i64;
        self.media_anchor_ms + elapsed_ms
    }

    /// Is the clock paused?
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Get the playback rate.
    #[must_use]
    pub fn rate(&self) -> f64 {
        self.rate
    }
}

impl Default for MediaClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Threshold for considering a frame "on time" (milliseconds).
const SYNC_THRESHOLD_MS: i64 = 20;

/// Maximum time to wait for an early frame before giving up (ms).
const MAX_WAIT_MS: i64 = 100;

/// Determine what to do with a video frame given its PTS.
///
/// Compares the frame's presentation timestamp against the master clock
/// and returns a [`SyncAction`].
#[must_use]
pub fn sync_video_frame(clock: &MediaClock, frame_pts_ms: i64) -> SyncAction {
    let clock_ms = clock.current_media_ms();
    let drift = frame_pts_ms - clock_ms;

    if drift < -SYNC_THRESHOLD_MS {
        // Frame is in the past — too late, skip it.
        SyncAction::Skip
    } else if drift > SYNC_THRESHOLD_MS {
        // Frame is in the future — wait.
        let wait = drift.min(MAX_WAIT_MS);
        SyncAction::Wait(Duration::from_millis(wait as u64))
    } else {
        // Frame is within threshold — present it.
        SyncAction::Present
    }
}

/// Calculate the audio-video drift in milliseconds.
///
/// Positive means video is ahead of audio. Negative means video is behind.
#[must_use]
pub fn calculate_drift(clock: &MediaClock, video_pts_ms: i64) -> i64 {
    video_pts_ms - clock.current_media_ms()
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_starts_paused_at_zero() {
        let clock = MediaClock::new();
        assert!(clock.is_paused());
        assert_eq!(clock.current_media_ms(), 0);
    }

    #[test]
    fn test_clock_play_pause() {
        let mut clock = MediaClock::new();
        clock.play();
        assert!(!clock.is_paused());

        // After a tiny delay, media time should advance
        std::thread::sleep(Duration::from_millis(10));
        let t = clock.current_media_ms();
        assert!(t >= 5, "expected >=5ms, got {t}");

        clock.pause();
        assert!(clock.is_paused());
        let paused_t = clock.current_media_ms();

        // While paused, time should not advance
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(clock.current_media_ms(), paused_t);
    }

    #[test]
    fn test_clock_seek() {
        let mut clock = MediaClock::new();
        clock.seek(5000);
        assert_eq!(clock.current_media_ms(), 5000);

        clock.play();
        std::thread::sleep(Duration::from_millis(10));
        assert!(clock.current_media_ms() >= 5005);
    }

    #[test]
    fn test_clock_sync_to_audio() {
        let mut clock = MediaClock::new();
        clock.play();
        std::thread::sleep(Duration::from_millis(10));

        // Audio reports it's at 100ms — re-anchor
        clock.sync_to_audio(100);
        let t = clock.current_media_ms();
        assert!((100..120).contains(&t), "expected ~100ms, got {t}");
    }

    #[test]
    fn test_clock_rate() {
        let mut clock = MediaClock::new();
        clock.set_rate(2.0);
        clock.play();

        std::thread::sleep(Duration::from_millis(50));
        let t = clock.current_media_ms();
        // At 2× speed, ~50ms wall time should yield ~100ms media time (±tolerance)
        assert!(t >= 70, "expected >=70ms at 2x, got {t}");
    }

    #[test]
    fn test_sync_frame_present() {
        let mut clock = MediaClock::new();
        clock.seek(1000);
        clock.play();

        // Frame at exactly current time → Present
        let action = sync_video_frame(&clock, 1000);
        assert_eq!(action, SyncAction::Present);
    }

    #[test]
    fn test_sync_frame_skip() {
        let mut clock = MediaClock::new();
        clock.play();
        std::thread::sleep(Duration::from_millis(50));

        // Frame from the past (PTS 0 when clock is at ~50ms)
        let action = sync_video_frame(&clock, 0);
        assert_eq!(action, SyncAction::Skip);
    }

    #[test]
    fn test_sync_frame_wait() {
        let mut clock = MediaClock::new();
        clock.play();

        // Frame far in the future
        let action = sync_video_frame(&clock, 5000);
        match action {
            SyncAction::Wait(d) => {
                assert!(d.as_millis() > 0);
                assert!(d.as_millis() <= 100); // capped at MAX_WAIT_MS
            }
            _ => panic!("expected Wait, got {:?}", action),
        }
    }

    #[test]
    fn test_calculate_drift() {
        let mut clock = MediaClock::new();
        clock.seek(1000);
        clock.play();

        // Video ahead
        let drift = calculate_drift(&clock, 1050);
        assert!(drift > 0);

        // Video behind
        let drift = calculate_drift(&clock, 950);
        assert!(drift < 0);
    }

    #[test]
    fn test_default_rate() {
        let clock = MediaClock::new();
        assert!((clock.rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pause_preserves_position() {
        let mut clock = MediaClock::new();
        clock.seek(2000);
        clock.play();
        std::thread::sleep(Duration::from_millis(10));
        clock.pause();
        let pos = clock.current_media_ms();

        clock.play();
        std::thread::sleep(Duration::from_millis(10));
        let new_pos = clock.current_media_ms();
        assert!(new_pos >= pos);
    }
}
