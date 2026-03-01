// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Storage-specific error types.

use thiserror::Error;

/// Errors from the storage subsystem.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A SQLite operation failed.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A quota was exceeded (e.g. localStorage 5 MB limit).
    #[error("quota exceeded for origin {origin}: {detail}")]
    QuotaExceeded { origin: String, detail: String },

    /// An invalid key or value was provided.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// Convenience alias.
pub type StorageResult<T> = Result<T, StorageError>;
