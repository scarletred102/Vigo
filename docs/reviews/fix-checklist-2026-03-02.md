# Fix Checklist — Review Follow-up (2026-03-02)

Track all review blockers and mark completed items here.

## CI / Build Gates
- [x] Fix clippy failure in `crates/vex-browser/src/extensions/content.rs` (`useless_vec`)
- [x] Re-run `just ci` and confirm clean pass

## Missing Deliverables / Incomplete Tasks
- [x] `P0.4.2` add `crates/vex-media/build.rs`
- [x] `P1.2.7` add `zig/platform/test_window.zig`
- [x] `P2.2.3` wire DNS mode into `HttpClient`
- [x] `P2.6.2` add 10-site HTTPS integration test
- [x] `P2.6.3` add privacy stripping integration test
- [x] `P2.6.4` add cached fetch benchmark
- [x] `P6.3.3` implement subpixel rendering support
- [x] `P6.4.3` implement async image loading pattern

## File / Deliverable Reference Mismatches
- [x] Reconcile `P1.1.2` deliverable path (`src/url.rs` vs `src/vex_url.rs`)
- [x] Reconcile `P4.2.2` (`declaration.rs`) reference
- [x] Reconcile `P4.2.3` (`rule.rs`) reference
- [x] Reconcile `P4.2.4` (`at_rule.rs`) reference

## Convention Compliance
- [x] Add MPL header to `zig/build.zig`
- [x] Remove `.unwrap()` / `.expect()` from non-test library code
- [x] Add `// SAFETY:` comments to all `unsafe` blocks lacking them in library code

## Documentation / Status Consistency
- [x] Reconcile `SESSION_LOG.md` completeness claims with current TASKS/CI reality

---

## Progress Log
- 2026-03-02: Checklist created from review findings.
- 2026-03-02: Added `crates/vex-media/build.rs` and marked `P0.4.2` done.
- 2026-03-02: Added `zig/platform/test_window.zig` (compiles with `zig build-exe`) and marked `P1.2.7` done.
- 2026-03-02: Added MPL header to `zig/build.zig`.
- 2026-03-02: Fixed clippy `useless_vec` in `crates/vex-browser/src/extensions/content.rs`; `just ci` now passes.
- 2026-03-02: Added `// SAFETY:` comments for all previously flagged `unsafe` closures in `vex-js` API modules.
- 2026-03-02: Removed `.unwrap()` / `.expect()` usages from non-test library code paths (scanner now reports 0 matches).
- 2026-03-02: Implemented DNS mode wiring in `vex-net::HttpClient` (including DoH-aware pre-resolution path) and marked `P2.2.3` done.
- 2026-03-02: Added `P2.6.2`/`P2.6.3`/`P2.6.4` network integration tests and benchmark in `crates/vex-net/tests/fetch_test.rs` (all `#[ignore]` for deterministic CI).
- 2026-03-02: Reconciled TASKS deliverable-path mismatches for `P1.1.2` and `P4.2.2`–`P4.2.4`.
- 2026-03-02: Implemented `P6.3.3` subpixel support in glyph atlas (quarter-pixel quantization + subpixel-mask handling).
- 2026-03-02: Implemented `P6.4.3` async image loading pattern in `vex-browser::image_loading` using `tokio::spawn_blocking` decode jobs.
- 2026-03-02: Added a validation note in `SESSION_LOG.md` confirming `just ci` is green and `TASKS.md` has 0 incomplete rows.
