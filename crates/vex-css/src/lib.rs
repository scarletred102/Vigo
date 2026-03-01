// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-css
//!
//! CSS parser and style system for the Vex browser engine.
//!
//! Parses stylesheets, resolves the cascade (specificity, origin,
//! inheritance), and produces `ComputedStyle` for every DOM element.
//! Supports media queries.

pub mod cascade;
pub mod computed;
pub mod media;
pub mod parser;
pub mod properties;
pub mod ua_stylesheet;
pub mod values;

// Re-export key public types.
pub use cascade::compute_styles;
pub use computed::ComputedStyle;
pub use media::{evaluate_media, parse_media_condition, MediaCondition};
pub use parser::{parse_inline_style, parse_stylesheet, Stylesheet};
