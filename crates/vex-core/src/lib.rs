// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-core
//!
//! Core types and primitives shared across the entire Vex browser engine.

pub mod color;
pub mod error;
pub mod geometry;
pub mod id;
pub mod string;
pub mod vex_url;

// Re-exports for ergonomic access.
pub use color::Color;
pub use error::{VexError, VexResult};
pub use geometry::{Insets, Point, Rect, Size};
pub use id::{IdAllocator, VexId};
pub use string::VexString;
pub use vex_url::VexUrl;

/// Engine version string.
pub fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Engine codename.
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
