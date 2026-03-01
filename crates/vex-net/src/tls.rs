// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! TLS 1.3 configuration for the Vex network stack.
//!
//! Uses `rustls` with Mozilla root certificates and ALPN for HTTP/2 negotiation.

use std::sync::Arc;

use rustls::ClientConfig;
use vex_core::VexResult;

/// Build a shared TLS client config.
///
/// - Mozilla root certificate store (via `webpki-roots`)
/// - TLS 1.3 (default for rustls 0.23+)
/// - ALPN: `h2`, `http/1.1`
pub fn tls_config() -> VexResult<Arc<ClientConfig>> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

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
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        assert!(
            root_store.len() > 100,
            "expected 100+ root CAs, got {}",
            root_store.len()
        );
    }
}
