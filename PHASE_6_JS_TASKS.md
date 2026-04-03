# Phase 6 — JavaScript Execution (Kickoff + Hardening)

Status: ✅ **Started and materially hardened**
Date: 2026-04-03
Scope: `crates/vex-js`, runtime integration via `crates/vex-browser` + `crates/vex-app`

---

## What was implemented in this pass

### 1) JS runtime API improvements
- Added browser-style animation APIs:
  - `requestAnimationFrame(callback)`
  - `cancelAnimationFrame(id)`
- Added high-resolution rAF callback timestamp delivery from runtime time origin.
- Mirrored selected global APIs onto `window` object:
  - `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`,
  - `requestAnimationFrame`, `cancelAnimationFrame`,
  - `fetch`.

### 2) JS DOM mutation -> render invalidation bridge
- Added JS dirty-node queue in `vex-js` (`api/dom_dirty.rs`).
- Added runtime drain API:
  - `JsRuntime::take_dom_dirty_nodes()`
- Wired mutation APIs to mark dirty nodes:
  - element mutators: `setAttribute`, `removeAttribute`, `appendChild`, `removeChild`, `insertBefore`
  - style mutators: `style.setProperty`, `style.removeProperty`, named style setters.

### 3) Script lifecycle fidelity
- Added execution of async external scripts in `Tab::load_html_with_resources`.
- Lifecycle sequencing improved:
  - `DOMContentLoaded` fired after deferred scripts,
  - async scripts executed,
  - `load` fired after async/resources.

### 4) Runtime loop integration (full stack)
- `vex-app` main loop now drains JS dirty nodes each frame.
- Dirty nodes are mapped to relayout invalidation:
  - mark dirty node(s),
  - relayout,
  - content-size refresh,
  - subsequent display-list/render/compositor update.

---

## Cross-phase 1→6 connectivity outcome

### Phase 1 (Network) -> Phase 6 (JS)
- `vex-net` now directly supports JS `fetch` + external script retrieval paths.

### Phase 2 (DOM) -> Phase 6 (JS)
- JS DOM APIs mutate shared DOM state (same document graph used by engine).

### Phase 6 (JS) -> Phase 3 (CSS)
- JS mutations trigger style recomputation from stored stylesheet set.

### Phase 6 (JS) -> Phase 4 (Layout)
- Dirty nodes trigger reflow invalidation + relayout.

### Phase 6 (JS) -> Phase 5 (Render/Compositor)
- Relayout refreshes display lists, renderer input, and compositor diagnostics.

---

## Validation (all passed)

- ✅ `cargo test -p vex-js -p vex-browser -p vex-app`
- ✅ `cargo clippy -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings`
- ✅ `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app`
- ✅ `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings`

---

## Files touched (Phase 6 kickoff pass)

- `crates/vex-js/src/api/dom_dirty.rs` (new)
- `crates/vex-js/src/api/mod.rs`
- `crates/vex-js/src/api/timers.rs`
- `crates/vex-js/src/api/element.rs`
- `crates/vex-js/src/api/style_proxy.rs`
- `crates/vex-js/src/api/window.rs`
- `crates/vex-js/src/context.rs`
- `crates/vex-browser/src/tab.rs`
- `crates/vex-app/src/main.rs`
- `PLAN.md`

---

## Next suggested Phase 6 expansions

- Promise/microtask/event-loop ordering fidelity improvements.
- MutationObserver wiring from JS DOM mutations.
- requestIdleCallback / AbortSignal integration with fetch lifecycle.
- broader Web IDL parity for element/document properties (live accessors).
