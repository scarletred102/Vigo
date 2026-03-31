// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DASH (Dynamic Adaptive Streaming over HTTP) MPD parser.
//!
//! Parses a minimal subset of MPEG-DASH Media Presentation Description
//! XML. Extracts adaptation sets, representations (quality levels),
//! segment templates/URLs, and duration.

use std::time::Duration;

use thiserror::Error;

/// Errors during MPD parsing.
#[derive(Debug, Error)]
pub enum DashError {
    #[error("invalid MPD: {0}")]
    InvalidMpd(String),
    #[error("missing required attribute: {0}")]
    MissingAttribute(String),
    #[error("invalid duration format: {0}")]
    InvalidDuration(String),
}

/// A parsed DASH Media Presentation Description.
#[derive(Debug, Clone)]
pub struct Mpd {
    /// Total media duration, if specified.
    pub media_duration: Option<Duration>,
    /// Minimum buffer time hint.
    pub min_buffer_time: Option<Duration>,
    /// Whether this is a live (dynamic) or VOD (static) presentation.
    pub is_live: bool,
    /// Base URL for resolving relative segment URLs.
    pub base_url: Option<String>,
    /// Periods in the presentation (usually one for VOD).
    pub periods: Vec<Period>,
}

/// A period within the MPD.
#[derive(Debug, Clone)]
pub struct Period {
    /// Period ID, if specified.
    pub id: Option<String>,
    /// Period start offset.
    pub start: Option<Duration>,
    /// Period duration.
    pub duration: Option<Duration>,
    /// Adaptation sets within this period.
    pub adaptation_sets: Vec<AdaptationSet>,
}

/// Content type of an adaptation set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Video,
    Audio,
    Text,
    Unknown,
}

impl ContentType {
    fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "video" => Self::Video,
            "audio" => Self::Audio,
            "text" | "subtitle" => Self::Text,
            _ => Self::Unknown,
        }
    }
}

/// An adaptation set (e.g., one for video, one for audio).
#[derive(Debug, Clone)]
pub struct AdaptationSet {
    /// Adaptation set ID.
    pub id: Option<u32>,
    /// Content type (video/audio/text).
    pub content_type: ContentType,
    /// MIME type (e.g., `video/mp4`).
    pub mime_type: Option<String>,
    /// Codec string (e.g., `avc1.4d401f`).
    pub codecs: Option<String>,
    /// Language (e.g., `en`).
    pub lang: Option<String>,
    /// Segment template (common template for all representations).
    pub segment_template: Option<SegmentTemplate>,
    /// Available quality levels.
    pub representations: Vec<Representation>,
}

/// A representation (quality level) within an adaptation set.
#[derive(Debug, Clone)]
pub struct Representation {
    /// Representation ID.
    pub id: String,
    /// Bandwidth in bits per second.
    pub bandwidth: u64,
    /// Video width (if video).
    pub width: Option<u32>,
    /// Video height (if video).
    pub height: Option<u32>,
    /// Codec string override.
    pub codecs: Option<String>,
    /// Per-representation segment template (overrides adaptation set's).
    pub segment_template: Option<SegmentTemplate>,
    /// Direct segment list (if not using template).
    pub segment_urls: Vec<String>,
}

/// Segment template for constructing segment URLs.
#[derive(Debug, Clone)]
pub struct SegmentTemplate {
    /// Initialisation segment URL pattern.
    pub initialization: Option<String>,
    /// Media segment URL pattern (with `$Number$`, `$Time$` placeholders).
    pub media: Option<String>,
    /// Start number for segment numbering.
    pub start_number: u32,
    /// Timescale (ticks per second).
    pub timescale: u32,
    /// Segment duration in timescale units.
    pub duration: Option<u64>,
}

impl SegmentTemplate {
    /// Resolve the initialisation URL for a given representation ID.
    #[must_use]
    pub fn resolve_init_url(&self, rep_id: &str) -> Option<String> {
        self.initialization
            .as_ref()
            .map(|tmpl| tmpl.replace("$RepresentationID$", rep_id))
    }

    /// Resolve a media segment URL for a given representation+number.
    #[must_use]
    pub fn resolve_media_url(&self, rep_id: &str, number: u32) -> Option<String> {
        self.media.as_ref().map(|tmpl| {
            tmpl.replace("$RepresentationID$", rep_id)
                .replace("$Number$", &number.to_string())
        })
    }

    /// Number of segments, given total duration.
    #[must_use]
    pub fn segment_count(&self, total_duration: Duration) -> u32 {
        let seg_dur = match self.duration {
            Some(d) if d > 0 => d,
            _ => return 0,
        };
        let total_ticks = total_duration.as_secs_f64() * f64::from(self.timescale);
        (total_ticks / seg_dur as f64).ceil() as u32
    }
}

// ──── Simple XML-attribute parser ──────────────────────────────
//
// DASH MPDs are XML but we only need attribute extraction, not a
// full XML parser. This keeps dependencies minimal.

/// Parse an ISO 8601 duration like `PT1H2M30.5S` → `Duration`.
pub fn parse_iso_duration(input: &str) -> Result<Duration, DashError> {
    let s = input.trim();
    if !s.starts_with("PT") && !s.starts_with("P") {
        return Err(DashError::InvalidDuration(input.to_string()));
    }

    let after_p = if let Some(rest) = s.strip_prefix("PT") {
        rest
    } else {
        // Skip 'P', look for 'T' — ignore date portion for simplicity
        let t_pos = s.find('T').unwrap_or(1);
        &s[t_pos + 1..]
    };

    let mut hours: f64 = 0.0;
    let mut minutes: f64 = 0.0;
    let mut seconds: f64 = 0.0;
    let mut num_start = 0;

    for (i, ch) in after_p.char_indices() {
        match ch {
            'H' => {
                hours = after_p[num_start..i]
                    .parse::<f64>()
                    .map_err(|_| DashError::InvalidDuration(input.to_string()))?;
                num_start = i + 1;
            }
            'M' => {
                minutes = after_p[num_start..i]
                    .parse::<f64>()
                    .map_err(|_| DashError::InvalidDuration(input.to_string()))?;
                num_start = i + 1;
            }
            'S' => {
                seconds = after_p[num_start..i]
                    .parse::<f64>()
                    .map_err(|_| DashError::InvalidDuration(input.to_string()))?;
            }
            _ => {}
        }
    }

    let total_secs = hours * 3600.0 + minutes * 60.0 + seconds;
    Ok(Duration::from_secs_f64(total_secs))
}

/// Extract the value of `attr_name` from an XML tag string.
///
/// Matches ` attr_name="..."` (preceded by a space to avoid substring matches
/// like "bandwidth" matching "width").
fn extract_attr<'a>(tag: &'a str, attr_name: &str) -> Option<&'a str> {
    let needle = format!(" {attr_name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

/// Extract optional u32 attribute.
fn attr_u32(tag: &str, name: &str) -> Option<u32> {
    extract_attr(tag, name).and_then(|v| v.parse().ok())
}

/// Extract optional u64 attribute.
fn attr_u64(tag: &str, name: &str) -> Option<u64> {
    extract_attr(tag, name).and_then(|v| v.parse().ok())
}

/// Parse a DASH MPD from XML text.
///
/// This is a lightweight line-by-line parser that handles the most
/// common MPD structures. It does not attempt full XML conformance.
pub fn parse_mpd(xml: &str) -> Result<Mpd, DashError> {
    let mut mpd = Mpd {
        media_duration: None,
        min_buffer_time: None,
        is_live: false,
        base_url: None,
        periods: Vec::new(),
    };

    let mut current_period: Option<Period> = None;
    let mut current_adaptation: Option<AdaptationSet> = None;
    let mut current_rep: Option<Representation> = None;
    let mut in_base_url = false;
    let mut base_url_text = String::new();

    for line in xml.lines() {
        let trimmed = line.trim();

        // MPD root element
        if trimmed.starts_with("<MPD") {
            if let Some(dur_str) = extract_attr(trimmed, "mediaPresentationDuration") {
                mpd.media_duration = parse_iso_duration(dur_str).ok();
            }
            if let Some(buf_str) = extract_attr(trimmed, "minBufferTime") {
                mpd.min_buffer_time = parse_iso_duration(buf_str).ok();
            }
            if let Some(t) = extract_attr(trimmed, "type") {
                mpd.is_live = t == "dynamic";
            }
        }

        // BaseURL
        if trimmed.starts_with("<BaseURL") && trimmed.contains('>') {
            if trimmed.contains("</BaseURL>") {
                // Single-line BaseURL
                let start = trimmed.find('>').map(|i| i + 1).unwrap_or(0);
                let end = trimmed.find("</BaseURL>").unwrap_or(trimmed.len());
                let url = &trimmed[start..end];
                if mpd.periods.is_empty() {
                    mpd.base_url = Some(url.to_string());
                }
            } else {
                in_base_url = true;
                base_url_text.clear();
            }
        } else if in_base_url {
            if trimmed.contains("</BaseURL>") {
                let end = trimmed.find("</BaseURL>").unwrap_or(trimmed.len());
                base_url_text.push_str(&trimmed[..end]);
                mpd.base_url = Some(base_url_text.clone());
                in_base_url = false;
            } else {
                base_url_text.push_str(trimmed);
            }
        }

        // Period
        if trimmed.starts_with("<Period") {
            let period = Period {
                id: extract_attr(trimmed, "id").map(String::from),
                start: extract_attr(trimmed, "start").and_then(|s| parse_iso_duration(s).ok()),
                duration: extract_attr(trimmed, "duration")
                    .and_then(|s| parse_iso_duration(s).ok()),
                adaptation_sets: Vec::new(),
            };
            current_period = Some(period);
        }
        if trimmed.starts_with("</Period") {
            if let Some(mut period) = current_period.take() {
                if let Some(adaptation) = current_adaptation.take() {
                    period.adaptation_sets.push(adaptation);
                }
                mpd.periods.push(period);
            }
        }

        // AdaptationSet
        if trimmed.starts_with("<AdaptationSet") {
            // If there's a pending representation, flush to adaptation set
            if let Some(rep) = current_rep.take() {
                if let Some(ad) = current_adaptation.as_mut() {
                    ad.representations.push(rep);
                }
            }
            // Flush previous adaptation set
            if let Some(adaptation) = current_adaptation.take() {
                if let Some(period) = current_period.as_mut() {
                    period.adaptation_sets.push(adaptation);
                }
            }
            let content_type = extract_attr(trimmed, "contentType")
                .map(ContentType::from_str)
                .or_else(|| {
                    extract_attr(trimmed, "mimeType").map(|m| {
                        if m.starts_with("video") {
                            ContentType::Video
                        } else if m.starts_with("audio") {
                            ContentType::Audio
                        } else if m.starts_with("text") {
                            ContentType::Text
                        } else {
                            ContentType::Unknown
                        }
                    })
                })
                .unwrap_or(ContentType::Unknown);

            current_adaptation = Some(AdaptationSet {
                id: attr_u32(trimmed, "id"),
                content_type,
                mime_type: extract_attr(trimmed, "mimeType").map(String::from),
                codecs: extract_attr(trimmed, "codecs").map(String::from),
                lang: extract_attr(trimmed, "lang").map(String::from),
                segment_template: None,
                representations: Vec::new(),
            });
        }
        if trimmed.starts_with("</AdaptationSet") {
            if let Some(rep) = current_rep.take() {
                if let Some(ad) = current_adaptation.as_mut() {
                    ad.representations.push(rep);
                }
            }
            if let Some(adaptation) = current_adaptation.take() {
                if let Some(period) = current_period.as_mut() {
                    period.adaptation_sets.push(adaptation);
                }
            }
        }

        // SegmentTemplate
        if trimmed.starts_with("<SegmentTemplate") {
            let tmpl = SegmentTemplate {
                initialization: extract_attr(trimmed, "initialization").map(String::from),
                media: extract_attr(trimmed, "media").map(String::from),
                start_number: attr_u32(trimmed, "startNumber").unwrap_or(1),
                timescale: attr_u32(trimmed, "timescale").unwrap_or(1),
                duration: attr_u64(trimmed, "duration"),
            };
            if current_rep.is_some() {
                if let Some(rep) = current_rep.as_mut() {
                    rep.segment_template = Some(tmpl);
                }
            } else if let Some(ad) = current_adaptation.as_mut() {
                ad.segment_template = Some(tmpl);
            }
        }

        // Representation
        if trimmed.starts_with("<Representation") {
            if let Some(rep) = current_rep.take() {
                if let Some(ad) = current_adaptation.as_mut() {
                    ad.representations.push(rep);
                }
            }
            let id = extract_attr(trimmed, "id").unwrap_or("0").to_string();
            let bandwidth = attr_u64(trimmed, "bandwidth").unwrap_or(0);
            current_rep = Some(Representation {
                id,
                bandwidth,
                width: attr_u32(trimmed, "width"),
                height: attr_u32(trimmed, "height"),
                codecs: extract_attr(trimmed, "codecs").map(String::from),
                segment_template: None,
                segment_urls: Vec::new(),
            });
        }
        if trimmed == "</Representation>" {
            if let Some(rep) = current_rep.take() {
                if let Some(ad) = current_adaptation.as_mut() {
                    ad.representations.push(rep);
                }
            }
        }
    }

    // Flush any remaining state
    if let Some(rep) = current_rep.take() {
        if let Some(ad) = current_adaptation.as_mut() {
            ad.representations.push(rep);
        }
    }
    if let Some(adaptation) = current_adaptation.take() {
        if let Some(period) = current_period.as_mut() {
            period.adaptation_sets.push(adaptation);
        }
    }
    if let Some(period) = current_period.take() {
        mpd.periods.push(period);
    }

    Ok(mpd)
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MPD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<MPD type="static" mediaPresentationDuration="PT1H30M45S" minBufferTime="PT2S">
  <BaseURL>https://cdn.example.com/video/</BaseURL>
  <Period id="1" duration="PT1H30M45S">
    <AdaptationSet id="1" contentType="video" mimeType="video/mp4" codecs="avc1.4d401f">
      <SegmentTemplate initialization="init-$RepresentationID$.mp4" media="seg-$RepresentationID$-$Number$.m4s" startNumber="1" timescale="90000" duration="180000"/>
      <Representation id="720p" bandwidth="2500000" width="1280" height="720"/>
      <Representation id="480p" bandwidth="1000000" width="854" height="480"/>
      <Representation id="360p" bandwidth="500000" width="640" height="360"/>
    </AdaptationSet>
    <AdaptationSet id="2" contentType="audio" mimeType="audio/mp4" codecs="mp4a.40.2" lang="en">
      <SegmentTemplate initialization="audio-init.mp4" media="audio-$Number$.m4s" startNumber="1" timescale="44100" duration="88200"/>
      <Representation id="audio" bandwidth="128000"/>
    </AdaptationSet>
  </Period>
</MPD>"#;

    #[test]
    fn test_parse_iso_duration_simple() {
        let d = parse_iso_duration("PT30S").unwrap();
        assert_eq!(d.as_secs(), 30);
    }

    #[test]
    fn test_parse_iso_duration_complex() {
        let d = parse_iso_duration("PT1H30M45S").unwrap();
        assert_eq!(d.as_secs(), 5445); // 1*3600 + 30*60 + 45
    }

    #[test]
    fn test_parse_iso_duration_fractional() {
        let d = parse_iso_duration("PT2.5S").unwrap();
        assert!((d.as_secs_f64() - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_iso_duration_minutes_only() {
        let d = parse_iso_duration("PT5M").unwrap();
        assert_eq!(d.as_secs(), 300);
    }

    #[test]
    fn test_parse_iso_duration_invalid() {
        assert!(parse_iso_duration("not_a_duration").is_err());
    }

    #[test]
    fn test_parse_mpd_basic() {
        let mpd = parse_mpd(SAMPLE_MPD).unwrap();
        assert!(!mpd.is_live);
        assert_eq!(mpd.media_duration.unwrap().as_secs(), 5445);
        assert_eq!(mpd.min_buffer_time.unwrap().as_secs(), 2);
        assert_eq!(
            mpd.base_url.as_deref(),
            Some("https://cdn.example.com/video/")
        );
        assert_eq!(mpd.periods.len(), 1);
    }

    #[test]
    fn test_parse_mpd_adaptation_sets() {
        let mpd = parse_mpd(SAMPLE_MPD).unwrap();
        let period = &mpd.periods[0];
        assert_eq!(period.adaptation_sets.len(), 2);

        let video = &period.adaptation_sets[0];
        assert_eq!(video.content_type, ContentType::Video);
        assert_eq!(video.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(video.representations.len(), 3);

        let audio = &period.adaptation_sets[1];
        assert_eq!(audio.content_type, ContentType::Audio);
        assert_eq!(audio.lang.as_deref(), Some("en"));
        assert_eq!(audio.representations.len(), 1);
    }

    #[test]
    fn test_parse_mpd_representations() {
        let mpd = parse_mpd(SAMPLE_MPD).unwrap();
        let reps = &mpd.periods[0].adaptation_sets[0].representations;

        assert_eq!(reps[0].id, "720p");
        assert_eq!(reps[0].bandwidth, 2_500_000);
        assert_eq!(reps[0].width, Some(1280));
        assert_eq!(reps[0].height, Some(720));

        assert_eq!(reps[1].id, "480p");
        assert_eq!(reps[1].bandwidth, 1_000_000);

        assert_eq!(reps[2].id, "360p");
        assert_eq!(reps[2].bandwidth, 500_000);
    }

    #[test]
    fn test_segment_template() {
        let mpd = parse_mpd(SAMPLE_MPD).unwrap();
        let tmpl = mpd.periods[0].adaptation_sets[0]
            .segment_template
            .as_ref()
            .unwrap();

        assert_eq!(tmpl.start_number, 1);
        assert_eq!(tmpl.timescale, 90_000);
        assert_eq!(tmpl.duration, Some(180_000));

        let init = tmpl.resolve_init_url("720p").unwrap();
        assert_eq!(init, "init-720p.mp4");

        let seg = tmpl.resolve_media_url("720p", 5).unwrap();
        assert_eq!(seg, "seg-720p-5.m4s");
    }

    #[test]
    fn test_segment_count() {
        let tmpl = SegmentTemplate {
            initialization: None,
            media: None,
            start_number: 1,
            timescale: 90_000,
            duration: Some(180_000), // 2 seconds per segment
        };
        let count = tmpl.segment_count(Duration::from_secs(10));
        assert_eq!(count, 5); // 10s / 2s = 5 segments
    }

    #[test]
    fn test_parse_live_mpd() {
        let live_mpd = r#"<MPD type="dynamic" minBufferTime="PT4S">
  <Period>
    <AdaptationSet contentType="video">
      <Representation id="v1" bandwidth="800000" width="640" height="360"/>
    </AdaptationSet>
  </Period>
</MPD>"#;
        let mpd = parse_mpd(live_mpd).unwrap();
        assert!(mpd.is_live);
        assert!(mpd.media_duration.is_none());
        assert_eq!(mpd.periods.len(), 1);
        assert_eq!(mpd.periods[0].adaptation_sets[0].representations.len(), 1);
    }
}
