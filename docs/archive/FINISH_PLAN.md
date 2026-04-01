# Vigo Browser — Finish Plan (Execution Blueprint)

Status: **Active**  
Owner: **Lila**  
Execution mode: **continuous development, checkpoint reporting (not micro-pauses)**

---

## 0) North Star

Build Vigo into a **daily-drivable, standards-aligned, secure, performant browser** with clear quality gates and release discipline.

"Finished" means:
1. Core browsing UX is reliable (tabs, nav, history, forms, downloads, session restore).
2. Web compatibility is measurable and improving (WPT + real-site matrix).
3. Security model is real (process isolation + sandbox + policy enforcement).
4. Performance is budgeted and enforced in CI.
5. Releases are predictable (smoke gates, rollback path, crash telemetry).

---

## 1) Research Baseline (authoritative inputs)

We will actively align implementation to:

- **Web Platform Tests (WPT)** as cross-browser conformance backbone
  - https://web-platform-tests.org/
  - https://github.com/web-platform-tests/wpt
- **wpt.fyi/Interop focus areas** for practical priority targeting
  - https://wpt.fyi/
- **MDN Browser Compat Data (BCD)** for API/feature support targeting
  - https://github.com/mdn/browser-compat-data
- **Can I Use** for market-relevant feature demand
  - https://caniuse.com/

These sources are used to choose what to implement next, not random guesswork.

---

## 2) Component-by-Component Completion Plan

## A. Browser Shell (Product UX)
Scope: window lifecycle, tab strip, omnibox, back/forward/reload, find, bookmarks, downloads, settings, session restore.

Definition of done:
- No loss/corruption of tab/session state across normal restarts.
- Omnibox deterministic behavior (URL vs search heuristics).
- Keyboard shortcuts stable and tested.
- Error pages for DNS/TLS/network/HTTP failures are actionable.

Work items:
- Harden startup/restore semantics (`--fresh`, `--url`, default restore policy).
- Add regression tests for tab close/reopen/session edge cases.
- Add deterministic smoke scenarios for navigation + history state.

---

## B. Networking + Resource Loader
Scope: DNS/DoH, HTTP/1.1/2/3, TLS, redirects, cache semantics, cookies, decompression, secure fetch policies.

Definition of done:
- Correct conditional requests + cache freshness behavior.
- Robust failures/timeouts/retries without UI hangs.
- Cookie policy behavior validated by tests (HttpOnly/SameSite/Secure).

Work items:
- Add network fault-injection tests (timeouts, bad certs, redirect loops).
- Add request lifecycle instrumentation for perf + DevTools.
- Tighten fetch-policy enforcement matrix (SOP/CORS/CSP by context).

---

## C. HTML/CSS/DOM/Layout Core
Scope: parser correctness, selector/cascade validity, layout tree correctness, hit-testing fidelity.

Definition of done:
- Deterministic rendering on golden corpus pages.
- Hit-test correctness for links/forms/interactive controls.
- Reflow/repaint invalidation correctness under DOM mutation.

Work items:
- Build a fixed corpus of reference pages (`refs/`) + screenshot diff gates.
- Expand layout regression tests for floats/flex/grid/positioned edge cases.
- Add stress tests for malformed HTML/CSS inputs.

---

## D. JS Runtime + Web APIs
Scope: script loading lifecycle, DOM bindings, events, timers, fetch, storage, workers, core APIs.

Definition of done:
- Predictable lifecycle (`DOMContentLoaded`, `load`, history/location operations).
- No script-induced UI deadlocks.
- Core API set sufficient for mainstream sites in target compatibility tier.

Work items:
- Maintain API priority queue using BCD + real-site breakage signals.
- Add contract tests for critical APIs (history/navigation/storage/events).
- Improve worker + messaging reliability under load.

---

## E. Rendering + GPU Pipeline
Scope: display list generation, text/image painting, clipping, opacity, scrolling, compositing.

Definition of done:
- Stable frame rendering with no artifact regressions on test corpus.
- Frame-time budgets tracked and enforced.
- GPU fallback/diagnostics clear when device/driver issues occur.

Work items:
- Add frame-time + memory KPI aggregation into CI reports.
- Add screenshot regression suite with thresholds.
- Improve robustness against driver/overlay quirks (Windows Vulkan stack noise).

---

## F. Security + Process Model
Scope: process boundaries, sandboxing, policy enforcement, secure defaults, crash containment.

Definition of done:
- Renderer isolation model functional and test-covered.
- Sandbox restrictions active in production mode.
- Sensitive APIs permission-gated with clear UX.

Work items:
- Move from logical process model toward stricter renderer separation.
- Harden sandbox policy presets + verification tests.
- Expand threat-model checklist (navigation, storage, network, extension boundaries).

---

## G. Storage + Offline
Scope: cookies, localStorage, sessionStorage, IndexedDB, service workers, cache interactions.

Definition of done:
- Persistence guarantees are explicit and tested.
- Data isolation by origin/tab semantics is correct.
- Service worker registration/update/unregister lifecycle reliable.

Work items:
- Add corruption/recovery tests for storage backends.
- Add migration/versioning rules for persisted schema.
- Expand service worker integration scenarios.

---

## H. Media + DRM Strategy
Scope: media playback pipeline, controls, sync, adaptive streaming, DRM fallback behavior.

Definition of done:
- Baseline media playback is reliable for non-DRM content.
- DRM fallback path deterministic and user-transparent.
- Failures produce diagnosable logs, not silent breakage.

Work items:
- Harden startup/shutdown/resource cleanup for media pipeline.
- Add end-to-end playback smoke cases (local + network samples).
- Keep DRM fallback strictly isolated and policy-controlled.

---

## I. DevTools + Extensions
Scope: practical debugging surface and extension runtime safety.

Definition of done:
- Console/network/sources/perf panels useful for real debugging.
- Extension messaging/permissions are constrained and test-covered.

Work items:
- Improve network panel fidelity with timing phases.
- Add extension permission regression tests + abuse checks.
- Define extension API stability policy (versioning/deprecations).

---

## J. Release Engineering + Quality Gates
Scope: automation, metrics, crash analytics, release train.

Definition of done:
- Every merge passes deterministic quality gates.
- Release candidates are reproducible and rollback-ready.
- Crash and perf trends are visible over time.

Work items:
- CI gates: format/lint/tests + smoke launch + KPI floor checks.
- Nightly compatibility run on corpus + prioritized WPT subset.
- Introduce release channels: dev -> beta -> stable.

---

## 3) Execution Order (Practical)

### Stage 1 — Reliable Core (Now)
Focus: shell + nav + loading + rendering determinism + smoke automation.

### Stage 2 — Compatibility Ramp
Focus: real-site matrix + prioritized WPT subsets + API gap closure.

### Stage 3 — Security/Isolation Hardening
Focus: process isolation, sandbox enforcement, policy hardening.

### Stage 4 — Performance + Polish
Focus: startup/frame/memory budgets, GPU robustness, UX polish.

### Stage 5 — Release Discipline
Focus: RC criteria, upgrade strategy, stability telemetry, channel strategy.

---

## 4) Hard Quality Gates (non-negotiable)

Every meaningful change must satisfy:
1. `cargo test --workspace` green.
2. MVP smoke run green (fresh launch + URL load + KPI artifact present).
3. No new warnings in touched area without explicit justification.
4. Commit hygiene: focused commits, no giant mixed diffs.

---

## 5) Immediate Active Sprint (already underway)

Completed in current sprint:
- Startup URL/search deterministic flow.
- Launch options for deterministic runs (`--fresh`, `--url`, `--new-tab`).
- MVP smoke script for repeatable runtime validation.

Next queued (in order):
1. Session restore vs fresh-launch behavior matrix tests.
2. Navigation/history/back-forward deterministic regression suite.
3. Real-site smoke corpus + screenshot baseline harness.
4. Prioritized WPT subset integration and trend reporting.

---

## 6) Reporting Cadence

- No micro milestone interruptions.
- Report only at **major checkpoints**:
  - stage completion,
  - high-risk blocker,
  - quality gate failure requiring decision.

This is the master execution contract for finishing Vigo.
