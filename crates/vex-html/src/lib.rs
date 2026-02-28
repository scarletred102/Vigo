// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-html
//!
//! HTML5-compliant parser for the Vex browser engine.
//!
//! Wraps `html5ever` with a custom `TreeSink` that builds a `vex-dom` tree.
//! Supports full document parsing and fragment parsing.

mod sink;
pub mod extract;
pub mod incremental;
pub mod parser;

pub use extract::{extract_scripts, extract_styles};
pub use incremental::HtmlParser;
pub use parser::{parse_html, parse_html_fragment};
