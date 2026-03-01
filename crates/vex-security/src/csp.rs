// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Content Security Policy (CSP) Level 2 parser and enforcement.
//!
//! Parses `Content-Security-Policy` header values into a structured
//! [`CspPolicy`] and checks resource loads against the active directives.
//!
//! Supported directives: `default-src`, `script-src`, `style-src`,
//! `img-src`, `connect-src`, `font-src`, `frame-src`, `media-src`.
//!
//! Supported source values: `'self'`, `'none'`, `'unsafe-inline'`,
//! `'unsafe-eval'`, `https:`, `data:`, `blob:`, host sources, nonces,
//! and hashes.

use std::collections::HashMap;

use vex_core::VexUrl;

use crate::error::{SecurityError, SecurityResult};

// ── Source values ──────────────────────────────────────────────────────

/// A single CSP source expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CspSource {
    /// `'self'` — same origin as the document.
    SelF,
    /// `'none'` — nothing is allowed.
    None,
    /// `'unsafe-inline'` — allow inline `<script>` / `<style>`.
    UnsafeInline,
    /// `'unsafe-eval'` — allow `eval()` and friends.
    UnsafeEval,
    /// A scheme (e.g. `https:`, `data:`, `blob:`).
    Scheme(String),
    /// A host source (e.g. `example.com`, `*.example.com`, `https://cdn.example.com`).
    Host(HostSource),
    /// A nonce (`'nonce-<base64>'`).
    Nonce(String),
    /// A hash (`'sha256-<base64>'`, `'sha384-…'`, `'sha512-…'`).
    Hash { algo: String, value: String },
}

/// A host-based source expression, supporting optional scheme, wildcards and port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSource {
    pub scheme: Option<String>,
    pub host: String,
    pub port: Option<String>,
}

// ── Directives ─────────────────────────────────────────────────────────

/// The known CSP directive names we handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Directive {
    DefaultSrc,
    ScriptSrc,
    StyleSrc,
    ImgSrc,
    ConnectSrc,
    FontSrc,
    FrameSrc,
    MediaSrc,
}

impl Directive {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "default-src" => Some(Self::DefaultSrc),
            "script-src" => Some(Self::ScriptSrc),
            "style-src" => Some(Self::StyleSrc),
            "img-src" => Some(Self::ImgSrc),
            "connect-src" => Some(Self::ConnectSrc),
            "font-src" => Some(Self::FontSrc),
            "frame-src" => Some(Self::FrameSrc),
            "media-src" => Some(Self::MediaSrc),
            _ => None,
        }
    }
}

// ── Resource type mapping ──────────────────────────────────────────────

/// The type of resource being loaded — maps to a CSP directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Script,
    Style,
    Image,
    Connect,
    Font,
    Frame,
    Media,
}

impl ResourceType {
    /// Which directive governs this resource type.
    fn directive(self) -> Directive {
        match self {
            Self::Script => Directive::ScriptSrc,
            Self::Style => Directive::StyleSrc,
            Self::Image => Directive::ImgSrc,
            Self::Connect => Directive::ConnectSrc,
            Self::Font => Directive::FontSrc,
            Self::Frame => Directive::FrameSrc,
            Self::Media => Directive::MediaSrc,
        }
    }
}

// ── Policy ─────────────────────────────────────────────────────────────

/// A parsed Content Security Policy.
#[derive(Debug, Clone, Default)]
pub struct CspPolicy {
    /// Directive name → list of allowed sources.
    pub directives: HashMap<Directive, Vec<CspSource>>,
}

impl CspPolicy {
    /// Parse a CSP header value.
    ///
    /// Directives are `;`-separated. Each directive is a space-separated
    /// list of source values preceded by the directive name.
    ///
    /// Unknown directives are silently ignored (per the spec).
    pub fn parse(header: &str) -> Self {
        let mut directives = HashMap::new();

        for raw_directive in header.split(';') {
            let tokens: Vec<&str> = raw_directive.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }
            let name = tokens[0];
            if let Some(directive) = Directive::from_str(name) {
                let sources: Vec<CspSource> =
                    tokens[1..].iter().filter_map(|t| parse_source(t)).collect();
                directives.insert(directive, sources);
            }
            // Unknown directives silently ignored per spec.
        }

        Self { directives }
    }

    /// Get the effective source list for a directive, falling back to
    /// `default-src` if the specific directive is absent.
    pub fn sources_for(&self, directive: Directive) -> Option<&[CspSource]> {
        self.directives
            .get(&directive)
            .or_else(|| self.directives.get(&Directive::DefaultSrc))
            .map(Vec::as_slice)
    }

    /// Check whether a resource URL is allowed by this policy.
    ///
    /// `page_origin` is the serialised origin of the document (for `'self'`).
    pub fn allows_url(
        &self,
        resource_type: ResourceType,
        resource_url: &VexUrl,
        page_origin: &str,
    ) -> bool {
        let directive = resource_type.directive();
        let sources = match self.sources_for(directive) {
            Some(s) => s,
            None => return true, // No directive — allow by default.
        };

        sources.iter().any(|src| match src {
            CspSource::None => false,
            CspSource::SelF => {
                let res_origin = resource_url.origin();
                res_origin == page_origin
            }
            CspSource::UnsafeInline | CspSource::UnsafeEval => false, // URL check only
            CspSource::Scheme(scheme) => resource_url.scheme() == scheme.as_str(),
            CspSource::Host(host_src) => host_matches(host_src, resource_url),
            CspSource::Nonce(_) | CspSource::Hash { .. } => false, // URL check only
        })
    }

    /// Check whether inline content (`<script>` or `<style>`) is permitted.
    ///
    /// Returns `true` if `'unsafe-inline'` is present, or if a matching
    /// nonce or hash is found.
    pub fn allows_inline(
        &self,
        resource_type: ResourceType,
        nonce: Option<&str>,
        hash: Option<(&str, &str)>,
    ) -> bool {
        let directive = resource_type.directive();
        let sources = match self.sources_for(directive) {
            Some(s) => s,
            None => return true,
        };

        sources.iter().any(|src| match src {
            CspSource::UnsafeInline => true,
            CspSource::Nonce(n) => nonce.is_some_and(|req_n| req_n == n),
            CspSource::Hash { algo, value } => {
                hash.is_some_and(|(req_algo, req_val)| algo == req_algo && value == req_val)
            }
            _ => false,
        })
    }

    /// Enforce a URL load — returns `Ok(())` or a CSP violation error.
    pub fn enforce_url(
        &self,
        resource_type: ResourceType,
        resource_url: &VexUrl,
        page_origin: &str,
    ) -> SecurityResult<()> {
        if self.allows_url(resource_type, resource_url, page_origin) {
            Ok(())
        } else {
            Err(SecurityError::CspViolation(format!(
                "blocked {} load from {} (violates {:?})",
                resource_type_name(resource_type),
                resource_url,
                resource_type.directive(),
            )))
        }
    }

    /// Enforce inline content — returns `Ok(())` or a CSP violation error.
    pub fn enforce_inline(
        &self,
        resource_type: ResourceType,
        nonce: Option<&str>,
        hash: Option<(&str, &str)>,
    ) -> SecurityResult<()> {
        if self.allows_inline(resource_type, nonce, hash) {
            Ok(())
        } else {
            Err(SecurityError::CspViolation(format!(
                "blocked inline {} (violates {:?})",
                resource_type_name(resource_type),
                resource_type.directive(),
            )))
        }
    }
}

// ── Parsing helpers ────────────────────────────────────────────────────

fn parse_source(token: &str) -> Option<CspSource> {
    let lower = token.to_lowercase();
    match lower.as_str() {
        "'self'" => Some(CspSource::SelF),
        "'none'" => Some(CspSource::None),
        "'unsafe-inline'" => Some(CspSource::UnsafeInline),
        "'unsafe-eval'" => Some(CspSource::UnsafeEval),
        _ if lower.starts_with("'nonce-") && lower.ends_with('\'') => {
            let nonce = &token[7..token.len() - 1];
            Some(CspSource::Nonce(nonce.to_owned()))
        }
        _ if (lower.starts_with("'sha256-")
            || lower.starts_with("'sha384-")
            || lower.starts_with("'sha512-"))
            && lower.ends_with('\'') =>
        {
            let inner = &token[1..token.len() - 1]; // strip quotes
            let dash = inner.find('-')?;
            let algo = &inner[..dash];
            let value = &inner[dash + 1..];
            Some(CspSource::Hash {
                algo: algo.to_owned(),
                value: value.to_owned(),
            })
        }
        _ if lower.ends_with(':') => {
            // Scheme source (https:, data:, etc.)
            Some(CspSource::Scheme(lower[..lower.len() - 1].to_owned()))
        }
        _ => {
            // Host source: optional scheme + host + optional port
            Some(CspSource::Host(parse_host_source(token)))
        }
    }
}

fn parse_host_source(token: &str) -> HostSource {
    let (scheme, rest) = if let Some(idx) = token.find("://") {
        (Some(token[..idx].to_lowercase()), &token[idx + 3..])
    } else {
        (None, token)
    };

    let (host, port) = if let Some(colon) = rest.rfind(':') {
        let maybe_port = &rest[colon + 1..];
        if maybe_port.chars().all(|c| c.is_ascii_digit()) {
            (rest[..colon].to_lowercase(), Some(maybe_port.to_owned()))
        } else {
            (rest.to_lowercase(), None)
        }
    } else {
        (rest.to_lowercase(), None)
    };

    HostSource { scheme, host, port }
}

fn host_matches(src: &HostSource, url: &VexUrl) -> bool {
    // Scheme check.
    if let Some(ref s) = src.scheme {
        if url.scheme() != s.as_str() {
            return false;
        }
    }

    // Host check.
    let url_host = match url.host() {
        Some(h) => h,
        None => return false,
    };
    let pattern = &src.host;
    if pattern.starts_with("*.") {
        let suffix = &pattern[1..]; // ".example.com"
        if url_host != &pattern[2..] && !url_host.ends_with(suffix) {
            return false;
        }
    } else if url_host != pattern.as_str() {
        return false;
    }

    // Port check.
    if let Some(ref p) = src.port {
        let url_port = url
            .inner()
            .port()
            .map(|p| p.to_string())
            .unwrap_or_default();
        if &url_port != p {
            return false;
        }
    }

    true
}

fn resource_type_name(rt: ResourceType) -> &'static str {
    match rt {
        ResourceType::Script => "script",
        ResourceType::Style => "style",
        ResourceType::Image => "image",
        ResourceType::Connect => "connect",
        ResourceType::Font => "font",
        ResourceType::Frame => "frame",
        ResourceType::Media => "media",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_policy() {
        let policy = CspPolicy::parse("default-src 'self'; script-src 'none'");
        assert_eq!(policy.directives.len(), 2);
        assert_eq!(
            policy.directives[&Directive::DefaultSrc],
            vec![CspSource::SelF]
        );
        assert_eq!(
            policy.directives[&Directive::ScriptSrc],
            vec![CspSource::None]
        );
    }

    #[test]
    fn parse_nonce_and_hash() {
        let policy =
            CspPolicy::parse("script-src 'nonce-abc123' 'sha256-abcdef' 'unsafe-inline'");
        let sources = &policy.directives[&Directive::ScriptSrc];
        assert_eq!(sources.len(), 3);
        assert!(matches!(&sources[0], CspSource::Nonce(n) if n == "abc123"));
        assert!(matches!(&sources[1], CspSource::Hash { algo, value } if algo == "sha256" && value == "abcdef"));
        assert_eq!(sources[2], CspSource::UnsafeInline);
    }

    #[test]
    fn parse_host_and_scheme_sources() {
        let policy = CspPolicy::parse("img-src https: data: *.cdn.example.com cdn.other.com:8080");
        let sources = &policy.directives[&Directive::ImgSrc];
        assert_eq!(sources.len(), 4);
        assert!(matches!(&sources[0], CspSource::Scheme(s) if s == "https"));
        assert!(matches!(&sources[1], CspSource::Scheme(s) if s == "data"));
        assert!(matches!(&sources[2], CspSource::Host(h) if h.host == "*.cdn.example.com"));
        assert!(
            matches!(&sources[3], CspSource::Host(h) if h.host == "cdn.other.com" && h.port.as_deref() == Some("8080"))
        );
    }

    #[test]
    fn allows_self_url() {
        let policy = CspPolicy::parse("script-src 'self'");
        let url = VexUrl::parse("https://example.com/app.js").unwrap();
        assert!(policy.allows_url(ResourceType::Script, &url, "https://example.com"));
        assert!(!policy.allows_url(ResourceType::Script, &url, "https://other.com"));
    }

    #[test]
    fn allows_wildcard_host() {
        let policy = CspPolicy::parse("img-src *.example.com");
        let sub = VexUrl::parse("https://cdn.example.com/img.png").unwrap();
        let root = VexUrl::parse("https://example.com/img.png").unwrap();
        let other = VexUrl::parse("https://evil.com/img.png").unwrap();
        assert!(policy.allows_url(ResourceType::Image, &sub, "https://x.com"));
        assert!(policy.allows_url(ResourceType::Image, &root, "https://x.com"));
        assert!(!policy.allows_url(ResourceType::Image, &other, "https://x.com"));
    }

    #[test]
    fn fallback_to_default_src() {
        let policy = CspPolicy::parse("default-src 'self'");
        let url = VexUrl::parse("https://example.com/font.woff2").unwrap();
        assert!(policy.allows_url(ResourceType::Font, &url, "https://example.com"));

        let other = VexUrl::parse("https://other.com/font.woff2").unwrap();
        assert!(!policy.allows_url(ResourceType::Font, &other, "https://example.com"));
    }

    #[test]
    fn enforce_url_blocks_disallowed() {
        let policy = CspPolicy::parse("script-src 'none'");
        let url = VexUrl::parse("https://evil.com/hack.js").unwrap();
        let err = policy
            .enforce_url(ResourceType::Script, &url, "https://example.com")
            .unwrap_err();
        assert!(matches!(err, SecurityError::CspViolation(_)));
    }

    #[test]
    fn inline_allowed_with_unsafe_inline() {
        let policy = CspPolicy::parse("script-src 'unsafe-inline'");
        assert!(policy.allows_inline(ResourceType::Script, None, None));
    }

    #[test]
    fn inline_allowed_with_matching_nonce() {
        let policy = CspPolicy::parse("script-src 'nonce-abc123'");
        assert!(policy.allows_inline(ResourceType::Script, Some("abc123"), None));
        assert!(!policy.allows_inline(ResourceType::Script, Some("wrong"), None));
        assert!(!policy.allows_inline(ResourceType::Script, None, None));
    }

    #[test]
    fn inline_blocked_without_directive() {
        let policy = CspPolicy::parse("script-src 'self'");
        assert!(!policy.allows_inline(ResourceType::Script, None, None));
    }
}
