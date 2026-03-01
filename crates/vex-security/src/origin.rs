// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web origin model per [RFC 6454](https://www.rfc-editor.org/rfc/rfc6454).
//!
//! An origin is either a *tuple* `(scheme, host, port)` for network schemes
//! (http, https, ftp, ws, wss) or *opaque* for everything else (`data:`,
//! `file:`, `about:`, `blob:` without a creator origin, etc.).

use std::fmt;

use vex_core::VexUrl;

/// A web origin — either a tuple or opaque.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A tuple origin with `(scheme, host, port)`.
    Tuple {
        scheme: String,
        host: String,
        port: u16,
    },
    /// An opaque origin that is never same-origin with anything (including itself).
    Opaque,
}

impl Origin {
    /// Derive an [`Origin`] from a [`VexUrl`].
    ///
    /// Network schemes (`http`, `https`, `ftp`, `ws`, `wss`) produce a
    /// [`Tuple`](Origin::Tuple) origin. All other schemes produce
    /// [`Opaque`](Origin::Opaque).
    pub fn from_url(url: &VexUrl) -> Self {
        let scheme = url.scheme();
        match scheme {
            "http" | "https" | "ftp" | "ws" | "wss" => {
                if let Some(host) = url.host() {
                    let port = url
                        .inner()
                        .port_or_known_default()
                        .unwrap_or(0);
                    Self::Tuple {
                        scheme: scheme.to_owned(),
                        host: host.to_owned(),
                        port,
                    }
                } else {
                    Self::Opaque
                }
            }
            _ => Self::Opaque,
        }
    }

    /// Returns `true` if two origins are the same.
    ///
    /// Two tuple origins are same-origin when their scheme, host, and port
    /// all match. Opaque origins are **never** same-origin — not even with
    /// themselves.
    pub fn same_origin(a: &Origin, b: &Origin) -> bool {
        match (a, b) {
            (
                Origin::Tuple {
                    scheme: s1,
                    host: h1,
                    port: p1,
                },
                Origin::Tuple {
                    scheme: s2,
                    host: h2,
                    port: p2,
                },
            ) => s1 == s2 && h1 == h2 && p1 == p2,
            _ => false,
        }
    }

    /// Returns `true` if this is an opaque origin.
    pub fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque)
    }

    /// Returns `true` if this is a tuple origin.
    pub fn is_tuple(&self) -> bool {
        matches!(self, Self::Tuple { .. })
    }

    /// Returns the ASCII serialisation of this origin.
    ///
    /// Tuple origins produce `"scheme://host:port"` (port omitted when it is
    /// the default for the scheme). Opaque origins produce `"null"`.
    pub fn serialize(&self) -> String {
        match self {
            Self::Tuple {
                scheme,
                host,
                port,
            } => {
                let default_port = default_port_for_scheme(scheme);
                if Some(*port) == default_port {
                    format!("{scheme}://{host}")
                } else {
                    format!("{scheme}://{host}:{port}")
                }
            }
            Self::Opaque => "null".to_owned(),
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.serialize())
    }
}

/// Returns the default port for well-known schemes, or `None`.
fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_origin_from_https_url() {
        let url = VexUrl::parse("https://example.com/path?q=1").unwrap();
        let origin = Origin::from_url(&url);
        assert_eq!(
            origin,
            Origin::Tuple {
                scheme: "https".into(),
                host: "example.com".into(),
                port: 443,
            }
        );
        assert!(origin.is_tuple());
        assert!(!origin.is_opaque());
    }

    #[test]
    fn tuple_origin_with_explicit_port() {
        let url = VexUrl::parse("http://example.com:8080/foo").unwrap();
        let origin = Origin::from_url(&url);
        assert_eq!(
            origin,
            Origin::Tuple {
                scheme: "http".into(),
                host: "example.com".into(),
                port: 8080,
            }
        );
        assert_eq!(origin.serialize(), "http://example.com:8080");
    }

    #[test]
    fn opaque_origin_for_data_url() {
        let url = VexUrl::parse("data:text/html,hello").unwrap();
        let origin = Origin::from_url(&url);
        assert!(origin.is_opaque());
        assert_eq!(origin.serialize(), "null");
    }

    #[test]
    fn opaque_origin_for_about_blank() {
        let url = VexUrl::parse("about:blank").unwrap();
        let origin = Origin::from_url(&url);
        assert!(origin.is_opaque());
    }

    #[test]
    fn same_origin_check() {
        let a = VexUrl::parse("https://example.com/a").unwrap();
        let b = VexUrl::parse("https://example.com/b").unwrap();
        let c = VexUrl::parse("http://example.com/a").unwrap();
        let d = VexUrl::parse("https://other.com/a").unwrap();
        let e = VexUrl::parse("https://example.com:9999/a").unwrap();

        let oa = Origin::from_url(&a);
        let ob = Origin::from_url(&b);
        let oc = Origin::from_url(&c);
        let od = Origin::from_url(&d);
        let oe = Origin::from_url(&e);

        // Same scheme+host+port → same origin (path irrelevant)
        assert!(Origin::same_origin(&oa, &ob));
        // Different scheme → not same origin
        assert!(!Origin::same_origin(&oa, &oc));
        // Different host → not same origin
        assert!(!Origin::same_origin(&oa, &od));
        // Different port → not same origin
        assert!(!Origin::same_origin(&oa, &oe));
    }

    #[test]
    fn opaque_origins_never_match() {
        let a = VexUrl::parse("data:text/html,a").unwrap();
        let b = VexUrl::parse("data:text/html,b").unwrap();
        let oa = Origin::from_url(&a);
        let ob = Origin::from_url(&b);
        assert!(!Origin::same_origin(&oa, &ob));
        // Not even with itself
        assert!(!Origin::same_origin(&oa, &oa));
    }

    #[test]
    fn serialize_omits_default_port() {
        let url = VexUrl::parse("https://example.com/foo").unwrap();
        let origin = Origin::from_url(&url);
        assert_eq!(origin.serialize(), "https://example.com");
        assert_eq!(origin.to_string(), "https://example.com");
    }
}
