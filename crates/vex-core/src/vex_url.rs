// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! URL wrapper with convenience accessors.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{VexError, VexResult};

/// Thin wrapper around [`url::Url`] with engine-specific helpers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VexUrl(url::Url);

impl VexUrl {
    pub fn parse(input: &str) -> VexResult<Self> {
        url::Url::parse(input).map(Self).map_err(VexError::from)
    }

    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    pub fn host(&self) -> Option<&str> {
        self.0.host_str()
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    pub fn origin(&self) -> String {
        self.0.origin().ascii_serialization()
    }

    pub fn is_https(&self) -> bool {
        self.0.scheme() == "https"
    }

    pub fn join(&self, relative: &str) -> VexResult<Self> {
        self.0.join(relative).map(Self).map_err(VexError::from)
    }

    pub fn query_pairs(&self) -> impl Iterator<Item = (std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>)> {
        self.0.query_pairs()
    }

    /// Access the inner `url::Url`.
    pub fn inner(&self) -> &url::Url {
        &self.0
    }
}

impl fmt::Display for VexUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for VexUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_url() {
        let u = VexUrl::parse("https://example.com/path?q=1").unwrap();
        assert_eq!(u.scheme(), "https");
        assert_eq!(u.host(), Some("example.com"));
        assert_eq!(u.path(), "/path");
    }

    #[test]
    fn invalid_url() {
        assert!(VexUrl::parse("://nope").is_err());
    }

    #[test]
    fn origin() {
        let u = VexUrl::parse("https://example.com:8080/foo").unwrap();
        assert_eq!(u.origin(), "https://example.com:8080");
    }

    #[test]
    fn is_https() {
        assert!(VexUrl::parse("https://x.com").unwrap().is_https());
        assert!(!VexUrl::parse("http://x.com").unwrap().is_https());
    }

    #[test]
    fn join_relative() {
        let base = VexUrl::parse("https://example.com/a/b").unwrap();
        let joined = base.join("../c").unwrap();
        assert_eq!(joined.path(), "/c");
    }

    #[test]
    fn query_pairs() {
        let u = VexUrl::parse("https://x.com?a=1&b=2").unwrap();
        let pairs: Vec<_> = u.query_pairs().collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn display() {
        let u = VexUrl::parse("https://example.com").unwrap();
        assert_eq!(u.to_string(), "https://example.com/");
    }
}
