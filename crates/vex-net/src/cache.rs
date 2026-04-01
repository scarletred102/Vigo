// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! In-memory HTTP cache with Cache-Control support.

use std::collections::HashMap;
use std::time::{Instant, SystemTime};

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
    /// `stale-while-revalidate=N` — stale responses may be used while background revalidation happens.
    pub stale_while_revalidate: Option<u64>,
    /// `stale-if-error=N` — stale responses may be used when revalidation/fetch fails.
    pub stale_if_error: Option<u64>,
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
        } else if let Some(val) = part.strip_prefix("stale-while-revalidate=") {
            dirs.stale_while_revalidate = val.trim().parse().ok();
        } else if let Some(val) = part.strip_prefix("stale-if-error=") {
            dirs.stale_if_error = val.trim().parse().ok();
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
    pub freshness_lifetime_secs: Option<u64>,
    pub vary_on: Vec<String>,
    pub vary_values: HashMap<String, String>,
    pub stored_at: Instant,
}

impl CachedResponse {
    /// Whether the cached response is still fresh per max-age.
    pub fn is_fresh(&self) -> bool {
        if self.directives.no_cache {
            return false;
        }
        if let Some(freshness_lifetime_secs) = self.freshness_lifetime_secs {
            self.stored_at.elapsed().as_secs() < freshness_lifetime_secs
        } else {
            false
        }
    }

    fn staleness_secs(&self) -> Option<u64> {
        let lifetime = self.freshness_lifetime_secs?;
        let age = self.stored_at.elapsed().as_secs();
        age.checked_sub(lifetime)
    }

    pub fn can_serve_while_revalidating(&self) -> bool {
        match (self.staleness_secs(), self.directives.stale_while_revalidate) {
            (Some(stale), Some(window)) => stale <= window,
            _ => false,
        }
    }

    pub fn can_serve_if_error(&self) -> bool {
        match (self.staleness_secs(), self.directives.stale_if_error) {
            (Some(stale), Some(window)) => stale <= window,
            _ => false,
        }
    }
}

fn parse_age(headers: &HashMap<String, String>) -> u64 {
    header_get_ci(headers, "age")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn parse_expires(headers: &HashMap<String, String>) -> Option<SystemTime> {
    header_get_ci(headers, "expires").and_then(|v| httpdate::parse_http_date(v).ok())
}

fn header_get_ci<'a>(headers: &'a HashMap<String, String>, name: &str) -> Option<&'a String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

fn parse_vary(headers: &HashMap<String, String>) -> Option<Vec<String>> {
    let vary = header_get_ci(headers, "vary")?;
    let mut fields = Vec::new();
    for token in vary.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if token == "*" {
            return Some(vec!["*".to_string()]);
        }
        fields.push(token.to_ascii_lowercase());
    }
    if fields.is_empty() {
        None
    } else {
        fields.sort();
        fields.dedup();
        Some(fields)
    }
}

fn build_vary_values(
    request_headers: &HashMap<String, String>,
    vary_on: &[String],
) -> HashMap<String, String> {
    vary_on
        .iter()
        .map(|name| {
            let value = header_get_ci(request_headers, name)
                .cloned()
                .unwrap_or_default();
            (name.clone(), value)
        })
        .collect()
}

fn request_matches_variant(
    request_headers: &HashMap<String, String>,
    entry: &CachedResponse,
) -> bool {
    entry.vary_on.iter().all(|name| {
        let req_value = header_get_ci(request_headers, name)
            .cloned()
            .unwrap_or_default();
        entry
            .vary_values
            .get(name)
            .map(|v| v == &req_value)
            .unwrap_or(false)
    })
}

fn freshness_lifetime_secs(
    headers: &HashMap<String, String>,
    directives: &CacheDirectives,
) -> Option<u64> {
    let age = parse_age(headers);

    if let Some(max_age) = directives.max_age {
        return Some(max_age.saturating_sub(age));
    }

    let expires = parse_expires(headers)?;
    let now = SystemTime::now();
    let ttl = expires.duration_since(now).ok()?.as_secs();
    Some(ttl.saturating_sub(age))
}

/// Simple in-memory HTTP cache keyed by URL.
pub struct HttpCache {
    entries: HashMap<String, Vec<CachedResponse>>,
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
    pub fn store(
        &mut self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
        status: u16,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) {
        let cc = headers
            .get("cache-control")
            .map(|h| parse_cache_control(h))
            .unwrap_or_default();

        if cc.no_store {
            return;
        }

        let vary_on = parse_vary(headers).unwrap_or_default();
        if vary_on.iter().any(|v| v == "*") {
            return;
        }
        let vary_values = build_vary_values(request_headers, &vary_on);

        let etag = headers.get("etag").cloned();
        let last_modified = headers.get("last-modified").cloned();
        let freshness_lifetime_secs = freshness_lifetime_secs(headers, &cc);

        let entry = CachedResponse {
            body: body.to_vec(),
            headers: headers.clone(),
            status,
            etag,
            last_modified,
            directives: cc,
            freshness_lifetime_secs,
            vary_on,
            vary_values,
            stored_at: Instant::now(),
        };

        let variants = self
            .entries
            .entry(url.inner().as_str().to_string())
            .or_default();

        if let Some(existing) = variants.iter_mut().find(|v| {
            v.vary_on == entry.vary_on && v.vary_values == entry.vary_values
        }) {
            *existing = entry;
        } else {
            variants.push(entry);
        }
    }

    /// Look up a cached response.
    pub fn get(
        &self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
    ) -> Option<&CachedResponse> {
        self.entries.get(url.inner().as_str()).and_then(|variants| {
            variants
                .iter()
                .filter(|entry| request_matches_variant(request_headers, entry))
                .max_by_key(|entry| entry.vary_on.len())
        })
    }

    /// Return a stale entry that can be served under `stale-if-error`.
    pub fn get_stale_if_error(
        &self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
    ) -> Option<&CachedResponse> {
        self.entries.get(url.inner().as_str()).and_then(|variants| {
            variants
                .iter()
                .filter(|entry| request_matches_variant(request_headers, entry))
                .find(|entry| !entry.is_fresh() && entry.can_serve_if_error())
        })
    }

    /// Remove a cached response.
    pub fn remove(&mut self, url: &VexUrl) {
        self.entries.remove(url.inner().as_str());
    }

    /// Refresh an existing cached entry from a `304 Not Modified` response.
    ///
    /// Per RFC behavior, a 304 can update metadata/headers while reusing the
    /// previously cached entity body.
    pub fn refresh_from_not_modified(
        &mut self,
        url: &VexUrl,
        request_headers: &HashMap<String, String>,
        response_headers: &HashMap<String, String>,
    ) -> bool {
        let Some(variants) = self.entries.get_mut(url.inner().as_str()) else {
            return false;
        };

        let Some(entry) = variants
            .iter_mut()
            .filter(|entry| request_matches_variant(request_headers, entry))
            .max_by_key(|entry| entry.vary_on.len())
        else {
            return false;
        };

        // Merge/override headers from the 304 response.
        for (k, v) in response_headers {
            entry.headers.insert(k.clone(), v.clone());
        }

        // Refresh validators/directives if provided.
        if let Some(etag) = response_headers.get("etag") {
            entry.etag = Some(etag.clone());
        }
        if let Some(last_modified) = response_headers.get("last-modified") {
            entry.last_modified = Some(last_modified.clone());
        }
        if let Some(cache_control) = response_headers.get("cache-control") {
            entry.directives = parse_cache_control(cache_control);
        }

        entry.freshness_lifetime_secs = freshness_lifetime_secs(&entry.headers, &entry.directives);

        // Entry was revalidated now.
        entry.stored_at = Instant::now();
        true
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
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
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn req_headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        test_headers(pairs)
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
    fn parse_cache_control_stale_directives() {
        let d = parse_cache_control("max-age=60, stale-while-revalidate=30, stale-if-error=300");
        assert_eq!(d.max_age, Some(60));
        assert_eq!(d.stale_while_revalidate, Some(30));
        assert_eq!(d.stale_if_error, Some(300));
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
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "max-age=3600")]);

        cache.store(&url, &req, 200, &headers, b"hello");
        let entry = cache.get(&url, &req).unwrap();
        assert_eq!(entry.status, 200);
        assert_eq!(entry.body, b"hello");
        assert!(entry.is_fresh());
    }

    #[test]
    fn no_store_prevents_caching() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/secret");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "no-store")]);

        cache.store(&url, &req, 200, &headers, b"private data");
        assert!(cache.get(&url, &req).is_none());
    }

    #[test]
    fn etag_extraction() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/res");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "max-age=60"), ("etag", "\"abc123\"")]);

        cache.store(&url, &req, 200, &headers, b"data");
        let entry = cache.get(&url, &req).unwrap();
        assert_eq!(entry.etag.as_deref(), Some("\"abc123\""));
    }

    #[test]
    fn remove_entry() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/del");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "max-age=60")]);

        cache.store(&url, &req, 200, &headers, b"data");
        assert_eq!(cache.len(), 1);

        cache.remove(&url);
        assert!(cache.is_empty());
    }

    #[test]
    fn no_cache_is_not_fresh() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/nc");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "no-cache, max-age=3600")]);

        cache.store(&url, &req, 200, &headers, b"data");
        let entry = cache.get(&url, &req).unwrap();
        assert!(!entry.is_fresh()); // no-cache means always revalidate
    }

    #[test]
    fn refresh_from_not_modified_updates_entry() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/revalidate");
        let req = req_headers(&[]);
        let initial = test_headers(&[
            ("cache-control", "max-age=1"),
            ("etag", "\"v1\""),
            ("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT"),
        ]);

        cache.store(&url, &req, 200, &initial, b"body");

        let update = test_headers(&[
            ("cache-control", "max-age=120"),
            ("etag", "\"v2\""),
            ("last-modified", "Wed, 21 Oct 2015 08:28:00 GMT"),
        ]);

        assert!(cache.refresh_from_not_modified(&url, &req, &update));

        let entry = cache.get(&url, &req).unwrap();
        assert_eq!(entry.etag.as_deref(), Some("\"v2\""));
        assert_eq!(entry.last_modified.as_deref(), Some("Wed, 21 Oct 2015 08:28:00 GMT"));
        assert_eq!(entry.directives.max_age, Some(120));
        assert_eq!(entry.freshness_lifetime_secs, Some(120));
    }

    #[test]
    fn freshness_uses_age_with_max_age() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/age");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "max-age=120"), ("age", "20")]);

        cache.store(&url, &req, 200, &headers, b"data");
        let entry = cache.get(&url, &req).unwrap();
        assert_eq!(entry.freshness_lifetime_secs, Some(100));
    }

    #[test]
    fn freshness_uses_expires_when_max_age_missing() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/expires");
        let req = req_headers(&[]);

        let future = SystemTime::now() + std::time::Duration::from_secs(120);
        let expires = httpdate::fmt_http_date(future);

        let headers = test_headers(&[("expires", &expires)]);

        cache.store(&url, &req, 200, &headers, b"data");
        let entry = cache.get(&url, &req).unwrap();
        assert!(entry.freshness_lifetime_secs.unwrap_or(0) > 0);
    }

    #[test]
    fn vary_creates_and_selects_variants() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/vary");
        let headers = test_headers(&[("cache-control", "max-age=60"), ("vary", "accept-language")]);

        let req_en = req_headers(&[("Accept-Language", "en")]);
        let req_fr = req_headers(&[("Accept-Language", "fr")]);

        cache.store(&url, &req_en, 200, &headers, b"hello");
        cache.store(&url, &req_fr, 200, &headers, b"bonjour");

        assert_eq!(cache.get(&url, &req_en).unwrap().body, b"hello");
        assert_eq!(cache.get(&url, &req_fr).unwrap().body, b"bonjour");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn vary_star_is_not_cached() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/vary-star");
        let req = req_headers(&[("Accept", "text/html")]);
        let headers = test_headers(&[("cache-control", "max-age=60"), ("vary", "*")]);

        cache.store(&url, &req, 200, &headers, b"data");
        assert!(cache.get(&url, &req).is_none());
    }

    #[test]
    fn stale_if_error_window_allows_stale_use() {
        let mut cache = HttpCache::new();
        let url = test_url("https://example.com/stale");
        let req = req_headers(&[]);
        let headers = test_headers(&[("cache-control", "max-age=1, stale-if-error=30")]);

        cache.store(&url, &req, 200, &headers, b"data");
        let entry = cache
            .entries
            .get_mut(url.inner().as_str())
            .and_then(|v| v.first_mut())
            .unwrap();
        entry.stored_at = Instant::now() - std::time::Duration::from_secs(5);

        let stale = cache.get_stale_if_error(&url, &req).unwrap();
        assert_eq!(stale.body, b"data");
    }

    #[test]
    fn parse_cache_control_robust_inputs_no_panic() {
        let cases = [
            "",
            ",,,",
            "max-age=not-a-number",
            "max-age=-1",
            "stale-if-error=abc",
            "public,private,no-store,no-cache,must-revalidate",
            "MAX-AGE=60, STALE-WHILE-REVALIDATE=30",
            "\u{0000}\u{0001}",
            "x=y;z",
        ];

        for case in cases {
            let _ = parse_cache_control(case);
        }
    }
}
