# Phase 1 + Phase 2 Gap Closure Sprint

Status: ✅ Implemented (2026-04-02)

This document tracks the additional hardening work implemented to close the previously identified Phase 1/2 parity gaps.

---

## Phase 1 (Network/Data) closures

### Protocol / transport parity
- [x] Added **HTTP/3 protocol-selection surface** (`http3.rs`) with explicit preference + validation gates.
- [x] Added **Alt-Svc parsing/cache** (`alt_svc.rs`) and runtime request/response integration hooks.
- [x] Added **proxy policy path** (`proxy.rs`) with env + no_proxy support and request integration headers.

### Cache maturity
- [x] Added **persistent disk cache** (`disk_cache.rs`) with JSON index + body files.
- [x] Added **tiered eviction** (cold/warm/hot based on access_count + LRU ordering within tier).
- [x] Added **cache partitioning** via internal top-level partition key (`x-vigo-partition-key`).
- [x] Added **range cache strategy** (`store_range` / `get_range`) for 206 partial content.
- [x] Added **background stale-while-revalidate job queue** (`take_revalidation_jobs`, `run_pending_revalidations`).

### Cookie / storage parity
- [x] Added **public suffix enforcement (embedded suffix safety set)** for Domain attribute validation.
- [x] Added modern cookie attrs parsing/storage:
  - `Partitioned` (CHIPS-style context binding)
  - `Priority`
  - `SameParty`
- [x] Added **encrypted cookie persistence** via `vex-crypto`:
  - `CookieJar::export_encrypted`
  - `CookieJar::import_encrypted`

### Security/network policy
- [x] Added **HSTS policy store + auto-upgrade** (`hsts.rs`) integrated into request flow.
- [x] Added **transport security policy pipeline** (`security_policy.rs`) for:
  - `Expect-CT` observation/enforcement hook
  - `Public-Key-Pins` policy observation hook
  - strict SCT/OCSP evidence evaluation hook

### Telemetry / ops
- [x] Added **persistent telemetry storage** (`telemetry_store.rs`):
  - JSONL request records
  - stats snapshot file
- [x] Integrated persistence writes in client record pipeline.

---

## Phase 2 (HTML/DOM) closures

### Encoding completeness
- [x] Upgraded byte parser to support:
  - BOM-aware UTF-8 / UTF-16LE / UTF-16BE
  - `Content-Type` charset sniffing
  - `<meta charset>` sniffing
  - legacy decoding path through `encoding_rs`
- [x] Added `parse_html_bytes_with_content_type` API.

### Preload integration
- [x] Added **speculative preload scanner** (`speculative.rs`) for streamed chunks.
- [x] Integrated scanner with incremental parser:
  - `HtmlParser::feed_and_scan_preloads`

### DOM spec depth hardening
- [x] Improved MutationObserver with **namespace-aware attribute notifications**.
- [x] Improved IntersectionObserver with **custom root element bounds support**.
- [x] Added additional browser-like document API helpers (`document_element`, `head`, `body`).

### Parser/sink robustness
- [x] Added parser diagnostics layer:
  - `diagnostics.rs`
  - `parse_html_with_diagnostics`
- [x] Kept sink behavior resilient with non-crashing fallbacks.

### Large-scale compatibility trajectory
- [x] Added compatibility-smoke corpus tests (`compat_smoke.rs`).
- [x] Added WPT-style smoke runner script (`scripts/wpt-smoke.ps1`).

---

## Validation

- [x] `cargo test -p vex-net -p vex-html -p vex-dom`
- [x] `cargo clippy -p vex-net -p vex-html -p vex-dom --all-targets -- -D warnings`

Both passing after this sprint.
