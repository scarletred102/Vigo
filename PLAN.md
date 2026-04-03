# Vigo Browser Development Plan (Ground-Up Roadmap)

This plan outlines the chronological sequence for building the Vigo browser from scratch. Each phase is mapped directly to our workspace's project architecture and crates, ensuring that dependencies are satisfied before subsequent layers are built.

## Phase 1: The Foundation (Networking & Data)
Before rendering, we must be able to securely fetch data from the internet.
* **Crate:** `crates/vex-net`
* **Tasks:**
  - Build a highly concurrent HTTP/HTTPS client.
  - Implement DNS resolution (and DoH).
  - Manage TCP and TLS 1.3 handshakes.
  - Implement the Network Resource Cache (`Cache-Control`, `ETags`).
  - *Dependency:* Openssl/Rustls support for `vex-crypto`.

### Phase 1.5: Production Hardening Track (Real-Browser Quality)
We do **not** treat Phase 1 as a demo milestone. This track upgrades `vex-net` to production browser behavior before moving to Phase 2.

* **Goal:** Standards-aligned, secure-by-default, observable network layer that can support real-world browsing workloads.
* **Scope:** `crates/vex-net`, plus integration hooks for `vex-privacy`, `vex-security`, `vex-storage`.
* **Status:** ✅ Completed (2026-04-02)

#### Workstream A — Transport & Connection Management
- [x] Wire DNS resolution results into actual connection dialing via `HttpConnector::new_with_resolver(HyperDnsResolver)`.
- [x] Add explicit connection pool controls (idle per host, idle timeout).
- [x] Split timeout model: connect timeout, first-byte timeout, full-body timeout.
- [x] Add bounded-concurrency fetch API (`fetch_many_limited`) and request cancellation hooks (`fetch_many_limited_with_cancel`).

#### Workstream B — HTTP Cache Correctness
- [x] Support `Expires` and `Age` semantics in freshness calculations.
- [x] Add `Vary`-aware cache keys/variants to prevent incorrect variant reuse.
- [x] Add stale serving policies (`stale-if-error`, `stale-while-revalidate`).
- [x] Expand conditional revalidation behavior and metadata refresh on 304 paths.

#### Workstream C — Cookie Compliance
- [x] Enforce `SameSite=None` + `Secure` requirement strictly.
- [x] Implement cookie prefix hardening (`__Secure-`, `__Host-`).
- [x] Canonicalize domain/path handling and deterministic cookie ordering.
- [x] Add persistent cookie jar option (`CookieJar::export_json` / `import_json`).

#### Workstream D — Redirect & Header Security
- [x] Strip sensitive headers (`Authorization`, `Cookie`, `Proxy-Authorization`) on cross-origin redirects.
- [x] Drop body/entity headers on method-converting redirects (e.g., POST→GET).
- [x] Add explicit redirect downgrade policy checks (`https` → `http`) and audit logs.

#### Workstream E — Observability & DevTools Readiness
- [x] Add request IDs and structured records (`NetworkRecord`) with cache outcomes.
- [x] Expose timing breakdown (`NetworkTimings`) for DNS/TTFB/body/total timing surfaces.
- [x] Emit aggregate metrics (`NetworkStats`) for cache-hit/miss/revalidate/error/stale-if-error counts.

#### Workstream F — Deterministic Test Infrastructure
- [x] Add local mock-server integration tests (no external internet dependency) for redirects, cache revalidation, stale-if-error fallback, compression, and Vary variants.
- [x] Add parser robustness/property-style tests for cookie parsing, cache-control parsing, and content-encoding handling.
- [x] Keep external network integration tests as optional smoke tests only.

#### Quality Gate for Phase 1.5 Exit
- ✅ `cargo test -p vex-net` passes with robust local integration coverage.
- ✅ `cargo clippy -p vex-net --all-targets -- -D warnings` passes.
- ✅ Core security/correctness scenarios (redirect credential stripping, cache revalidation, cookie policy) are tested and green.

## Phase 2: HTML Parsing & The DOM
Once we fetch the raw bytes, we need to convert them into a structured tree of nodes.
* **Crates:** `crates/vex-html`, `crates/vex-dom`
* **Tasks:**
  - Character decoding (UTF-8 processing).
  - Tokenizer state machine (emitting HTML tokens from raw text).
  - Tree Construction (building the Document Object Model in memory).
  - Handle malformed HTML (tag balancing algorithms).
  - Run the lightweight preload scanner to fetch assets early via `vex-net`.

### Phase 2.5: Production Hardening Track (Parser + DOM Quality)
We treat Phase 2 as production infrastructure, not parser demos.

* **Goal:** Robust, streaming-safe parsing and DOM APIs suitable for real browsing workloads.
* **Scope:** `crates/vex-html`, `crates/vex-dom`.
* **Status:** ✅ Completed (2026-04-02)

#### Workstream A — Decoding & Streaming Robustness
- [x] Add byte-oriented parser API with BOM-aware UTF-8/UTF-16 decoding (`parse_html_bytes`).
- [x] Make incremental parser UTF-8 boundary-safe across chunk splits.
- [x] Ensure invalid byte sequences are handled safely via replacement, not panics.

#### Workstream B — Preload Discovery
- [x] Add lightweight preload candidate extraction for scripts, styles, modulepreload, preload/prefetch links, and images.
- [x] Add deterministic candidate priority ordering and deduplication.
- [x] Support case-insensitive multi-token `rel` parsing.

#### Workstream C — Parser/Sink Resilience
- [x] Replace panic-prone sink paths with graceful fallbacks where possible.
- [x] Preserve malformed-HTML tolerance while avoiding internal hard crashes.

#### Workstream D — DOM API Ergonomics
- [x] Add browser-like document helpers (`document_element`, `head`, `body`).
- [x] Keep query/serialize/event APIs lint-clean under strict clippy.

#### Workstream E — Coverage & Quality Gates
- [x] Increase deep nesting parsing coverage (100-level nesting test).
- [x] Add integration assertions for byte parser API.
- [x] `cargo test -p vex-dom -p vex-html` passes.
- [x] `cargo clippy -p vex-dom -p vex-html --all-targets -- -D warnings` passes.

## Phase 3: The CSS Engine
While building the DOM, we parse stylesheets to determine how elements look.
* **Crate:** `crates/vex-css`
* **Tasks:**
  - Lexing & Parsing CSS text into an Abstract Syntax Tree (AST).
  - Building the CSS Object Model (CSSOM).
  - Resolving the Cascade (processing inheritance, specifity, and computing absolute values like pixels).

### Phase 3.5: Production Hardening Track (CSS Parser + Cascade Quality)
Phase 3 is treated as production-critical style infrastructure, not just parser coverage.

* **Goal:** Correct, media-aware, performance-conscious CSS parsing/cascade behavior suitable for real page workloads.
* **Scope:** `crates/vex-css`, with selector-path optimization support in `crates/vex-dom`.
* **Status:** ✅ Completed (2026-04-02)

#### Workstream A — Parser Robustness
- [x] Harden declaration tokenization to split by `;` safely across strings/comments/function arguments.
- [x] Make `!important` parsing case-insensitive and resilient to trailing whitespace.
- [x] Keep shorthand/longhand expansion behavior stable under improved declaration tokenization.

#### Workstream B — Media Query Correctness
- [x] Extend media condition parsing to support OR semantics (`or` keyword + comma-separated query lists).
- [x] Enforce media-condition filtering during cascade matching (rules apply only when viewport/media matches).
- [x] Add media-gating tests at matching and compute-style integration levels.

#### Workstream C — Cascade Matching Performance
- [x] Add direct element selector matching API (`vex_dom::matches_selector`) to avoid full-tree scans.
- [x] Replace `query_selector_all(...).contains(element)` matching path with direct selector checks in CSS cascade matching.
- [x] Remove fragile hard-coded document-root assumptions from CSS declaration matching.

#### Workstream D — Specificity & Selector Semantics Correctness
- [x] Add canonical specificity extraction from parsed selector structures (`selectors` packed specificity decoding).
- [x] Use parsed-selector specificity in cascade matching with heuristic fallback only for parse-failure cases.
- [x] Add regression tests for modern functional selector specificity semantics (`:where`, `:is`).

#### Workstream E — Quality Gates
- [x] `cargo test -p vex-css` passes.
- [x] `cargo clippy -p vex-css --all-targets -- -D warnings` passes.
- [x] Cross-phase regression gates pass:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css --all-targets -- -D warnings`

## Phase 4: Layout & Geometry
Combine the DOM and CSSOM to figure out the exact physical location of every element on the screen.
* **Crate:** `crates/vex-layout`
* **Tasks:**
  - Construct the Render Tree (dropping `display: none` elements).
  - Implement Flow formatting (Block and Inline Contexts).
  - Implement advanced math engines: Flexbox and CSS Grid.
  - Text layout logic (wrapping, kerning, utilizing HarfBuzz).
  - Reflow engine (fast invalidation when JS modifies styles).

### Phase 4.5: Production Hardening Track (Layout/Geometry Quality)
Phase 4 is treated as browser-core infrastructure, not a demo geometry pass.

* **Goal:** Browser-grade layout behavior with reliable inline flow, positioning, hit-testing, and pragmatic reflow invalidation hooks.
* **Scope:** `crates/vex-layout` (+ selector/layout-tree integration from earlier phases).
* **Status:** ✅ Completed (2026-04-02)

#### Workstream A — Render Tree Fidelity
- [x] Keep `display: none` exclusion stable in layout tree generation.
- [x] Add `display: contents` flattening behavior (children participate without wrapper box generation).
- [x] Preserve non-empty whitespace text nodes for inline formatting participation.

#### Workstream B — Flow Formatting & Text Layout
- [x] Wire inline formatting context into block layout for inline-only block/anonymous containers.
- [x] Activate `TextEngine` in runtime pipeline (not just standalone module/tests).
- [x] Improve auto-height computation from laid-out extents (max child bottom) instead of naive sum-only behavior.
- [x] Improve inline measurement for whitespace handling and inline element text-content sizing.

#### Workstream C — Positioning & Interaction Fidelity
- [x] Add baseline sticky positioning behavior (`position: sticky`) with viewport threshold clamping.
- [x] Make hit-testing scroll-offset aware so scrollable containers map input to visible children correctly.

#### Workstream D — Reflow / Invalidation Hooks
- [x] Add incremental reflow planning API (`ReflowPlan`) with dirty-node tracking.
- [x] Add no-dirty fast path reuse (`reflow_document(..., previous, plan)`) for cheap no-op updates.

#### Workstream E — Phase 4 Quality Gates
- [x] `cargo test -p vex-layout` passes.
- [x] `cargo clippy -p vex-layout --all-targets -- -D warnings` passes.
- [x] Cross-phase regression gates pass:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout --all-targets -- -D warnings`

## Phase 5: Graphics, Paint & Compositing
Translate layout rectangles into hardware-accelerated pixels.
* **Crates:** `crates/vex-render`, `zig/compositor`, `zig/alloc`
* **Tasks:**
  - Generate Paint Records (display lists ordered by `z-index`).
  - Rasterization (converting draw instructions into bitmaps/tiles) via 2D graphics libraries.
  - Separate pages into Layer Trees (for hardware-accelerated transforms).
  - Compositor Thread (`zig/compositor`): Stitch tiles together and send to the GPU via OpenGL/Vulkan for smooth 60fps scrolling.

### Phase 5.5: Production Hardening Track (Graphics/Paint/Compositing Quality)
Phase 5 is treated as a browser-grade rendering stack, not a basic draw pass.

* **Goal:** High-fidelity, GPU-accelerated paint/compositing with practical browser-grade features: layered composition, damage tracking, tile scheduling, image/text correctness, and UX-focused scrolling behavior.
* **Scope:** `crates/vex-render` (with integration hooks for compositor-facing subsystems).
* **Status:** ✅ Completed (2026-04-02)

#### Workstream A — Display List Intelligence
- [x] Added display-list geometric introspection (`command_bounds`, `DisplayList::bounds`).
- [x] Added command-category telemetry (`DisplayListStats`) to support renderer diagnostics and perf analysis.

#### Workstream B — Compositor Infrastructure
- [x] Added compositor layerization heuristics (`build_layers`) inspired by modern browser promotion rules (opacity, transform, positioned, filter, clip, animation).
- [x] Added basic occlusion culling (`cull_fully_occluded`) for opaque top-layer coverage.
- [x] Added tile-grid scheduler (`TileGrid`) for tiled raster/compositing workflows with dirty and viewport-priority tile selection.

#### Workstream C — Damage Tracking & Partial Repaint
- [x] Added display-list diff damage computation (`compute_damage`).
- [x] Added damage merge compaction (`merge_damage`) for reduced redraw region count.

#### Workstream D — Renderer Pipeline Upgrades
- [x] Integrated **image rendering pipeline** (`DrawImage`) via GPU textured quads and CPU/GPU image atlas synchronization.
- [x] Added image upload APIs (`upload_image`, `upload_image_bytes`) and atlas cache visibility (`cached_image_count`).
- [x] Added clip-aware and opacity-aware instance extraction for rect/text/image paths.
- [x] Improved border rendering fidelity with dashed/dotted segmentation instead of solid-only fallback.

#### Workstream E — Painter Fidelity
- [x] Added z-index-aware sibling paint ordering for improved stacking behavior.
- [x] Integrated native-like form control painting path into main display-list painter flow.

#### Workstream F — UX Features Users Expect
- [x] Added smooth scrolling + kinetic fling model in `ScrollState` (`scroll_by_smooth`, `fling`, `tick`, animation state).
- [x] Added fallible screenshot/offscreen APIs (`try_render_to_pixels`) to avoid hard panics in no-adapter/driver-failure scenarios.

#### Workstream G — Phase 5 Quality Gates
- [x] `cargo test -p vex-render` passes.
- [x] `cargo clippy -p vex-render --all-targets -- -D warnings` passes.
- [x] Cross-phase regression gates pass:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render --all-targets -- -D warnings`

### Cross-Phase 3/4/5 Integration Pass (2026-04-03)
This pass verified that CSS state, layout invalidation/reflow, and render/compositor behavior are connected in the live runtime — not just implemented in isolated crates.

* **Status:** ✅ Completed

#### Integration Workstream A — Dynamic Style + Reflow Wiring
- [x] `Tab` now stores parsed stylesheets and recomputes styles during relayout.
- [x] Relayout now routes through `reflow_document` with dirty-node/full-reflow planning support.

#### Integration Workstream B — DOM Interaction State Propagation
- [x] Click/focus handling now updates `ElementState` flags (`:focus`, `:focus-within`, `:checked`) to align selector state with user interaction.
- [x] Default checkbox/radio click behavior now mutates form state and emits input/change events.

#### Integration Workstream C — Runtime Pipeline Connections
- [x] App event loop now uses smooth scrolling path (`scroll_by_smooth`) and frame ticking.
- [x] App frame loop now executes damage diffing (`compute_damage`), tile dirty marking (`TileGrid`), and layer diagnostics (`build_layers` + occlusion cull).
- [x] Browser relayout trigger paths now mark dirty nodes and refresh content bounds after stateful edits/clicks.

#### Integration Workstream D — End-to-End Gates
- [x] `cargo test -p vex-browser -p vex-app -p vex-css -p vex-layout -p vex-render` passes.
- [x] `cargo clippy -p vex-browser -p vex-app -p vex-css -p vex-layout -p vex-render --all-targets -- -D warnings` passes.
- [x] Expanded stack validation passes:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-browser -p vex-app`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-browser -p vex-app --all-targets -- -D warnings`

## Phase 6: JavaScript Execution
Embed a high-performance engine to parse and execute JS.
* **Crate:** `crates/vex-js`
* **Tasks:**
  - Integrate a JS engine (e.g., V8 or SpiderMonkey, or build interpreter bindings).
  - Memory isolation & Garbage Collection bindings.
  - Prepare execution contexts and global objects.

### Phase 6.5: Production Hardening Track (JS Runtime + Runtime Integration)
Phase 6 is treated as a browser-runtime foundation, not just script eval.

* **Goal:** Robust JS execution with realistic browser runtime APIs and reliable mutation-to-render connectivity.
* **Scope:** `crates/vex-js`, `crates/vex-browser`, `crates/vex-app`.
* **Status:** ✅ Completed (2026-04-03)

#### Workstream A — Runtime API Surface Improvements
- [x] Added `requestAnimationFrame` / `cancelAnimationFrame` APIs.
- [x] Added high-resolution timestamp delivery for rAF callbacks.
- [x] Mirrored timer/fetch globals onto `window.*` surface for browser-like parity.

#### Workstream B — JS DOM Mutation Invalidation Bridge
- [x] Added JS-side dirty-node queue (`__vex_dom_dirty_nodes`) and runtime drain API (`take_dom_dirty_nodes`).
- [x] Wired element mutation APIs (`setAttribute`, `removeAttribute`, `appendChild`, `removeChild`, `insertBefore`) to mark dirty nodes.
- [x] Wired `element.style` mutation APIs to mark dirty nodes.

#### Workstream C — Script Lifecycle Fidelity
- [x] Executed async external scripts in tab load pipeline.
- [x] Improved lifecycle sequencing: `DOMContentLoaded` before async script execution, `load` after async/resources.

#### Workstream D — App-Loop Integration (Phase 6 → 3/4/5)
- [x] App loop drains runtime dirty-node queue each frame.
- [x] Dirty nodes now feed layout invalidation + relayout + content-size refresh.
- [x] Resulting display list and render/compositor diagnostics update in the same frame loop.

#### Workstream E — Memory/GC Root Integration
- [x] JS runtime now owns a DOM `GcRootSet` and clears/reseeds roots per document registration.
- [x] Element/document proxy creation paths root referenced DOM nodes for lifetime safety accounting.
- [x] Added runtime regression coverage for proxy-rooted node tracking.

### Cross-Phase 1/2/3/4/5/6 Connectivity Pass (2026-04-03)
This pass validated intended phase boundaries and data flow:

- [x] **Phase 1 → 6**: network stack (`vex-net`) feeds JS `fetch()` and external/async script retrieval.
- [x] **Phase 2 → 6**: DOM tree (`vex-html` + `vex-dom`) is mutable from JS APIs and event callbacks.
- [x] **Phase 6 → 3**: JS DOM/style mutations trigger style recomputation from stored stylesheets.
- [x] **Phase 6 → 4**: JS-driven dirty nodes trigger reflow planning + relayout.
- [x] **Phase 6 → 5**: relayout regenerates display lists and feeds compositor diagnostics/renderer.

#### Cross-Phase 1-6 Quality Gates
- [x] `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app` passes.
- [x] `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings` passes.

## Phase 7: The Bridge & Interactivity
Connect the JS Engine to the Rendering Engine and OS interactions.
* **Crates:** `crates/vex-dom` (bindings), `crates/vex-app`
* **Tasks:**
  - Implement WebIDL & DOM Bindings (allowing JS to call `document.createElement`).
  - Build the core Event Loop (handling clicks, timers, microtasks, `requestAnimationFrame`).

### Phase 7.5: Production Hardening Track (Bridge/Event Loop Integration)
Phase 7 is treated as runtime glue infrastructure with browser-loop behavior requirements.

* **Goal:** Robust JS↔DOM↔layout/render interactivity with event-loop semantics suitable for real-page scripts.
* **Scope:** `crates/vex-js`, `crates/vex-browser`, `crates/vex-app` (+ downstream style/layout/render effects).
* **Status:** ✅ Completed (2026-04-03)

#### Workstream A — DOM Binding Surface
- [x] Document/element binding paths support `createElement`, queries, attribute mutation, tree mutation, events.
- [x] Proxy creation is now tied to runtime GC-root accounting via `GcRootSet`.

#### Workstream B — Event Loop Semantics
- [x] Timers integrated in runtime tick path (`setTimeout` / `setInterval`).
- [x] `requestAnimationFrame` / `cancelAnimationFrame` implemented with high-resolution timestamp callback argument.
- [x] Implemented `queueMicrotask` bridge and runtime microtask queue draining.
- [x] Microtasks are flushed after script eval, event dispatch, lifecycle events, and timer callbacks.

#### Workstream C — Bridge-to-Render Connectivity (1→7)
- [x] JS DOM/style mutations are propagated to app-loop relayout invalidation.
- [x] Relayout updates feed Phase-5 display-list/render/compositor frame path.
- [x] End-to-end event input (click/key/scroll) remains connected through JS callbacks to render updates.

#### Workstream D — Phase 1-7 Quality Gates
- [x] `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app` passes.
- [x] `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings` passes.

## Phase 8: Web APIs, Storage, & Media
Provide standard browser APIs to the isolated web page.
* **Crates:** `crates/vex-storage`, `crates/vex-media`, `zig/media`
* **Tasks:**
  - Implement `localStorage`, `sessionStorage`, and persistent Cookies using embedded SQLite.
  - Hook into OS hardware for Audio/Video decoding (HTML5 `<video>`).
  - Advanced Web APIs (WebSockets via `vex-net`, WebRTC, etc.).

### Phase 8.5: Production Hardening Track (Storage + Media + API Wiring)
Phase 8 is treated as a runtime feature-completeness and persistence/connectivity layer.

* **Goal:** Ensure browser-grade persistence/media APIs are actually wired into live page runtimes.
* **Scope:** `crates/vex-storage`, `crates/vex-media`, `crates/vex-js`, `crates/vex-browser`, `crates/vex-app`.
* **Status:** ✅ Completed (2026-04-03)

#### Subtask Group A — Storage API Wiring (one-by-one)
- [x] **A1**: Wire `window.localStorage` into runtime for every loaded page.
- [x] **A2**: Wire `window.sessionStorage` with tab-scoped persistence across navigations.
- [x] **A3**: Wire `window.indexedDB` in runtime (origin-scoped backing store).
- [x] **A4**: Wire `document.cookie` getter/setter with persistent cookie store.
- [x] **A5**: Mirror storage APIs onto `window.*` and global scope consistency.

#### Subtask Group B — Persistence Hardening
- [x] **B1**: Added persistent cookie store constructor in JS API layer.
- [x] **B2**: Added origin/file-backed IndexedDB builder (`build_indexed_db_with_dir`).
- [x] **B3**: Added runtime storage root setup + in-memory fallback behavior on IO failure.
- [x] **B4**: Added regression tests for storage API exposure and localStorage persistence.

#### Subtask Group C — Network↔Storage Connectivity
- [x] **C1**: Persist `Set-Cookie` response header from phase-1 network fetch pipeline.
- [x] **C2**: Ensure cookies persisted via network are available to phase-8 `document.cookie` path.

#### Subtask Group D — Media Runtime Integration
- [x] **D1**: Added phase-8 media model tracking on tab state (`media_elements`, `media_formats`).
- [x] **D2**: Discover `<video>` / `<audio>` nodes at load-time and map to `vex-media` element models.
- [x] **D3**: Resolve media `src` and `<source>` fallback, detect media formats (`mp4`, `mp3`, etc.).
- [x] **D4**: Added tab-level media discovery/format tests.

#### Subtask Group E — Phase 1-8 Connectivity Validation
- [x] **E1**: Verified phase1→8 runtime flow: network fetch -> cookie persistence -> JS cookie/storage APIs.
- [x] **E2**: Verified DOM phase2→8 flow: media tags map to phase-8 media state models.
- [x] **E3**: Verified phase6/7 event loop remains stable with phase8 APIs wired.

#### Phase 1-8 Quality Gates
- [x] `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media` passes.
- [x] `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media --all-targets -- -D warnings` passes.

## Phase 9: Multi-Process Architecture & Security
Ensure heavy isolation to protect users against malicious scripts.
* **Crates:** `crates/vex-security`, `crates/vex-privacy`, `vigo_adblock`
* **Tasks:**
  - OS-Level Sandboxing (separating UI, Renderer, GPU, and Network into distinct isolated processes).
  - Inter-Process Communication (IPC) pipelines to pass safe messages between processes.
  - Implement Cross-Origin Resource Sharing (CORS) and Content Security Policy (CSP).
  - Integrate native Ad/Tracker blocking at the network edge (`vigo_adblock`).

### Phase 9.5: Production Hardening Track (Security/Privacy/Isolation Integration)
Phase 9 is treated as a security-integration milestone, not only module implementation.

* **Goal:** Activate and connect security/privacy/process-isolation systems in live runtime flows.
* **Scope:** `vex-security`, `vex-privacy`, `vex-browser`, `vex-app`, `vex-net`.
* **Status:** ✅ Completed (2026-04-03)

#### Subtask Group A — Network Edge Privacy (improvised expansion)
- [x] **A1**: Routed top-level navigations through privacy middleware (`fetch_filtered`).
- [x] **A2**: Enabled default ad/tracker blocking domain list for runtime navigation.
- [x] **A3**: Applied privacy middleware to external resource prefetch pipeline.
- [x] **A4**: Preserved tracking-strip, HTTPS upgrade, and header-sanitization flow on outgoing requests.

#### Subtask Group B — CSP/CORS/SOP Activation in Real Pipeline
- [x] **B1**: Connected `secure_fetch` to subresource loading path (`script`/`style`).
- [x] **B2**: Bound CSP extraction from main response headers into a reusable `SecurityContext`.
- [x] **B3**: Enforced CSP + CORS validation during external resource fetch.
- [x] **B4**: Added diagnostics for security rejections at resource-load time.

#### Subtask Group C — Cookie / Security State Continuity
- [x] **C1**: Persisted `Set-Cookie` headers from network responses into shared cookie storage.
- [x] **C2**: Kept phase8 cookie APIs connected while phase9 security filters are active.

#### Subtask Group D — Process Isolation + Sandbox Wiring (single-process milestone bridge)
- [x] **D1**: Added renderer-process synchronization helper in app loop.
- [x] **D2**: Added automatic spawn/mark-running/terminate flow based on active tab lifecycle.
- [x] **D3**: Added sandbox policy application hook on renderer process binding.
- [x] **D4**: Added IPC `LoadUrl` synchronization from tab navigation state to renderer tracking.

#### Subtask Group E — Cross-Phase 1-9 Connectivity
- [x] **E1**: Phase1 network requests now pass through phase9 privacy filtering on top-level navigation.
- [x] **E2**: Phase1/2/6/7 external script/style fetches now pass phase9 secure-fetch checks.
- [x] **E3**: Phase8 storage and cookie persistence remains connected under phase9 filtering.
- [x] **E4**: Phase9 process/sandbox orchestration coexists with prior render/event/runtime loops.

#### Subtask Group F — Regression Coverage
- [x] **F1**: Added tab tests for privacy blocking and cookie-header parsing.
- [x] **F2**: Added app-level test for renderer-process synchronization across tab lifecycle.

#### Phase 1-9 Quality Gates
- [x] `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media -p vex-security -p vex-privacy` passes.
- [x] `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media -p vex-security -p vex-privacy --all-targets -- -D warnings` passes.

## Phase 10: The Browser UI, Extensions & Sync
The final layer: the actual graphical application the user interacts with.
* **Crates:** `crates/vex-browser`, `crates/vex-sync`, `extensions/`, `sync-server/`
* **Tasks:**
  - Window Management: Draw the Tabs, Omnibox, and Bookmarks UI.
  - Extension API: Implement WebExtensions APIs (Manifest V3 compatible) for third-party add-ons.
  - Sync Services: Sync passwords and history via `vex-sync` and the external `sync-server`.

### Phase 10.5: Production Hardening Track (UI/UX + Extensions + Sync)
Phase 10 is treated as a full product integration layer across prior engine phases.

* **Goal:** Ship a connected browser application where UI, extension runtime, and encrypted sync operate as one system.
* **Scope:** `vex-app`, `vex-browser`, `vex-js`, `vex-sync`, `vex-crypto`, `sync-server`.
* **Status:** ✅ Completed (2026-04-03)

#### Subtask Group A — UX Continuity & State Persistence
- [x] **A1**: Load settings from persisted profile storage at startup.
- [x] **A2**: Persist settings on clean shutdown together with session/bookmarks.
- [x] **A3**: Keep session/tab/bookmark persistence connected to the live event loop lifecycle.

#### Subtask Group B — Extension UX Integration (improvised expansion)
- [x] **B1**: Added runtime extension action-bar model rebuild from active extension manifests.
- [x] **B2**: Rendered extension action controls directly into nav-bar composition path.
- [x] **B3**: Added extension action hit-testing + click routing in mouse input path.
- [x] **B4**: Implemented popup flow for browser actions with popup HTML.
- [x] **B5**: Added fallback click-event dispatch into extension runtime messaging when no popup.

#### Subtask Group C — Sync Client Real Transport Activation
- [x] **C1**: Implemented authenticated sync push transport (batch records) in `vex-sync`.
- [x] **C2**: Implemented pull transport with timestamp-based incremental fetch.
- [x] **C3**: Added encrypted collection blob helpers for ergonomic app integration.
- [x] **C4**: Added sync timestamp progression handling in client state.

#### Subtask Group D — App-Level Sync Runtime Orchestration
- [x] **D1**: Added sync runtime bootstrap from environment credentials and KDF key derivation.
- [x] **D2**: Added initial pull-then-push handshake flow on browser startup.
- [x] **D3**: Added periodic sync scheduler in main loop.
- [x] **D4**: Added final sync push on graceful shutdown.
- [x] **D5**: Synced concrete collections: bookmarks, settings, open tabs, and history URL snapshots.

#### Subtask Group E — Phase 1-10 Connectivity
- [x] **E1**: Phase1 networking + phase9 security/privacy layers remain active while phase10 sync/extension flows run.
- [x] **E2**: Phase6/7 JS runtime bridges remain connected to extension message dispatch and browser action events.
- [x] **E3**: Phase8 storage/session continuity remains active with new phase10 settings/sync lifecycle.
- [x] **E4**: Phase9 process/sandbox orchestration remains active in the app loop alongside phase10 UX layers.

#### Subtask Group F — Regression Coverage
- [x] **F1**: Added extension popup-source loader test.
- [x] **F2**: Added extension action hit-test/layout app test.
- [x] **F3**: Added synced-history payload snapshot/dedupe test.
- [x] **F4**: Added synced-settings merge behavior test preserving runtime privacy state.

#### Phase 1-10 Quality Gates
- [x] `cargo test -p vex-core -p vex-net -p vex-dom -p vex-html -p vex-css -p vex-layout -p vex-js -p vex-render -p vex-media -p vex-storage -p vex-security -p vex-privacy -p vex-crypto -p vex-sync -p vex-browser -p vex-app` passes.
- [x] `cargo clippy -p vex-core -p vex-net -p vex-dom -p vex-html -p vex-css -p vex-layout -p vex-js -p vex-render -p vex-media -p vex-storage -p vex-security -p vex-privacy -p vex-crypto -p vex-sync -p vex-browser -p vex-app --all-targets -- -D warnings` passes.
