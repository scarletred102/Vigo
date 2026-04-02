// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Alt-Svc header parsing and alternative service tracking.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use vex_core::VexUrl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltSvcEntry {
    pub protocol: String,
    pub authority: String,
    pub expires_at: SystemTime,
}

#[derive(Debug, Default)]
pub struct AltSvcCache {
    by_origin: HashMap<String, Vec<AltSvcEntry>>,
}

impl AltSvcCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_response(&mut self, url: &VexUrl, headers: &HashMap<String, String>) {
        let origin = url.origin();
        let header = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("alt-svc"))
            .map(|(_, v)| v.as_str());
        let Some(header) = header else {
            return;
        };

        let parsed = parse_alt_svc(header);
        if !parsed.is_empty() {
            self.by_origin.insert(origin, parsed);
        }
    }

    pub fn preferred(&self, url: &VexUrl, protocol: &str) -> Option<&AltSvcEntry> {
        let now = SystemTime::now();
        self.by_origin
            .get(&url.origin())?
            .iter()
            .find(|e| e.protocol == protocol && e.expires_at > now)
    }
}

/// Parse `Alt-Svc` header value.
///
/// Example: `h3=":443"; ma=3600, h2=":443"; ma=3600`
pub fn parse_alt_svc(value: &str) -> Vec<AltSvcEntry> {
    let mut out = Vec::new();

    for item in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let mut protocol = None;
        let mut authority = None;
        let mut max_age = 86400u64;

        for (idx, part) in item.split(';').map(str::trim).enumerate() {
            if idx == 0 {
                if let Some((proto, auth)) = part.split_once('=') {
                    protocol = Some(proto.trim().to_ascii_lowercase());
                    authority = Some(auth.trim().trim_matches('"').to_string());
                }
                continue;
            }

            let lower = part.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("ma=") {
                if let Ok(parsed) = v.trim().parse::<u64>() {
                    max_age = parsed;
                }
            }
        }

        if let (Some(protocol), Some(authority)) = (protocol, authority) {
            out.push(AltSvcEntry {
                protocol,
                authority,
                expires_at: SystemTime::now() + Duration::from_secs(max_age),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_alt_svc() {
        let entries = parse_alt_svc("h3=\":443\"; ma=3600, h2=\":443\"; ma=1800");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].protocol, "h3");
        assert_eq!(entries[1].protocol, "h2");
    }
}
