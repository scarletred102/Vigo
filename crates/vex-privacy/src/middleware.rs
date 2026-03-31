// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Privacy middleware — integrates all privacy filters into a single layer.

use vex_core::{VexError, VexResult};
use vex_net::types::Request;

use crate::adblock::AdblockEngine;
use crate::headers::sanitize_headers;
use crate::https::enforce_https;
use crate::tracking::strip_tracking_params;

/// Configuration for the privacy layer.
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    /// Strip tracking query parameters.
    pub strip_tracking: bool,
    /// Enforce HTTPS-only mode.
    pub https_only: bool,
    /// Sanitize outgoing headers.
    pub sanitize_headers: bool,
    /// Enable ad/tracker domain blocking.
    pub adblock: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            strip_tracking: true,
            https_only: true,
            sanitize_headers: true,
            adblock: true,
        }
    }
}

/// Composable privacy middleware.
///
/// Wraps all privacy sub-systems and applies them to outgoing requests.
pub struct PrivacyLayer {
    config: PrivacyConfig,
    adblock_engine: AdblockEngine,
}

impl PrivacyLayer {
    /// Create a new privacy layer with default config and no blocked domains.
    pub fn new() -> Self {
        Self {
            config: PrivacyConfig::default(),
            adblock_engine: AdblockEngine::new(std::iter::empty::<String>()),
        }
    }

    /// Create a privacy layer with custom configuration and an adblock engine.
    pub fn with_config(config: PrivacyConfig, adblock_engine: AdblockEngine) -> Self {
        Self {
            config,
            adblock_engine,
        }
    }

    /// Access the current configuration.
    pub fn config(&self) -> &PrivacyConfig {
        &self.config
    }

    /// Process a request through all privacy filters.
    ///
    /// - Strips tracking parameters from the URL.
    /// - Upgrades HTTP to HTTPS.
    /// - Sanitizes outgoing headers.
    /// - Blocks requests to ad/tracker domains.
    ///
    /// Returns `Err` if the request should be blocked.
    pub fn process_request(&self, request: &mut Request) -> VexResult<()> {
        // 1. Check adblock — block before doing any other work
        if self.config.adblock && self.adblock_engine.is_blocked(&request.url) {
            return Err(VexError::Network(format!(
                "blocked by privacy filter: {}",
                request.url.host().unwrap_or("unknown")
            )));
        }

        // 2. Strip tracking parameters
        if self.config.strip_tracking {
            request.url = strip_tracking_params(&request.url);
        }

        // 3. Enforce HTTPS
        if self.config.https_only {
            if let Some(upgraded) = enforce_https(&request.url) {
                request.url = upgraded;
            }
        }

        // 4. Sanitize headers
        if self.config.sanitize_headers {
            let host = request.url.host().map(|h| h.to_string());
            sanitize_headers(&mut request.headers, host.as_deref());
        }

        Ok(())
    }
}

impl Default for PrivacyLayer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adblock::AdblockEngine;

    #[test]
    fn blocks_adblock_domain() {
        let engine = AdblockEngine::new(vec!["ads.example.com".to_string()]);
        let layer = PrivacyLayer::with_config(PrivacyConfig::default(), engine);

        let mut req = Request::get("https://ads.example.com/banner.js").unwrap();
        let result = layer.process_request(&mut req);
        assert!(result.is_err());
    }

    #[test]
    fn allows_clean_domain() {
        let layer = PrivacyLayer::new();
        let mut req = Request::get("https://example.com/page").unwrap();
        assert!(layer.process_request(&mut req).is_ok());
    }

    #[test]
    fn strips_tracking_params() {
        let layer = PrivacyLayer::new();
        let mut req = Request::get("https://example.com/page?utm_source=test&q=hello").unwrap();
        layer.process_request(&mut req).unwrap();
        assert!(req.url.inner().as_str().contains("q=hello"));
        assert!(!req.url.inner().as_str().contains("utm_source"));
    }

    #[test]
    fn upgrades_http_to_https() {
        let layer = PrivacyLayer::new();
        let mut req = Request::get("http://example.com/page").unwrap();
        layer.process_request(&mut req).unwrap();
        assert!(req.url.is_https());
    }

    #[test]
    fn sanitizes_headers() {
        let layer = PrivacyLayer::new();
        let mut req = Request::get("https://example.com/").unwrap();
        req.headers
            .insert("x-client-data".to_string(), "abc".to_string());
        req.headers
            .insert("accept".to_string(), "text/html".to_string());

        layer.process_request(&mut req).unwrap();
        assert!(!req.headers.contains_key("x-client-data"));
        assert!(req.headers.contains_key("accept"));
    }

    #[test]
    fn disabled_tracking_strip() {
        let config = PrivacyConfig {
            strip_tracking: false,
            ..Default::default()
        };
        let layer =
            PrivacyLayer::with_config(config, AdblockEngine::new(std::iter::empty::<String>()));

        let mut req = Request::get("https://example.com/?utm_source=x").unwrap();
        layer.process_request(&mut req).unwrap();
        assert!(req.url.inner().as_str().contains("utm_source"));
    }
}
