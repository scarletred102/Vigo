// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Media source loading pipeline.
//!
//! When a `<video src="...">` or `<audio src="...">` is encountered,
//! this module fetches the URL via `vex-net`, detects the container
//! format, and initiates decoding via the Zig ffmpeg bindings.

use std::path::Path;

use thiserror::Error;

/// Errors during media loading.
#[derive(Debug, Error)]
pub enum MediaLoadError {
    #[error("unsupported media format: {0}")]
    UnsupportedFormat(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("decoder not available")]
    DecoderNotAvailable,
    #[error("decode error: {0}")]
    Decode(String),
}

/// Known container/codec formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Mp4,
    WebM,
    Ogg,
    Mp3,
    Flac,
    Wav,
    Aac,
    Unknown,
}

impl MediaFormat {
    /// Detect format from Content-Type header value.
    #[must_use]
    pub fn from_content_type(ct: &str) -> Self {
        let ct_lower = ct.to_ascii_lowercase();
        if ct_lower.contains("video/mp4") || ct_lower.contains("audio/mp4") {
            Self::Mp4
        } else if ct_lower.contains("video/webm") || ct_lower.contains("audio/webm") {
            Self::WebM
        } else if ct_lower.contains("video/ogg") || ct_lower.contains("audio/ogg") {
            Self::Ogg
        } else if ct_lower.contains("audio/mpeg") || ct_lower.contains("audio/mp3") {
            Self::Mp3
        } else if ct_lower.contains("audio/flac") {
            Self::Flac
        } else if ct_lower.contains("audio/wav") || ct_lower.contains("audio/wave") {
            Self::Wav
        } else if ct_lower.contains("audio/aac") {
            Self::Aac
        } else {
            Self::Unknown
        }
    }

    /// Detect format from file extension.
    #[must_use]
    pub fn from_extension(path: &str) -> Self {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "mp4" | "m4v" | "m4a" => Self::Mp4,
            "webm" => Self::WebM,
            "ogg" | "ogv" | "oga" => Self::Ogg,
            "mp3" => Self::Mp3,
            "flac" => Self::Flac,
            "wav" => Self::Wav,
            "aac" => Self::Aac,
            _ => Self::Unknown,
        }
    }

    /// Whether this format typically contains video.
    #[must_use]
    pub fn has_video(self) -> bool {
        matches!(self, Self::Mp4 | Self::WebM | Self::Ogg)
    }

    /// MIME type string for this format.
    #[must_use]
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Mp4 => "video/mp4",
            Self::WebM => "video/webm",
            Self::Ogg => "video/ogg",
            Self::Mp3 => "audio/mpeg",
            Self::Flac => "audio/flac",
            Self::Wav => "audio/wav",
            Self::Aac => "audio/aac",
            Self::Unknown => "application/octet-stream",
        }
    }
}

/// Detect media format from magic bytes (file signature).
#[must_use]
pub fn detect_format_from_bytes(data: &[u8]) -> MediaFormat {
    if data.len() < 12 {
        return MediaFormat::Unknown;
    }

    // ftyp box — MP4
    if &data[4..8] == b"ftyp" {
        return MediaFormat::Mp4;
    }
    // WebM (EBML header)
    if data[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return MediaFormat::WebM;
    }
    // OGG
    if &data[0..4] == b"OggS" {
        return MediaFormat::Ogg;
    }
    // FLAC
    if &data[0..4] == b"fLaC" {
        return MediaFormat::Flac;
    }
    // WAV (RIFF....WAVE)
    if &data[0..4] == b"RIFF" && data.len() >= 12 && &data[8..12] == b"WAVE" {
        return MediaFormat::Wav;
    }
    // MP3 (ID3 tag or sync word)
    if &data[0..3] == b"ID3" || (data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) {
        return MediaFormat::Mp3;
    }

    MediaFormat::Unknown
}

/// State of a media loading operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    /// Not started.
    Idle,
    /// Fetching the resource.
    Fetching,
    /// Probing format / opening decoder.
    Opening,
    /// Ready — decoder is open and metadata is available.
    Ready,
    /// Error — loading failed.
    Failed,
}

/// Tracks the loading of a media resource.
#[derive(Debug)]
pub struct MediaLoader {
    url: String,
    format: MediaFormat,
    state: LoadState,
    bytes_loaded: u64,
    content_length: Option<u64>,
}

impl MediaLoader {
    /// Create a new loader for the given URL.
    #[must_use]
    pub fn new(url: &str) -> Self {
        let format = MediaFormat::from_extension(url);
        Self {
            url: url.to_string(),
            format,
            state: LoadState::Idle,
            bytes_loaded: 0,
            content_length: None,
        }
    }

    /// The target URL.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Detected or guessed format.
    #[must_use]
    pub fn format(&self) -> MediaFormat {
        self.format
    }

    /// Current loading state.
    #[must_use]
    pub fn state(&self) -> LoadState {
        self.state
    }

    /// Bytes loaded so far.
    #[must_use]
    pub fn bytes_loaded(&self) -> u64 {
        self.bytes_loaded
    }

    /// Total content length, if known from HTTP headers.
    #[must_use]
    pub fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    /// Loading progress (0.0 – 1.0). Returns `None` if length is unknown.
    #[must_use]
    pub fn progress(&self) -> Option<f64> {
        self.content_length.map(|total| {
            if total == 0 {
                1.0
            } else {
                self.bytes_loaded as f64 / total as f64
            }
        })
    }

    /// Transition to fetching state.
    pub fn start_fetch(&mut self) {
        self.state = LoadState::Fetching;
    }

    /// Record received bytes and optionally update format detection.
    pub fn on_data(&mut self, chunk: &[u8]) {
        if self.bytes_loaded == 0 && self.format == MediaFormat::Unknown {
            self.format = detect_format_from_bytes(chunk);
        }
        self.bytes_loaded += chunk.len() as u64;
    }

    /// Set total content length (from Content-Length header).
    pub fn set_content_length(&mut self, len: u64) {
        self.content_length = Some(len);
    }

    /// Set format from Content-Type header.
    pub fn set_content_type(&mut self, ct: &str) {
        let detected = MediaFormat::from_content_type(ct);
        if detected != MediaFormat::Unknown {
            self.format = detected;
        }
    }

    /// Transition to opening/decoding state.
    pub fn start_opening(&mut self) {
        self.state = LoadState::Opening;
    }

    /// Mark as ready.
    pub fn mark_ready(&mut self) {
        self.state = LoadState::Ready;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.state = LoadState::Failed;
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_content_type() {
        assert_eq!(
            MediaFormat::from_content_type("video/mp4"),
            MediaFormat::Mp4
        );
        assert_eq!(
            MediaFormat::from_content_type("video/webm"),
            MediaFormat::WebM
        );
        assert_eq!(
            MediaFormat::from_content_type("audio/mpeg"),
            MediaFormat::Mp3
        );
        assert_eq!(
            MediaFormat::from_content_type("audio/flac"),
            MediaFormat::Flac
        );
        assert_eq!(
            MediaFormat::from_content_type("text/html"),
            MediaFormat::Unknown
        );
    }

    #[test]
    fn test_format_from_extension() {
        assert_eq!(MediaFormat::from_extension("video.mp4"), MediaFormat::Mp4);
        assert_eq!(MediaFormat::from_extension("audio.webm"), MediaFormat::WebM);
        assert_eq!(MediaFormat::from_extension("song.mp3"), MediaFormat::Mp3);
        assert_eq!(MediaFormat::from_extension("track.flac"), MediaFormat::Flac);
        assert_eq!(
            MediaFormat::from_extension("data.bin"),
            MediaFormat::Unknown
        );
    }

    #[test]
    fn test_detect_mp4_magic() {
        let mut data = [0u8; 12];
        data[4..8].copy_from_slice(b"ftyp");
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Mp4);
    }

    #[test]
    fn test_detect_webm_magic() {
        let data = [0x1A, 0x45, 0xDF, 0xA3, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::WebM);
    }

    #[test]
    fn test_detect_ogg_magic() {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(b"OggS");
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Ogg);
    }

    #[test]
    fn test_detect_wav_magic() {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[8..12].copy_from_slice(b"WAVE");
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Wav);
    }

    #[test]
    fn test_detect_mp3_id3_magic() {
        let mut data = [0u8; 12];
        data[0..3].copy_from_slice(b"ID3");
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Mp3);
    }

    #[test]
    fn test_detect_flac_magic() {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(b"fLaC");
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Flac);
    }

    #[test]
    fn test_detect_unknown() {
        let data = [0u8; 12];
        assert_eq!(detect_format_from_bytes(&data), MediaFormat::Unknown);
    }

    #[test]
    fn test_detect_short_data() {
        assert_eq!(detect_format_from_bytes(&[0; 4]), MediaFormat::Unknown);
    }

    #[test]
    fn test_has_video() {
        assert!(MediaFormat::Mp4.has_video());
        assert!(MediaFormat::WebM.has_video());
        assert!(!MediaFormat::Mp3.has_video());
        assert!(!MediaFormat::Flac.has_video());
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(MediaFormat::Mp4.mime_type(), "video/mp4");
        assert_eq!(MediaFormat::Mp3.mime_type(), "audio/mpeg");
    }

    #[test]
    fn test_loader_new() {
        let l = MediaLoader::new("https://example.com/video.mp4");
        assert_eq!(l.url(), "https://example.com/video.mp4");
        assert_eq!(l.format(), MediaFormat::Mp4);
        assert_eq!(l.state(), LoadState::Idle);
        assert_eq!(l.bytes_loaded(), 0);
        assert_eq!(l.content_length(), None);
    }

    #[test]
    fn test_loader_state_transitions() {
        let mut l = MediaLoader::new("v.mp4");
        l.start_fetch();
        assert_eq!(l.state(), LoadState::Fetching);

        l.set_content_length(1000);
        l.on_data(&[0; 500]);
        assert_eq!(l.bytes_loaded(), 500);
        assert!((l.progress().unwrap() - 0.5).abs() < f64::EPSILON);

        l.start_opening();
        assert_eq!(l.state(), LoadState::Opening);

        l.mark_ready();
        assert_eq!(l.state(), LoadState::Ready);
    }

    #[test]
    fn test_loader_format_detection_from_bytes() {
        let mut l = MediaLoader::new("stream"); // Unknown extension
        assert_eq!(l.format(), MediaFormat::Unknown);

        let mut magic = [0u8; 12];
        magic[4..8].copy_from_slice(b"ftyp");
        l.on_data(&magic);
        assert_eq!(l.format(), MediaFormat::Mp4);
    }

    #[test]
    fn test_loader_format_from_content_type() {
        let mut l = MediaLoader::new("stream");
        l.set_content_type("video/webm; codecs=vp9");
        assert_eq!(l.format(), MediaFormat::WebM);
    }

    #[test]
    fn test_loader_failed_state() {
        let mut l = MediaLoader::new("bad.mp4");
        l.start_fetch();
        l.mark_failed();
        assert_eq!(l.state(), LoadState::Failed);
    }
}
