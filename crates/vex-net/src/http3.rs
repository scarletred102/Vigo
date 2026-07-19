// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Experimental HTTP/3 transport surface.
//!
//! This module provides the protocol-selection scaffolding required to route
//! future requests through QUIC/HTTP3 while preserving compatibility fallback.

use vex_core::{VexError, VexResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpVersionPreference {
    #[default]
    Auto,
    Http11,
    Http2,
    Http3,
}

#[derive(Debug, Clone, Default)]
pub struct Http3Config {
    pub enabled: bool,
    pub alt_svc_upgrade: bool,
}

/// Validate whether HTTP/3 may be attempted under current config.
pub fn validate_http3_attempt(
    config: &Http3Config,
    preference: HttpVersionPreference,
) -> VexResult<()> {
    if preference == HttpVersionPreference::Http3 && !config.enabled {
        return Err(VexError::Network(
            "HTTP/3 requested but not enabled in client config".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http3_request_requires_enabled_flag() {
        let cfg = Http3Config {
            enabled: false,
            alt_svc_upgrade: true,
        };
        assert!(validate_http3_attempt(&cfg, HttpVersionPreference::Http3).is_err());
    }
}
