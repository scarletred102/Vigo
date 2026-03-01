// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Security-specific error types.

use thiserror::Error;

/// Errors originating from the security subsystem.
#[derive(Debug, Error)]
pub enum SecurityError {
    /// A request was blocked by the Same-Origin Policy.
    #[error("same-origin policy violation: {0}")]
    SopViolation(String),

    /// A CORS preflight or response check failed.
    #[error("CORS error: {0}")]
    CorsBlocked(String),

    /// A resource was blocked by Content Security Policy.
    #[error("CSP violation: {0}")]
    CspViolation(String),

    /// A CSP directive could not be parsed.
    #[error("CSP parse error: {0}")]
    CspParse(String),

    /// Mixed-content resource blocked (HTTP on HTTPS page).
    #[error("mixed content blocked: {0}")]
    MixedContent(String),

    /// A sandboxing or process-isolation error.
    #[error("sandbox error: {0}")]
    Sandbox(String),
}

/// Convenience alias for security results.
pub type SecurityResult<T> = Result<T, SecurityError>;
