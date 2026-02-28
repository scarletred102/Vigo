// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-css
//!
//! CSS parser and style system for the Vex browser engine.
//!
//! Parses stylesheets, resolves the cascade (specificity, origin,
//! inheritance), and produces `ComputedStyle` for every DOM element.
//! Supports media queries.

pub mod values;
pub mod properties;
pub mod parser;
pub mod cascade;
pub mod computed;
pub mod ua_stylesheet;
pub mod media;

// Re-export key public types.
pub use computed::ComputedStyle;
pub use cascade::compute_styles;
pub use parser::{parse_stylesheet, parse_inline_style, Stylesheet};
pub use media::{MediaCondition, evaluate_media, parse_media_condition};
