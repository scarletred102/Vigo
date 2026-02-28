// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Unified error types for the Vex engine.

use thiserror::Error;

/// All errors that can occur within the Vex engine.
#[derive(Debug, Error)]
pub enum VexError {
    #[error("network: {0}")]
    Network(String),

    #[error("parse: {0}")]
    Parse(String),

    #[error("css: {0}")]
    Css(String),

    #[error("layout: {0}")]
    Layout(String),

    #[error("js: {0}")]
    Js(String),

    #[error("platform: {0}")]
    Platform(String),

    #[error("storage: {0}")]
    Storage(String),

    #[error("internal: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}

/// Convenience alias used throughout the engine.
pub type VexResult<T> = Result<T, VexError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format() {
        let e = VexError::Network("timeout".into());
        assert_eq!(e.to_string(), "network: timeout");
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let e: VexError = io.into();
        assert!(matches!(e, VexError::Io(_)));
    }

    #[test]
    fn from_url_error() {
        let bad = url::Url::parse("://nope").unwrap_err();
        let e: VexError = bad.into();
        assert!(matches!(e, VexError::Url(_)));
    }
}
