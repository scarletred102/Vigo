// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Streaming integration: HLS/DASH playlist → ABR → segment fetch → pipeline.
//!
//! This module wires the HLS M3U8 parser, DASH MPD parser, and ABR
//! controller together into a coherent streaming session. When a media
//! URL points at a `.m3u8` or `.mpd` manifest, a [`StreamingSession`]
//! manages quality selection and segment fetching.

use std::time::{Duration, Instant};

use crate::abr::{AbrController, QualityLevel};
#[allow(unused_imports)]
use crate::dash::{ContentType, Mpd};
use crate::hls::{MasterPlaylist, MediaPlaylist};

/// Which streaming protocol is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingProtocol {
    Hls,
    Dash,
}

/// A pending segment fetch.
#[derive(Debug, Clone)]
pub struct SegmentRequest {
    /// Absolute URL of the segment.
    pub url: String,
    /// Expected duration of the segment.
    pub duration: Duration,
    /// Byte range, if applicable (offset, length).
    pub byte_range: Option<(u64, u64)>,
    /// Segment index within the playlist.
    pub index: usize,
}

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Waiting for manifest to be fetched.
    LoadingManifest,
    /// Manifest parsed, requesting segments.
    Active,
    /// All segments fetched from the playlist.
    Complete,
    /// Error occurred.
    Error,
}

/// Manages streaming playback for one media element.
#[derive(Debug)]
pub struct StreamingSession {
    /// Which protocol we're using.
    protocol: StreamingProtocol,
    /// Current state.
    state: SessionState,
    /// ABR controller (quality level selection).
    abr: AbrController,
    /// Index of the next segment to fetch (in the current quality).
    next_segment_index: usize,
    /// Total number of segments in the active media playlist.
    total_segments: usize,
    /// Buffer level: total unfetched-but-scheduled segment duration.
    buffer_level_secs: f64,
    /// URLs of segments at each quality level (outer = quality, inner = segments).
    segment_urls: Vec<Vec<String>>,
    /// Segment durations (assumed uniform across qualities).
    segment_durations: Vec<Duration>,
    /// When the session started.
    _started_at: Instant,
}

impl StreamingSession {
    // ── HLS ────────────────────────────────────────────────────

    /// Create a session from a parsed HLS master playlist.
    #[must_use]
    pub fn from_hls_master(master: &MasterPlaylist) -> Self {
        let levels: Vec<QualityLevel> = master
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| QualityLevel {
                index: i,
                bandwidth: v.bandwidth,
                label: match (v.width, v.height) {
                    (Some(_w), Some(h)) => format!("{h}p"),
                    _ => format!("{}kbps", v.bandwidth / 1000),
                },
            })
            .collect();

        let segment_urls: Vec<Vec<String>> = master
            .variants
            .iter()
            .map(|v| vec![v.uri.clone()])
            .collect();

        Self {
            protocol: StreamingProtocol::Hls,
            state: SessionState::LoadingManifest,
            abr: AbrController::new(levels),
            next_segment_index: 0,
            total_segments: 0,
            buffer_level_secs: 0.0,
            segment_urls,
            segment_durations: Vec::new(),
            _started_at: Instant::now(),
        }
    }

    /// Update the session with a parsed media playlist (one quality level).
    pub fn set_hls_media_playlist(&mut self, playlist: &MediaPlaylist) {
        self.total_segments = playlist.segments.len();
        self.segment_durations = playlist.segments.iter().map(|s| s.duration).collect();
        self.state = SessionState::Active;
        tracing::info!(
            segments = self.total_segments,
            "HLS media playlist loaded"
        );
    }

    // ── DASH ───────────────────────────────────────────────────

    /// Create a session from a parsed DASH MPD.
    #[must_use]
    pub fn from_dash_mpd(mpd: &Mpd) -> Self {
        let mut levels = Vec::new();
        let mut segment_urls = Vec::new();

        for period in &mpd.periods {
            for adapt_set in &period.adaptation_sets {
                if adapt_set.content_type != ContentType::Video {
                    continue;
                }
                for (i, rep) in adapt_set.representations.iter().enumerate() {
                    levels.push(QualityLevel {
                        index: i,
                        bandwidth: rep.bandwidth,
                        label: match rep.height {
                            Some(h) => format!("{h}p"),
                            None => format!("{}kbps", rep.bandwidth / 1000),
                        },
                    });
                    // Collect segment URLs from the representation.
                    if rep.segment_urls.is_empty() {
                        let base = mpd.base_url.as_deref().unwrap_or("");
                        segment_urls.push(vec![base.to_string()]);
                    } else {
                        segment_urls.push(rep.segment_urls.clone());
                    }
                }
            }
        }

        Self {
            protocol: StreamingProtocol::Dash,
            state: SessionState::Active,
            abr: AbrController::new(levels),
            next_segment_index: 0,
            total_segments: 0,
            buffer_level_secs: 0.0,
            segment_urls,
            segment_durations: Vec::new(),
            _started_at: Instant::now(),
        }
    }

    // ── Common API ─────────────────────────────────────────────

    /// Current session state.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Which protocol is in use.
    #[must_use]
    pub fn protocol(&self) -> StreamingProtocol {
        self.protocol
    }

    /// Current quality level index.
    #[must_use]
    pub fn current_quality(&self) -> usize {
        self.abr.current_level()
    }

    /// Number of available quality levels.
    #[must_use]
    pub fn quality_count(&self) -> usize {
        self.abr.level_count()
    }

    /// Buffer level in seconds.
    #[must_use]
    pub fn buffer_level_secs(&self) -> f64 {
        self.buffer_level_secs
    }

    /// Get the next segment to fetch, applying ABR selection first.
    ///
    /// Returns `None` if all segments have been fetched or the session
    /// is not in `Active` state.
    #[must_use]
    pub fn next_segment(&mut self) -> Option<SegmentRequest> {
        if self.state != SessionState::Active {
            return None;
        }
        if self.next_segment_index >= self.total_segments {
            self.state = SessionState::Complete;
            return None;
        }

        // Ask ABR for quality decision.
        let _decision = self.abr.decide(self.buffer_level_secs);
        let quality = self.abr.current_level();

        // Build segment URL.
        let url = self
            .segment_urls
            .get(quality)
            .and_then(|urls| urls.get(self.next_segment_index))
            .cloned()
            .unwrap_or_default();

        let duration = self
            .segment_durations
            .get(self.next_segment_index)
            .copied()
            .unwrap_or(Duration::from_secs(4));

        let request = SegmentRequest {
            url,
            duration,
            byte_range: None,
            index: self.next_segment_index,
        };

        self.next_segment_index += 1;
        Some(request)
    }

    /// Report a completed segment download for throughput tracking.
    pub fn report_download(&mut self, bytes: u64, elapsed: Duration) {
        if elapsed.as_millis() == 0 {
            return;
        }
        let bps = (bytes * 8 * 1000) / elapsed.as_millis() as u64;
        self.abr.record_throughput(bps);
    }

    /// Update the buffer level (called as segments are fetched).
    pub fn set_buffer_level(&mut self, secs: f64) {
        self.buffer_level_secs = secs;
    }

    /// Mark session as errored.
    pub fn mark_error(&mut self) {
        self.state = SessionState::Error;
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hls::{Segment, VariantStream};

    fn sample_master() -> MasterPlaylist {
        MasterPlaylist {
            variants: vec![
                VariantStream {
                    bandwidth: 500_000,
                    average_bandwidth: None,
                    width: Some(640),
                    height: Some(360),
                    codecs: None,
                    uri: "low/playlist.m3u8".to_string(),
                    frame_rate: None,
                    audio: None,
                },
                VariantStream {
                    bandwidth: 2_000_000,
                    average_bandwidth: None,
                    width: Some(1920),
                    height: Some(1080),
                    codecs: None,
                    uri: "high/playlist.m3u8".to_string(),
                    frame_rate: None,
                    audio: None,
                },
            ],
            version: Some(3),
        }
    }

    fn sample_media_playlist() -> MediaPlaylist {
        MediaPlaylist {
            target_duration: Duration::from_secs(6),
            segments: vec![
                Segment {
                    duration: Duration::from_secs(6),
                    uri: "seg0.ts".to_string(),
                    title: None,
                    byte_range: None,
                    encryption: None,
                },
                Segment {
                    duration: Duration::from_secs(6),
                    uri: "seg1.ts".to_string(),
                    title: None,
                    byte_range: None,
                    encryption: None,
                },
                Segment {
                    duration: Duration::from_secs(4),
                    uri: "seg2.ts".to_string(),
                    title: None,
                    byte_range: None,
                    encryption: None,
                },
            ],
            media_sequence: 0,
            ended: true,
            version: Some(3),
            encryption: None,
        }
    }

    #[test]
    fn hls_session_creation() {
        let master = sample_master();
        let session = StreamingSession::from_hls_master(&master);
        assert_eq!(session.protocol(), StreamingProtocol::Hls);
        assert_eq!(session.state(), SessionState::LoadingManifest);
        assert_eq!(session.quality_count(), 2);
    }

    #[test]
    fn hls_session_with_media_playlist() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        let media = sample_media_playlist();
        session.set_hls_media_playlist(&media);
        assert_eq!(session.state(), SessionState::Active);
    }

    #[test]
    fn next_segment_returns_requests() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        let media = sample_media_playlist();
        session.set_hls_media_playlist(&media);

        // Populate segment URLs for the active quality.
        session.segment_urls = vec![
            vec![
                "seg0.ts".to_string(),
                "seg1.ts".to_string(),
                "seg2.ts".to_string(),
            ],
            vec![
                "seg0_hd.ts".to_string(),
                "seg1_hd.ts".to_string(),
                "seg2_hd.ts".to_string(),
            ],
        ];
        session.total_segments = 3;

        let seg = session.next_segment();
        assert!(seg.is_some());
        assert_eq!(seg.unwrap().index, 0);

        let seg = session.next_segment();
        assert!(seg.is_some());
        assert_eq!(seg.unwrap().index, 1);
    }

    #[test]
    fn session_completes_when_all_fetched() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        session.state = SessionState::Active;
        session.total_segments = 1;
        session.segment_urls = vec![vec!["seg.ts".to_string()]];
        session.segment_durations = vec![Duration::from_secs(4)];

        let _seg = session.next_segment();
        assert_eq!(session.next_segment_index, 1);

        let none = session.next_segment();
        assert!(none.is_none());
        assert_eq!(session.state(), SessionState::Complete);
    }

    #[test]
    fn report_download_updates_abr() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        session.report_download(100_000, Duration::from_millis(500));
        // Should not panic — throughput sample recorded.
    }

    #[test]
    fn buffer_level_tracking() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        session.set_buffer_level(12.5);
        assert!((session.buffer_level_secs() - 12.5).abs() < 0.01);
    }

    #[test]
    fn mark_error_stops_segments() {
        let master = sample_master();
        let mut session = StreamingSession::from_hls_master(&master);
        session.state = SessionState::Active;
        session.total_segments = 5;
        session.mark_error();
        assert_eq!(session.state(), SessionState::Error);
        assert!(session.next_segment().is_none());
    }
}
