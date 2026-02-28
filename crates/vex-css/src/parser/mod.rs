// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! CSS parser modules.

pub mod stylesheet;

pub use stylesheet::{parse_stylesheet, parse_inline_style, Stylesheet, CssRule};
