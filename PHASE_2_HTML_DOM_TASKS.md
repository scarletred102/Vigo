# Phase 2 — HTML Parsing & DOM (Production Execution)

Status: ✅ **Completed**
Date: 2026-04-02
Owner: Vigo engine team

This file tracks Phase 2 execution and the production hardening work added on top of the base plan.

---

## Scope (from PLAN.md)
1. Character decoding.
2. HTML tokenization/tree construction.
3. Malformed HTML handling.
4. DOM construction/query/serialization.
5. Lightweight preload discovery integration surface.

---

## Task Breakdown & Completion

### A) HTML Parsing Core (`vex-html`)
- [x] A1. Full document parsing API (`parse_html`).
- [x] A2. Fragment parsing API (`parse_html_fragment`).
- [x] A3. Streaming parser (`HtmlParser`) for incremental chunks.
- [x] A4. Script/style extraction (`extract_scripts`, `extract_styles`).
- [x] A5. Live-page and benchmark tests maintained.

### B) Byte Decoding & Streaming Hardening
- [x] B1. Add `parse_html_bytes` with BOM-aware UTF-8/UTF-16 decoding.
- [x] B2. Make incremental parsing UTF-8 boundary-safe for split multibyte code points.
- [x] B3. Ensure invalid byte sequences are replaced safely (U+FFFD) without panic.

### C) Lightweight Preload Scanner
- [x] C1. Add `extract_preload_candidates` API.
- [x] C2. Include discovery for: `script[src]`, `link[rel=stylesheet]`, `modulepreload`, `preload/prefetch`, `img[src]`, `source[srcset]`.
- [x] C3. Add deterministic priority ordering + dedupe by `(kind,url)`.

### D) DOM Core (`vex-dom`)
- [x] D1. Arena/tree/traversal/document/query primitives already complete.
- [x] D2. Serializer and selector matching already complete.
- [x] D3. Add browser-like helper accessors: `document_element`, `head`, `body`.
- [x] D4. Resolve strict clippy blocker in `mutation_observer` tests.

### E) Extractor & Sink Robustness
- [x] E1. Style extraction now handles case-insensitive/multi-token `rel` values.
- [x] E2. Inline script extraction trims whitespace-only wrappers.
- [x] E3. Hardened sink fallback behavior to avoid panic-prone edge paths in `elem_name`, `get_template_contents`, and orphan sibling insertion scenarios.

### F) Test Coverage Expansion
- [x] F1. Increased deeply nested parser test from 50 to 100 levels.
- [x] F2. Added parse-bytes integration test to `parse_tests`.
- [x] F3. Added module-level tests for preload extraction and BOM decoding APIs.

### G) Validation & Quality Gates
- [x] G1. `cargo test -p vex-dom -p vex-html` passes.
- [x] G2. `cargo clippy -p vex-dom -p vex-html --all-targets -- -D warnings` passes.

---

## Net New Improvements Implemented
1. `parse_html_bytes` API (UTF-8/UTF-16 BOM aware decoding).
2. Incremental parser now preserves split UTF-8 sequence correctness.
3. New preload extraction subsystem (`preload.rs`) with prioritization and dedupe.
4. Case-insensitive/multi-token stylesheet link detection.
5. `Document` convenience methods for `document_element`, `head`, and `body`.
6. Panic-prone sink paths replaced with safer fallbacks.
7. Phase-2 test coverage increased for depth and byte parsing.

---

## Exit Decision

Phase 2 is complete and production-hardened for Vigo’s HTML + DOM layer.

Next (when requested): Phase 3 (`vex-css`) execution and hardening.
