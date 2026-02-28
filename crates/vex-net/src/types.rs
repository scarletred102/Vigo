// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP request/response types for the Vex network stack.

use std::collections::HashMap;

use vex_core::VexUrl;

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
}

impl Method {
    /// Convert to hyper's `http::Method`.
    pub fn to_http(&self) -> http::Method {
        match self {
            Self::Get => http::Method::GET,
            Self::Post => http::Method::POST,
            Self::Put => http::Method::PUT,
            Self::Delete => http::Method::DELETE,
            Self::Head => http::Method::HEAD,
            Self::Options => http::Method::OPTIONS,
        }
    }
}

/// An outgoing HTTP request.
#[derive(Debug, Clone)]
pub struct Request {
    pub url: VexUrl,
    pub method: Method,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// Shorthand GET request for a URL string.
    pub fn get(url: &str) -> vex_core::VexResult<Self> {
        Ok(Self {
            url: VexUrl::parse(url)?,
            method: Method::Get,
            headers: HashMap::new(),
            body: None,
        })
    }

    /// Shorthand POST request.
    pub fn post(url: &str, body: Vec<u8>) -> vex_core::VexResult<Self> {
        Ok(Self {
            url: VexUrl::parse(url)?,
            method: Method::Post,
            headers: HashMap::new(),
            body: Some(body),
        })
    }
}

/// An HTTP response.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub url: VexUrl,
    pub was_cached: bool,
}

impl Response {
    /// Read the body as UTF-8 text.
    pub fn text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Check if the response indicates success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_request() {
        let req = Request::get("https://example.com").unwrap();
        assert_eq!(req.method, Method::Get);
        assert!(req.url.is_https());
        assert!(req.body.is_none());
    }

    #[test]
    fn post_request() {
        let req = Request::post("https://example.com/api", b"hello".to_vec()).unwrap();
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.body.as_deref(), Some(b"hello".as_slice()));
    }

    #[test]
    fn method_to_http() {
        assert_eq!(Method::Get.to_http(), http::Method::GET);
        assert_eq!(Method::Post.to_http(), http::Method::POST);
        assert_eq!(Method::Delete.to_http(), http::Method::DELETE);
    }

    #[test]
    fn response_text() {
        let resp = Response {
            status: 200,
            headers: HashMap::new(),
            body: b"<html>hello</html>".to_vec(),
            url: VexUrl::parse("https://example.com").unwrap(),
            was_cached: false,
        };
        assert_eq!(resp.text().unwrap(), "<html>hello</html>");
        assert!(resp.is_success());
    }

    #[test]
    fn response_status_check() {
        let resp = Response {
            status: 404,
            headers: HashMap::new(),
            body: vec![],
            url: VexUrl::parse("https://example.com/404").unwrap(),
            was_cached: false,
        };
        assert!(!resp.is_success());
    }
}
