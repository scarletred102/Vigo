// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-privacy
//!
//! Privacy engine for the Vex browser engine.
//!
//! Ad/tracker blocking, tracking parameter stripping,
//! HTTPS-only mode, header sanitization, referrer policy.

pub mod adblock;
pub mod headers;
pub mod https;
pub mod middleware;
pub mod tracking;

// Re-export the main public API.
pub use adblock::AdblockEngine;
pub use headers::sanitize_headers;
pub use https::enforce_https;
pub use middleware::PrivacyLayer;
pub use tracking::strip_tracking_params;
