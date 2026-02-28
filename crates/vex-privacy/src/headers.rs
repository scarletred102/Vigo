// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP header sanitization for privacy.
//!
//! Removes known tracking/fingerprinting headers and enforces strict referrer policy.

use std::collections::HashMap;

/// Headers that should be removed from outgoing requests for privacy.
const STRIP_HEADERS: &[&str] = &[
    "x-client-data",
    "sec-browsing-topics",
    "attribution-reporting-eligible",
    "attribution-reporting-register-source",
    "attribution-reporting-register-trigger",
];

/// Sanitize request headers for privacy.
///
/// - Strips known tracking headers.
/// - Enforces strict cross-origin referrer (origin only).
pub fn sanitize_headers(headers: &mut HashMap<String, String>, request_host: Option<&str>) {
    // Remove tracking headers (case-insensitive check)
    headers.retain(|key, _| {
        let lower = key.to_ascii_lowercase();
        !STRIP_HEADERS.contains(&lower.as_str())
    });

    // Enforce strict referrer: cross-origin → origin-only
    if let Some(referrer) = headers.get("referer").cloned() {
        if let (Some(req_host), Ok(ref_url)) = (request_host, url::Url::parse(&referrer)) {
            if ref_url.host_str() != Some(req_host) {
                // Cross-origin: reduce to origin only
                let origin = ref_url.origin().ascii_serialization();
                headers.insert("referer".to_string(), origin);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn strips_tracking_headers() {
        let mut h = headers(&[
            ("content-type", "text/html"),
            ("X-Client-Data", "secret"),
            ("Sec-Browsing-Topics", "topic1"),
        ]);
        sanitize_headers(&mut h, None);
        assert!(h.contains_key("content-type"));
        assert!(!h.contains_key("X-Client-Data"));
        assert!(!h.contains_key("Sec-Browsing-Topics"));
    }

    #[test]
    fn preserves_normal_headers() {
        let mut h = headers(&[
            ("accept", "*/*"),
            ("content-type", "application/json"),
            ("authorization", "Bearer xyz"),
        ]);
        sanitize_headers(&mut h, None);
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn cross_origin_referrer_reduced() {
        let mut h = headers(&[("referer", "https://other.com/secret/path?q=1")]);
        sanitize_headers(&mut h, Some("example.com"));
        assert_eq!(h["referer"], "https://other.com");
    }

    #[test]
    fn same_origin_referrer_kept() {
        let mut h = headers(&[("referer", "https://example.com/page?q=1")]);
        sanitize_headers(&mut h, Some("example.com"));
        // Same origin: full referrer preserved
        assert_eq!(h["referer"], "https://example.com/page?q=1");
    }
}
