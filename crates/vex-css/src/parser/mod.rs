// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS parser modules.

pub mod stylesheet;

pub use stylesheet::{parse_stylesheet, parse_inline_style, Stylesheet, CssRule};
