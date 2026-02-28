// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! # vex-core
//!
//! Core types and primitives shared across the entire Vex browser engine.
//!
//! This crate provides:
//! - Geometry primitives (`Point`, `Size`, `Rect`, `Insets`)
//! - Color type with CSS parsing
//! - URL wrapper (`VexUrl`)
//! - Node ID allocator (`VexId`)
//! - Unified error types (`VexError`, `VexResult`)

pub fn engine_version() -> &'static str {
    "0.1.0"
}

pub fn engine_name() -> &'static str {
    "Vex"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string() {
        assert_eq!(engine_version(), "0.1.0");
    }

    #[test]
    fn name_string() {
        assert_eq!(engine_name(), "Vex");
    }
}
