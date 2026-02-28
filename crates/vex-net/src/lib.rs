// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-net
//!
//! Network stack for the Vex browser engine.
//!
//! Provides HTTP/1.1 + HTTP/2 client with TLS 1.3 (rustls),
//! DNS resolution with DoH support, caching, compression,
//! cookie handling, and redirect following.

pub mod cache;
pub mod client;
pub mod cookies;
pub mod decompress;
pub mod dns;
pub mod tls;
pub mod types;

// Re-export the main public API.
pub use cache::HttpCache;
pub use client::{ClientConfig, HttpClient};
pub use cookies::CookieJar;
pub use dns::{DnsMode, DnsResolver, DoHProvider};
pub use types::{Method, Request, Response};
