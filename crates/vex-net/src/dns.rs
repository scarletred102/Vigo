// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DNS resolution with DNS-over-HTTPS (DoH) support.

use std::future::Future;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper_util::client::legacy::connect::dns::Name;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use tower::Service;
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
#[derive(Clone)]
pub struct DnsResolver {
    inner: TokioAsyncResolver,
    mode: DnsMode,
}

impl DnsResolver {
    /// Create a new resolver with the given mode.
    pub fn new(mode: DnsMode) -> VexResult<Self> {
        let inner = match &mode {
            DnsMode::System => match TokioAsyncResolver::tokio_from_system_conf() {
                Ok(resolver) => resolver,
                Err(err) => {
                    tracing::warn!(
                        "failed to load system DNS config: {err}; falling back to default resolver"
                    );
                    TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
                }
            },
            DnsMode::DoH(provider) => {
                TokioAsyncResolver::tokio(provider.resolver_config(), ResolverOpts::default())
            }
        };

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

/// Hyper-compatible DNS service that delegates lookups to [`DnsResolver`].
#[derive(Clone)]
pub struct HyperDnsResolver {
    resolver: DnsResolver,
}

impl HyperDnsResolver {
    pub fn new(resolver: DnsResolver) -> Self {
        Self { resolver }
    }
}

impl Service<Name> for HyperDnsResolver {
    type Response = std::vec::IntoIter<SocketAddr>;
    type Error = std::io::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let resolver = self.resolver.clone();
        let host = name.as_str().to_string();
        Box::pin(async move {
            let ips = resolver.resolve(&host).await.map_err(|e| {
                std::io::Error::other(format!("DNS resolve failed: {e}"))
            })?;
            let addrs: Vec<SocketAddr> = ips
                .into_iter()
                // Hyper sets the destination port later; resolver provides host IPs.
                .map(|ip| SocketAddr::new(ip, 0))
                .collect();
            Ok(addrs.into_iter())
        })
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

    #[tokio::test]
    async fn hyper_dns_resolver_service_returns_addrs() {
        let resolver = DnsResolver::new(DnsMode::System).expect("resolver init");
        let mut service = HyperDnsResolver::new(resolver);

        let name: Name = "localhost".parse().expect("valid host name");
        let addrs = service.call(name).await.expect("resolve localhost");
        assert!(addrs.into_iter().next().is_some());
    }
}
