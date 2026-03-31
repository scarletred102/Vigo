// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Cross-Origin Resource Sharing (CORS) enforcement.
//!
//! Implements the [Fetch Standard §3 CORS protocol](https://fetch.spec.whatwg.org/#http-cors-protocol):
//!
//! - Simple request detection (no preflight needed).
//! - Preflight header generation (`Access-Control-Request-*`).
//! - Preflight / actual-response validation.
//! - `Access-Control-Expose-Headers` filtering.

use std::collections::{HashMap, HashSet};

use crate::error::{SecurityError, SecurityResult};

// ── Simple-request thresholds ──────────────────────────────────────────

/// Methods that never require a preflight.
const SIMPLE_METHODS: &[&str] = &["GET", "HEAD", "POST"];

/// Headers that never trigger a preflight (case-insensitive comparison).
const SIMPLE_HEADERS: &[&str] = &[
    "accept",
    "accept-language",
    "content-language",
    "content-type",
];

// ── Public types ───────────────────────────────────────────────────────

/// Configuration for an outgoing cross-origin request.
#[derive(Debug, Clone)]
pub struct CorsRequest {
    /// The serialised origin of the requesting page (e.g. `"https://example.com"`).
    pub origin: String,
    /// The HTTP method (upper-case, e.g. `"GET"`, `"PUT"`).
    pub method: String,
    /// Non-simple request headers the caller intends to send.
    pub headers: Vec<String>,
    /// Whether the request should include credentials (cookies, auth).
    pub with_credentials: bool,
}

/// Whether a cross-origin request is simple or needs a preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorsMode {
    /// No preflight required — send the actual request directly.
    Simple,
    /// An OPTIONS preflight must be sent first.
    Preflight,
}

/// Headers to attach to the OPTIONS preflight request.
#[derive(Debug, Clone)]
pub struct PreflightHeaders {
    /// `Origin` header value.
    pub origin: String,
    /// `Access-Control-Request-Method` header value.
    pub request_method: String,
    /// `Access-Control-Request-Headers` header value (comma-separated).
    /// `None` if there are no non-simple headers.
    pub request_headers: Option<String>,
}

// ── Classification ─────────────────────────────────────────────────────

/// Determine whether a cross-origin request is simple or needs preflight.
pub fn classify_cors(req: &CorsRequest) -> CorsMode {
    // Non-simple method → preflight.
    if !SIMPLE_METHODS.contains(&req.method.to_uppercase().as_str()) {
        return CorsMode::Preflight;
    }

    // Any non-simple header → preflight.
    for h in &req.headers {
        let lower = h.to_lowercase();
        if !SIMPLE_HEADERS.contains(&lower.as_str()) {
            return CorsMode::Preflight;
        }
        // Content-Type must be one of the simple types.
        if lower == "content-type" {
            // We don't know the value here — but having Content-Type in the
            // list at all only triggers preflight if the value isn't simple.
            // The caller should exclude Content-Type from `headers` when the
            // value *is* one of the three simple types.
        }
    }

    CorsMode::Simple
}

/// Build the headers for an OPTIONS preflight request.
pub fn build_preflight_headers(req: &CorsRequest) -> PreflightHeaders {
    let non_simple: Vec<&String> = req
        .headers
        .iter()
        .filter(|h| !SIMPLE_HEADERS.contains(&h.to_lowercase().as_str()))
        .collect();

    PreflightHeaders {
        origin: req.origin.clone(),
        request_method: req.method.clone(),
        request_headers: if non_simple.is_empty() {
            None
        } else {
            Some(
                non_simple
                    .iter()
                    .map(|h| h.to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
    }
}

// ── Validation ─────────────────────────────────────────────────────────

/// Validate a preflight (OPTIONS) response.
///
/// Checks `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`,
/// and `Access-Control-Allow-Headers`.
pub fn validate_preflight(
    request: &CorsRequest,
    response_headers: &HashMap<String, String>,
) -> SecurityResult<()> {
    validate_allow_origin(&request.origin, request.with_credentials, response_headers)?;
    validate_allow_methods(&request.method, response_headers)?;
    validate_allow_headers(&request.headers, response_headers)?;
    Ok(())
}

/// Validate the `Access-Control-Allow-Origin` header on an actual response.
///
/// The server must echo the request origin or use `"*"` (wildcard is
/// forbidden when credentials are included).
pub fn validate_response(
    request_origin: &str,
    with_credentials: bool,
    response_headers: &HashMap<String, String>,
) -> SecurityResult<()> {
    validate_allow_origin(request_origin, with_credentials, response_headers)
}

/// Return the set of response header names the page is allowed to read.
///
/// Always includes the [CORS-safelisted response headers][safe] plus any
/// names listed in `Access-Control-Expose-Headers`.
///
/// [safe]: https://fetch.spec.whatwg.org/#cors-safelisted-response-header-name
pub fn exposed_headers(response_headers: &HashMap<String, String>) -> HashSet<String> {
    let mut allowed: HashSet<String> = [
        "cache-control",
        "content-language",
        "content-length",
        "content-type",
        "expires",
        "last-modified",
        "pragma",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();

    if let Some(expose) = get_header(response_headers, "access-control-expose-headers") {
        for name in expose.split(',') {
            let trimmed = name.trim().to_lowercase();
            if !trimmed.is_empty() {
                allowed.insert(trimmed);
            }
        }
    }

    allowed
}

// ── Internal helpers ───────────────────────────────────────────────────

fn validate_allow_origin(
    request_origin: &str,
    with_credentials: bool,
    headers: &HashMap<String, String>,
) -> SecurityResult<()> {
    let allowed = get_header(headers, "access-control-allow-origin").ok_or_else(|| {
        SecurityError::CorsBlocked("missing Access-Control-Allow-Origin header".into())
    })?;

    if allowed == "*" {
        if with_credentials {
            return Err(SecurityError::CorsBlocked(
                "wildcard Access-Control-Allow-Origin not allowed with credentials".into(),
            ));
        }
        return Ok(());
    }

    if allowed != request_origin {
        return Err(SecurityError::CorsBlocked(format!(
            "origin '{}' not allowed (server allows '{}')",
            request_origin, allowed,
        )));
    }

    Ok(())
}

fn validate_allow_methods(method: &str, headers: &HashMap<String, String>) -> SecurityResult<()> {
    let allowed = get_header(headers, "access-control-allow-methods").unwrap_or_default();
    let methods: Vec<&str> = allowed.split(',').map(str::trim).collect();

    if methods.iter().any(|m| m.eq_ignore_ascii_case(method)) || method == "*" {
        Ok(())
    } else {
        Err(SecurityError::CorsBlocked(format!(
            "method '{}' not in Access-Control-Allow-Methods ({})",
            method, allowed,
        )))
    }
}

fn validate_allow_headers(
    request_headers: &[String],
    response_headers: &HashMap<String, String>,
) -> SecurityResult<()> {
    let non_simple: Vec<&String> = request_headers
        .iter()
        .filter(|h| !SIMPLE_HEADERS.contains(&h.to_lowercase().as_str()))
        .collect();

    if non_simple.is_empty() {
        return Ok(());
    }

    let allowed_raw =
        get_header(response_headers, "access-control-allow-headers").unwrap_or_default();
    let allowed: HashSet<String> = allowed_raw
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();

    for h in &non_simple {
        if !allowed.contains(&h.to_lowercase()) {
            return Err(SecurityError::CorsBlocked(format!(
                "header '{}' not in Access-Control-Allow-Headers",
                h,
            )));
        }
    }
    Ok(())
}

/// Case-insensitive header lookup.
fn get_header<'a>(headers: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    let lower = name.to_lowercase();
    for (k, v) in headers {
        if k.to_lowercase() == lower {
            return Some(v.as_str());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn simple_get_no_preflight() {
        let req = CorsRequest {
            origin: "https://example.com".into(),
            method: "GET".into(),
            headers: vec![],
            with_credentials: false,
        };
        assert_eq!(classify_cors(&req), CorsMode::Simple);
    }

    #[test]
    fn put_needs_preflight() {
        let req = CorsRequest {
            origin: "https://example.com".into(),
            method: "PUT".into(),
            headers: vec![],
            with_credentials: false,
        };
        assert_eq!(classify_cors(&req), CorsMode::Preflight);
    }

    #[test]
    fn custom_header_needs_preflight() {
        let req = CorsRequest {
            origin: "https://example.com".into(),
            method: "GET".into(),
            headers: vec!["X-Custom-Token".into()],
            with_credentials: false,
        };
        assert_eq!(classify_cors(&req), CorsMode::Preflight);
    }

    #[test]
    fn preflight_response_valid() {
        let req = CorsRequest {
            origin: "https://example.com".into(),
            method: "PUT".into(),
            headers: vec!["X-Token".into()],
            with_credentials: false,
        };
        let resp = headers(&[
            ("Access-Control-Allow-Origin", "https://example.com"),
            ("Access-Control-Allow-Methods", "PUT, DELETE"),
            ("Access-Control-Allow-Headers", "x-token, x-other"),
        ]);
        assert!(validate_preflight(&req, &resp).is_ok());
    }

    #[test]
    fn preflight_missing_origin_rejected() {
        let req = CorsRequest {
            origin: "https://example.com".into(),
            method: "PUT".into(),
            headers: vec![],
            with_credentials: false,
        };
        let resp = headers(&[("Access-Control-Allow-Methods", "PUT")]);
        assert!(validate_preflight(&req, &resp).is_err());
    }

    #[test]
    fn wildcard_with_credentials_rejected() {
        let resp = headers(&[("Access-Control-Allow-Origin", "*")]);
        let err = validate_response("https://a.com", true, &resp);
        assert!(err.is_err());
    }

    #[test]
    fn expose_headers_includes_defaults_plus_custom() {
        let resp = headers(&[("Access-Control-Expose-Headers", "X-Request-Id, X-Trace")]);
        let exposed = exposed_headers(&resp);
        assert!(exposed.contains("content-type"));
        assert!(exposed.contains("x-request-id"));
        assert!(exposed.contains("x-trace"));
        assert!(!exposed.contains("authorization"));
    }
}
