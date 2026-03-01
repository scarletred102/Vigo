// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Content script injection — URL pattern matching and injection timing.
//!
//! When a page loads, content scripts whose match patterns match the
//! page URL are injected into an isolated JavaScript world.

use std::path::PathBuf;

/// When to inject a content script relative to page load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InjectionTime {
    /// Before any page script runs.
    DocumentStart,
    /// After DOM is ready (DOMContentLoaded).
    DocumentEnd,
    /// After page load (default).
    DocumentIdle,
}

impl InjectionTime {
    /// Parse from manifest string.
    pub fn parse_str(s: &str) -> Self {
        match s {
            "document_start" => Self::DocumentStart,
            "document_end" => Self::DocumentEnd,
            _ => Self::DocumentIdle,
        }
    }

    /// Manifest key string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentStart => "document_start",
            Self::DocumentEnd => "document_end",
            Self::DocumentIdle => "document_idle",
        }
    }
}

/// Content script definition from the manifest.
#[derive(Debug, Clone)]
pub struct ContentScriptDef {
    /// URL match patterns.
    pub matches: Vec<String>,
    /// JS files to inject (paths relative to extension root).
    pub js_files: Vec<PathBuf>,
    /// When to inject.
    pub run_at: InjectionTime,
}

/// A resolved content script ready for injection.
#[derive(Debug, Clone)]
pub struct ContentScript {
    /// Extension ID that owns this script.
    pub extension_id: String,
    /// Compiled match patterns.
    pub patterns: Vec<MatchPattern>,
    /// JavaScript source code to inject.
    pub source: String,
    /// When to inject.
    pub run_at: InjectionTime,
}

impl ContentScript {
    /// Check if this script should be injected into the given URL.
    pub fn matches_url(&self, url: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(url))
    }
}

/// A compiled URL match pattern.
///
/// Supports the format: `<scheme>://<host>/<path>`
/// Special: `<all_urls>` matches everything.
#[derive(Debug, Clone)]
pub struct MatchPattern {
    /// The raw pattern string.
    pub raw: String,
    /// Whether this is the `<all_urls>` wildcard.
    pub all_urls: bool,
    /// Scheme part ("*", "http", "https").
    pub scheme: String,
    /// Host part (may start with "*." for subdomain match).
    pub host: String,
    /// Path part (may contain "*" wildcards).
    pub path: String,
}

impl MatchPattern {
    /// Parse a match pattern string.
    ///
    /// Returns `None` if the pattern is malformed.
    pub fn parse(pattern: &str) -> Option<Self> {
        if pattern == "<all_urls>" {
            return Some(Self {
                raw: pattern.to_owned(),
                all_urls: true,
                scheme: String::new(),
                host: String::new(),
                path: String::new(),
            });
        }

        // Split scheme from rest.
        let (scheme, rest) = pattern.split_once("://")?;
        // Split host from path.
        let (host, path) = match rest.find('/') {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, "/*"),
        };

        Some(Self {
            raw: pattern.to_owned(),
            all_urls: false,
            scheme: scheme.to_owned(),
            host: host.to_owned(),
            path: path.to_owned(),
        })
    }

    /// Check if a URL matches this pattern.
    pub fn matches(&self, url: &str) -> bool {
        if self.all_urls {
            return url.starts_with("http://") || url.starts_with("https://");
        }

        // Extract scheme from URL.
        let Some((url_scheme, rest)) = url.split_once("://") else {
            return false;
        };

        // Check scheme. Wildcard "*" matches http/https only (per extension spec).
        if self.scheme == "*" {
            if url_scheme != "http" && url_scheme != "https" {
                return false;
            }
        } else if self.scheme != url_scheme {
            return false;
        }

        // Extract host and path from URL.
        let (url_host, url_path) = match rest.find('/') {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, "/"),
        };

        // Check host.
        if !self.host_matches(url_host) {
            return false;
        }

        // Check path.
        self.path_matches(url_path)
    }

    /// Check if a URL host matches the pattern host.
    fn host_matches(&self, url_host: &str) -> bool {
        if self.host == "*" {
            return true;
        }
        if let Some(suffix) = self.host.strip_prefix("*.") {
            // Subdomain wildcard: matches the suffix itself or any subdomain.
            url_host == suffix || url_host.ends_with(&format!(".{suffix}"))
        } else {
            url_host == self.host
        }
    }

    /// Check if a URL path matches the pattern path.
    fn path_matches(&self, url_path: &str) -> bool {
        if self.path == "/*" || self.path == "*" {
            return true;
        }
        // Simple wildcard matching: split on "*" and check each segment.
        let segments: Vec<&str> = self.path.split('*').collect();
        let mut remaining = url_path;
        for (i, segment) in segments.iter().enumerate() {
            if segment.is_empty() {
                continue;
            }
            match remaining.find(segment) {
                Some(pos) => {
                    if i == 0 && pos != 0 {
                        return false; // First segment must match from start.
                    }
                    remaining = &remaining[pos + segment.len()..];
                }
                None => return false,
            }
        }
        true
    }
}

/// Collect all content scripts from loaded extensions that match a URL,
/// grouped by injection time.
pub fn scripts_for_url<'a>(scripts: &'a [ContentScript], url: &str) -> Vec<&'a ContentScript> {
    scripts.iter().filter(|s| s.matches_url(url)).collect()
}

/// Sort scripts by injection time (start → end → idle).
pub fn sort_by_injection_time(scripts: &mut [&ContentScript]) {
    scripts.sort_by_key(|s| match s.run_at {
        InjectionTime::DocumentStart => 0,
        InjectionTime::DocumentEnd => 1,
        InjectionTime::DocumentIdle => 2,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── InjectionTime ──

    #[test]
    fn injection_time_from_str() {
        assert_eq!(
            InjectionTime::parse_str("document_start"),
            InjectionTime::DocumentStart
        );
        assert_eq!(
            InjectionTime::parse_str("document_end"),
            InjectionTime::DocumentEnd
        );
        assert_eq!(
            InjectionTime::parse_str("document_idle"),
            InjectionTime::DocumentIdle
        );
        assert_eq!(
            InjectionTime::parse_str("unknown"),
            InjectionTime::DocumentIdle
        );
    }

    #[test]
    fn injection_time_roundtrip() {
        let times = [
            InjectionTime::DocumentStart,
            InjectionTime::DocumentEnd,
            InjectionTime::DocumentIdle,
        ];
        for t in times {
            assert_eq!(InjectionTime::parse_str(t.as_str()), t);
        }
    }

    // ── MatchPattern ──

    #[test]
    fn parse_all_urls() {
        let pattern = MatchPattern::parse("<all_urls>").unwrap();
        assert!(pattern.all_urls);
        assert!(pattern.matches("https://example.com/"));
        assert!(pattern.matches("http://anything.test/path"));
        assert!(!pattern.matches("ftp://example.com/"));
    }

    #[test]
    fn parse_specific_pattern() {
        let pattern = MatchPattern::parse("https://example.com/path/*").unwrap();
        assert!(!pattern.all_urls);
        assert_eq!(pattern.scheme, "https");
        assert_eq!(pattern.host, "example.com");
        assert_eq!(pattern.path, "/path/*");
    }

    #[test]
    fn wildcard_scheme() {
        let pattern = MatchPattern::parse("*://example.com/*").unwrap();
        assert!(pattern.matches("http://example.com/"));
        assert!(pattern.matches("https://example.com/page"));
    }

    #[test]
    fn subdomain_wildcard() {
        let pattern = MatchPattern::parse("*://*.example.com/*").unwrap();
        assert!(pattern.matches("https://www.example.com/"));
        assert!(pattern.matches("https://sub.example.com/page"));
        assert!(pattern.matches("https://example.com/"));
        assert!(!pattern.matches("https://notexample.com/"));
    }

    #[test]
    fn exact_host() {
        let pattern = MatchPattern::parse("https://specific.com/*").unwrap();
        assert!(pattern.matches("https://specific.com/"));
        assert!(pattern.matches("https://specific.com/path/to/page"));
        assert!(!pattern.matches("https://sub.specific.com/"));
        assert!(!pattern.matches("http://specific.com/"));
    }

    #[test]
    fn path_matching() {
        let pattern = MatchPattern::parse("https://example.com/api/*").unwrap();
        assert!(pattern.matches("https://example.com/api/users"));
        assert!(pattern.matches("https://example.com/api/"));
        assert!(!pattern.matches("https://example.com/other"));
    }

    #[test]
    fn parse_invalid_pattern() {
        assert!(MatchPattern::parse("not-a-pattern").is_none());
    }

    // ── ContentScript ──

    #[test]
    fn content_script_matches() {
        let script = ContentScript {
            extension_id: "test".to_owned(),
            patterns: vec![
                MatchPattern::parse("*://*.example.com/*").unwrap(),
                MatchPattern::parse("https://other.test/*").unwrap(),
            ],
            source: "console.log('injected')".to_owned(),
            run_at: InjectionTime::DocumentIdle,
        };
        assert!(script.matches_url("https://www.example.com/page"));
        assert!(script.matches_url("https://other.test/api"));
        assert!(!script.matches_url("https://unrelated.com/"));
    }

    // ── scripts_for_url ──

    #[test]
    fn filter_scripts_for_url() {
        let scripts = vec![
            ContentScript {
                extension_id: "ext1".to_owned(),
                patterns: vec![MatchPattern::parse("*://*.example.com/*").unwrap()],
                source: "// ext1".to_owned(),
                run_at: InjectionTime::DocumentIdle,
            },
            ContentScript {
                extension_id: "ext2".to_owned(),
                patterns: vec![MatchPattern::parse("https://other.test/*").unwrap()],
                source: "// ext2".to_owned(),
                run_at: InjectionTime::DocumentStart,
            },
        ];
        let matching = scripts_for_url(&scripts, "https://www.example.com/page");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].extension_id, "ext1");
    }

    #[test]
    fn sort_scripts_by_time() {
        let scripts = vec![
            ContentScript {
                extension_id: "idle".to_owned(),
                patterns: vec![MatchPattern::parse("<all_urls>").unwrap()],
                source: String::new(),
                run_at: InjectionTime::DocumentIdle,
            },
            ContentScript {
                extension_id: "start".to_owned(),
                patterns: vec![MatchPattern::parse("<all_urls>").unwrap()],
                source: String::new(),
                run_at: InjectionTime::DocumentStart,
            },
            ContentScript {
                extension_id: "end".to_owned(),
                patterns: vec![MatchPattern::parse("<all_urls>").unwrap()],
                source: String::new(),
                run_at: InjectionTime::DocumentEnd,
            },
        ];
        let mut refs: Vec<&ContentScript> = scripts.iter().collect();
        sort_by_injection_time(&mut refs);
        assert_eq!(refs[0].run_at, InjectionTime::DocumentStart);
        assert_eq!(refs[1].run_at, InjectionTime::DocumentEnd);
        assert_eq!(refs[2].run_at, InjectionTime::DocumentIdle);
    }
}
