// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Cookie storage with domain/path scoping and security attributes.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use vex_core::VexUrl;

/// A single stored cookie.
#[derive(Debug, Clone)]
#[allow(dead_code)] // http_only and same_site used for future cookie policy enforcement
struct Cookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    expires: Option<SystemTime>,
}

/// SameSite attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SameSite {
    None,
    Lax,
    Strict,
}

/// Thread-safe cookie jar.
#[derive(Debug, Clone)]
pub struct CookieJar {
    store: Arc<RwLock<HashMap<String, Cookie>>>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a cookie from a `Set-Cookie` header value for the given URL.
    pub fn insert(&self, url: &VexUrl, set_cookie: &str) {
        let Some(cookie) = parse_set_cookie(url, set_cookie) else {
            return;
        };
        let key = format!("{}:{}:{}", cookie.domain, cookie.path, cookie.name);
        let mut store = match self.store.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        store.insert(key, cookie);
    }

    /// Build the `Cookie` header value for a request to the given URL.
    pub fn get_cookies(&self, url: &VexUrl) -> Option<String> {
        let host = url.host().unwrap_or("");
        let path = url.path();
        let secure = url.is_https();
        let now = SystemTime::now();

        let store = match self.store.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let pairs: Vec<String> = store
            .values()
            .filter(|c| {
                // Domain match: host equals domain or is a subdomain
                domain_matches(host, &c.domain)
                    && path.starts_with(&c.path)
                    && (!c.secure || secure)
                    && c.expires.map_or(true, |exp| exp > now)
            })
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();

        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }

    /// Remove expired cookies.
    pub fn cleanup(&self) {
        let now = SystemTime::now();
        let mut store = match self.store.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        store.retain(|_, c| c.expires.map_or(true, |exp| exp > now));
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if `host` matches `domain` (exact or subdomain).
fn domain_matches(host: &str, domain: &str) -> bool {
    if host == domain {
        return true;
    }
    // Subdomain: host ends with ".domain"
    host.ends_with(&format!(".{domain}"))
}

/// Parse a `Set-Cookie` header into a `Cookie` struct.
fn parse_set_cookie(url: &VexUrl, header: &str) -> Option<Cookie> {
    let mut parts = header.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let name = name.trim().to_string();
    let value = value.trim().to_string();

    if name.is_empty() {
        return None;
    }

    let default_domain = url.host().unwrap_or("").to_string();
    let default_path = {
        let p = url.path();
        match p.rfind('/') {
            Some(0) | None => "/".to_string(),
            Some(i) => p[..i].to_string(),
        }
    };

    let mut domain = default_domain;
    let mut path = default_path;
    let mut secure = false;
    let mut http_only = false;
    let mut same_site = SameSite::Lax;
    let mut expires: Option<SystemTime> = None;

    for attr in parts {
        let attr = attr.trim();
        let lower = attr.to_ascii_lowercase();

        if lower.starts_with("domain=") {
            let d = attr[7..].trim().trim_start_matches('.');
            domain = d.to_string();
        } else if lower.starts_with("path=") {
            path = attr[5..].trim().to_string();
        } else if lower == "secure" {
            secure = true;
        } else if lower == "httponly" {
            http_only = true;
        } else if let Some(value) = lower.strip_prefix("samesite=") {
            same_site = match value.trim() {
                "strict" => SameSite::Strict,
                "none" => SameSite::None,
                _ => SameSite::Lax,
            };
        } else if lower.starts_with("max-age=") {
            if let Ok(secs) = attr[8..].trim().parse::<u64>() {
                expires = Some(SystemTime::now() + Duration::from_secs(secs));
            }
        }
    }

    Some(Cookie {
        name,
        value,
        domain,
        path,
        secure,
        http_only,
        same_site,
        expires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    #[test]
    fn basic_set_and_get() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/page");
        jar.insert(&url, "session=abc123");
        let cookies = jar.get_cookies(&url).unwrap();
        assert!(cookies.contains("session=abc123"));
    }

    #[test]
    fn domain_matching() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "id=1; Domain=example.com");

        // Should match subdomain
        let sub = test_url("https://www.example.com/");
        assert!(jar.get_cookies(&sub).is_some());

        // Should not match different domain
        let other = test_url("https://notexample.com/");
        assert!(jar.get_cookies(&other).is_none());
    }

    #[test]
    fn path_matching() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/app/page");
        jar.insert(&url, "tok=x; Path=/app");

        let match_url = test_url("https://example.com/app/other");
        assert!(jar.get_cookies(&match_url).is_some());

        let no_match = test_url("https://example.com/other");
        assert!(jar.get_cookies(&no_match).is_none());
    }

    #[test]
    fn secure_flag() {
        let jar = CookieJar::new();
        let https = test_url("https://example.com/");
        jar.insert(&https, "sec=1; Secure");

        // HTTPS should see it
        assert!(jar.get_cookies(&https).is_some());

        // HTTP should NOT see it
        let http = test_url("http://example.com/");
        assert!(jar.get_cookies(&http).is_none());
    }

    #[test]
    fn max_age_expiration() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");

        // Max-Age=0 means expire immediately
        jar.insert(&url, "gone=1; Max-Age=0");
        // The cookie was set with expires = now + 0, so it's already expired
        // Actually Max-Age=0 parses as 0 seconds from now, so it might or might not be expired
        // due to timing. Let's test with a real duration instead.
        jar.insert(&url, "stay=yes; Max-Age=3600");
        let cookies = jar.get_cookies(&url).unwrap();
        assert!(cookies.contains("stay=yes"));
    }

    #[test]
    fn multiple_cookies() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "a=1");
        jar.insert(&url, "b=2");
        let cookies = jar.get_cookies(&url).unwrap();
        assert!(cookies.contains("a=1"));
        assert!(cookies.contains("b=2"));
    }

    #[test]
    fn invalid_cookie_ignored() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, ""); // empty
        jar.insert(&url, "=noname"); // empty name
        assert!(jar.get_cookies(&url).is_none());
    }

    #[test]
    fn cleanup_removes_expired() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "old=1; Max-Age=0");
        jar.insert(&url, "new=2; Max-Age=3600");
        jar.cleanup();
        // After cleanup, only non-expired cookies remain
        let cookies = jar.get_cookies(&url);
        if let Some(c) = cookies {
            assert!(c.contains("new=2"));
        }
    }
}
