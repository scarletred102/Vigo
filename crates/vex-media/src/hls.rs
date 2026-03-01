// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HLS (HTTP Live Streaming) M3U8 playlist parser.
//!
//! Parses both master playlists (variant streams / quality levels)
//! and media playlists (segment lists). Extracts: variant streams,
//! segment URLs, duration, encryption info.

use std::time::Duration;

use thiserror::Error;

/// Errors during HLS parsing.
#[derive(Debug, Error)]
pub enum HlsError {
    #[error("not an M3U8 playlist")]
    NotM3u8,
    #[error("invalid tag: {0}")]
    InvalidTag(String),
    #[error("missing required info")]
    MissingInfo,
}

/// A variant stream in a master playlist.
#[derive(Debug, Clone)]
pub struct VariantStream {
    /// Bandwidth in bits per second.
    pub bandwidth: u64,
    /// Average bandwidth, if specified.
    pub average_bandwidth: Option<u64>,
    /// Resolution width.
    pub width: Option<u32>,
    /// Resolution height.
    pub height: Option<u32>,
    /// Codec string.
    pub codecs: Option<String>,
    /// URI of the media playlist.
    pub uri: String,
    /// Frame rate, if specified.
    pub frame_rate: Option<f64>,
    /// Audio group ID.
    pub audio: Option<String>,
}

/// A media segment in a media playlist.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Segment duration.
    pub duration: Duration,
    /// Segment URI.
    pub uri: String,
    /// Segment title/comment (from EXTINF).
    pub title: Option<String>,
    /// Byte range, if specified (offset, length).
    pub byte_range: Option<(u64, u64)>,
    /// Encryption info for this segment.
    pub encryption: Option<EncryptionInfo>,
}

/// Encryption method for a segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionMethod {
    None,
    Aes128,
    SampleAes,
}

/// Encryption info for HLS segments.
#[derive(Debug, Clone)]
pub struct EncryptionInfo {
    /// Encryption method.
    pub method: EncryptionMethod,
    /// Key URI.
    pub uri: Option<String>,
    /// Initialization vector (hex string).
    pub iv: Option<String>,
}

/// Parsed HLS master playlist.
#[derive(Debug, Clone)]
pub struct MasterPlaylist {
    /// Version (from #EXT-X-VERSION).
    pub version: Option<u32>,
    /// Variant streams (quality levels).
    pub variants: Vec<VariantStream>,
}

/// Parsed HLS media playlist.
#[derive(Debug, Clone)]
pub struct MediaPlaylist {
    /// Version.
    pub version: Option<u32>,
    /// Target segment duration.
    pub target_duration: Duration,
    /// Media sequence number of the first segment.
    pub media_sequence: u64,
    /// Whether the playlist is complete (has ENDLIST).
    pub ended: bool,
    /// Segments.
    pub segments: Vec<Segment>,
    /// Current encryption state.
    pub encryption: Option<EncryptionInfo>,
}

/// Determine playlist type from content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistType {
    Master,
    Media,
}

/// Detect whether the given text is a master or media playlist.
#[must_use]
pub fn detect_playlist_type(content: &str) -> Option<PlaylistType> {
    if !content.starts_with("#EXTM3U") {
        return None;
    }
    if content.contains("#EXT-X-STREAM-INF") {
        Some(PlaylistType::Master)
    } else if content.contains("#EXTINF") || content.contains("#EXT-X-TARGETDURATION") {
        Some(PlaylistType::Media)
    } else {
        Some(PlaylistType::Master) // default to master if ambiguous
    }
}

/// Parse a master playlist from M3U8 text.
pub fn parse_master_playlist(content: &str) -> Result<MasterPlaylist, HlsError> {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");
    if !first.starts_with("#EXTM3U") {
        return Err(HlsError::NotM3u8);
    }

    let mut version = None;
    let mut variants = Vec::new();
    let mut pending_variant: Option<VariantStreamBuilder> = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(v) = trimmed.strip_prefix("#EXT-X-VERSION:") {
            version = v.trim().parse().ok();
        } else if let Some(attrs) = trimmed.strip_prefix("#EXT-X-STREAM-INF:") {
            pending_variant = Some(parse_stream_inf(attrs));
        } else if !trimmed.starts_with('#') {
            // URI line
            if let Some(mut builder) = pending_variant.take() {
                builder.uri = trimmed.to_string();
                variants.push(builder.build());
            }
        }
    }

    Ok(MasterPlaylist { version, variants })
}

/// Parse a media playlist from M3U8 text.
pub fn parse_media_playlist(content: &str) -> Result<MediaPlaylist, HlsError> {
    let mut lines = content.lines().peekable();
    let first = lines.next().unwrap_or("");
    if !first.starts_with("#EXTM3U") {
        return Err(HlsError::NotM3u8);
    }

    let mut version = None;
    let mut target_duration = Duration::from_secs(10);
    let mut media_sequence: u64 = 0;
    let mut ended = false;
    let mut segments = Vec::new();
    let mut current_encryption: Option<EncryptionInfo> = None;
    let mut pending_duration: Option<(Duration, Option<String>)> = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(v) = trimmed.strip_prefix("#EXT-X-VERSION:") {
            version = v.trim().parse().ok();
        } else if let Some(v) = trimmed.strip_prefix("#EXT-X-TARGETDURATION:") {
            if let Ok(secs) = v.trim().parse::<u64>() {
                target_duration = Duration::from_secs(secs);
            }
        } else if let Some(v) = trimmed.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            media_sequence = v.trim().parse().unwrap_or(0);
        } else if trimmed == "#EXT-X-ENDLIST" {
            ended = true;
        } else if let Some(attrs) = trimmed.strip_prefix("#EXT-X-KEY:") {
            current_encryption = Some(parse_key_tag(attrs));
        } else if let Some(info) = trimmed.strip_prefix("#EXTINF:") {
            pending_duration = Some(parse_extinf(info));
        } else if !trimmed.starts_with('#') {
            // Segment URI
            if let Some((dur, title)) = pending_duration.take() {
                segments.push(Segment {
                    duration: dur,
                    uri: trimmed.to_string(),
                    title,
                    byte_range: None,
                    encryption: current_encryption.clone(),
                });
            }
        }
    }

    Ok(MediaPlaylist {
        version,
        target_duration,
        media_sequence,
        ended,
        segments,
        encryption: current_encryption,
    })
}

/// Total duration of all segments.
#[must_use]
pub fn total_duration(playlist: &MediaPlaylist) -> Duration {
    playlist.segments.iter().map(|s| s.duration).sum()
}

// ──── Internal helpers ─────────────────────────────────────────

struct VariantStreamBuilder {
    bandwidth: u64,
    average_bandwidth: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    codecs: Option<String>,
    frame_rate: Option<f64>,
    audio: Option<String>,
    uri: String,
}

impl VariantStreamBuilder {
    fn build(self) -> VariantStream {
        VariantStream {
            bandwidth: self.bandwidth,
            average_bandwidth: self.average_bandwidth,
            width: self.width,
            height: self.height,
            codecs: self.codecs,
            frame_rate: self.frame_rate,
            audio: self.audio,
            uri: self.uri,
        }
    }
}

fn parse_stream_inf(attrs: &str) -> VariantStreamBuilder {
    let mut builder = VariantStreamBuilder {
        bandwidth: 0,
        average_bandwidth: None,
        width: None,
        height: None,
        codecs: None,
        frame_rate: None,
        audio: None,
        uri: String::new(),
    };

    for part in split_attrs(attrs) {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("BANDWIDTH=") {
            builder.bandwidth = val.parse().unwrap_or(0);
        } else if let Some(val) = part.strip_prefix("AVERAGE-BANDWIDTH=") {
            builder.average_bandwidth = val.parse().ok();
        } else if let Some(val) = part.strip_prefix("RESOLUTION=") {
            if let Some((w, h)) = val.split_once('x') {
                builder.width = w.parse().ok();
                builder.height = h.parse().ok();
            }
        } else if let Some(val) = part.strip_prefix("CODECS=") {
            builder.codecs = Some(val.trim_matches('"').to_string());
        } else if let Some(val) = part.strip_prefix("FRAME-RATE=") {
            builder.frame_rate = val.parse().ok();
        } else if let Some(val) = part.strip_prefix("AUDIO=") {
            builder.audio = Some(val.trim_matches('"').to_string());
        }
    }

    builder
}

fn parse_extinf(info: &str) -> (Duration, Option<String>) {
    let (dur_part, title_part) = info.split_once(',').unwrap_or((info, ""));
    let secs: f64 = dur_part.trim().parse().unwrap_or(0.0);
    let title = if title_part.trim().is_empty() {
        None
    } else {
        Some(title_part.trim().to_string())
    };
    (Duration::from_secs_f64(secs), title)
}

fn parse_key_tag(attrs: &str) -> EncryptionInfo {
    let mut method = EncryptionMethod::None;
    let mut uri = None;
    let mut iv = None;

    for part in split_attrs(attrs) {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("METHOD=") {
            method = match val {
                "AES-128" => EncryptionMethod::Aes128,
                "SAMPLE-AES" => EncryptionMethod::SampleAes,
                _ => EncryptionMethod::None,
            };
        } else if let Some(val) = part.strip_prefix("URI=") {
            uri = Some(val.trim_matches('"').to_string());
        } else if let Some(val) = part.strip_prefix("IV=") {
            iv = Some(val.to_string());
        }
    }

    EncryptionInfo { method, uri, iv }
}

/// Split HLS attribute string by commas, respecting quoted strings.
fn split_attrs(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if !in_quotes => {
                parts.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER_M3U8: &str = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,CODECS=\"avc1.42e00a,mp4a.40.2\"\n\
        360p.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=1400000,RESOLUTION=842x480,CODECS=\"avc1.4d401e,mp4a.40.2\"\n\
        480p.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=2800000,RESOLUTION=1280x720,CODECS=\"avc1.4d401f,mp4a.40.2\"\n\
        720p.m3u8\n";

    const MEDIA_M3U8: &str = "#EXTM3U\n\
        #EXT-X-VERSION:3\n\
        #EXT-X-TARGETDURATION:10\n\
        #EXT-X-MEDIA-SEQUENCE:0\n\
        #EXTINF:9.5,\n\
        segment0.ts\n\
        #EXTINF:10.0,\n\
        segment1.ts\n\
        #EXTINF:8.2,\n\
        segment2.ts\n\
        #EXT-X-ENDLIST\n";

    #[test]
    fn test_detect_master() {
        assert_eq!(
            detect_playlist_type(MASTER_M3U8),
            Some(PlaylistType::Master)
        );
    }

    #[test]
    fn test_detect_media() {
        assert_eq!(detect_playlist_type(MEDIA_M3U8), Some(PlaylistType::Media));
    }

    #[test]
    fn test_detect_non_m3u8() {
        assert_eq!(detect_playlist_type("not a playlist"), None);
    }

    #[test]
    fn test_parse_master_playlist() {
        let pl = parse_master_playlist(MASTER_M3U8).unwrap();
        assert_eq!(pl.version, Some(3));
        assert_eq!(pl.variants.len(), 3);

        assert_eq!(pl.variants[0].bandwidth, 800_000);
        assert_eq!(pl.variants[0].width, Some(640));
        assert_eq!(pl.variants[0].height, Some(360));
        assert_eq!(pl.variants[0].uri, "360p.m3u8");

        assert_eq!(pl.variants[2].bandwidth, 2_800_000);
        assert_eq!(pl.variants[2].width, Some(1280));
        assert_eq!(pl.variants[2].height, Some(720));
        assert_eq!(pl.variants[2].uri, "720p.m3u8");
    }

    #[test]
    fn test_parse_master_codecs() {
        let pl = parse_master_playlist(MASTER_M3U8).unwrap();
        assert_eq!(
            pl.variants[0].codecs.as_deref(),
            Some("avc1.42e00a,mp4a.40.2")
        );
    }

    #[test]
    fn test_parse_media_playlist() {
        let pl = parse_media_playlist(MEDIA_M3U8).unwrap();
        assert_eq!(pl.version, Some(3));
        assert_eq!(pl.target_duration.as_secs(), 10);
        assert_eq!(pl.media_sequence, 0);
        assert!(pl.ended);
        assert_eq!(pl.segments.len(), 3);
    }

    #[test]
    fn test_media_segment_durations() {
        let pl = parse_media_playlist(MEDIA_M3U8).unwrap();
        assert!((pl.segments[0].duration.as_secs_f64() - 9.5).abs() < 0.01);
        assert!((pl.segments[1].duration.as_secs_f64() - 10.0).abs() < 0.01);
        assert!((pl.segments[2].duration.as_secs_f64() - 8.2).abs() < 0.01);
    }

    #[test]
    fn test_media_segment_uris() {
        let pl = parse_media_playlist(MEDIA_M3U8).unwrap();
        assert_eq!(pl.segments[0].uri, "segment0.ts");
        assert_eq!(pl.segments[1].uri, "segment1.ts");
        assert_eq!(pl.segments[2].uri, "segment2.ts");
    }

    #[test]
    fn test_total_duration() {
        let pl = parse_media_playlist(MEDIA_M3U8).unwrap();
        let total = total_duration(&pl);
        assert!((total.as_secs_f64() - 27.7).abs() < 0.1);
    }

    #[test]
    fn test_encrypted_playlist() {
        let encrypted = "#EXTM3U\n\
            #EXT-X-VERSION:3\n\
            #EXT-X-TARGETDURATION:10\n\
            #EXT-X-KEY:METHOD=AES-128,URI=\"https://keys.example.com/key.bin\",IV=0x00000001\n\
            #EXTINF:10.0,\n\
            enc_segment0.ts\n\
            #EXT-X-ENDLIST\n";
        let pl = parse_media_playlist(encrypted).unwrap();
        assert_eq!(pl.segments.len(), 1);
        let enc = pl.segments[0].encryption.as_ref().unwrap();
        assert_eq!(enc.method, EncryptionMethod::Aes128);
        assert_eq!(enc.uri.as_deref(), Some("https://keys.example.com/key.bin"));
        assert_eq!(enc.iv.as_deref(), Some("0x00000001"));
    }

    #[test]
    fn test_not_m3u8_error() {
        assert!(parse_master_playlist("not a playlist").is_err());
        assert!(parse_media_playlist("not a playlist").is_err());
    }

    #[test]
    fn test_live_playlist_no_endlist() {
        let live = "#EXTM3U\n\
            #EXT-X-TARGETDURATION:6\n\
            #EXT-X-MEDIA-SEQUENCE:100\n\
            #EXTINF:6.0,\n\
            live100.ts\n\
            #EXTINF:6.0,\n\
            live101.ts\n";
        let pl = parse_media_playlist(live).unwrap();
        assert!(!pl.ended);
        assert_eq!(pl.media_sequence, 100);
        assert_eq!(pl.segments.len(), 2);
    }
}
