// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-privacy
//!
//! Privacy engine for the Vex browser engine.
//!
//! Ad/tracker blocking, tracking parameter stripping,
//! HTTPS-only mode, header sanitization, referrer policy.

pub mod adblock;
pub mod canvas;
pub mod fonts;
pub mod headers;
pub mod https;
pub mod middleware;
pub mod tracking;
pub mod webgl;

// Re-export the main public API.
pub use adblock::AdblockEngine;
pub use canvas::{apply_canvas_noise, CanvasFingerprintConfig};
pub use fonts::{filter_fonts, is_font_allowed, FontRestrictionConfig};
pub use headers::sanitize_headers;
pub use https::enforce_https;
pub use middleware::PrivacyLayer;
pub use tracking::strip_tracking_params;
pub use webgl::{filter_extensions, mask_webgl_params, WebGlMask};
