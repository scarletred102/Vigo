// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Incremental (streaming) HTML parser.
//!
//! Allows feeding HTML chunks as they arrive from the network, then
//! finishing to produce the final [`Document`].

use html5ever::ParseOpts;
use tendril::stream::TendrilSink;

use vex_dom::Document;

use crate::sink::VexSink;

/// A streaming HTML parser that accepts incremental input.
///
/// ```ignore
/// let mut parser = HtmlParser::new();
/// parser.feed(b"<html><body>");
/// parser.feed(b"<p>Hello</p>");
/// let doc = parser.finish();
/// ```
pub struct HtmlParser {
    inner: html5ever::Parser<VexSink>,
}

impl HtmlParser {
    /// Create a new incremental parser.
    pub fn new() -> Self {
        let sink = VexSink::new();
        let inner = html5ever::parse_document(sink, ParseOpts::default());
        Self { inner }
    }

    /// Feed a chunk of bytes. The data is converted from UTF-8.
    ///
    /// Invalid UTF-8 bytes are replaced with U+FFFD.
    pub fn feed(&mut self, chunk: &[u8]) {
        let text = String::from_utf8_lossy(chunk);
        self.inner.process(text.into_owned().into());
    }

    /// Feed a string slice directly.
    pub fn feed_str(&mut self, chunk: &str) {
        self.inner.process(chunk.into());
    }

    /// Finish parsing and return the completed DOM.
    pub fn finish(self) -> Document {
        self.inner.finish()
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_basic() {
        let mut parser = HtmlParser::new();
        parser.feed_str("<html><head>");
        parser.feed_str("</head><body>");
        parser.feed_str("<p>Hello</p>");
        parser.feed_str("</body></html>");
        let doc = parser.finish();
        assert!(doc.root_element().is_some());
    }

    #[test]
    fn incremental_bytes() {
        let mut parser = HtmlParser::new();
        parser.feed(b"<p>Test</p>");
        let doc = parser.finish();
        let text = doc.text_content(doc.root());
        assert!(text.contains("Test"));
    }

    #[test]
    fn incremental_split_tag() {
        // Split a tag across two chunks
        let mut parser = HtmlParser::new();
        parser.feed_str("<di");
        parser.feed_str("v>content</div>");
        let doc = parser.finish();
        let divs = doc.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);
    }

    #[test]
    fn incremental_empty() {
        let parser = HtmlParser::new();
        let doc = parser.finish();
        // Even empty input produces a document with html/head/body
        assert!(doc.root_element().is_some());
    }
}
