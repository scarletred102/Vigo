// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Security-aware fetch pipeline.
//!
//! Wraps [`vex_net::HttpClient`] with SOP classification, CORS handling,
//! and CSP enforcement. This is the integration point where the fully
//! tested but previously disconnected security modules become active.
//!
//! # Usage
//!
//! ```ignore
//! let ctx = SecurityContext::new(page_origin, csp_policy);
//! let response = secure_fetch(&client, &request, &ctx).await?;
//! ```

use std::collections::HashMap;

use tracing::{debug, warn};
use vex_core::{VexError, VexResult, VexUrl};
use vex_security::{
    classify_cors, classify_fetch, validate_response, CorsMode, CorsRequest, CspPolicy, FetchType,
    Origin, ResourceType,
};

/// Security context for the current page load.
///
/// Carries the page origin and active CSP so that every sub-resource
/// fetch can be checked without repeating header parsing.
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Origin of the page making the request.
    pub page_origin: Origin,
    /// Serialised page origin for CSP `'self'` and CORS header comparison.
    pub page_origin_str: String,
    /// Active Content Security Policy (empty = allow all).
    pub csp: CspPolicy,
}

impl SecurityContext {
    /// Create a new context from the page URL and an optional CSP header.
    pub fn new(page_url: &VexUrl, csp_header: Option<&str>) -> Self {
        let page_origin = Origin::from_url(page_url);
        let page_origin_str = page_origin.serialize();
        let csp = csp_header.map(CspPolicy::parse).unwrap_or_default();
        Self {
            page_origin,
            page_origin_str,
            csp,
        }
    }

    /// Create a permissive context (no CSP, opaque origin).
    ///
    /// Used for top-level navigations where security checks are relaxed.
    pub fn permissive() -> Self {
        Self {
            page_origin: Origin::Opaque,
            page_origin_str: String::new(),
            csp: CspPolicy::default(),
        }
    }
}

/// Perform a security-checked fetch.
///
/// 1. **CSP check** — blocks the request if the target URL violates the
///    active Content Security Policy.
/// 2. **SOP classification** — determines same-origin vs cross-origin.
/// 3. **CORS** — for cross-origin requests, classifies whether a
///    preflight is needed and validates the response headers.
///
/// The `resource_type` tells CSP which directive to check
/// (e.g. `ResourceType::Script` checks `script-src`).
pub async fn secure_fetch(
    client: &vex_net::HttpClient,
    request: vex_net::Request,
    ctx: &SecurityContext,
    resource_type: ResourceType,
) -> VexResult<vex_net::Response> {
    // ── Step 1: CSP enforcement ──────────────────────────────────────
    if !ctx.csp.directives.is_empty()
        && !ctx
            .csp
            .allows_url(resource_type, &request.url, &ctx.page_origin_str)
    {
        warn!(
            url = %request.url,
            origin = %ctx.page_origin_str,
            "CSP blocked resource load"
        );
        return Err(VexError::Network(format!(
            "CSP violation: {} blocked by content security policy",
            request.url,
        )));
    }

    // ── Step 2: SOP classification ───────────────────────────────────
    let fetch_type = classify_fetch(&ctx.page_origin, &request.url);
    debug!(url = %request.url, ?fetch_type, "security classification");

    // ── Step 3: Fetch (with CORS handling for cross-origin) ──────────
    let response = client.fetch(request.clone()).await?;

    if fetch_type == FetchType::CrossOrigin {
        // Build a CORS request descriptor from what we sent
        let cors_req = CorsRequest {
            origin: ctx.page_origin_str.clone(),
            method: request.method.to_http().to_string(),
            headers: request.headers.keys().map(|k| k.to_lowercase()).collect(),
            with_credentials: request.headers.contains_key("cookie")
                || request.headers.contains_key("authorization"),
        };

        let cors_mode = classify_cors(&cors_req);
        debug!(?cors_mode, "CORS classification");

        // For preflight requests we'd need to send an OPTIONS first.
        // Log the classification — full preflight implementation requires
        // another round-trip which we defer until integration testing.
        if cors_mode == CorsMode::Preflight {
            debug!(url = %request.url, "CORS preflight would be required");
        }

        // Validate the actual response CORS headers
        if let Err(e) = validate_response(
            &ctx.page_origin_str,
            cors_req.with_credentials,
            &response.headers,
        ) {
            warn!(url = %request.url, "CORS validation failed: {e}");
            return Err(VexError::Network(format!("CORS error: {e}")));
        }
    }

    Ok(response)
}

/// Extract the CSP header value from response headers, if present.
pub fn extract_csp(headers: &HashMap<String, String>) -> Option<&str> {
    headers.get("content-security-policy").map(|s| s.as_str())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_context_from_url() {
        let url = VexUrl::parse("https://example.com/page").unwrap();
        let ctx = SecurityContext::new(&url, None);
        assert_eq!(ctx.page_origin_str, "https://example.com");
        assert!(ctx.csp.directives.is_empty());
    }

    #[test]
    fn security_context_with_csp() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let ctx = SecurityContext::new(&url, Some("script-src 'self'; img-src *"));
        assert!(!ctx.csp.directives.is_empty());
    }

    #[test]
    fn csp_blocks_disallowed_url() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let ctx = SecurityContext::new(&url, Some("script-src 'self'"));

        // Same-origin script: allowed
        let same = VexUrl::parse("https://example.com/app.js").unwrap();
        assert!(ctx
            .csp
            .allows_url(ResourceType::Script, &same, &ctx.page_origin_str));

        // Cross-origin script: blocked
        let cross = VexUrl::parse("https://evil.com/hack.js").unwrap();
        assert!(!ctx
            .csp
            .allows_url(ResourceType::Script, &cross, &ctx.page_origin_str));
    }

    #[test]
    fn permissive_context_has_empty_csp() {
        let ctx = SecurityContext::permissive();
        assert!(ctx.csp.directives.is_empty());
    }

    #[test]
    fn extract_csp_from_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-security-policy".to_string(),
            "default-src 'self'".to_string(),
        );
        assert_eq!(extract_csp(&headers), Some("default-src 'self'"));

        let empty: HashMap<String, String> = HashMap::new();
        assert_eq!(extract_csp(&empty), None);
    }

    #[test]
    fn cross_origin_classified_correctly() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let ctx = SecurityContext::new(&url, None);

        let same = VexUrl::parse("https://example.com/api").unwrap();
        assert_eq!(
            classify_fetch(&ctx.page_origin, &same),
            FetchType::SameOrigin
        );

        let cross = VexUrl::parse("https://api.other.com/data").unwrap();
        assert_eq!(
            classify_fetch(&ctx.page_origin, &cross),
            FetchType::CrossOrigin
        );
    }
}
