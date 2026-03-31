// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-storage
//!
//! Persistent storage for the Vex browser engine.
//!
//! - **Cookies** — SQLite-backed [`CookieStore`] with domain/path/expiry.
//! - **localStorage** — Per-origin key-value store with 5 MB quota.
//! - **sessionStorage** — In-memory per-origin key-value store (per tab).
//! - **IndexedDB** — Simplified SQLite-backed object store with indexes.

pub mod cookies;
pub mod error;
pub mod indexed_db;
pub mod local_storage;
pub mod session_storage;

pub use cookies::{CookieStore, PersistentCookie};
pub use error::{StorageError, StorageResult};
pub use indexed_db::IdbDatabase;
pub use local_storage::LocalStorage;
pub use session_storage::SessionStorage;
