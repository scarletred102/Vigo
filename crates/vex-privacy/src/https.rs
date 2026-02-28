// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTPS-only mode: upgrades HTTP to HTTPS, with exemptions for local addresses.

use vex_core::VexUrl;

/// Upgrade a URL from HTTP to HTTPS.
///
/// Returns `true` if the URL was upgraded, `false` if already HTTPS or exempt.
///
/// Exempt hosts: `localhost`, `127.0.0.1`, `[::1]`, `.local`, `.onion`.
pub fn enforce_https(url: &VexUrl) -> Option<VexUrl> {
    if url.scheme() != "http" {
        return None; // Already HTTPS or other scheme
    }

    let host = url.host().unwrap_or("");

    // Exempt local/special addresses
    if is_exempt(host) {
        return None;
    }

    // Upgrade http:// → https://
    let upgraded = url.inner().as_str().replacen("http://", "https://", 1);
    VexUrl::parse(&upgraded).ok()
}

/// Check if a host is exempt from HTTPS enforcement.
fn is_exempt(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "[::1]"
        || host.ends_with(".local")
        || host.ends_with(".onion")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    #[test]
    fn upgrades_http() {
        let u = url("http://example.com/page");
        let upgraded = enforce_https(&u).unwrap();
        assert_eq!(upgraded.inner().as_str(), "https://example.com/page");
    }

    #[test]
    fn already_https() {
        let u = url("https://example.com/");
        assert!(enforce_https(&u).is_none());
    }

    #[test]
    fn exempt_localhost() {
        let u = url("http://localhost:3000/api");
        assert!(enforce_https(&u).is_none());
    }

    #[test]
    fn exempt_loopback() {
        let u = url("http://127.0.0.1:8080/");
        assert!(enforce_https(&u).is_none());
    }

    #[test]
    fn exempt_onion() {
        let u = url("http://abc123.onion/");
        assert!(enforce_https(&u).is_none());
    }

    #[test]
    fn exempt_local_domain() {
        let u = url("http://mydevbox.local/");
        assert!(enforce_https(&u).is_none());
    }

    #[test]
    fn preserves_path_and_query() {
        let u = url("http://example.com/path?q=test&page=2#frag");
        let upgraded = enforce_https(&u).unwrap();
        assert!(upgraded.inner().as_str().starts_with("https://example.com/path?q=test"));
    }
}
