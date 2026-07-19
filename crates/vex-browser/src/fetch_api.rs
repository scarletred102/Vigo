// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web Fetch API implementation.
//!
//! Provides the browser-level `Request`, `Response`, `Headers`, and `Body` types
//! conforming to the WHATWG Fetch specification:
//! <https://fetch.spec.whatwg.org/>

use std::collections::HashMap;

// ── RequestMethod ────────────────────────────────────────────────────────────

/// HTTP methods as used by the Fetch API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum RequestMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
    Custom(String),
}

impl RequestMethod {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Patch => "PATCH",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn from_name(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Self::Get,
            "HEAD" => Self::Head,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "DELETE" => Self::Delete,
            "CONNECT" => Self::Connect,
            "OPTIONS" => Self::Options,
            "TRACE" => Self::Trace,
            "PATCH" => Self::Patch,
            other => Self::Custom(other.to_string()),
        }
    }

    /// Safe methods do not alter server state.
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Options | Self::Trace)
    }
}

// ── RequestMode ──────────────────────────────────────────────────────────────

/// The mode of a fetch request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestMode {
    #[default]
    Cors,
    NoCors,
    SameOrigin,
    Navigate,
}

impl RequestMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cors => "cors",
            Self::NoCors => "no-cors",
            Self::SameOrigin => "same-origin",
            Self::Navigate => "navigate",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "cors" => Some(Self::Cors),
            "no-cors" => Some(Self::NoCors),
            "same-origin" => Some(Self::SameOrigin),
            "navigate" => Some(Self::Navigate),
            _ => None,
        }
    }
}

// ── RequestCredentials ───────────────────────────────────────────────────────

/// Credentials mode for a fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestCredentials {
    Omit,
    #[default]
    SameOrigin,
    Include,
}

impl RequestCredentials {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Omit => "omit",
            Self::SameOrigin => "same-origin",
            Self::Include => "include",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "omit" => Some(Self::Omit),
            "same-origin" => Some(Self::SameOrigin),
            "include" => Some(Self::Include),
            _ => None,
        }
    }
}

// ── RequestCache ─────────────────────────────────────────────────────────────

/// Cache mode for a fetch request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestCache {
    #[default]
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

impl RequestCache {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::NoStore => "no-store",
            Self::Reload => "reload",
            Self::NoCache => "no-cache",
            Self::ForceCache => "force-cache",
            Self::OnlyIfCached => "only-if-cached",
        }
    }
}

// ── RequestRedirect ──────────────────────────────────────────────────────────

/// How to handle redirects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestRedirect {
    #[default]
    Follow,
    Error,
    Manual,
}

impl RequestRedirect {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Follow => "follow",
            Self::Error => "error",
            Self::Manual => "manual",
        }
    }
}

// ── ReferrerPolicy ───────────────────────────────────────────────────────────

/// Referrer policy per the W3C spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    #[default]
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NoReferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::Origin => "origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::SameOrigin => "same-origin",
            Self::StrictOrigin => "strict-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "no-referrer" => Some(Self::NoReferrer),
            "no-referrer-when-downgrade" => Some(Self::NoReferrerWhenDowngrade),
            "origin" => Some(Self::Origin),
            "origin-when-cross-origin" => Some(Self::OriginWhenCrossOrigin),
            "same-origin" => Some(Self::SameOrigin),
            "strict-origin" => Some(Self::StrictOrigin),
            "strict-origin-when-cross-origin" | "" => Some(Self::StrictOriginWhenCrossOrigin),
            "unsafe-url" => Some(Self::UnsafeUrl),
            _ => None,
        }
    }
}

// ── Headers ──────────────────────────────────────────────────────────────────

/// Fetch API Headers object.
///
/// Headers are stored as a case-insensitive multi-map.
#[derive(Debug, Clone, Default)]
pub struct FetchHeaders {
    map: HashMap<String, Vec<String>>,
}

impl FetchHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a value to a header (allows multiple values per key).
    pub fn append(&mut self, name: &str, value: &str) {
        let key = name.to_ascii_lowercase();
        self.map.entry(key).or_default().push(value.to_string());
    }

    /// Set a header (replaces all existing values for that key).
    pub fn set(&mut self, name: &str, value: &str) {
        let key = name.to_ascii_lowercase();
        self.map.insert(key, vec![value.to_string()]);
    }

    /// Get the first value for a header.
    pub fn get(&self, name: &str) -> Option<&str> {
        let key = name.to_ascii_lowercase();
        self.map
            .get(&key)
            .and_then(|v| v.first().map(|s| s.as_str()))
    }

    /// Get all values for a header, joined by ", ".
    pub fn get_all(&self, name: &str) -> Option<String> {
        let key = name.to_ascii_lowercase();
        self.map.get(&key).map(|v| v.join(", "))
    }

    /// Check if a header exists.
    pub fn has(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        self.map.contains_key(&key)
    }

    /// Delete a header.
    pub fn delete(&mut self, name: &str) {
        let key = name.to_ascii_lowercase();
        self.map.remove(&key);
    }

    /// Iterate over all header entries (key, combined values).
    pub fn entries(&self) -> impl Iterator<Item = (&str, String)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.join(", ")))
    }

    /// Get the number of distinct header names.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Check if a header name is forbidden for setting via JS.
    pub fn is_forbidden_header(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "accept-charset"
                | "accept-encoding"
                | "access-control-request-headers"
                | "access-control-request-method"
                | "connection"
                | "content-length"
                | "cookie"
                | "cookie2"
                | "date"
                | "dnt"
                | "expect"
                | "host"
                | "keep-alive"
                | "origin"
                | "referer"
                | "te"
                | "trailer"
                | "transfer-encoding"
                | "upgrade"
                | "via"
        ) || lower.starts_with("proxy-")
            || lower.starts_with("sec-")
    }
}

// ── Body ─────────────────────────────────────────────────────────────────────

/// The body of a request or response.
#[derive(Debug, Clone, Default)]
pub enum BodyContent {
    /// No body.
    #[default]
    None,
    /// UTF-8 text body.
    Text(String),
    /// Binary body.
    Bytes(Vec<u8>),
    /// URL-encoded form data.
    FormData(HashMap<String, String>),
}

impl BodyContent {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Get the byte length of the body.
    pub fn byte_length(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Text(s) => s.len(),
            Self::Bytes(b) => b.len(),
            Self::FormData(map) => {
                // Approximate URL-encoded length
                map.iter()
                    .map(|(k, v)| k.len() + v.len() + 2)
                    .sum::<usize>()
            }
        }
    }

    /// Convert to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::None => Vec::new(),
            Self::Text(s) => s.as_bytes().to_vec(),
            Self::Bytes(b) => b.clone(),
            Self::FormData(map) => {
                let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{k}={v}")).collect();
                pairs.join("&").into_bytes()
            }
        }
    }

    /// Try to interpret body as text.
    pub fn as_text(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Text(s) => Some(s.clone()),
            Self::Bytes(b) => String::from_utf8(b.clone()).ok(),
            Self::FormData(map) => {
                let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{k}={v}")).collect();
                Some(pairs.join("&"))
            }
        }
    }
}

// ── FetchRequest ─────────────────────────────────────────────────────────────

/// A Fetch API Request object.
#[derive(Debug, Clone)]
pub struct FetchRequest {
    /// The URL of the request.
    pub url: String,
    /// HTTP method.
    pub method: RequestMethod,
    /// Headers.
    pub headers: FetchHeaders,
    /// Body content.
    pub body: BodyContent,
    /// Request mode.
    pub mode: RequestMode,
    /// Credentials mode.
    pub credentials: RequestCredentials,
    /// Cache mode.
    pub cache: RequestCache,
    /// Redirect mode.
    pub redirect: RequestRedirect,
    /// Referrer policy.
    pub referrer_policy: ReferrerPolicy,
    /// Integrity hash (SRI).
    pub integrity: String,
    /// Signal abort controller ID (optional).
    pub signal_id: Option<u64>,
    /// Whether keepalive is enabled.
    pub keepalive: bool,
}

impl FetchRequest {
    /// Create a simple GET request.
    pub fn get(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: RequestMethod::Get,
            headers: FetchHeaders::new(),
            body: BodyContent::None,
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
            redirect: RequestRedirect::Follow,
            referrer_policy: ReferrerPolicy::default(),
            integrity: String::new(),
            signal_id: None,
            keepalive: false,
        }
    }

    /// Create a POST request with text body.
    pub fn post(url: &str, body: &str) -> Self {
        let mut headers = FetchHeaders::new();
        headers.set("content-type", "text/plain;charset=UTF-8");
        Self {
            url: url.to_string(),
            method: RequestMethod::Post,
            headers,
            body: BodyContent::Text(body.to_string()),
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
            redirect: RequestRedirect::Follow,
            referrer_policy: ReferrerPolicy::default(),
            integrity: String::new(),
            signal_id: None,
            keepalive: false,
        }
    }

    /// Create a POST request with JSON body.
    pub fn post_json(url: &str, json: &str) -> Self {
        let mut headers = FetchHeaders::new();
        headers.set("content-type", "application/json");
        Self {
            url: url.to_string(),
            method: RequestMethod::Post,
            headers,
            body: BodyContent::Text(json.to_string()),
            mode: RequestMode::Cors,
            credentials: RequestCredentials::SameOrigin,
            cache: RequestCache::Default,
            redirect: RequestRedirect::Follow,
            referrer_policy: ReferrerPolicy::default(),
            integrity: String::new(),
            signal_id: None,
            keepalive: false,
        }
    }

    /// Whether this request has a body.
    pub fn has_body(&self) -> bool {
        !self.body.is_none()
    }

    /// Whether the method allows a body.
    pub fn method_allows_body(&self) -> bool {
        !matches!(self.method, RequestMethod::Get | RequestMethod::Head)
    }
}

// ── ResponseType ─────────────────────────────────────────────────────────────

/// The type of a Fetch API response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseType {
    #[default]
    Basic,
    Cors,
    Default,
    Error,
    Opaque,
    OpaqueRedirect,
}

impl ResponseType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Basic => "basic",
            Self::Cors => "cors",
            Self::Default => "default",
            Self::Error => "error",
            Self::Opaque => "opaque",
            Self::OpaqueRedirect => "opaqueredirect",
        }
    }
}

// ── FetchResponse ────────────────────────────────────────────────────────────

/// A Fetch API Response object.
#[derive(Debug, Clone)]
pub struct FetchResponse {
    /// The response type.
    pub response_type: ResponseType,
    /// The final URL (after redirects).
    pub url: String,
    /// Whether redirected.
    pub redirected: bool,
    /// HTTP status code.
    pub status: u16,
    /// HTTP status text.
    pub status_text: String,
    /// Response headers.
    pub headers: FetchHeaders,
    /// Response body.
    pub body: BodyContent,
    /// Whether the body has been consumed.
    pub body_used: bool,
}

impl FetchResponse {
    /// Create a successful response.
    pub fn ok(body: BodyContent) -> Self {
        Self {
            response_type: ResponseType::Basic,
            url: String::new(),
            redirected: false,
            status: 200,
            status_text: "OK".to_string(),
            headers: FetchHeaders::new(),
            body,
            body_used: false,
        }
    }

    /// Create an error response.
    pub fn error() -> Self {
        Self {
            response_type: ResponseType::Error,
            url: String::new(),
            redirected: false,
            status: 0,
            status_text: String::new(),
            headers: FetchHeaders::new(),
            body: BodyContent::None,
            body_used: false,
        }
    }

    /// Create a redirect response.
    pub fn redirect(url: &str, status: u16) -> Option<Self> {
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return None;
        }
        let mut headers = FetchHeaders::new();
        headers.set("location", url);
        Some(Self {
            response_type: ResponseType::Basic,
            url: url.to_string(),
            redirected: true,
            status,
            status_text: String::new(),
            headers,
            body: BodyContent::None,
            body_used: false,
        })
    }

    /// Whether the response is OK (status 200-299).
    pub fn ok_status(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Consume the body as text.
    pub fn text(&mut self) -> Option<String> {
        if self.body_used {
            return None;
        }
        self.body_used = true;
        self.body.as_text()
    }

    /// Consume the body as bytes.
    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        if self.body_used {
            return None;
        }
        self.body_used = true;
        Some(self.body.to_bytes())
    }

    /// Clone the response (body not consumed by cloning).
    pub fn clone_response(&self) -> Self {
        self.clone()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── RequestMethod tests ──

    #[test]
    fn request_method_from_name() {
        assert_eq!(RequestMethod::from_name("get"), RequestMethod::Get);
        assert_eq!(RequestMethod::from_name("POST"), RequestMethod::Post);
        assert_eq!(RequestMethod::from_name("PATCH"), RequestMethod::Patch);
        assert_eq!(
            RequestMethod::from_name("CUSTOM"),
            RequestMethod::Custom("CUSTOM".into())
        );
    }

    #[test]
    fn request_method_safe() {
        assert!(RequestMethod::Get.is_safe());
        assert!(RequestMethod::Head.is_safe());
        assert!(!RequestMethod::Post.is_safe());
        assert!(!RequestMethod::Delete.is_safe());
    }

    #[test]
    fn request_method_as_str() {
        assert_eq!(RequestMethod::Get.as_str(), "GET");
        assert_eq!(RequestMethod::Post.as_str(), "POST");
    }

    // ── Headers tests ──

    #[test]
    fn headers_basic() {
        let mut h = FetchHeaders::new();
        assert!(h.is_empty());
        h.set("Content-Type", "text/html");
        assert_eq!(h.get("content-type"), Some("text/html"));
        assert!(h.has("Content-Type"));
        assert!(!h.has("X-Custom"));
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn headers_append_multi() {
        let mut h = FetchHeaders::new();
        h.append("Accept", "text/html");
        h.append("Accept", "application/json");
        assert_eq!(h.get("accept"), Some("text/html"));
        assert_eq!(
            h.get_all("accept"),
            Some("text/html, application/json".to_string())
        );
    }

    #[test]
    fn headers_delete() {
        let mut h = FetchHeaders::new();
        h.set("X-Custom", "value");
        assert!(h.has("x-custom"));
        h.delete("x-custom");
        assert!(!h.has("x-custom"));
    }

    #[test]
    fn headers_case_insensitive() {
        let mut h = FetchHeaders::new();
        h.set("Content-Type", "text/html");
        assert_eq!(h.get("CONTENT-TYPE"), Some("text/html"));
        assert_eq!(h.get("content-type"), Some("text/html"));
    }

    #[test]
    fn headers_forbidden() {
        assert!(FetchHeaders::is_forbidden_header("cookie"));
        assert!(FetchHeaders::is_forbidden_header("Host"));
        assert!(FetchHeaders::is_forbidden_header("Sec-Fetch-Mode"));
        assert!(FetchHeaders::is_forbidden_header("Proxy-Authorization"));
        assert!(!FetchHeaders::is_forbidden_header("X-Custom"));
        assert!(!FetchHeaders::is_forbidden_header("Accept"));
    }

    // ── Body tests ──

    #[test]
    fn body_text() {
        let body = BodyContent::Text("hello".into());
        assert_eq!(body.byte_length(), 5);
        assert_eq!(body.as_text(), Some("hello".to_string()));
        assert_eq!(body.to_bytes(), b"hello");
        assert!(!body.is_none());
    }

    #[test]
    fn body_bytes() {
        let body = BodyContent::Bytes(vec![1, 2, 3]);
        assert_eq!(body.byte_length(), 3);
        assert_eq!(body.to_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn body_none() {
        let body = BodyContent::None;
        assert!(body.is_none());
        assert_eq!(body.byte_length(), 0);
        assert_eq!(body.as_text(), None);
    }

    #[test]
    fn body_form_data() {
        let mut map = HashMap::new();
        map.insert("key".into(), "value".into());
        let body = BodyContent::FormData(map);
        assert!(!body.is_none());
        let text = body.as_text().unwrap();
        assert!(text.contains("key=value"));
    }

    // ── FetchRequest tests ──

    #[test]
    fn request_get() {
        let req = FetchRequest::get("https://example.com");
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, RequestMethod::Get);
        assert!(!req.has_body());
        assert!(!req.method_allows_body());
    }

    #[test]
    fn request_post_text() {
        let req = FetchRequest::post("https://example.com", "data");
        assert_eq!(req.method, RequestMethod::Post);
        assert!(req.has_body());
        assert!(req.method_allows_body());
        assert_eq!(
            req.headers.get("content-type"),
            Some("text/plain;charset=UTF-8")
        );
    }

    #[test]
    fn request_post_json() {
        let req = FetchRequest::post_json("https://example.com", r#"{"a":1}"#);
        assert_eq!(req.headers.get("content-type"), Some("application/json"));
    }

    // ── FetchResponse tests ──

    #[test]
    fn response_ok() {
        let mut resp = FetchResponse::ok(BodyContent::Text("hello".into()));
        assert!(resp.ok_status());
        assert_eq!(resp.status, 200);
        assert!(!resp.body_used);
        let text = resp.text();
        assert_eq!(text, Some("hello".to_string()));
        assert!(resp.body_used);
        // Second consumption returns None
        assert_eq!(resp.text(), None);
    }

    #[test]
    fn response_error() {
        let resp = FetchResponse::error();
        assert_eq!(resp.response_type, ResponseType::Error);
        assert_eq!(resp.status, 0);
        assert!(!resp.ok_status());
    }

    #[test]
    fn response_redirect() {
        let resp = FetchResponse::redirect("https://example.com/new", 301).unwrap();
        assert_eq!(resp.status, 301);
        assert!(resp.redirected);
        assert_eq!(
            resp.headers.get("location"),
            Some("https://example.com/new")
        );
    }

    #[test]
    fn response_redirect_invalid_status() {
        assert!(FetchResponse::redirect("https://example.com", 200).is_none());
        assert!(FetchResponse::redirect("https://example.com", 404).is_none());
    }

    #[test]
    fn response_bytes() {
        let mut resp = FetchResponse::ok(BodyContent::Bytes(vec![0xDE, 0xAD]));
        let bytes = resp.bytes().unwrap();
        assert_eq!(bytes, vec![0xDE, 0xAD]);
        assert!(resp.body_used);
    }

    // ── Enum round-trips ──

    #[test]
    fn request_mode_roundtrip() {
        for mode in [
            RequestMode::Cors,
            RequestMode::NoCors,
            RequestMode::SameOrigin,
            RequestMode::Navigate,
        ] {
            assert_eq!(RequestMode::from_name(mode.as_str()), Some(mode));
        }
    }

    #[test]
    fn credentials_roundtrip() {
        for cred in [
            RequestCredentials::Omit,
            RequestCredentials::SameOrigin,
            RequestCredentials::Include,
        ] {
            assert_eq!(RequestCredentials::from_name(cred.as_str()), Some(cred));
        }
    }

    #[test]
    fn referrer_policy_roundtrip() {
        for p in [
            ReferrerPolicy::NoReferrer,
            ReferrerPolicy::NoReferrerWhenDowngrade,
            ReferrerPolicy::Origin,
            ReferrerPolicy::OriginWhenCrossOrigin,
            ReferrerPolicy::SameOrigin,
            ReferrerPolicy::StrictOrigin,
            ReferrerPolicy::StrictOriginWhenCrossOrigin,
            ReferrerPolicy::UnsafeUrl,
        ] {
            assert_eq!(ReferrerPolicy::from_name(p.as_str()), Some(p));
        }
    }

    #[test]
    fn response_type_as_str() {
        assert_eq!(ResponseType::Basic.as_str(), "basic");
        assert_eq!(ResponseType::Opaque.as_str(), "opaque");
        assert_eq!(ResponseType::Error.as_str(), "error");
    }
}
