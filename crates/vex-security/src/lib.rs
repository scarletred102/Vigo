// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-security
//!
//! Security enforcement for the Vex browser engine.
//!
//! - **Origin model** — [`Origin`] per RFC 6454, with [`Origin::same_origin`] checks.
//! - **Same-Origin Policy** — DOM and fetch cross-origin enforcement.
//! - **CORS** — preflight OPTIONS, header validation, credential handling.
//! - **Content Security Policy** — CSP Level 2 directive parsing & enforcement.
//! - **Process sandboxing** — tab isolation and restricted tokens.

pub mod cors;
pub mod csp;
pub mod error;
pub mod origin;
pub mod sop;

pub use cors::{
    build_preflight_headers, classify_cors, exposed_headers, validate_preflight, validate_response,
    CorsMode, CorsRequest, PreflightHeaders,
};
pub use csp::{CspPolicy, CspSource, Directive, ResourceType};
pub use error::{SecurityError, SecurityResult};
pub use origin::Origin;
pub use sop::{check_dom_access, check_storage_access, classify_fetch, FetchType};
