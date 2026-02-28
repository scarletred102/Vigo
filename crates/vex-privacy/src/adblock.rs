// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Domain-based ad/tracker blocking.

use std::collections::HashSet;

use vex_core::VexUrl;

/// A simple domain blocklist engine.
///
/// Blocks exact domains and their subdomains.
#[derive(Debug, Clone)]
pub struct AdblockEngine {
    blocked: HashSet<String>,
}

impl AdblockEngine {
    /// Create an engine from an iterator of blocked domain strings.
    pub fn new(domains: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let blocked = domains.into_iter().map(Into::into).collect();
        Self { blocked }
    }

    /// Load blocked domains from text (one domain per line, `#` comments).
    pub fn from_list(text: &str) -> Self {
        let domains = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(String::from);
        Self::new(domains)
    }

    /// Check if a URL's host is blocked.
    pub fn is_blocked(&self, url: &VexUrl) -> bool {
        let Some(host) = url.host() else {
            return false;
        };

        // Check exact match
        if self.blocked.contains(host) {
            return true;
        }

        // Check parent domains (subdomain matching)
        // e.g., if "ads.example.com" is blocked, "foo.ads.example.com" is also blocked
        let mut domain = host;
        while let Some(dot_pos) = domain.find('.') {
            let parent = &domain[dot_pos + 1..];
            if self.blocked.contains(parent) {
                return true;
            }
            domain = parent;
        }

        false
    }

    /// Number of blocked domains loaded.
    pub fn len(&self) -> usize {
        self.blocked.len()
    }

    /// Whether the blocklist is empty.
    pub fn is_empty(&self) -> bool {
        self.blocked.is_empty()
    }
}

impl Default for AdblockEngine {
    fn default() -> Self {
        Self::new(std::iter::empty::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    fn engine() -> AdblockEngine {
        AdblockEngine::new([
            "ads.example.com",
            "tracker.net",
            "analytics.evil.org",
        ])
    }

    #[test]
    fn exact_match() {
        let e = engine();
        assert!(e.is_blocked(&url("https://ads.example.com/pixel.gif")));
        assert!(e.is_blocked(&url("https://tracker.net/")));
    }

    #[test]
    fn subdomain_blocked() {
        let e = engine();
        assert!(e.is_blocked(&url("https://foo.tracker.net/track")));
        assert!(e.is_blocked(&url("https://bar.baz.tracker.net/")));
    }

    #[test]
    fn non_match_passes() {
        let e = engine();
        assert!(!e.is_blocked(&url("https://example.com/")));
        assert!(!e.is_blocked(&url("https://good-site.org/")));
    }

    #[test]
    fn no_host_passes() {
        let e = engine();
        assert!(!e.is_blocked(&url("data:text/html,hello")));
    }

    #[test]
    fn from_list_format() {
        let list = r#"
# Block list for Vigo
ads.example.com
tracker.net

# Analytics
analytics.evil.org
"#;
        let e = AdblockEngine::from_list(list);
        assert_eq!(e.len(), 3);
        assert!(e.is_blocked(&url("https://ads.example.com/")));
    }

    #[test]
    fn empty_engine() {
        let e = AdblockEngine::default();
        assert!(e.is_empty());
        assert!(!e.is_blocked(&url("https://anything.com/")));
    }
}
