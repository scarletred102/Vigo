// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! URL API and URLSearchParams.
//!
//! Provides parsing and manipulation for URLs and their query
//! parameters, matching the WHATWG URL Standard.

use std::collections::BTreeMap;

// ── URLSearchParams ──────────────────────────────────────────────────────────

/// URL query string parameters with ordered key-value pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct UrlSearchParams {
    /// Ordered list of key-value pairs.
    params: Vec<(String, String)>,
}

impl Default for UrlSearchParams {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlSearchParams {
    pub fn new() -> Self {
        Self { params: Vec::new() }
    }

    /// Parse from a query string (with or without leading '?').
    pub fn parse(query: &str) -> Self {
        let query = query.strip_prefix('?').unwrap_or(query);
        let params = query
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|pair| {
                if let Some((k, v)) = pair.split_once('=') {
                    (decode_component(k), decode_component(v))
                } else {
                    (decode_component(pair), String::new())
                }
            })
            .collect();
        Self { params }
    }

    /// Append a key-value pair.
    pub fn append(&mut self, key: &str, value: &str) {
        self.params.push((key.to_string(), value.to_string()));
    }

    /// Delete all entries with the given key.
    pub fn delete(&mut self, key: &str) {
        self.params.retain(|(k, _)| k != key);
    }

    /// Get the first value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Get all values for a key.
    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.params
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Check if a key exists.
    pub fn has(&self, key: &str) -> bool {
        self.params.iter().any(|(k, _)| k == key)
    }

    /// Set a key to a value, replacing any existing entries with that key.
    pub fn set(&mut self, key: &str, value: &str) {
        let mut found = false;
        self.params.retain_mut(|(k, v)| {
            if k == key {
                if !found {
                    found = true;
                    *v = value.to_string();
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });
        if !found {
            self.params.push((key.to_string(), value.to_string()));
        }
    }

    /// Sort parameters by key (stable sort).
    pub fn sort(&mut self) {
        self.params.sort_by(|a, b| a.0.cmp(&b.0));
    }

    /// Number of parameters.
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Whether there are no parameters.
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Iterate over key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.params.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Serialize to a query string (without leading '?').
    pub fn to_string_without_prefix(&self) -> String {
        self.params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    encode_component(k)
                } else {
                    format!("{}={}", encode_component(k), encode_component(v))
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Get unique keys.
    pub fn keys(&self) -> Vec<&str> {
        let mut seen = BTreeMap::new();
        for (k, _) in &self.params {
            seen.entry(k.as_str()).or_insert(());
        }
        seen.keys().copied().collect()
    }
}

impl std::fmt::Display for UrlSearchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_without_prefix())
    }
}

// ── URL parts ────────────────────────────────────────────────────────────────

/// Parsed URL object (simplified WHATWG URL).
#[derive(Debug, Clone, PartialEq)]
pub struct WebUrl {
    pub scheme: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub search_params: UrlSearchParams,
    pub fragment: String,
}

impl WebUrl {
    /// Parse a URL string.
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        // Scheme
        let (scheme, rest) = input.split_once("://")?;
        let scheme = scheme.to_lowercase();

        // Fragment
        let (rest, fragment) = if let Some((r, f)) = rest.split_once('#') {
            (r, f.to_string())
        } else {
            (rest, String::new())
        };

        // Query
        let (rest, search) = if let Some((r, q)) = rest.split_once('?') {
            (r, UrlSearchParams::parse(q))
        } else {
            (rest, UrlSearchParams::new())
        };

        // Userinfo
        let (userinfo, host_path) = if let Some((u, hp)) = rest.split_once('@') {
            (u, hp)
        } else {
            ("", rest)
        };

        let (username, password) = if let Some((u, p)) = userinfo.split_once(':') {
            (u.to_string(), p.to_string())
        } else {
            (userinfo.to_string(), String::new())
        };

        // Host and path
        let (host_port, path) = if let Some((hp, p)) = host_path.split_once('/') {
            (hp, format!("/{p}"))
        } else {
            (host_path, "/".to_string())
        };

        // Port
        let (host, port) = if let Some((h, p)) = host_port.rsplit_once(':') {
            (h.to_string(), p.parse().ok())
        } else {
            (host_port.to_string(), None)
        };

        Some(Self {
            scheme,
            username,
            password,
            host,
            port,
            path,
            search_params: search,
            fragment,
        })
    }

    /// Produce the origin string.
    pub fn origin(&self) -> String {
        let port_str = self
            .port
            .map(|p| format!(":{p}"))
            .unwrap_or_default();
        format!("{}://{}{}", self.scheme, self.host, port_str)
    }

    /// Produce the href string (full URL).
    pub fn href(&self) -> String {
        let mut s = format!("{}://", self.scheme);

        if !self.username.is_empty() {
            s.push_str(&self.username);
            if !self.password.is_empty() {
                s.push(':');
                s.push_str(&self.password);
            }
            s.push('@');
        }

        s.push_str(&self.host);
        if let Some(port) = self.port {
            s.push(':');
            s.push_str(&port.to_string());
        }
        s.push_str(&self.path);

        let query = self.search_params.to_string_without_prefix();
        if !query.is_empty() {
            s.push('?');
            s.push_str(&query);
        }

        if !self.fragment.is_empty() {
            s.push('#');
            s.push_str(&self.fragment);
        }

        s
    }
}

impl std::fmt::Display for WebUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.href())
    }
}

// ── Encoding ─────────────────────────────────────────────────────────────────

fn decode_component(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'+' {
            result.push(' ');
        } else if b == b'%' {
            let h1 = chars.next().and_then(|c| (c as char).to_digit(16));
            let h2 = chars.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(hi), Some(lo)) = (h1, h2) {
                result.push((hi * 16 + lo) as u8 as char);
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn encode_component(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{b:02X}"));
            }
        }
    }
    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── URLSearchParams ─────────────────────────────────────────

    #[test]
    fn parse_search_params() {
        let p = UrlSearchParams::parse("a=1&b=2&c=3");
        assert_eq!(p.len(), 3);
        assert_eq!(p.get("a"), Some("1"));
        assert_eq!(p.get("b"), Some("2"));
    }

    #[test]
    fn parse_with_question_mark() {
        let p = UrlSearchParams::parse("?x=hello&y=world");
        assert_eq!(p.get("x"), Some("hello"));
    }

    #[test]
    fn append_and_get_all() {
        let mut p = UrlSearchParams::new();
        p.append("color", "red");
        p.append("color", "blue");
        let all = p.get_all("color");
        assert_eq!(all, vec!["red", "blue"]);
    }

    #[test]
    fn set_replaces() {
        let mut p = UrlSearchParams::parse("x=1&x=2&y=3");
        p.set("x", "99");
        assert_eq!(p.get_all("x"), vec!["99"]);
        assert_eq!(p.len(), 2); // x=99, y=3
    }

    #[test]
    fn delete_key() {
        let mut p = UrlSearchParams::parse("a=1&b=2&a=3");
        p.delete("a");
        assert_eq!(p.len(), 1);
        assert!(!p.has("a"));
    }

    #[test]
    fn sort_params() {
        let mut p = UrlSearchParams::parse("c=3&a=1&b=2");
        p.sort();
        let keys: Vec<&str> = p.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn to_string_roundtrip() {
        let p = UrlSearchParams::parse("key=value&foo=bar");
        let s = p.to_string();
        assert!(s.contains("key=value"));
        assert!(s.contains("foo=bar"));
    }

    #[test]
    fn decode_plus_as_space() {
        let p = UrlSearchParams::parse("q=hello+world");
        assert_eq!(p.get("q"), Some("hello world"));
    }

    #[test]
    fn decode_percent() {
        let p = UrlSearchParams::parse("name=%E4%B8%96%E7%95%8C");
        assert!(p.has("name"));
    }

    // ── WebUrl ──────────────────────────────────────────────────

    #[test]
    fn parse_simple_url() {
        let u = WebUrl::parse("https://example.com/path").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
        assert_eq!(u.path, "/path");
    }

    #[test]
    fn parse_url_with_port() {
        let u = WebUrl::parse("http://localhost:3000/api").unwrap();
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, Some(3000));
    }

    #[test]
    fn parse_url_with_query() {
        let u = WebUrl::parse("https://search.com/q?term=rust&lang=en").unwrap();
        assert_eq!(u.search_params.get("term"), Some("rust"));
        assert_eq!(u.search_params.get("lang"), Some("en"));
    }

    #[test]
    fn parse_url_with_fragment() {
        let u = WebUrl::parse("https://example.com/page#section").unwrap();
        assert_eq!(u.fragment, "section");
    }

    #[test]
    fn parse_url_with_userinfo() {
        let u = WebUrl::parse("ftp://user:pass@ftp.example.com/files").unwrap();
        assert_eq!(u.username, "user");
        assert_eq!(u.password, "pass");
        assert_eq!(u.host, "ftp.example.com");
    }

    #[test]
    fn origin() {
        let u = WebUrl::parse("https://example.com:8080/path").unwrap();
        assert_eq!(u.origin(), "https://example.com:8080");
    }

    #[test]
    fn href_roundtrip() {
        let input = "https://example.com/path";
        let u = WebUrl::parse(input).unwrap();
        assert_eq!(u.href(), input);
    }

    #[test]
    fn invalid_url() {
        assert!(WebUrl::parse("not a url").is_none());
    }

    #[test]
    fn keys_deduped() {
        let p = UrlSearchParams::parse("a=1&b=2&a=3");
        let keys = p.keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn empty_params() {
        let p = UrlSearchParams::new();
        assert!(p.is_empty());
        assert_eq!(p.to_string(), "");
    }
}
