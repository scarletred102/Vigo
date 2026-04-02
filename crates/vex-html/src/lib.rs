// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-html
//!
//! HTML5-compliant parser for the Vex browser engine.
//!
//! Wraps `html5ever` with a custom `TreeSink` that builds a `vex-dom` tree.
//! Supports full document parsing and fragment parsing.

pub mod extract;
pub mod incremental;
pub mod diagnostics;
pub mod parser;
pub mod preload;
pub mod speculative;
mod sink;

pub use extract::{extract_scripts, extract_styles};
pub use incremental::HtmlParser;
pub use parser::{
	parse_html, parse_html_bytes, parse_html_bytes_with_content_type, parse_html_fragment,
	parse_html_with_diagnostics,
};
pub use diagnostics::{diagnose_html, DiagnosticSeverity, ParseDiagnostic};
pub use preload::{extract_preload_candidates, PreloadCandidate, PreloadKind};
pub use speculative::{SpeculativePreloadHint, SpeculativePreloadScanner};
