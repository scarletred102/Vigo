// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DNS resolution with DNS-over-HTTPS (DoH) support.

use std::net::IpAddr;

use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use vex_core::{VexError, VexResult};

/// DNS resolution mode.
#[derive(Debug, Clone)]
pub enum DnsMode {
    /// Use the system's default DNS configuration.
    System,
    /// Use DNS-over-HTTPS with a specific provider.
    DoH(DoHProvider),
}

/// Well-known DoH providers.
#[derive(Debug, Clone, Copy)]
pub enum DoHProvider {
    Cloudflare,
    Google,
}

impl DoHProvider {
    fn resolver_config(self) -> ResolverConfig {
        match self {
            Self::Cloudflare => ResolverConfig::cloudflare_https(),
            Self::Google => ResolverConfig::google_https(),
        }
    }
}

/// Async DNS resolver.
pub struct DnsResolver {
    inner: TokioAsyncResolver,
    mode: DnsMode,
}

impl DnsResolver {
    /// Create a new resolver with the given mode.
    pub fn new(mode: DnsMode) -> VexResult<Self> {
        let config = match &mode {
            DnsMode::System => ResolverConfig::default(),
            DnsMode::DoH(provider) => provider.resolver_config(),
        };

        let inner = TokioAsyncResolver::tokio(config, ResolverOpts::default());

        Ok(Self { inner, mode })
    }

    /// Resolve a hostname to IP addresses.
    pub async fn resolve(&self, host: &str) -> VexResult<Vec<IpAddr>> {
        let lookup = self
            .inner
            .lookup_ip(host)
            .await
            .map_err(|e| VexError::Network(format!("DNS resolve failed for {host}: {e}")))?;

        let addrs: Vec<IpAddr> = lookup.iter().collect();

        if addrs.is_empty() {
            return Err(VexError::Network(format!("no addresses for {host}")));
        }

        tracing::debug!(?addrs, host, mode = ?self.mode, "DNS resolved");
        Ok(addrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_system_resolver() {
        let _resolver = DnsResolver::new(DnsMode::System).expect("system DNS should init");
    }

    #[test]
    fn create_doh_resolver() {
        let _resolver =
            DnsResolver::new(DnsMode::DoH(DoHProvider::Cloudflare)).expect("DoH should init");
    }
}
