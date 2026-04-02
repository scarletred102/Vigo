// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Transport security policy tracking (Expect-CT / HPKP-style pins / OCSP hooks).

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use vex_core::{VexError, VexResult, VexUrl};

#[derive(Debug, Clone)]
struct ExpectCtPolicy {
    enforce: bool,
    expires_at: SystemTime,
}

#[derive(Debug, Clone)]
struct PinPolicy {
    pins: Vec<String>,
    expires_at: SystemTime,
}

#[derive(Debug, Default)]
pub struct TransportSecurityPolicy {
    expect_ct: HashMap<String, ExpectCtPolicy>,
    pinning: HashMap<String, PinPolicy>,
}

impl TransportSecurityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_response(&mut self, url: &VexUrl, headers: &HashMap<String, String>) {
        let host = match url.host() {
            Some(h) => h.to_ascii_lowercase(),
            None => return,
        };

        if let Some(expect_ct) = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("expect-ct"))
            .map(|(_, v)| v.as_str())
        {
            let mut enforce = false;
            let mut max_age = None;
            for part in expect_ct.split(',').map(str::trim) {
                let lower = part.to_ascii_lowercase();
                if lower == "enforce" {
                    enforce = true;
                } else if let Some(v) = lower.strip_prefix("max-age=") {
                    max_age = v.trim().parse::<u64>().ok();
                }
            }

            if let Some(max_age) = max_age {
                self.expect_ct.insert(
                    host.clone(),
                    ExpectCtPolicy {
                        enforce,
                        expires_at: SystemTime::now() + Duration::from_secs(max_age),
                    },
                );
            }
        }

        if let Some(hpkp) = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("public-key-pins"))
            .map(|(_, v)| v.as_str())
        {
            let mut pins = Vec::new();
            let mut max_age = None;
            for part in hpkp.split(';').map(str::trim) {
                let lower = part.to_ascii_lowercase();
                if let Some(v) = lower.strip_prefix("max-age=") {
                    max_age = v.trim().parse::<u64>().ok();
                    continue;
                }

                if lower.starts_with("pin-sha256=") {
                    let pin = part
                        .split_once('=')
                        .map(|(_, rhs)| rhs.trim().trim_matches('"').to_string());
                    if let Some(pin) = pin {
                        pins.push(pin);
                    }
                }
            }

            if let Some(max_age) = max_age {
                self.pinning.insert(
                    host,
                    PinPolicy {
                        pins,
                        expires_at: SystemTime::now() + Duration::from_secs(max_age),
                    },
                );
            }
        }
    }

    pub fn evaluate_tls_evidence(
        &self,
        host: &str,
        has_sct: bool,
        has_ocsp: bool,
        strict_evidence: bool,
    ) -> VexResult<()> {
        if !strict_evidence {
            return Ok(());
        }

        let now = SystemTime::now();
        let host = host.to_ascii_lowercase();

        if let Some(expect_ct) = self.expect_ct.get(&host) {
            if expect_ct.expires_at > now && expect_ct.enforce && !has_sct {
                return Err(VexError::Network(
                    "Expect-CT enforcement failed (missing SCT evidence)".to_string(),
                ));
            }
        }

        if let Some(pin_policy) = self.pinning.get(&host) {
            if pin_policy.expires_at > now && pin_policy.pins.is_empty() {
                return Err(VexError::Network(
                    "HPKP policy invalid (no usable pins)".to_string(),
                ));
            }
        }

        if !has_ocsp {
            return Err(VexError::Network(
                "OCSP evidence missing for strict transport policy".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expect_ct_enforce_requires_sct_when_strict() {
        let mut p = TransportSecurityPolicy::new();
        let url = VexUrl::parse("https://example.com/").unwrap();
        let headers = HashMap::from([("expect-ct".to_string(), "max-age=60, enforce".to_string())]);
        p.observe_response(&url, &headers);

        let res = p.evaluate_tls_evidence("example.com", false, true, true);
        assert!(res.is_err());
    }

    #[test]
    fn non_strict_mode_allows_missing_evidence() {
        let p = TransportSecurityPolicy::new();
        assert!(p
            .evaluate_tls_evidence("example.com", false, false, false)
            .is_ok());
    }
}
