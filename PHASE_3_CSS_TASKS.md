# Phase 3 — CSS Engine (Execution + Hardening)

Status: ✅ **Completed**
Date: 2026-04-02
Scope: `crates/vex-css` (+ selector-path optimization in `crates/vex-dom`)

---

## Baseline audit summary

Initial Phase-3 baseline was already functionally broad (218 tests green), but several production weakspots existed:

1. Cascade matching used `query_selector_all` per selector/per element and then `contains(element)` — expensive and root-assumption-prone.
2. `@media` rules were parsed but not enforced in cascade declaration matching.
3. Declaration splitting used naive `split(';')` and could break on strings/functions/comments.
4. `!important` parsing only detected lower-case exact suffix.
5. Media parser lacked robust OR support (`or` keyword and comma-separated query lists).

---

## Completed improvements

### A) Selector matching path hardening (`vex-dom` + `vex-css`)
- Added `vex_dom::matches_selector(arena, element_id, selector_str) -> Result<bool, String>`.
- Added `vex_dom::matches_selector_list(arena, element_id, &SelectorList)` for parsed-selector matching.
- Refactored `vex-css` cascade matching to use direct element matching instead of full-tree selector scans.
- Added tests for new direct matching API.

### A.1) Specificity correctness hardening (browser-grade selector semantics)
- Added canonical specificity extraction from parsed selector structures (`selector.specificity()` decoding), rather than relying purely on heuristic token counting.
- Wired cascade matching to compute specificity from parsed selectors when available, with heuristic fallback only for parse-failure cases.
- Added tests for high-impact functional pseudo-class semantics:
  - `:where(...)` zero specificity
  - `:is(...)` max-argument specificity

### B) Media-aware cascade correctness
- Updated `collect_matching_declarations` signature to include viewport context.
- Enforced `rule.media_condition` filtering using `parse_media_condition` + `evaluate_media`.
- Added tests validating media-filtered declaration behavior.
- Added integration test in compute pipeline verifying media rule application by viewport.

### C) Parser robustness
- Implemented safe declaration block splitting:
  - respects quoted strings,
  - skips comments,
  - preserves function argument semicolons/commas,
  - avoids malformed naive splitting.
- Added robust `split_important` helper with ASCII case-insensitive `!important` detection.
- Added parser tests for comments, quote/function-safe splitting, and uppercase `!IMPORTANT`.

### D) Media query parser improvements
- Added support for comma-separated query lists as OR semantics.
- Added explicit `or` keyword parsing support.
- Added tests for OR/comma parsing and evaluation.

---

## Phase-3 re-audit (existing-browser weakspot pass)

We re-opened Phase 3 to target real-world weakspots often seen in browser style engines:

1. **Specificity edge-case divergence** on modern selector functions (`:where`, `:is`, `:has`) when implementations use heuristic counters.
2. **Repeated selector parsing/matching overhead** in style recalc hot paths.

Implemented mitigations:
- Parsed-selector list matching API (avoids repeated parse in hot path where pre-parsed selectors are available).
- Canonical specificity path from selector engine internals, reducing edge-case divergence risk.

Re-audit validation:
- ✅ `cargo test -p vex-dom -p vex-css`
- ✅ `cargo clippy -p vex-dom -p vex-css --all-targets -- -D warnings`
- ✅ Cross-phase regression gates remained green.

---

## Quality gates

All gates passed after implementation:

- ✅ `cargo test -p vex-css`
- ✅ `cargo clippy -p vex-css --all-targets -- -D warnings`
- ✅ `cargo test -p vex-dom`
- ✅ `cargo clippy -p vex-dom --all-targets -- -D warnings`
- ✅ Cross-phase regression:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css --all-targets -- -D warnings`

---

## Exit decision

Phase 3 is complete and hardened for production-quality parser/cascade behavior within current engine scope.

Next phase candidate: **Phase 4 (Layout & Geometry)** with similar hardening workflow (audit → weakspot closure → strict gates).
