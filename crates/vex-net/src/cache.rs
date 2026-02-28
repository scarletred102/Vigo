// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! In-memory HTTP cache with Cache-Control support.

use std::collections::HashMap;
use std::time::Instant;

use vex_core::VexUrl;

/// Parsed Cache-Control directives.
#[derive(Debug, Clone, Default)]
pub struct CacheDirectives {
    /// `max-age=N` — freshness lifetime in seconds.
    pub max_age: Option<u64>,
    /// `no-cache` — must revalidate before use.
    pub no_cache: bool,
    /// `no-store` — must not cache at all.
    pub no_store: bool,
    /// `must-revalidate` — stale responses not usable without revalidation.
    pub must_revalidate: bool,
    /// `public` — response may be cached by shared caches.
    pub is_public: bool,
    /// `private` — response is for the single user only.
    pub is_private: bool,
}

/// Parse a `Cache-Control` header value into directives.
pub fn parse_cache_control(header: &str) -> CacheDirectives {
    let mut dirs = CacheDirectives::default();
    for part in header.split(',') {
        let part = part.trim().to_ascii_lowercase();
        if let Some(val) = part.strip_prefix("max-age=") {
            dirs.max_age = val.trim().parse().ok();
        } else if part == "no-cache" {
            dirs.no_cache = true;
        } else if part == "no-store" {
            dirs.no_store = true;
        } else if part == "must-revalidate" {
            dirs.must_revalidate = true;
        } else if part == "public" {
            dirs.is_public = true;
        } else if part == "private" {
            dirs.is_private = true;
        }
    }
    dirs
}

/// A cached HTTP response.
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
    pub status: u16,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub directives: CacheDirectives,
    pub stored_at: Instant,
}

impl CachedResponse {
    /// Whether the cached response is still fresh per max-age.
    pub fn is_fresh(&self) -> bool {
        if self.directives.no_cache {
            return false;
        }
        if let Some(max_age) = self.directives.max_age {
            self.stored_at.elapsed().as_secs() < max_age
        } else {
            false
        }
    }
}

/// Simple in-memory HTTP cache keyed by URL.
pub struct HttpCache {
    entries: HashMap<String, CachedResponse>,
}

impl HttpCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Store a response in the cache.
    ///
    /// Parses Cache-Control headers. If `no-store`, the response is **not** cached.
    pub fn store(&mut self, url: &VexUrl, status: u16, headers: &HashMap<String, String>, body: &[u8]) {
        let cc = headers
            .get("cache-control")
            .map(|h| parse_cache_control(h))
            .unwrap_or_default();

        if cc.no_store {
            return;
        }

        let etag = headers.get("etag").cloned();
        let last_modified = headers.get("last-modified").cloned();

        self.entries.insert(
            url.inner().as_str().to_string(),
            CachedResponse {
                body: body.to_vec(),
                headers: headers.clone(),
                status,
                etag,
                last_modified,
                directives: cc,
                stored_at: Instant::now(),
            },
        );
    }

    /// Look up a cached response.
    pub fn get(&self, url: &VexUrl) -> Option<&CachedResponse> {
        self.entries.get(url.inner().as_str())
    }

    /// Remove a cached response.
    pub fn remove(&mut self, url: &VexUrl) {
        self.entries.remove(url.inner().as_str());
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for HttpCache {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    fn test_headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn parse_cache_control_max_age() {
        let d = parse_cache_control("max-age=3600");
        assert_eq!(d.max_age, Some(3600));
        assert!(!d.no_cache);
        assert!(!d.no_store);
    }

    #[test]
    fn parse_cache_control_multiple() {
        let d = parse_cache_control("public, max-age=86400, must-revalidate");
        assert_eq!(d.max_age, Some(86400));
        assert!(d.is_public);
        assert!(d.must_revalidate);
    }

    #[test]
    fn parse_cache_control_no_store() {
        let d = parse_cache_control("no-store, no-cache");
        assert!(d.no_store);
        assert!(d.no_cache);
        assert_eq!(d.max_age, None);
    }

    #[test]
    fn parse_cache_control_private() {
        let d = parse_cache_control("private, max-age=0");
        assert!(d.is_private);
        assert_eq!(d.max_age, Some(0));
    }

    #[test]
    fn parse_empty_string() {
        let d = parse_cache_control("");
        assert!(!d.no_cache);
        assert!(!d.no_store);
        assert_eq!(d.max_age, None);
    }

    #[test]
    fn store_and_retrieve() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/page");
        let headers = test_headers(&[("cache-control", "max-age=3600")]);

        cache.store(&url, 200, &headers, b"hello");
        let entry = cache.get(&url).unwrap();
        assert_eq!(entry.status, 200);
        assert_eq!(entry.body, b"hello");
        assert!(entry.is_fresh());
    }

    #[test]
    fn no_store_prevents_caching() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/secret");
        let headers = test_headers(&[("cache-control", "no-store")]);

        cache.store(&url, 200, &headers, b"private data");
        assert!(cache.get(&url).is_none());
    }

    #[test]
    fn etag_extraction() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/res");
        let headers = test_headers(&[
            ("cache-control", "max-age=60"),
            ("etag", "\"abc123\""),
        ]);

        cache.store(&url, 200, &headers, b"data");
        let entry = cache.get(&url).unwrap();
        assert_eq!(entry.etag.as_deref(), Some("\"abc123\""));
    }

    #[test]
    fn remove_entry() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/del");
        let headers = test_headers(&[("cache-control", "max-age=60")]);

        cache.store(&url, 200, &headers, b"data");
        assert_eq!(cache.len(), 1);

        cache.remove(&url);
        assert!(cache.is_empty());
    }

    #[test]
    fn no_cache_is_not_fresh() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/nc");
        let headers = test_headers(&[("cache-control", "no-cache, max-age=3600")]);

        cache.store(&url, 200, &headers, b"data");
        let entry = cache.get(&url).unwrap();
        assert!(!entry.is_fresh()); // no-cache means always revalidate
    }
}
