// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-net
//!
//! Network stack for the Vex browser engine.
//!
//! Provides HTTP/1.1 + HTTP/2 client with TLS 1.3 (rustls),
//! DNS resolution with DoH support, caching, compression,
//! cookie handling, and redirect following.

pub mod alt_svc;
pub mod cache;
pub mod client;
pub mod cookies;
pub mod decompress;
pub mod disk_cache;
pub mod dns;
pub mod hsts;
pub mod http3;
pub mod proxy;
pub mod security_policy;
pub mod telemetry;
pub mod telemetry_store;
pub mod tls;
pub mod types;
pub mod websocket;

// Re-export the main public API.
pub use alt_svc::{AltSvcCache, AltSvcEntry};
pub use cache::HttpCache;
pub use client::{ClientConfig, HttpClient};
pub use cookies::{CookieAccess, CookieJar, NavigationKind, SameSite};
pub use disk_cache::{DiskCache, DiskCacheConfig};
pub use dns::{DnsMode, DnsResolver, DoHProvider};
pub use hsts::HstsStore;
pub use http3::{Http3Config, HttpVersionPreference};
pub use proxy::ProxyConfig;
pub use security_policy::TransportSecurityPolicy;
pub use telemetry::{
    build_waterfall, CacheOutcome, NetworkRecord, NetworkStats, NetworkTimings, WaterfallEntry,
};
pub use telemetry_store::TelemetryStore;
pub use types::{Method, Request, Response};
pub use websocket::{CloseFrame, ReadyState, WebSocket, WsConfig, WsError, WsMessage};
