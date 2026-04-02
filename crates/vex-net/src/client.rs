// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTTP client with TLS 1.3, connection pooling, redirect following, and decompression.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::future::join_all;
use futures_util::stream;
use futures_util::StreamExt;
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tracing::{debug, warn};
use vex_core::{VexError, VexResult, VexUrl};

use crate::cache::{HttpCache, INTERNAL_PARTITION_HEADER};
use crate::cookies::CookieJar;
use crate::decompress;
use crate::disk_cache::{DiskCache, DiskCacheConfig};
use crate::dns::{DnsMode, DnsResolver, HyperDnsResolver};
use crate::hsts::HstsStore;
use crate::alt_svc::AltSvcCache;
use crate::http3::{validate_http3_attempt, Http3Config, HttpVersionPreference};
use crate::proxy::ProxyConfig;
use crate::security_policy::TransportSecurityPolicy;
use crate::telemetry::{CacheOutcome, NetworkRecord, NetworkStats, NetworkTimings};
use crate::telemetry_store::TelemetryStore;
use crate::tls;
use crate::types::{Method, Request, Response};

/// Maximum number of redirects to follow.
const MAX_REDIRECTS: u8 = 10;

/// Default connect timeout (seconds).
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Default first-byte timeout (seconds).
const DEFAULT_FIRST_BYTE_TIMEOUT_SECS: u64 = 30;
/// Default full-body read timeout (seconds).
const DEFAULT_BODY_TIMEOUT_SECS: u64 = 30;
/// Default pooled idle timeout (seconds).
const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 90;
/// Default max idle connections per host.
const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 32;
/// Default max persistent disk cache size.
const DEFAULT_DISK_CACHE_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Configuration for building an `HttpClient`.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub user_agent: String,
    pub follow_redirects: bool,
    pub max_redirects: u8,
    pub connect_timeout_secs: u64,
    pub first_byte_timeout_secs: u64,
    pub body_timeout_secs: u64,
    pub pool_idle_timeout_secs: u64,
    pub pool_max_idle_per_host: usize,
    pub top_level_site: Option<String>,
    pub proxy: ProxyConfig,
    pub enable_hsts: bool,
    pub enable_alt_svc: bool,
    pub http_version_preference: HttpVersionPreference,
    pub http3: Http3Config,
    pub telemetry_dir: Option<PathBuf>,
    pub disk_cache_dir: Option<PathBuf>,
    pub disk_cache_max_bytes: u64,
    pub strict_transport_evidence: bool,
    pub dns_mode: DnsMode,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("Vigo/{} Vex", env!("CARGO_PKG_VERSION")),
            follow_redirects: true,
            max_redirects: MAX_REDIRECTS,
            connect_timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
            first_byte_timeout_secs: DEFAULT_FIRST_BYTE_TIMEOUT_SECS,
            body_timeout_secs: DEFAULT_BODY_TIMEOUT_SECS,
            pool_idle_timeout_secs: DEFAULT_POOL_IDLE_TIMEOUT_SECS,
            pool_max_idle_per_host: DEFAULT_POOL_MAX_IDLE_PER_HOST,
            top_level_site: None,
            proxy: ProxyConfig::from_env(),
            enable_hsts: true,
            enable_alt_svc: true,
            http_version_preference: HttpVersionPreference::Auto,
            http3: Http3Config::default(),
            telemetry_dir: None,
            disk_cache_dir: None,
            disk_cache_max_bytes: DEFAULT_DISK_CACHE_MAX_BYTES,
            strict_transport_evidence: false,
            dns_mode: DnsMode::System,
        }
    }
}

/// The main HTTP client.
pub struct HttpClient {
    inner: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector<
            HyperDnsResolver,
        >>,
        Full<Bytes>,
    >,
    config: ClientConfig,
    cookies: CookieJar,
    cache: Mutex<HttpCache>,
    request_counter: AtomicU64,
    network_records: Mutex<Vec<NetworkRecord>>,
    network_stats: Mutex<NetworkStats>,
    hsts_store: Mutex<HstsStore>,
    alt_svc_cache: Mutex<AltSvcCache>,
    telemetry_store: Option<Mutex<TelemetryStore>>,
    disk_cache: Option<Mutex<DiskCache>>,
    revalidation_jobs: Mutex<Vec<(VexUrl, HashMap<String, String>)>>,
    transport_security: Mutex<TransportSecurityPolicy>,
}

impl HttpClient {
    /// Create a new HTTP client with default configuration.
    pub fn new() -> VexResult<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new HTTP client with custom configuration.
    pub fn with_config(config: ClientConfig) -> VexResult<Self> {
        validate_http3_attempt(&config.http3, config.http_version_preference)?;

        let tls = tls::tls_config()?;
        let dns_resolver = DnsResolver::new(config.dns_mode.clone())?;
        let hyper_dns = HyperDnsResolver::new(dns_resolver);

        let mut http = HttpConnector::new_with_resolver(hyper_dns);
        http.enforce_http(false);
        http.set_nodelay(true);
        http.set_happy_eyeballs_timeout(Some(Duration::from_millis(300)));
        http.set_connect_timeout(Some(Duration::from_secs(config.connect_timeout_secs)));

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config((*tls).clone())
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http);

        let mut client_builder = Client::builder(TokioExecutor::new());
        client_builder.pool_timer(hyper_util::rt::TokioTimer::new());
        client_builder.pool_idle_timeout(Duration::from_secs(config.pool_idle_timeout_secs));
        client_builder.pool_max_idle_per_host(config.pool_max_idle_per_host);
        let inner = client_builder.build(https);

        let telemetry_store = match config.telemetry_dir.as_ref() {
            Some(dir) => Some(Mutex::new(TelemetryStore::new(dir)?)),
            None => None,
        };

        let disk_cache = match config.disk_cache_dir.as_ref() {
            Some(dir) => Some(Mutex::new(DiskCache::new(DiskCacheConfig {
                dir: dir.clone(),
                max_bytes: config.disk_cache_max_bytes,
            })?)),
            None => None,
        };

        Ok(Self {
            inner,
            config,
            cookies: CookieJar::new(),
            cache: Mutex::new(HttpCache::new()),
            request_counter: AtomicU64::new(0),
            network_records: Mutex::new(Vec::new()),
            network_stats: Mutex::new(NetworkStats::default()),
            hsts_store: Mutex::new(HstsStore::new()),
            alt_svc_cache: Mutex::new(AltSvcCache::new()),
            telemetry_store,
            disk_cache,
            revalidation_jobs: Mutex::new(Vec::new()),
            transport_security: Mutex::new(TransportSecurityPolicy::new()),
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

    /// Snapshot all collected network records.
    pub fn network_records(&self) -> Vec<NetworkRecord> {
        self.network_records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }

    /// Drain and return all collected network records.
    pub fn take_network_records(&self) -> Vec<NetworkRecord> {
        self.network_records
            .lock()
            .map(|mut records| std::mem::take(&mut *records))
            .unwrap_or_default()
    }

    /// Snapshot aggregated network stats.
    pub fn network_stats(&self) -> NetworkStats {
        self.network_stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    /// Drain pending stale-while-revalidate jobs.
    pub fn take_revalidation_jobs(&self) -> Vec<(VexUrl, HashMap<String, String>)> {
        self.revalidation_jobs
            .lock()
            .map(|mut jobs| std::mem::take(&mut *jobs))
            .unwrap_or_default()
    }

    /// Execute all queued stale-while-revalidate jobs.
    pub async fn run_pending_revalidations(&self) -> usize {
        let jobs = self.take_revalidation_jobs();
        let count = jobs.len();

        for (url, headers) in jobs {
            let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = self
                .do_fetch(request_id, &url, Method::Get, &headers, &None)
                .await;
        }

        count
    }

    /// Fetch a URL, returning the response.
    ///
    /// Handles: TLS, decompression, redirects, cookies.
    pub async fn fetch(&self, request: Request) -> VexResult<Response> {
        self.fetch_filtered(request, |_| Ok(())).await
    }

    /// Fetch many requests concurrently.
    ///
    /// This is useful for preload/resource fan-out phases where multiple
    /// independent subresources should be fetched in parallel.
    pub async fn fetch_many(&self, requests: Vec<Request>) -> Vec<VexResult<Response>> {
        join_all(requests.into_iter().map(|request| self.fetch(request))).await
    }

    /// Fetch many requests with a bounded in-flight concurrency.
    ///
    /// Results preserve the input order.
    pub async fn fetch_many_limited(
        &self,
        requests: Vec<Request>,
        max_in_flight: usize,
    ) -> Vec<VexResult<Response>> {
        if requests.is_empty() {
            return Vec::new();
        }

        let limit = max_in_flight.max(1);

        let mut indexed_results = stream::iter(requests.into_iter().enumerate())
            .map(|(idx, request)| async move { (idx, self.fetch(request).await) })
            .buffer_unordered(limit)
            .collect::<Vec<_>>()
            .await;

        indexed_results.sort_by_key(|(idx, _)| *idx);
        indexed_results.into_iter().map(|(_, result)| result).collect()
    }

    /// Fetch many requests with bounded concurrency and cooperative cancellation.
    ///
    /// If `cancel` becomes `true`, pending requests return a cancellation error.
    pub async fn fetch_many_limited_with_cancel(
        &self,
        requests: Vec<Request>,
        max_in_flight: usize,
        cancel: Arc<AtomicBool>,
    ) -> Vec<VexResult<Response>> {
        if requests.is_empty() {
            return Vec::new();
        }

        let limit = max_in_flight.max(1);

        let mut indexed_results = stream::iter(requests.into_iter().enumerate())
            .map(|(idx, request)| {
                let cancel = Arc::clone(&cancel);
                async move {
                    if cancel.load(Ordering::Relaxed) {
                        (
                            idx,
                            Err(VexError::Network(
                                "request cancelled by scheduler".to_string(),
                            )),
                        )
                    } else {
                        (idx, self.fetch(request).await)
                    }
                }
            })
            .buffer_unordered(limit)
            .collect::<Vec<_>>()
            .await;

        indexed_results.sort_by_key(|(idx, _)| *idx);
        indexed_results.into_iter().map(|(_, result)| result).collect()
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

        let request_id = self.request_counter.fetch_add(1, Ordering::Relaxed) + 1;

        let mut current_url = request.url.clone();
        let mut current_method = request.method;
        let mut current_headers = request.headers.clone();
        if let Some(site) = self.config.top_level_site.as_deref() {
            current_headers.insert(INTERNAL_PARTITION_HEADER.to_string(), site.to_string());
        }

        if let Some(proxy_url) = self.config.proxy.proxy_for(&current_url) {
            current_headers
                .entry("x-vigo-proxy".to_string())
                .or_insert_with(|| proxy_url.to_string());
        }

        let mut current_body = request.body.clone();
        let mut redirects = 0u8;

        loop {
            if self.config.enable_hsts {
                if let Ok(store) = self.hsts_store.lock() {
                    if let Some(upgraded) = store.upgrade_url(&current_url) {
                        current_url = upgraded;
                    }
                }
            }

            let resp = match self
                .do_fetch(
                    request_id,
                    &current_url,
                    current_method,
                    &current_headers,
                    &current_body,
                )
                .await
            {
                Ok(resp) => resp,
                Err(err) if current_method == Method::Get => {
                    let stale = self
                        .cache
                        .lock()
                        .ok()
                        .and_then(|cache| cache.get_stale_if_error(&current_url, &current_headers).cloned());

                    if let Some(stale) = stale {
                        warn!(url = %current_url, "network failure served from stale-if-error cache: {err}");
                        self.push_network_record(NetworkRecord {
                            request_id,
                            method: method_name(current_method).to_string(),
                            url: current_url.to_string(),
                            status: Some(stale.status),
                            cache_outcome: CacheOutcome::StaleIfErrorHit,
                            was_cached: true,
                            timings: NetworkTimings {
                                dns_ms: None,
                                ttfb_ms: None,
                                body_read_ms: None,
                                total_ms: 0,
                            },
                            error: Some(err.to_string()),
                        });
                        return Ok(cached_to_response(&current_url, &stale));
                    }

                    if is_connect_error(&err) {
                        warn!(url = %current_url, "transient connect failure, retrying once");
                        self.do_fetch(
                            request_id,
                            &current_url,
                            current_method,
                            &current_headers,
                            &current_body,
                        )
                        .await?
                    } else {
                        return Err(err);
                    }
                }
                Err(err) => return Err(err),
            };

            // Handle redirects
            if self.config.follow_redirects
                && is_redirect(resp.status)
                && redirects < self.config.max_redirects
            {
                if let Some(location) = resp.headers.get("location") {
                    let prev_url = current_url.clone();
                    let next_url = resolve_redirect(&current_url, location)?;
                    debug!(status = resp.status, to = %next_url, "following redirect");

                    // 303: always change to GET
                    if resp.status == 303 {
                        current_method = Method::Get;
                        current_body = None;
                        strip_entity_headers(&mut current_headers);
                    }
                    // 301/302: change to GET for POST (historical browser behavior)
                    if (resp.status == 301 || resp.status == 302) && current_method == Method::Post
                    {
                        current_method = Method::Get;
                        current_body = None;
                        strip_entity_headers(&mut current_headers);
                    }

                    if is_https_downgrade(&prev_url, &next_url) {
                        warn!(
                            from = %prev_url,
                            to = %next_url,
                            "redirect attempted HTTPS->HTTP downgrade; sensitive headers will be stripped"
                        );
                    }

                    sanitize_headers_for_redirect(&prev_url, &next_url, &mut current_headers);

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
        request_id: u64,
        url: &VexUrl,
        method: Method,
        extra_headers: &HashMap<String, String>,
        body: &Option<Vec<u8>>,
    ) -> VexResult<Response> {
        let started = Instant::now();
        let dns_ms: Option<u64> = None;

        let requested_range = if method == Method::Get {
            parse_request_range(extra_headers)
        } else {
            None
        };

        if let Some((start, end)) = requested_range {
            if let Ok(cache) = self.cache.lock() {
                if let Some(range_hit) = cache.get_range(url, extra_headers, start, end) {
                    self.push_network_record(NetworkRecord {
                        request_id,
                        method: method_name(method).to_string(),
                        url: url.to_string(),
                        status: Some(range_hit.status),
                        cache_outcome: CacheOutcome::FreshHit,
                        was_cached: true,
                        timings: NetworkTimings {
                            dns_ms,
                            ttfb_ms: None,
                            body_read_ms: None,
                            total_ms: millis(started.elapsed()),
                        },
                        error: None,
                    });

                    return Ok(Response {
                        status: range_hit.status,
                        headers: range_hit.headers.clone(),
                        body: range_hit.body.clone(),
                        url: url.clone(),
                        was_cached: true,
                    });
                }
            }
        }

        // ── Cache lookup (GET only) ──────────────────────────────────
        if method == Method::Get {
            if let Ok(cache) = self.cache.lock() {
                if let Some(cached) = cache.get(url, extra_headers) {
                    if cached.is_fresh() {
                        debug!(url = %url, "cache hit (fresh)");
                        self.push_network_record(NetworkRecord {
                            request_id,
                            method: method_name(method).to_string(),
                            url: url.to_string(),
                            status: Some(cached.status),
                            cache_outcome: CacheOutcome::FreshHit,
                            was_cached: true,
                            timings: NetworkTimings {
                                dns_ms,
                                ttfb_ms: None,
                                body_read_ms: None,
                                total_ms: millis(started.elapsed()),
                            },
                            error: None,
                        });
                        return Ok(cached_to_response(url, cached));
                    }

                    if cached.can_serve_while_revalidating() {
                        debug!(url = %url, "cache hit (stale-while-revalidate)");
                        if let Ok(mut jobs) = self.revalidation_jobs.lock() {
                            jobs.push((url.clone(), extra_headers.clone()));
                        }
                        self.push_network_record(NetworkRecord {
                            request_id,
                            method: method_name(method).to_string(),
                            url: url.to_string(),
                            status: Some(cached.status),
                            cache_outcome: CacheOutcome::StaleWhileRevalidateHit,
                            was_cached: true,
                            timings: NetworkTimings {
                                dns_ms,
                                ttfb_ms: None,
                                body_read_ms: None,
                                total_ms: millis(started.elapsed()),
                            },
                            error: None,
                        });
                        return Ok(cached_to_response(url, cached));
                    }
                }
            }

            if let Some(disk_cache) = self.disk_cache.as_ref() {
                if let Ok(mut disk_cache) = disk_cache.lock() {
                    if let Some(cached) = disk_cache.get(url, extra_headers) {
                        debug!(url = %url, "disk cache hit (fresh)");

                        // Hydrate memory cache with disk hit for faster follow-up access.
                        if let Ok(mut mem_cache) = self.cache.lock() {
                            mem_cache.store(
                                url,
                                extra_headers,
                                cached.status,
                                &cached.headers,
                                &cached.body,
                            );
                        }

                        self.push_network_record(NetworkRecord {
                            request_id,
                            method: method_name(method).to_string(),
                            url: url.to_string(),
                            status: Some(cached.status),
                            cache_outcome: CacheOutcome::FreshHit,
                            was_cached: true,
                            timings: NetworkTimings {
                                dns_ms,
                                ttfb_ms: None,
                                body_read_ms: None,
                                total_ms: millis(started.elapsed()),
                            },
                            error: None,
                        });
                        return Ok(cached_to_response(url, &cached));
                    }
                }
            }
        }

        // Grab conditional-request validators before the network call
        let (cached_etag, cached_last_modified) = if method == Method::Get {
            self.cache
                .lock()
                .ok()
                .and_then(|c| {
                    c.get(url, extra_headers)
                        .map(|e| (e.etag.clone(), e.last_modified.clone()))
                })
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

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

        if self.config.enable_alt_svc {
            if let Ok(alt_svc) = self.alt_svc_cache.lock() {
                if let Some(alt) = alt_svc.preferred(url, "h3") {
                    builder = builder.header("alt-used", alt.authority.as_str());
                }
            }
        }

        // Conditional headers for cache revalidation
        if let Some(ref etag) = cached_etag {
            builder = builder.header("if-none-match", etag.as_str());
        }
        if let Some(ref lm) = cached_last_modified {
            builder = builder.header("if-modified-since", lm.as_str());
        }

        // Add cookies
        if let Some(cookie_header) = self.cookies.get_cookies_filtered_with_context(
            url,
            crate::cookies::CookieAccess::HttpRequest,
            crate::cookies::NavigationKind::SameSite,
            self.config.top_level_site.as_deref(),
        ) {
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
        let ttfb_started = Instant::now();
        let hyper_resp = match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.first_byte_timeout_secs),
            self.inner.request(hyper_req),
        )
        .await
        {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                let msg = format!("request failed: {e}; debug={e:?}");
                self.push_network_record(NetworkRecord {
                    request_id,
                    method: method_name(method).to_string(),
                    url: url.to_string(),
                    status: None,
                    cache_outcome: CacheOutcome::Miss,
                    was_cached: false,
                    timings: NetworkTimings {
                        dns_ms,
                        ttfb_ms: Some(millis(ttfb_started.elapsed())),
                        body_read_ms: None,
                        total_ms: millis(started.elapsed()),
                    },
                    error: Some(msg.clone()),
                });
                return Err(VexError::Network(msg));
            }
            Err(_) => {
                let msg = format!(
                    "request timed out waiting for first byte after {}s",
                    self.config.first_byte_timeout_secs
                );
                self.push_network_record(NetworkRecord {
                    request_id,
                    method: method_name(method).to_string(),
                    url: url.to_string(),
                    status: None,
                    cache_outcome: CacheOutcome::Miss,
                    was_cached: false,
                    timings: NetworkTimings {
                        dns_ms,
                        ttfb_ms: Some(millis(ttfb_started.elapsed())),
                        body_read_ms: None,
                        total_ms: millis(started.elapsed()),
                    },
                    error: Some(msg.clone()),
                });
                return Err(VexError::Network(msg));
            }
        };
        let ttfb_ms = Some(millis(ttfb_started.elapsed()));

        let status = hyper_resp.status().as_u16();

        // Collect response headers
        let mut headers = HashMap::new();
        for (k, v) in hyper_resp.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.as_str().to_string(), val.to_string());
            }
        }

        if self.config.enable_hsts {
            if let Ok(mut hsts) = self.hsts_store.lock() {
                hsts.observe_response(url, &headers);
            }
        }
        if let Ok(mut sec) = self.transport_security.lock() {
            sec.observe_response(url, &headers);
        }
        if self.config.enable_alt_svc {
            if let Ok(mut alt) = self.alt_svc_cache.lock() {
                alt.observe_response(url, &headers);
            }
        }

        if url.is_https() {
            if let Some(host) = url.host() {
                if let Ok(sec) = self.transport_security.lock() {
                    sec.evaluate_tls_evidence(
                        host,
                        false, // future: wire SCT evidence from TLS stack
                        false, // future: wire OCSP evidence from TLS stack
                        self.config.strict_transport_evidence,
                    )?;
                }
            }
        }

        // Handle 304 Not Modified — reuse cached body
        if status == 304 && method == Method::Get {
            if let Ok(mut cache) = self.cache.lock() {
                cache.refresh_from_not_modified(url, extra_headers, &headers);
                if let Some(cached) = cache.get(url, extra_headers) {
                    debug!(url = %url, "304 Not Modified, using cached body");
                    self.push_network_record(NetworkRecord {
                        request_id,
                        method: method_name(method).to_string(),
                        url: url.to_string(),
                        status: Some(cached.status),
                        cache_outcome: CacheOutcome::Revalidated304,
                        was_cached: true,
                        timings: NetworkTimings {
                            dns_ms,
                            ttfb_ms,
                            body_read_ms: None,
                            total_ms: millis(started.elapsed()),
                        },
                        error: None,
                    });
                    return Ok(cached_to_response(url, cached));
                }
            }

            if let Some(disk_cache) = self.disk_cache.as_ref() {
                if let Ok(mut disk_cache) = disk_cache.lock() {
                    let _ = disk_cache.refresh_from_not_modified(url, extra_headers, &headers);
                }
            }
        }

        // Store cookies from Set-Cookie headers
        for value in hyper_resp.headers().get_all("set-cookie") {
            if let Ok(v) = value.to_str() {
                self.cookies
                    .insert_with_context(url, v, self.config.top_level_site.as_deref());
            }
        }

        // Read body
        let body_started = Instant::now();
        let raw_body = match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.body_timeout_secs),
            hyper_resp.into_body().collect(),
        )
        .await
        {
            Ok(Ok(body)) => body.to_bytes().to_vec(),
            Ok(Err(e)) => {
                let msg = format!("failed to read body: {e}");
                self.push_network_record(NetworkRecord {
                    request_id,
                    method: method_name(method).to_string(),
                    url: url.to_string(),
                    status: Some(status),
                    cache_outcome: CacheOutcome::Miss,
                    was_cached: false,
                    timings: NetworkTimings {
                        dns_ms,
                        ttfb_ms,
                        body_read_ms: Some(millis(body_started.elapsed())),
                        total_ms: millis(started.elapsed()),
                    },
                    error: Some(msg.clone()),
                });
                return Err(VexError::Network(msg));
            }
            Err(_) => {
                let msg = format!(
                    "request body read timed out after {}s",
                    self.config.body_timeout_secs
                );
                self.push_network_record(NetworkRecord {
                    request_id,
                    method: method_name(method).to_string(),
                    url: url.to_string(),
                    status: Some(status),
                    cache_outcome: CacheOutcome::Miss,
                    was_cached: false,
                    timings: NetworkTimings {
                        dns_ms,
                        ttfb_ms,
                        body_read_ms: Some(millis(body_started.elapsed())),
                        total_ms: millis(started.elapsed()),
                    },
                    error: Some(msg.clone()),
                });
                return Err(VexError::Network(msg));
            }
        };
        let body_read_ms = Some(millis(body_started.elapsed()));

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
                cache.store(url, extra_headers, status, &headers, &body);
                if status == 206 {
                    if let Some((start, end)) = parse_content_range(&headers) {
                        cache.store_range(url, extra_headers, start, end, status, &headers, &body);
                    }
                }
            }

            if let Some(disk_cache) = self.disk_cache.as_ref() {
                if let Ok(mut disk_cache) = disk_cache.lock() {
                    let _ = disk_cache.store(url, extra_headers, status, &headers, &body);
                }
            }
        }

        debug!(status, bytes = body.len(), "response received");

        self.push_network_record(NetworkRecord {
            request_id,
            method: method_name(method).to_string(),
            url: url.to_string(),
            status: Some(status),
            cache_outcome: CacheOutcome::Miss,
            was_cached: false,
            timings: NetworkTimings {
                dns_ms,
                ttfb_ms,
                body_read_ms,
                total_ms: millis(started.elapsed()),
            },
            error: None,
        });

        Ok(Response {
            status,
            headers,
            body,
            url: url.clone(),
            was_cached: false,
        })
    }

    fn push_network_record(&self, record: NetworkRecord) {
        if let Ok(mut stats) = self.network_stats.lock() {
            stats.total_requests += 1;

            if record.was_cached {
                stats.cache_hits += 1;
            } else {
                stats.cache_misses += 1;
            }

            if record.error.is_some() {
                stats.errors += 1;
            }

            match record.cache_outcome {
                CacheOutcome::Revalidated304 => stats.revalidated_304 += 1,
                CacheOutcome::StaleIfErrorHit => stats.stale_if_error_hits += 1,
                _ => {}
            }
        }

        if let Some(store) = self.telemetry_store.as_ref() {
            if let Ok(store) = store.lock() {
                let _ = store.append_record(&record);
                let _ = store.write_stats(&self.network_stats());
            }
        }

        if let Ok(mut records) = self.network_records.lock() {
            records.push(record);
        }
    }
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Check whether the error looks like a transient TCP/TLS connect failure.
fn is_connect_error(err: &VexError) -> bool {
    match err {
        VexError::Network(msg) => {
            let m = msg.to_lowercase();
            m.contains("connect")
                || m.contains("tcp")
                || m.contains("reset")
                || m.contains("refused")
                || m.contains("timed out")
        }
        VexError::Io(_) => true,
        _ => false,
    }
}

fn resolve_redirect(base: &VexUrl, location: &str) -> VexResult<VexUrl> {
    // Location can be absolute or relative
    if location.starts_with("http://") || location.starts_with("https://") {
        VexUrl::parse(location)
    } else {
        base.join(location)
    }
}

fn millis(d: Duration) -> u64 {
    d.as_millis().min(u64::MAX as u128) as u64
}

fn method_name(method: Method) -> &'static str {
    match method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
        Method::Head => "HEAD",
        Method::Options => "OPTIONS",
    }
}

fn parse_request_range(headers: &HashMap<String, String>) -> Option<(u64, u64)> {
    let value = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("range"))
        .map(|(_, v)| v.as_str())?;

    let bytes = value.strip_prefix("bytes=")?;
    let (start, end) = bytes.split_once('-')?;
    let start = start.trim().parse::<u64>().ok()?;
    let end = end.trim().parse::<u64>().ok()?;
    if end < start {
        return None;
    }
    Some((start, end))
}

fn parse_content_range(headers: &HashMap<String, String>) -> Option<(u64, u64)> {
    let value = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-range"))
        .map(|(_, v)| v.as_str())?;

    // bytes START-END/TOTAL
    let value = value.strip_prefix("bytes ")?;
    let (range, _) = value.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let start = start.trim().parse::<u64>().ok()?;
    let end = end.trim().parse::<u64>().ok()?;
    if end < start {
        return None;
    }
    Some((start, end))
}

fn cached_to_response(url: &VexUrl, cached: &crate::cache::CachedResponse) -> Response {
    Response {
        status: cached.status,
        headers: cached.headers.clone(),
        body: cached.body.clone(),
        url: url.clone(),
        was_cached: true,
    }
}

fn strip_entity_headers(headers: &mut HashMap<String, String>) {
    headers.retain(|k, _| {
        let lk = k.to_ascii_lowercase();
        lk != "content-type" && lk != "content-length" && lk != "content-encoding"
    });
}

fn same_origin(a: &VexUrl, b: &VexUrl) -> bool {
    a.inner().scheme() == b.inner().scheme()
        && a.inner().host_str() == b.inner().host_str()
        && a.inner().port_or_known_default() == b.inner().port_or_known_default()
}

fn sanitize_headers_for_redirect(
    from: &VexUrl,
    to: &VexUrl,
    headers: &mut HashMap<String, String>,
) {
    let cross_origin = !same_origin(from, to);
    let https_to_http_downgrade = is_https_downgrade(from, to);

    if cross_origin || https_to_http_downgrade {
        headers.retain(|k, _| {
            let lk = k.to_ascii_lowercase();
            lk != "authorization" && lk != "proxy-authorization" && lk != "cookie"
        });
    }
}

fn is_https_downgrade(from: &VexUrl, to: &VexUrl) -> bool {
    from.is_https() && to.scheme() == "http"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_system_dns() {
        let config = ClientConfig::default();
        assert!(matches!(config.dns_mode, DnsMode::System));
    }

    #[tokio::test]
    async fn fetch_many_with_empty_input_returns_empty() {
        let client = HttpClient::new().expect("client should init");
        let results = client.fetch_many(Vec::new()).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn fetch_many_limited_with_empty_input_returns_empty() {
        let client = HttpClient::new().expect("client should init");
        let results = client.fetch_many_limited(Vec::new(), 4).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn fetch_many_limited_with_cancel_on_empty_input_returns_empty() {
        let client = HttpClient::new().expect("client should init");
        let cancel = Arc::new(AtomicBool::new(false));
        let results = client
            .fetch_many_limited_with_cancel(Vec::new(), 4, cancel)
            .await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn fetch_many_limited_with_cancel_short_circuits_requests() {
        let client = HttpClient::new().expect("client should init");
        let cancel = Arc::new(AtomicBool::new(true));
        let req = Request::get("https://example.com").expect("request build");

        let results = client
            .fetch_many_limited_with_cancel(vec![req], 1, cancel)
            .await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn redirect_sanitization_strips_credentials_on_cross_origin() {
        let from = VexUrl::parse("https://a.example/path").unwrap();
        let to = VexUrl::parse("https://b.example/next").unwrap();
        let mut headers = HashMap::from([
            ("Authorization".to_string(), "Bearer x".to_string()),
            ("Cookie".to_string(), "sid=1".to_string()),
            ("X-Test".to_string(), "ok".to_string()),
        ]);

        sanitize_headers_for_redirect(&from, &to, &mut headers);

        assert!(headers.contains_key("X-Test"));
        assert!(!headers.contains_key("Authorization"));
        assert!(!headers.contains_key("Cookie"));
    }

    #[test]
    fn redirect_sanitization_keeps_credentials_same_origin() {
        let from = VexUrl::parse("https://example.com/a").unwrap();
        let to = VexUrl::parse("https://example.com/b").unwrap();
        let mut headers = HashMap::from([("authorization".to_string(), "Bearer x".to_string())]);

        sanitize_headers_for_redirect(&from, &to, &mut headers);
        assert!(headers.contains_key("authorization"));
    }

    #[test]
    fn strip_entity_headers_removes_content_headers() {
        let mut headers = HashMap::from([
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Content-Length".to_string(), "12".to_string()),
            ("X-Test".to_string(), "ok".to_string()),
        ]);

        strip_entity_headers(&mut headers);
        assert!(headers.contains_key("X-Test"));
        assert!(!headers.contains_key("Content-Type"));
        assert!(!headers.contains_key("Content-Length"));
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
        let req_headers = HashMap::new();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        {
            let mut cache = client.cache().lock().unwrap();
            cache.store(&url, &req_headers, 200, &headers, b"cached body");
        }

        let cache = client.cache().lock().unwrap();
        let entry = cache.get(&url, &req_headers).unwrap();
        assert_eq!(entry.status, 200);
        assert_eq!(entry.body, b"cached body");
        assert!(entry.is_fresh());
    }

    #[test]
    fn network_stats_initially_zeroed() {
        let client = HttpClient::new().expect("client init");
        let stats = client.network_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn default_config_enables_hsts_and_alt_svc() {
        let cfg = ClientConfig::default();
        assert!(cfg.enable_hsts);
        assert!(cfg.enable_alt_svc);
    }
}
