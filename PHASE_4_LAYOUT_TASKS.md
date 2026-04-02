# Phase 4 — Layout & Geometry (Execution + Hardening)

Status: ✅ **Completed**
Date: 2026-04-02
Scope: `crates/vex-layout`

---

## Baseline weakspot audit

The pre-hardening layout crate had broad module coverage (block/flex/grid/float/positioned/text/hit-test), but high-impact runtime gaps remained:

1. **Inline formatting path was effectively dormant** in the main layout pipeline (block layout did not execute real inline line layout with text shaping).
2. **Text node treatment in tree building was overly lossy** (`trim()`-based skip removed whitespace-only nodes that matter for inline flow).
3. **`display: contents` behavior was not flattened** at render-tree generation.
4. **Hit-testing ignored scroll offsets**, causing pointer mismatch in scrolled containers.
5. **`position: sticky` lacked concrete behavior** (treated as static/placeholder path).
6. **No explicit incremental reflow API** for no-op fast paths when dirty set is empty.

---

## Implemented improvements

### A) Inline flow activation + text fidelity
- Threaded `NodeArena` and `TextEngine` through runtime layout dispatch (`layout_document` → block/flex/grid/float recursion).
- Integrated inline formatting context in block layout for inline-only child runs.
- Upgraded auto-height resolution to use actual laid-out extents (furthest child margin-box bottom), improving multiline/positioned cases.
- Improved inline text measurement:
  - white-space normalization path (`normal`/`nowrap`/`pre`/`pre-wrap`/`pre-line` basics),
  - inline element text-content measurement fallback when width/height are auto.

### B) Render-tree fidelity
- Added `display: contents` flattening logic in tree builder.
- Preserved non-empty text nodes (including whitespace-only text nodes) for inline formatting participation.

### C) Positioning + interaction correctness
- Implemented baseline sticky positioning behavior with viewport threshold clamping and containing-block bounds.
- Made hit-testing scroll-aware by mapping points with ancestor scroll offset during child traversal.

### D) Reflow invalidation API
- Added `reflow.rs` with:
  - `ReflowPlan { dirty_nodes, full_reflow }`,
  - dirty-node marking helpers,
  - `reflow_document(...)` no-dirty fast path (reuse previous tree) + fallback full layout.
- Enabled `LayoutBox: Clone` to support fast-path tree reuse.

---

## Validation

All strict gates passed after implementation:

- ✅ `cargo test -p vex-layout`
- ✅ `cargo clippy -p vex-layout --all-targets -- -D warnings`
- ✅ Cross-phase regression:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout --all-targets -- -D warnings`

---

## Phase 4 exit decision

Phase 4 is complete for the current engine scope and now includes practical browser-grade hardening in:

- render tree generation,
- inline flow/text layout activation,
- sticky/scroll interaction behavior,
- and incremental reflow control surfaces.

Next phase candidate: **Phase 5 (Graphics, Paint & Compositing)** with similar hardening workflow (audit → weakspot closure → strict gates).
