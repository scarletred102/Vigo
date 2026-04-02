// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Incremental (streaming) HTML parser.
//!
//! Allows feeding HTML chunks as they arrive from the network, then
//! finishing to produce the final [`Document`].

use html5ever::ParseOpts;
use tendril::stream::TendrilSink;

use vex_dom::Document;

use crate::sink::VexSink;
use crate::{SpeculativePreloadHint, SpeculativePreloadScanner};

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
    pending_utf8: Vec<u8>,
    speculative_scanner: SpeculativePreloadScanner,
}

impl HtmlParser {
    /// Create a new incremental parser.
    pub fn new() -> Self {
        let sink = VexSink::new();
        let inner = html5ever::parse_document(sink, ParseOpts::default());
        Self {
            inner,
            pending_utf8: Vec::new(),
            speculative_scanner: SpeculativePreloadScanner::new(),
        }
    }

    /// Feed a chunk of bytes. The data is converted from UTF-8.
    ///
    /// Invalid UTF-8 bytes are replaced with U+FFFD.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.pending_utf8.extend_from_slice(chunk);
        self.process_pending(false);
    }

    /// Feed bytes and return speculative preload hints discovered in this chunk.
    pub fn feed_and_scan_preloads(&mut self, chunk: &[u8]) -> Vec<SpeculativePreloadHint> {
        let hints = self.speculative_scanner.feed(chunk);
        self.feed(chunk);
        hints
    }

    /// Feed a string slice directly.
    pub fn feed_str(&mut self, chunk: &str) {
        // Flush any partial byte state first to preserve order.
        self.process_pending(true);
        self.inner.process(chunk.into());
    }

    /// Finish parsing and return the completed DOM.
    pub fn finish(self) -> Document {
        let mut this = self;
        this.process_pending(true);
        this.inner.finish()
    }

    fn process_pending(&mut self, flush_all: bool) {
        loop {
            match std::str::from_utf8(&self.pending_utf8) {
                Ok(valid) => {
                    if !valid.is_empty() {
                        self.inner.process(valid.into());
                    }
                    self.pending_utf8.clear();
                    break;
                }
                Err(err) => {
                    let valid_up_to = err.valid_up_to();
                    if valid_up_to > 0 {
                        let valid = unsafe {
                            // SAFETY: `valid_up_to` is guaranteed valid UTF-8 prefix by `from_utf8`.
                            std::str::from_utf8_unchecked(&self.pending_utf8[..valid_up_to])
                        };
                        self.inner.process(valid.into());
                        self.pending_utf8.drain(..valid_up_to);
                        continue;
                    }

                    if let Some(error_len) = err.error_len() {
                        // Invalid sequence. Consume and replace with U+FFFD.
                        self.inner.process("�".into());
                        self.pending_utf8.drain(..error_len);
                        continue;
                    }

                    // Incomplete sequence at end.
                    if flush_all {
                        let lossy = String::from_utf8_lossy(&self.pending_utf8).into_owned();
                        if !lossy.is_empty() {
                            self.inner.process(lossy.into());
                        }
                        self.pending_utf8.clear();
                    }
                    break;
                }
            }
        }
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
    fn incremental_split_utf8_boundary() {
        // "😀" in UTF-8 split across chunks.
        let mut parser = HtmlParser::new();
        parser.feed(b"<p>");
        parser.feed(&[0xF0, 0x9F]);
        parser.feed(&[0x98, 0x80]);
        parser.feed(b"</p>");

        let doc = parser.finish();
        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "😀");
    }

    #[test]
    fn incremental_invalid_utf8_replaced() {
        let mut parser = HtmlParser::new();
        parser.feed(b"<p>");
        parser.feed(&[0xFF]);
        parser.feed(b"</p>");
        let doc = parser.finish();

        let p = doc.get_elements_by_tag_name("p");
        assert_eq!(p.len(), 1);
        assert_eq!(doc.text_content(p[0]), "�");
    }

    #[test]
    fn incremental_empty() {
        let parser = HtmlParser::new();
        let doc = parser.finish();
        // Even empty input produces a document with html/head/body
        assert!(doc.root_element().is_some());
    }

    #[test]
    fn incremental_speculative_hints() {
        let mut parser = HtmlParser::new();
        let hints = parser.feed_and_scan_preloads(
            b"<link rel='stylesheet' href='/app.css'><script src='/app.js'></script>",
        );
        assert!(hints.iter().any(|h| h.url == "/app.css"));
        assert!(hints.iter().any(|h| h.url == "/app.js"));
    }
}
