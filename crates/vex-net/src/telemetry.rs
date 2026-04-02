// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Structured network telemetry records for diagnostics and DevTools.

use serde::{Deserialize, Serialize};

/// Cache outcome for a network request attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CacheOutcome {
    FreshHit,
    StaleWhileRevalidateHit,
    Revalidated304,
    StaleIfErrorHit,
    #[default]
    Miss,
}

/// Timing breakdown captured during request processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkTimings {
    pub dns_ms: Option<u64>,
    pub ttfb_ms: Option<u64>,
    pub body_read_ms: Option<u64>,
    pub total_ms: u64,
}

/// One telemetry record per completed request attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRecord {
    pub request_id: u64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub cache_outcome: CacheOutcome,
    pub was_cached: bool,
    pub timings: NetworkTimings,
    pub error: Option<String>,
}

/// Aggregated network metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub revalidated_304: u64,
    pub stale_if_error_hits: u64,
    pub errors: u64,
}

/// Timeline row suitable for simple DevTools waterfall rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallEntry {
    pub request_id: u64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub cache_outcome: CacheOutcome,
    pub offset_ms: u64,
    pub dns_ms: Option<u64>,
    pub ttfb_ms: Option<u64>,
    pub body_read_ms: Option<u64>,
    pub total_ms: u64,
    pub error: Option<String>,
}

/// Build a deterministic waterfall timeline from captured records.
pub fn build_waterfall(records: &[NetworkRecord]) -> Vec<WaterfallEntry> {
    let mut offset = 0u64;
    let mut out = Vec::with_capacity(records.len());

    for rec in records {
        out.push(WaterfallEntry {
            request_id: rec.request_id,
            method: rec.method.clone(),
            url: rec.url.clone(),
            status: rec.status,
            cache_outcome: rec.cache_outcome,
            offset_ms: offset,
            dns_ms: rec.timings.dns_ms,
            ttfb_ms: rec.timings.ttfb_ms,
            body_read_ms: rec.timings.body_read_ms,
            total_ms: rec.timings.total_ms,
            error: rec.error.clone(),
        });

        offset = offset.saturating_add(rec.timings.total_ms);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cache_outcome_is_miss() {
        assert_eq!(CacheOutcome::default(), CacheOutcome::Miss);
    }

    #[test]
    fn network_record_can_store_success() {
        let rec = NetworkRecord {
            request_id: 1,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            status: Some(200),
            cache_outcome: CacheOutcome::FreshHit,
            was_cached: true,
            timings: NetworkTimings {
                dns_ms: Some(1),
                ttfb_ms: Some(2),
                body_read_ms: Some(3),
                total_ms: 6,
            },
            error: None,
        };

        assert_eq!(rec.status, Some(200));
        assert!(rec.was_cached);
        assert_eq!(rec.timings.total_ms, 6);
    }

    #[test]
    fn network_stats_default_zeroed() {
        let stats = NetworkStats::default();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn waterfall_offsets_are_monotonic() {
        let records = vec![
            NetworkRecord {
                request_id: 1,
                method: "GET".into(),
                url: "https://a".into(),
                status: Some(200),
                cache_outcome: CacheOutcome::Miss,
                was_cached: false,
                timings: NetworkTimings {
                    dns_ms: Some(1),
                    ttfb_ms: Some(2),
                    body_read_ms: Some(3),
                    total_ms: 10,
                },
                error: None,
            },
            NetworkRecord {
                request_id: 2,
                method: "GET".into(),
                url: "https://b".into(),
                status: Some(200),
                cache_outcome: CacheOutcome::FreshHit,
                was_cached: true,
                timings: NetworkTimings {
                    dns_ms: None,
                    ttfb_ms: None,
                    body_read_ms: None,
                    total_ms: 5,
                },
                error: None,
            },
        ];

        let wf = build_waterfall(&records);
        assert_eq!(wf.len(), 2);
        assert_eq!(wf[0].offset_ms, 0);
        assert_eq!(wf[1].offset_ms, 10);
    }
}
