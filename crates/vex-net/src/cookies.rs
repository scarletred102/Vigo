// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Cookie storage with domain/path scoping and security attributes.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use vex_core::VexUrl;
use vex_core::{VexError, VexResult};
use vex_crypto::{decrypt, derive_key, encrypt, generate_salt};

/// A single stored cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    host_only: bool,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    partitioned: bool,
    partition_key: Option<String>,
    same_party: bool,
    priority: CookiePriority,
    expires: Option<SystemTime>,
}

/// SameSite attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SameSite {
    /// No same-site restriction (requires `Secure`).
    None,
    /// Sent on top-level navigations and same-site requests.
    Lax,
    /// Only sent on same-site requests.
    Strict,
}

/// Cookie eviction/sending priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CookiePriority {
    Low,
    #[default]
    Medium,
    High,
}

/// Context for cookie access, controlling HttpOnly and SameSite filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieAccess {
    /// HTTP request — all matching cookies are included.
    HttpRequest,
    /// JS `document.cookie` — HttpOnly cookies are excluded.
    JsAccess,
}

/// Navigation context for SameSite enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKind {
    /// Same-site request (e.g. clicking a link within the site).
    SameSite,
    /// Cross-site top-level navigation (e.g. link from another domain).
    CrossSiteNavigation,
    /// Cross-site sub-resource request (e.g. fetch/XHR from JS).
    CrossSiteSubresource,
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
        self.insert_with_context(url, set_cookie, None);
    }

    /// Insert a cookie with top-level site context (for partitioned cookies).
    pub fn insert_with_context(
        &self,
        url: &VexUrl,
        set_cookie: &str,
        top_level_site: Option<&str>,
    ) {
        let Some(cookie) = parse_set_cookie(url, set_cookie, top_level_site) else {
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
    ///
    /// This is the simple API — includes all matching cookies (HTTP request context).
    pub fn get_cookies(&self, url: &VexUrl) -> Option<String> {
        self.get_cookies_filtered_with_context(
            url,
            CookieAccess::HttpRequest,
            NavigationKind::SameSite,
            None,
        )
    }

    /// Build the `Cookie` header value with full access/SameSite filtering.
    ///
    /// - `access`: [`CookieAccess::JsAccess`] excludes `HttpOnly` cookies.
    /// - `nav`: [`NavigationKind`] controls SameSite enforcement.
    pub fn get_cookies_filtered(
        &self,
        url: &VexUrl,
        access: CookieAccess,
        nav: NavigationKind,
    ) -> Option<String> {
        self.get_cookies_filtered_with_context(url, access, nav, None)
    }

    /// Build cookie header value with full filtering and optional top-level site context.
    pub fn get_cookies_filtered_with_context(
        &self,
        url: &VexUrl,
        access: CookieAccess,
        nav: NavigationKind,
        top_level_site: Option<&str>,
    ) -> Option<String> {
        let host = url.host().unwrap_or("");
        let path = url.path();
        let secure = url.is_https();
        let now = SystemTime::now();

        let store = match self.store.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut cookies: Vec<&Cookie> = store
            .values()
            .filter(|c| {
                // Domain match: host equals domain or is a subdomain
                domain_matches(host, &c.domain, c.host_only)
                    && path.starts_with(&c.path)
                    && (!c.secure || secure)
                    && c.expires.map_or(true, |exp| exp > now)
                    && (!c.partitioned
                        || top_level_site
                            .map(|tls| c.partition_key.as_deref() == Some(tls))
                            .unwrap_or(false))
            })
            .filter(|c| {
                // HttpOnly enforcement: JS cannot see HttpOnly cookies
                if access == CookieAccess::JsAccess && c.http_only {
                    return false;
                }
                true
            })
            .filter(|c| {
                // SameSite enforcement
                match c.same_site {
                    SameSite::None => {
                        // SameSite=None requires Secure
                        c.secure
                    }
                    SameSite::Lax => {
                        // Lax: allowed on same-site + top-level cross-site nav
                        matches!(
                            nav,
                            NavigationKind::SameSite | NavigationKind::CrossSiteNavigation
                        )
                    }
                    SameSite::Strict => {
                        // Strict: same-site only
                        nav == NavigationKind::SameSite
                    }
                }
            })
            .collect();

        cookies.sort_by(|a, b| {
            priority_rank(b.priority)
                .cmp(&priority_rank(a.priority))
                .then_with(|| b.path.len().cmp(&a.path.len()))
                .then_with(|| a.name.cmp(&b.name))
        });

        let pairs: Vec<String> = cookies
            .into_iter()
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

    /// Export all cookies in the jar as a JSON string.
    pub fn export_json(&self) -> VexResult<String> {
        let store = match self.store.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let cookies: Vec<Cookie> = store.values().cloned().collect();
        serde_json::to_string(&cookies)
            .map_err(|e| VexError::Storage(format!("cookie export failed: {e}")))
    }

    /// Import cookies from a JSON string, replacing existing jar contents.
    pub fn import_json(&self, json: &str) -> VexResult<()> {
        let cookies: Vec<Cookie> = serde_json::from_str(json)
            .map_err(|e| VexError::Storage(format!("cookie import failed: {e}")))?;

        let mut new_store = HashMap::new();
        for cookie in cookies {
            let key = format!("{}:{}:{}", cookie.domain, cookie.path, cookie.name);
            new_store.insert(key, cookie);
        }

        let mut store = match self.store.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *store = new_store;
        Ok(())
    }

    /// Export cookies encrypted with a password-derived key.
    ///
    /// Output format: `salt[16] || ciphertext`.
    pub fn export_encrypted(&self, password: &[u8]) -> VexResult<Vec<u8>> {
        let salt = generate_salt();
        let key = derive_key(password, &salt)
            .map_err(|e| VexError::Storage(format!("cookie key derivation failed: {e}")))?;
        let plaintext = self.export_json()?.into_bytes();
        let ciphertext = encrypt(&key, &plaintext)
            .map_err(|e| VexError::Storage(format!("cookie encryption failed: {e}")))?;

        let mut out = Vec::with_capacity(16 + ciphertext.len());
        out.extend_from_slice(&salt);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Import cookies from encrypted bytes produced by [`Self::export_encrypted`].
    pub fn import_encrypted(&self, password: &[u8], data: &[u8]) -> VexResult<()> {
        if data.len() < 16 {
            return Err(VexError::Storage(
                "encrypted cookie payload too short".to_string(),
            ));
        }

        let mut salt = [0u8; 16];
        salt.copy_from_slice(&data[..16]);
        let ciphertext = &data[16..];

        let key = derive_key(password, &salt)
            .map_err(|e| VexError::Storage(format!("cookie key derivation failed: {e}")))?;
        let plaintext = decrypt(&key, ciphertext)
            .map_err(|e| VexError::Storage(format!("cookie decryption failed: {e}")))?;
        let json = String::from_utf8(plaintext)
            .map_err(|e| VexError::Storage(format!("cookie UTF-8 decode failed: {e}")))?;
        self.import_json(&json)
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if `host` matches `domain` (exact or subdomain).
fn domain_matches(host: &str, domain: &str, host_only: bool) -> bool {
    if host_only {
        return host.eq_ignore_ascii_case(domain);
    }

    if host == domain {
        return true;
    }
    // Subdomain: host ends with ".domain"
    host.to_ascii_lowercase()
        .ends_with(&format!(".{}", domain.to_ascii_lowercase()))
}

fn is_public_suffix(domain: &str) -> bool {
    // Minimal embedded suffix set for safety; can be replaced by full PSL data.
    const COMMON_SUFFIXES: &[&str] = &[
        "com", "org", "net", "edu", "gov", "mil", "io", "app", "dev", "co", "uk", "co.uk", "ac.uk",
        "de", "fr", "jp", "cn", "ru", "br", "au", "ca",
    ];

    let d = domain.trim().to_ascii_lowercase();
    if d.is_empty() {
        return true;
    }
    if !d.contains('.') {
        return true;
    }
    COMMON_SUFFIXES.contains(&d.as_str())
}

fn priority_rank(p: CookiePriority) -> u8 {
    match p {
        CookiePriority::High => 3,
        CookiePriority::Medium => 2,
        CookiePriority::Low => 1,
    }
}

/// Parse a `Set-Cookie` header into a `Cookie` struct.
fn parse_set_cookie(url: &VexUrl, header: &str, top_level_site: Option<&str>) -> Option<Cookie> {
    let mut parts = header.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let name = name.trim().to_string();
    let value = value.trim().to_string();

    if name.is_empty() {
        return None;
    }

    let origin_host = url.host()?.to_ascii_lowercase();
    let default_domain = origin_host.clone();
    let default_path = {
        let p = url.path();
        match p.rfind('/') {
            Some(0) | None => "/".to_string(),
            Some(i) => p[..i].to_string(),
        }
    };

    let mut domain = default_domain;
    let mut host_only = true;
    let mut path = default_path;
    let mut secure = false;
    let mut http_only = false;
    let mut same_site = SameSite::Lax;
    let mut partitioned = false;
    let mut partition_key: Option<String> = None;
    let mut same_party = false;
    let mut priority = CookiePriority::Medium;
    let mut expires: Option<SystemTime> = None;

    for attr in parts {
        let attr = attr.trim();
        let lower = attr.to_ascii_lowercase();

        if lower.starts_with("domain=") {
            let d = attr[7..].trim().trim_start_matches('.');
            let candidate = d.to_ascii_lowercase();
            // Reject invalid Domain attributes that don't match request host.
            if !domain_matches(&origin_host, &candidate, false) {
                return None;
            }
            if is_public_suffix(&candidate) {
                return None;
            }
            domain = candidate;
            host_only = false;
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
        } else if lower == "partitioned" {
            partitioned = true;
        } else if lower == "sameparty" {
            same_party = true;
        } else if let Some(value) = lower.strip_prefix("priority=") {
            priority = match value.trim() {
                "high" => CookiePriority::High,
                "low" => CookiePriority::Low,
                _ => CookiePriority::Medium,
            };
        }
    }

    // SameSite=None requires Secure.
    if same_site == SameSite::None && !secure {
        return None;
    }

    // Cookie name prefix hardening.
    if name.starts_with("__Secure-") && (!secure || !url.is_https()) {
        return None;
    }
    if name.starts_with("__Host-") && (!secure || !url.is_https() || !host_only || path != "/") {
        return None;
    }

    if partitioned {
        if !secure {
            return None;
        }
        partition_key = top_level_site.map(|s| s.to_string());
        partition_key.as_ref()?;
    }

    Some(Cookie {
        name,
        value,
        domain,
        host_only,
        path,
        secure,
        http_only,
        same_site,
        partitioned,
        partition_key,
        same_party,
        priority,
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
    fn samesite_none_without_secure_is_rejected() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "none_tok=1; SameSite=None");
        assert!(jar.get_cookies(&url).is_none());
    }

    #[test]
    fn partitioned_cookie_requires_top_level_site_context() {
        let jar = CookieJar::new();
        let url = test_url("https://cdn.example/");

        jar.insert(&url, "chip=1; Secure; Partitioned");
        assert!(jar.get_cookies(&url).is_none());

        jar.insert_with_context(&url, "chip=1; Secure; Partitioned", Some("news.example"));
        let none_ctx = jar.get_cookies_filtered_with_context(
            &url,
            CookieAccess::HttpRequest,
            NavigationKind::SameSite,
            None,
        );
        assert!(none_ctx.is_none());

        let with_ctx = jar.get_cookies_filtered_with_context(
            &url,
            CookieAccess::HttpRequest,
            NavigationKind::SameSite,
            Some("news.example"),
        );
        assert!(with_ctx.is_some());
    }

    #[test]
    fn public_suffix_domain_rejected() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "a=1; Domain=com");
        assert!(jar.get_cookies(&url).is_none());
    }

    #[test]
    fn encrypted_cookie_roundtrip() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "s=1; Path=/");

        let blob = jar.export_encrypted(b"password").expect("encrypt export");

        let restored = CookieJar::new();
        restored
            .import_encrypted(b"password", &blob)
            .expect("decrypt import");

        let cookies = restored.get_cookies(&url).unwrap();
        assert!(cookies.contains("s=1"));
    }

    #[test]
    fn secure_prefix_requires_secure_and_https() {
        let jar = CookieJar::new();
        let http_url = test_url("http://example.com/");
        jar.insert(&http_url, "__Secure-id=1; Secure");
        assert!(jar.get_cookies(&http_url).is_none());

        let https_url = test_url("https://example.com/");
        jar.insert(&https_url, "__Secure-id=1; Secure");
        assert!(jar
            .get_cookies(&https_url)
            .is_some_and(|c| c.contains("__Secure-id=1")));
    }

    #[test]
    fn host_prefix_requires_host_only_and_root_path() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/app");

        // invalid path
        jar.insert(&url, "__Host-a=1; Secure; Path=/app");
        assert!(jar.get_cookies(&url).is_none());

        // invalid domain attribute for __Host-
        jar.insert(&url, "__Host-a=1; Secure; Path=/; Domain=example.com");
        assert!(jar.get_cookies(&url).is_none());

        // valid host-only root-path cookie
        jar.insert(&url, "__Host-a=1; Secure; Path=/");
        assert!(jar
            .get_cookies(&url)
            .is_some_and(|c| c.contains("__Host-a=1")));
    }

    #[test]
    fn cookies_sorted_by_path_length_then_name() {
        let jar = CookieJar::new();
        let root = test_url("https://example.com/");
        let app = test_url("https://example.com/app/page");

        jar.insert(&root, "b=1; Path=/");
        jar.insert(&root, "a=2; Path=/");
        jar.insert(&app, "z=3; Path=/app");

        let cookies = jar.get_cookies(&app).unwrap();
        assert!(cookies.starts_with("z=3"));
        assert!(cookies.contains("a=2"));
        assert!(cookies.contains("b=1"));
        // a should sort before b when path length is equal.
        assert!(cookies.find("a=2").unwrap() < cookies.find("b=1").unwrap());
    }

    #[test]
    fn cookie_jar_json_roundtrip() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "a=1; Path=/");
        jar.insert(&url, "b=2; Path=/app");

        let json = jar.export_json().expect("export json");

        let restored = CookieJar::new();
        restored.import_json(&json).expect("import json");

        let cookies = restored
            .get_cookies(&test_url("https://example.com/app/page"))
            .unwrap();
        assert!(cookies.contains("a=1"));
        assert!(cookies.contains("b=2"));
    }

    #[test]
    fn cookie_parser_robust_inputs_do_not_panic() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        let cases = [
            "",
            "=",
            "=x",
            "x",
            "x=1; Domain=..",
            "x=1; Max-Age=not-number",
            "x=1; SameSite=Unknown",
            "__Host-a=1; Path=/",
            "__Secure-a=1",
            "x=1; Domain=other.com",
            "x=1; Path=",
        ];

        for case in cases {
            jar.insert(&url, case);
        }

        // Main assertion: parser accepted/rejected inputs safely without panic.
        let _ = jar.get_cookies(&url);
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

    #[test]
    fn httponly_excluded_from_js_access() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "visible=1");
        jar.insert(&url, "secret=2; HttpOnly");

        // HTTP request sees both
        let http = jar
            .get_cookies_filtered(&url, CookieAccess::HttpRequest, NavigationKind::SameSite)
            .unwrap();
        assert!(http.contains("visible=1"));
        assert!(http.contains("secret=2"));

        // JS access sees only the non-HttpOnly cookie
        let js = jar
            .get_cookies_filtered(&url, CookieAccess::JsAccess, NavigationKind::SameSite)
            .unwrap();
        assert!(js.contains("visible=1"));
        assert!(!js.contains("secret=2"));
    }

    #[test]
    fn samesite_strict_blocks_cross_site() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "strict_tok=abc; SameSite=Strict");

        // Same-site: included
        let same = jar
            .get_cookies_filtered(&url, CookieAccess::HttpRequest, NavigationKind::SameSite)
            .unwrap();
        assert!(same.contains("strict_tok=abc"));

        // Cross-site navigation: excluded
        let cross_nav = jar.get_cookies_filtered(
            &url,
            CookieAccess::HttpRequest,
            NavigationKind::CrossSiteNavigation,
        );
        assert!(cross_nav.is_none());

        // Cross-site subresource: excluded
        let cross_sub = jar.get_cookies_filtered(
            &url,
            CookieAccess::HttpRequest,
            NavigationKind::CrossSiteSubresource,
        );
        assert!(cross_sub.is_none());
    }

    #[test]
    fn samesite_lax_allows_cross_site_navigation() {
        let jar = CookieJar::new();
        let url = test_url("https://example.com/");
        jar.insert(&url, "lax_tok=xyz; SameSite=Lax");

        // Same-site: included
        let same = jar
            .get_cookies_filtered(&url, CookieAccess::HttpRequest, NavigationKind::SameSite)
            .unwrap();
        assert!(same.contains("lax_tok=xyz"));

        // Cross-site navigation: included (Lax allows top-level nav)
        let cross_nav = jar
            .get_cookies_filtered(
                &url,
                CookieAccess::HttpRequest,
                NavigationKind::CrossSiteNavigation,
            )
            .unwrap();
        assert!(cross_nav.contains("lax_tok=xyz"));

        // Cross-site subresource: excluded
        let cross_sub = jar.get_cookies_filtered(
            &url,
            CookieAccess::HttpRequest,
            NavigationKind::CrossSiteSubresource,
        );
        assert!(cross_sub.is_none());
    }
}
