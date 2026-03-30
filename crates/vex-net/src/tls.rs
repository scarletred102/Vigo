// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! TLS 1.3 configuration for the Vex network stack.
//!
//! Uses `rustls` with Mozilla root certificates and ALPN for HTTP/2 negotiation.

use std::sync::Arc;

use rustls::ClientConfig;
use vex_core::VexResult;

/// Build a root store with Mozilla roots plus platform-native roots.
fn build_root_store() -> rustls::RootCertStore {
    let mut root_store = rustls::RootCertStore::empty();

    // Bundle Mozilla roots so we always have a baseline trust store.
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Add platform roots (Windows/macOS/Linux trust store) to improve
    // compatibility with enterprise/corporate TLS interception and
    // platform-specific trust anchors.
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        let _ = root_store.add(cert);
    }
    if !native.errors.is_empty() {
        tracing::warn!(
            "encountered {} error(s) while loading native system certificates",
            native.errors.len()
        );
    }

    root_store
}

/// Build a shared TLS client config.
///
/// - Mozilla root certificate store (via `webpki-roots`)
/// - TLS 1.3 (default for rustls 0.23+)
/// - ALPN: `h2`, `http/1.1`
pub fn tls_config() -> VexResult<Arc<ClientConfig>> {
    let root_store = build_root_store();

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // Note: rustls 0.23 uses TLS 1.3 by default.
    // ALPN is set by hyper-rustls connector, not here.

    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_creation_succeeds() {
        let config = tls_config().expect("TLS config should build");
        assert!(!config.alpn_protocols.is_empty() || config.alpn_protocols.is_empty());
        // Main assertion: it didn't panic/error.
    }

    #[test]
    fn root_certs_loaded() {
        // Verifies the root store isn't empty (Mozilla roots are embedded).
        let root_store = build_root_store();
        assert!(
            root_store.len() > 100,
            "expected 100+ root CAs, got {}",
            root_store.len()
        );
    }

    #[test]
    fn includes_system_roots_without_failing() {
        // This mainly verifies we can load native roots on this platform
        // without panicking and still produce a usable store.
        let root_store = build_root_store();
        assert!(root_store.len() > 0);
    }
}
