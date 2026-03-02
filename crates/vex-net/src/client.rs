// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTTP client with TLS 1.3, connection pooling, redirect following, and decompression.

use std::collections::HashMap;
use std::sync::Mutex;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tracing::{debug, warn};
use vex_core::{VexError, VexResult, VexUrl};

use crate::cache::HttpCache;
use crate::cookies::CookieJar;
use crate::decompress;
use crate::dns::{DnsMode, DnsResolver};
use crate::tls;
use crate::types::{Method, Request, Response};

/// Maximum number of redirects to follow.
const MAX_REDIRECTS: u8 = 10;

/// Default request timeout (seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Configuration for building an `HttpClient`.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub user_agent: String,
    pub follow_redirects: bool,
    pub max_redirects: u8,
    pub timeout_secs: u64,
    pub dns_mode: DnsMode,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("Vigo/{} Vex", env!("CARGO_PKG_VERSION")),
            follow_redirects: true,
            max_redirects: MAX_REDIRECTS,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            dns_mode: DnsMode::System,
        }
    }
}

/// The main HTTP client.
pub struct HttpClient {
    inner: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        Full<Bytes>,
    >,
    config: ClientConfig,
    cookies: CookieJar,
    dns_resolver: DnsResolver,
    cache: Mutex<HttpCache>,
}

impl HttpClient {
    /// Create a new HTTP client with default configuration.
    pub fn new() -> VexResult<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new HTTP client with custom configuration.
    pub fn with_config(config: ClientConfig) -> VexResult<Self> {
        let tls = tls::tls_config()?;
        let dns_resolver = DnsResolver::new(config.dns_mode.clone())?;

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config((*tls).clone())
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build();

        let inner = Client::builder(TokioExecutor::new()).build(https);

        Ok(Self {
            inner,
            config,
            cookies: CookieJar::new(),
            dns_resolver,
            cache: Mutex::new(HttpCache::new()),
        })
    }

    /// Create a client with an explicit DNS mode.
    pub fn with_dns_mode(mode: DnsMode) -> VexResult<Self> {
        let config = ClientConfig {
            dns_mode: mode,
            ..ClientConfig::default()
        };
        Self::with_config(config)
    }

    /// Access the cookie jar.
    pub fn cookie_jar(&self) -> &CookieJar {
        &self.cookies
    }

    /// Access the HTTP cache (locked).
    pub fn cache(&self) -> &Mutex<HttpCache> {
        &self.cache
    }

    /// Fetch a URL, returning the response.
    ///
    /// Handles: TLS, decompression, redirects, cookies.
    pub async fn fetch(&self, request: Request) -> VexResult<Response> {
        self.fetch_filtered(request, |_| Ok(())).await
    }

    /// Fetch a URL after running the request through a filter.
    ///
    /// The `filter` closure can modify the request (e.g., stripping tracking
    /// parameters, sanitizing headers) or return `Err` to block the request
    /// entirely. This is the integration point for `vex-privacy::PrivacyLayer`.
    ///
    /// ```ignore
    /// let privacy = PrivacyLayer::new();
    /// client.fetch_filtered(request, |req| privacy.process_request(req)).await?;
    /// ```
    pub async fn fetch_filtered<F>(&self, mut request: Request, filter: F) -> VexResult<Response>
    where
        F: FnOnce(&mut Request) -> VexResult<()>,
    {
        filter(&mut request)?;

        let mut current_url = request.url.clone();
        let mut current_method = request.method;
        let mut redirects = 0u8;

        loop {
            let resp = self
                .do_fetch(
                    &current_url,
                    current_method,
                    &request.headers,
                    &request.body,
                )
                .await?;

            // Handle redirects
            if self.config.follow_redirects
                && is_redirect(resp.status)
                && redirects < self.config.max_redirects
            {
                if let Some(location) = resp.headers.get("location") {
                    let next_url = resolve_redirect(&current_url, location)?;
                    debug!(status = resp.status, to = %next_url, "following redirect");

                    // 303: always change to GET
                    if resp.status == 303 {
                        current_method = Method::Get;
                    }
                    // 301/302: change to GET for POST (historical browser behavior)
                    if (resp.status == 301 || resp.status == 302) && current_method == Method::Post
                    {
                        current_method = Method::Get;
                    }

                    current_url = next_url;
                    redirects += 1;
                    continue;
                }
            }

            return Ok(resp);
        }
    }

    /// Perform a single (non-redirect-following) HTTP fetch.
    ///
    /// Checks the in-memory cache first. On cache hit with a fresh response,
    /// returns without a network round-trip. Stale entries with ETag or
    /// Last-Modified trigger conditional requests (If-None-Match /
    /// If-Modified-Since); a 304 reuses the cached body.
    async fn do_fetch(
        &self,
        url: &VexUrl,
        method: Method,
        extra_headers: &HashMap<String, String>,
        body: &Option<Vec<u8>>,
    ) -> VexResult<Response> {
        // ── Cache lookup (GET only) ──────────────────────────────────
        if method == Method::Get {
            if let Ok(cache) = self.cache.lock() {
                if let Some(cached) = cache.get(url) {
                    if cached.is_fresh() {
                        debug!(url = %url, "cache hit (fresh)");
                        return Ok(Response {
                            status: cached.status,
                            headers: cached.headers.clone(),
                            body: cached.body.clone(),
                            url: url.clone(),
                            was_cached: true,
                        });
                    }
                }
            }
        }

        // Grab conditional-request validators before the network call
        let (cached_etag, cached_last_modified) = if method == Method::Get {
            self.cache
                .lock()
                .ok()
                .and_then(|c| c.get(url).map(|e| (e.etag.clone(), e.last_modified.clone())))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        if let Some(host) = url.host() {
            // Resolve using the configured resolver (system DNS or DoH).
            let resolved = self.dns_resolver.resolve(host).await?;
            debug!(host, ?resolved, "DNS resolved");
        }

        let uri: hyper::Uri = url
            .inner()
            .as_str()
            .parse()
            .map_err(|e| VexError::Network(format!("invalid URI: {e}")))?;

        let mut builder = hyper::Request::builder()
            .method(method.to_http())
            .uri(&uri)
            .header("user-agent", &self.config.user_agent)
            .header("accept", "*/*")
            .header("accept-encoding", "gzip, deflate, br, zstd");

        // Conditional headers for cache revalidation
        if let Some(ref etag) = cached_etag {
            builder = builder.header("if-none-match", etag.as_str());
        }
        if let Some(ref lm) = cached_last_modified {
            builder = builder.header("if-modified-since", lm.as_str());
        }

        // Add cookies
        if let Some(cookie_header) = self.cookies.get_cookies(url) {
            builder = builder.header("cookie", cookie_header);
        }

        // Add user-specified headers
        for (k, v) in extra_headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        let req_body = match body {
            Some(b) => Full::new(Bytes::from(b.clone())),
            None => Full::new(Bytes::new()),
        };

        let hyper_req = builder
            .body(req_body)
            .map_err(|e| VexError::Network(format!("request build failed: {e}")))?;

        debug!(method = %method.to_http(), %uri, "sending request");

        let hyper_resp = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            self.inner.request(hyper_req),
        )
        .await
        .map_err(|_| {
            VexError::Network(format!(
                "request timed out after {}s",
                self.config.timeout_secs
            ))
        })?
        .map_err(|e| VexError::Network(format!("request failed: {e}")))?;

        let status = hyper_resp.status().as_u16();

        // Collect response headers
        let mut headers = HashMap::new();
        for (k, v) in hyper_resp.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.as_str().to_string(), val.to_string());
            }
        }

        // Handle 304 Not Modified — reuse cached body
        if status == 304 && method == Method::Get {
            if let Ok(cache) = self.cache.lock() {
                if let Some(cached) = cache.get(url) {
                    debug!(url = %url, "304 Not Modified, using cached body");
                    return Ok(Response {
                        status: cached.status,
                        headers: cached.headers.clone(),
                        body: cached.body.clone(),
                        url: url.clone(),
                        was_cached: true,
                    });
                }
            }
        }

        // Store cookies from Set-Cookie headers
        for value in hyper_resp.headers().get_all("set-cookie") {
            if let Ok(v) = value.to_str() {
                self.cookies.insert(url, v);
            }
        }

        // Read body
        let raw_body = hyper_resp
            .into_body()
            .collect()
            .await
            .map_err(|e| VexError::Network(format!("failed to read body: {e}")))?
            .to_bytes()
            .to_vec();

        // Decompress if needed
        let encoding = headers
            .get("content-encoding")
            .map(|s| s.as_str())
            .unwrap_or("");
        let body = if encoding.is_empty() || encoding == "identity" {
            raw_body
        } else {
            match decompress::decompress(encoding, &raw_body) {
                Ok(decompressed) => decompressed,
                Err(e) => {
                    warn!(%encoding, "decompression failed, using raw body: {e}");
                    raw_body
                }
            }
        };

        // Store in cache (GET only)
        if method == Method::Get {
            if let Ok(mut cache) = self.cache.lock() {
                cache.store(url, status, &headers, &body);
            }
        }

        debug!(status, bytes = body.len(), "response received");

        Ok(Response {
            status,
            headers,
            body,
            url: url.clone(),
            was_cached: false,
        })
    }
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn resolve_redirect(base: &VexUrl, location: &str) -> VexResult<VexUrl> {
    // Location can be absolute or relative
    if location.starts_with("http://") || location.starts_with("https://") {
        VexUrl::parse(location)
    } else {
        base.join(location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_system_dns() {
        let config = ClientConfig::default();
        assert!(matches!(config.dns_mode, DnsMode::System));
    }

    #[test]
    fn client_accepts_doh_mode() {
        let client = HttpClient::with_dns_mode(DnsMode::DoH(crate::dns::DoHProvider::Cloudflare));
        assert!(client.is_ok());
    }

    #[test]
    fn cache_is_initialized_and_accessible() {
        let client = HttpClient::new().unwrap();
        let cache = client.cache().lock().unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_stores_and_retrieves_entries() {
        let client = HttpClient::new().unwrap();
        let url = VexUrl::parse("https://example.com/page").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        {
            let mut cache = client.cache().lock().unwrap();
            cache.store(&url, 200, &headers, b"cached body");
        }

        let cache = client.cache().lock().unwrap();
        let entry = cache.get(&url).unwrap();
        assert_eq!(entry.status, 200);
        assert_eq!(entry.body, b"cached body");
        assert!(entry.is_fresh());
    }
}
