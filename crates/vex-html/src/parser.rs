// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Public parsing API.

use html5ever::{namespace_url, ns, LocalName, ParseOpts, QualName};
use tendril::TendrilSink;

use vex_dom::Document;

use crate::sink::VexSink;

/// Parse a full HTML document and return the DOM tree.
///
/// ```
/// let doc = vex_html::parse_html("<p>Hello</p>");
/// assert!(doc.root_element().is_some());
/// ```
pub fn parse_html(input: &str) -> Document {
    let sink = VexSink::new();
    html5ever::parse_document(sink, ParseOpts::default()).one(input)
}

/// Parse an HTML fragment in the context of a given element tag.
///
/// Useful for `innerHTML`-style parsing.
pub fn parse_html_fragment(input: &str, context_tag: &str) -> Document {
    let sink = VexSink::new();
    let context = QualName::new(None, ns!(html), LocalName::from(context_tag));
    html5ever::parse_fragment(sink, ParseOpts::default(), context, vec![]).one(input)
}
