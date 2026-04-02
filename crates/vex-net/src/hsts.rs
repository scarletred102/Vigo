// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HSTS policy store and URL upgrade logic.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use vex_core::VexUrl;

#[derive(Debug, Clone)]
struct HstsEntry {
    expires_at: SystemTime,
    include_subdomains: bool,
}

#[derive(Debug, Default)]
pub struct HstsStore {
    entries: HashMap<String, HstsEntry>,
}

impl HstsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_response(&mut self, url: &VexUrl, headers: &HashMap<String, String>) {
        let host = match url.host() {
            Some(h) => h.to_ascii_lowercase(),
            None => return,
        };

        let hsts = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("strict-transport-security"))
            .map(|(_, v)| v.as_str());
        let Some(hsts) = hsts else {
            return;
        };

        let mut max_age: Option<u64> = None;
        let mut include_subdomains = false;

        for part in hsts.split(';').map(str::trim) {
            let lower = part.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("max-age=") {
                max_age = v.trim().parse::<u64>().ok();
            } else if lower == "includesubdomains" {
                include_subdomains = true;
            }
        }

        let Some(max_age) = max_age else {
            return;
        };

        if max_age == 0 {
            self.entries.remove(&host);
            return;
        }

        self.entries.insert(
            host,
            HstsEntry {
                expires_at: SystemTime::now() + Duration::from_secs(max_age),
                include_subdomains,
            },
        );
    }

    pub fn should_upgrade_host(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        let now = SystemTime::now();

        if let Some(entry) = self.entries.get(&host) {
            if entry.expires_at > now {
                return true;
            }
        }

        // Check parent domains with includeSubdomains.
        let mut parts: Vec<&str> = host.split('.').collect();
        while parts.len() > 1 {
            parts.remove(0);
            let candidate = parts.join(".");
            if let Some(entry) = self.entries.get(&candidate) {
                if entry.expires_at > now && entry.include_subdomains {
                    return true;
                }
            }
        }

        false
    }

    pub fn upgrade_url(&self, url: &VexUrl) -> Option<VexUrl> {
        if url.scheme() != "http" {
            return None;
        }
        let host = url.host()?;
        if !self.should_upgrade_host(host) {
            return None;
        }

        let mut upgraded = url.inner().clone();
        upgraded
            .set_scheme("https")
            .expect("http->https scheme change should be valid");
        VexUrl::parse(upgraded.as_str()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(v: &str) -> HashMap<String, String> {
        HashMap::from([("strict-transport-security".to_string(), v.to_string())])
    }

    #[test]
    fn hsts_upgrades_http_url() {
        let mut store = HstsStore::new();
        let https = VexUrl::parse("https://example.com/").unwrap();
        store.observe_response(&https, &headers("max-age=3600"));

        let http = VexUrl::parse("http://example.com/path").unwrap();
        let upgraded = store.upgrade_url(&http).unwrap();
        assert_eq!(upgraded.scheme(), "https");
        assert_eq!(upgraded.host(), Some("example.com"));
    }

    #[test]
    fn include_subdomains_applies_to_children() {
        let mut store = HstsStore::new();
        let https = VexUrl::parse("https://example.com/").unwrap();
        store.observe_response(&https, &headers("max-age=3600; includeSubDomains"));

        assert!(store.should_upgrade_host("a.b.example.com"));
    }
}
