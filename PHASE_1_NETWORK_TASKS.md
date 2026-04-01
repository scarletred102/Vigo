# Phase 1 — Foundation (Networking & Data)

Status: ✅ **Completed**
Date: 2026-04-02
Owner: Vigo engine team

This file decomposes Phase 1 from `PLAN.md` into executable tasks and records completion.

---

## Scope (from PLAN.md)
1. Highly concurrent HTTP/HTTPS client.
2. DNS resolution (system + DoH).
3. TCP and TLS 1.3 handshakes.
4. Network resource cache (`Cache-Control`, `ETag`).
5. Rustls/OpenSSL dependency path for cryptographic transport support.

---

## Task Breakdown & Completion

### A) HTTP/HTTPS Client Core
- [x] A1. Build hyper-based client with HTTPS connector and HTTP/1.1 + HTTP/2 enabled.
- [x] A2. Add request timeout and connection tuning (nodelay, happy-eyeballs, connect timeout).
- [x] A3. Add redirect handling (301/302/303/307/308), method rewrite behavior, redirect cap.
- [x] A4. Add response decompression (`gzip`, `deflate`, `br`, `zstd`).
- [x] A5. Add explicit concurrent fan-out API for subresource preloads: `HttpClient::fetch_many`.
- [x] A6. Add bounded-concurrency fan-out API for scheduler-style loading: `HttpClient::fetch_many_limited`.
- [x] A7. Add bounded-concurrency + cancellation API: `fetch_many_limited_with_cancel`.
- [x] A8. Add split timeout model in client config (connect / first-byte / body).
- [x] A9. Add explicit pool tuning config (pool idle timeout, max idle per host).

**Implemented in:** `crates/vex-net/src/client.rs`, `decompress.rs`

### B) DNS Resolution + DoH
- [x] B1. Support resolver modes: `System` and `DoH(provider)`.
- [x] B2. Add DoH providers (Cloudflare/Google).
- [x] B3. Fix system DNS behavior to use platform/system DNS config first.
- [x] B4. Add safe fallback to default resolver when system config cannot load.
- [x] B5. Wire resolver into actual connection dialing path via Hyper connector service.

**Implemented in:** `crates/vex-net/src/dns.rs`

### C) TLS 1.3 + Trust Store
- [x] C1. Build rustls client config.
- [x] C2. Load Mozilla trust roots.
- [x] C3. Load native OS trust roots and merge.
- [x] C4. Enable HTTP protocol negotiation path via hyper-rustls connector.

**Implemented in:** `crates/vex-net/src/tls.rs`

### D) Cache + Revalidation
- [x] D1. Parse `Cache-Control` directives (`max-age`, `no-cache`, `no-store`, etc.).
- [x] D2. Serve fresh GET responses from in-memory cache.
- [x] D3. Send conditional revalidation headers (`If-None-Match`, `If-Modified-Since`).
- [x] D4. Handle `304 Not Modified` by returning cached body.
- [x] D5. Refresh cache metadata on `304` (headers/validators/directives + timestamp).
- [x] D6. Support `Age` and `Expires` in freshness lifetime calculations.
- [x] D7. Add `Vary`-aware cache variant selection.
- [x] D8. Add stale serving support (`stale-if-error`, `stale-while-revalidate`).

**Implemented in:** `crates/vex-net/src/cache.rs`, `client.rs`

### E) Cookie Compliance Hardening
- [x] E1. Enforce strict `SameSite=None` + `Secure` requirement.
- [x] E2. Enforce cookie prefixes: `__Secure-` and `__Host-`.
- [x] E3. Add host-only domain matching behavior and stricter domain checks.
- [x] E4. Ensure deterministic cookie header ordering.
- [x] E5. Add cookie jar JSON persistence helpers (`export_json` / `import_json`).

**Implemented in:** `crates/vex-net/src/cookies.rs`

### F) Redirect & Header Security
- [x] F1. Strip sensitive credentials on cross-origin redirects.
- [x] F2. Strip sensitive credentials on HTTPS→HTTP downgrade redirects.
- [x] F3. Drop request body + entity headers on method-converting redirects.
- [x] F4. Emit explicit audit warning on HTTPS downgrade redirects.

**Implemented in:** `crates/vex-net/src/client.rs`

### G) Telemetry & Metrics
- [x] G1. Add per-request IDs and structured telemetry records (`NetworkRecord`).
- [x] G2. Add timing breakdown (`NetworkTimings`) for TTFB/body/total.
- [x] G3. Add aggregate metrics snapshots (`NetworkStats`).
- [x] G4. Expose record snapshot/drain APIs (`network_records`, `take_network_records`).

**Implemented in:** `crates/vex-net/src/telemetry.rs`, `client.rs`

### H) Deterministic Local Integration Coverage
- [x] H1. Add local redirect-following test using in-process TCP server.
- [x] H2. Add local ETag revalidation + 304 cache reuse test.
- [x] H3. Add local gzip decode test.
- [x] H4. Add local stale-if-error fallback test when origin is unavailable.
- [x] H5. Add local Vary variant isolation test.

**Implemented in:** `crates/vex-net/tests/local_integration_test.rs`

### I) Validation & Quality Gates
- [x] I1. `cargo test -p vex-net` passes.
- [x] I2. `cargo clippy -p vex-net --all-targets -- -D warnings` passes.
- [x] I3. Keep network integration tests in place as ignored tests for online environments.
- [x] I4. Re-validate after each hardening change with strict clippy gate (`-D warnings`).
- [x] I5. Add parser robustness/property-style tests for cache-control, cookie parsing, and content-encoding handling.

---

## Net New Changes in This Phase-1 Execution
1. `DnsResolver::new(DnsMode::System)` now uses `TokioAsyncResolver::tokio_from_system_conf()` with fallback + warning.
2. `HttpCache` gains `refresh_from_not_modified(...)` for 304 metadata refresh.
3. `HttpClient` now applies cache refresh when receiving 304.
4. `HttpClient::fetch_many(...)` added to support explicit concurrent fetch fan-out.
5. Clippy-clean updates in tests/TLS assertions.
6. Redirect safety hardening: cross-origin (and HTTPS→HTTP downgrade) redirects strip `Authorization`, `Proxy-Authorization`, and `Cookie` headers.
7. Redirect method/body hygiene: method-converting redirects now drop request body and entity headers (`Content-Type`, `Content-Length`, `Content-Encoding`).
8. Cache freshness now considers `Age` and `Expires` in addition to `Cache-Control: max-age`.
9. Added bounded concurrency fetch API (`fetch_many_limited`) with input-order-preserving results.
10. Added resolver-backed connector DNS service (`HyperDnsResolver`) so DNS mode affects real connection dialing.
11. Added split timeout config fields (`connect_timeout_secs`, `first_byte_timeout_secs`, `body_timeout_secs`).
12. Added explicit pool config (`pool_idle_timeout_secs`, `pool_max_idle_per_host`).
13. Added cancellable bounded concurrency fetch API (`fetch_many_limited_with_cancel`).
14. Added `Vary`-aware cache variants and stale-serving policies (`stale-if-error`, `stale-while-revalidate`).
15. Hardened cookies with prefix enforcement, host-only semantics, and deterministic ordering.
16. Added cookie jar persistence export/import JSON hooks.
17. Added structured telemetry (`NetworkRecord`) and metrics snapshotting (`NetworkStats`).
18. Added deterministic local integration tests (redirect, revalidation, compression, stale fallback, vary).
19. Added robustness/property-style tests for parser hardening.

---

## Exit Decision

Phase 1 (including production hardening) is considered complete for Vigo’s networking foundation because all required subsystems exist, are wired together, and pass strict test/lint gates with deterministic local integration coverage.

Next: continue Phase 1 maintenance only when requested; otherwise Phase 2 (`vex-html` + `vex-dom`) may begin from `PLAN.md`.
