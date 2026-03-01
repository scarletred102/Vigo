# Vigo / Vex Engine — Session Log

> **Read this first each session.** Single source of truth for what's done, what's working, and what's next.

---

## Current State

**Phase 0 ✅ — Phase 1 ✅ — Phase 2 ✅ — Phase 3 ✅ — Phase 4 (CSS) ✅ — Phase 5 (Layout) ✅ — Phase 6 (GPU Rendering) ✅ — Phase 7 (JavaScript) ✅ — Phase 8 (Browser Chrome) ✅ — Phase 9 (Media) ✅ — Phase 10 (Security, Storage & Web Compat) ✅**

> Validation (2026-03-02): `just ci` passes clean and `TASKS.md` currently has 0 incomplete rows.

The full rendering pipeline is operational: parse HTML → build DOM → extract `<style>` → parse CSS → compute styles → lay out boxes → build display list → GPU render with text, rectangles, and borders. JavaScript engine (Boa 0.19) runs inline/external/defer/async scripts with DOM manipulation, event handling, fetch API, timers, and form interaction. Browser chrome layer provides tab management, navigation history, bookmarks, browsing history, find-in-page, downloads, zoom, keyboard shortcuts, context menus, and settings. Security layer enforces Same-Origin Policy, CORS preflight, CSP Level 2. Storage layer provides persistent cookies, localStorage, sessionStorage, and simplified IndexedDB — all SQLite-backed. Privacy hardening includes canvas fingerprint noise, WebGL parameter masking, and font enumeration restriction. WPT runner framework classifies and triages test results by subsystem.

```bash
cargo run -p vex-app                          # opens window + renders welcome.html via full pipeline
cargo test --workspace                        # 839 tests (16 ignored)
cargo clippy --workspace --all-targets        # 0 warnings
cd zig && zig build test                      # 22 Zig tests
```

---

## Test Summary

| Crate | Tests | Notes |
|-------|------:|-------|
| vex-core | 39 | geometry, color, id, url, error |
| vex-css | 89 | parser, selectors, cascade, computed styles |
| vex-dom | 103 | arena tree, queries, serialize, iterators, forms, text editing, form submission |
| vex-html | 11 | html5ever parsing, fragments, malformed HTML |
| vex-html (parse tests) | 17 | round-trip integration tests |
| vex-html (parse bench) | 3 | 100KB, nested, attribute-heavy benchmarks |
| vex-html (live page) | 2 | `#[ignore]` — network-required e2e tests |
| vex-layout | 43 | block, inline, flex, positioned, stacking, text |
| vex-layout (bench) | 2+1 | 200-element, deep nesting; 5000-element `#[ignore]` |
| vex-net | 33 | client, cookies, dns, decompress, types |
| vex-net (fetch) | 4 | `#[ignore]` — live HTTPS integration tests |
| vex-privacy | 50 | tracking, adblock, HTTPS-only, headers, canvas noise, WebGL masking, font restriction |
| vex-render | 57 | display list, painter, renderer, glyph atlas, image atlas, screenshot, scroll, form painter |
| vex-js | 75 | Boa context, console, timers, fetch, window, document, element, style proxy, events, GC roots, script runner, lifecycle, EME |
| vex-media | 102 | HLS, DASH, ABR, controls, media element, loading, PiP, sync, video render |
| vex-browser | 154 | tabs, navigation, session, error pages, links, chrome, shortcuts, bookmarks, history, find, downloads, zoom, settings, process model, sandbox, crash recovery, WPT runner |
| vex-security | 32 | origin model, SOP enforcement, CORS preflight, CSP parser + enforcement |
| vex-storage | 25 | persistent cookies, localStorage, sessionStorage, IndexedDB |
| vex-browser (drm) | 4 | DRM fallback integration tests |
| doc-tests | 3 | vex-html parser, vex-js context |
| **Total** | **~839 pass, 16 ignored** | 0 failures |
| Zig | 22 | arena, pool, frame allocators |

---

## What's Built

### Rust Crates (16 total, under `crates/`)

| Crate | Status | What it does |
|-------|--------|-------------|
| `vex-core` | **39 tests ✅** | Error types, geometry (Point/Size/Rect/Insets), Color (hex/css/named), VexId (arena index + allocator), VexUrl (url::Url wrapper) |
| `vex-net` | **37 tests ✅** | HTTP/1.1+2 client (hyper+rustls), TLS 1.3, DNS/DoH (hickory), gzip/br/zstd decompression, cookie jar (domain/path/secure/expiry), redirect following (301-308), Request/Response/Method types |
| `vex-privacy` | **50 tests ✅** | Tracking param stripper (50+ params), domain adblock engine (subdomain matching), HTTPS-only mode, header sanitization (X-Client-Data, Sec-Browsing-Topics, Attribution-Reporting), cross-origin referrer reduction, canvas fingerprint protection (deterministic per-session noise), WebGL parameter masking (vendor/renderer/extensions), font enumeration restriction (30 curated web-safe fonts) |
| `vex-dom` | **103 tests ✅** | Arena-allocated DOM tree (Document/Element/Text/Comment/Doctype), tree manipulation (append/insert/remove/reparent), depth-first/children/ancestor iterators, attribute map, query selectors (getElementById, getElementsByTagName/ClassName, querySelector/All), text_content, HTML serializer, form element model (InputState/InputType/FormStateMap), text input editing (insert/delete/cursor/selection), form submission (collect data, URL-encode, build GET/POST requests) |
| `vex-html` | **33 tests ✅** | html5ever TreeSink integration, full-document and fragment parsing, handles malformed HTML/entities/void elements/script raw text, live-page e2e test (example.com + httpbin.org), parse benchmarks (100KB, nested, attribute-heavy) |
| `vex-css` | **89 tests ✅** | Tokenizer, parser (selectors + declarations), specificity, cascade engine, style computation (compute_styles → HashMap<VexId, ComputedStyle>), all CSS value types (length/color/display/position/overflow/flex), UA defaults, inheritance |
| `vex-layout` | **48 tests ✅** | Block layout (width calc, margin collapsing, overflow clip), inline layout (line boxes, text-align), flex layout (grow/shrink, justify-content 6 values, align-items 5 values, wrap), text measurement (cosmic-text), positioned elements (relative/absolute/fixed), stacking contexts, layout pipeline (layout_document), debug dump, performance benchmarks |
| `vex-render` | **57 tests ✅** | Display list (FillRect/DrawBorder/DrawText/DrawImage/PushClip/PopClip/PushOpacity/PopOpacity), painter (walks layout tree → display list, viewport culling, visibility/opacity), GPU renderer (rect + text pipelines, WGSL shaders rect/text/image, instanced drawing, batching, alpha blending), glyph atlas (shelf packing, cosmic-text rasterizer, LRU eviction), image atlas (4096×4096, shelf packing), image decoder (PNG/JPEG/WebP/GIF/BMP), screenshot (offscreen render to PNG, pixel_diff comparison), scroll state, form control painter (text input, checkbox, radio, button, select, password masking), Zig FFI (platform_ffi.rs), Window wrapper, Event enum, wgpu GPU context |
| `vex-app` | **Runs ✅** | Entry point binary. Parses embedded `welcome.html` through full pipeline (HTML→DOM→CSS→Layout→Display List), composites with browser chrome (tab bar, address bar, accent line), renders via Renderer with text + rect pipelines, scroll support, FPS counter |
| `vex-js` | **75 tests ✅** | Boa 0.19 JS engine (JsRuntime context, eval/execute), Console API (log/warn/error/info/debug → tracing), Timer API (setTimeout/setInterval/clearTimeout/clearInterval), Fetch API (Promise-based, delegates to vex-net), Window object (location/history/navigator/dimensions), Document proxy (getElementById/querySelector/createElement), Element proxy (getAttribute/setAttribute/appendChild/removeChild/insertBefore, textContent, innerHTML, className, style), Style proxy (getPropertyValue/setProperty/removeProperty, camelCase ↔ kebab-case), Event bridge (addEventListener/removeEventListener → DOM EventListenerMap, dispatch with JS callback invocation), GC root set (reference-counted VexId tracking), Script runner (ExecutionPlan: blocking/defer/async classification and ordered execution), Lifecycle events (DOMContentLoaded, load), EME interception |
| `vex-browser` | **154 tests ✅** | Tab model (Tab/TabId/LoadingState, load_html/load_url/build_page_pipeline), Tab manager (new/close/switch/move/duplicate/reopen), Navigation history (per-tab back/forward stack), Session state (save/load to JSON), Error pages (DNS/TLS/connection/HTTP/crash templates), Link handling (resolve_link_click, normalize_url_input), UI chrome layout (ChromeLayout with tab_bar/nav_bar/bookmark_bar/content_area regions), Tab bar rendering (render_tab_bar/hit_test_tab_bar → TabBarAction), Nav bar rendering (back/forward/reload/address bar/HTTPS indicator), Keyboard shortcuts (20+ browser actions, Ctrl+T/W/L/R/F, F5/F6/F11/F12), Context menu (page/link/image menus with hit testing), Bookmarks (add/remove/update/search/folders, JSON persistence), Browsing history (record_visit/search/recent, dedup, max records), Find in page (DOM text search, match navigation, case-insensitive), Downloads (start/progress/complete/cancel/clear_finished), Zoom (15 preset levels 25-300%, snap-to-nearest), Settings (general/appearance/privacy/content/downloads, JSON persistence), Process model (ProcessRole/ProcessStatus/IpcMessage, ProcessManager, spawn/terminate), Windows sandbox (SandboxConfig, job objects, restricted tokens), Crash recovery (CrashRecovery, rate-limited auto-restart, error pages), WPT runner (discover/run/report/triage by subsystem) |
| `vex-security` | **32 tests ✅** | Origin model (Tuple/Opaque, from_url, same_origin, serialize), SOP enforcement (dom/fetch/storage access checks), CORS preflight (build/validate preflight, expose headers), CSP Level 2 (parse directive+source expressions, enforce URL/inline with nonce/hash support, default-src fallback) |
| `vex-storage` | **25 tests ✅** | Persistent cookies (SQLite, domain/path/expiry/SameSite), localStorage (per-origin SQLite, 5MB quota), sessionStorage (in-memory per-tab, drop_tab), simplified IndexedDB (SQLite object stores, CRUD, JSON values) |
| `vex-media` | **102 tests ✅** | HLS/DASH parsing, adaptive bitrate, media element model, media loading, picture-in-picture, A/V sync, video rendering, player controls |
| Others | Stubs | `vex-crypto`, `vex-sync` |

### Zig Modules (5, under `zig/`)

| Module | Status | What it does |
|--------|--------|-------------|
| `platform` | **Working ✅** | Win32 windowing — CreateWindowExW, PeekMessageW, WndProc → EventC structs. DPI via GetDpiForWindow. Raw handle extraction (HWND + HINSTANCE). |
| `alloc` | **22 tests ✅** | Arena (bump), Pool (fixed-size blocks), Frame (double-buffered per-frame). C ABI exports. |
| `compositor` | Stub | Empty init/shutdown exports |
| `media` | Stub | Empty init/shutdown exports |
| `text` | Stub | Empty init/shutdown exports |

---

## Key APIs

```rust
// Network
let client = vex_net::HttpClient::new()?;
let response = client.fetch(Request::get("https://example.com")?).await?;
let html = response.text()?;

// HTML Parsing
let document = vex_html::parse_html(&html);

// DOM Queries
let h1 = document.query_selector("h1")?.unwrap();
let text = document.text_content(h1);
let divs = document.get_elements_by_tag_name("div");
let html_str = vex_dom::serialize::serialize(document.arena(), root);

// CSS
let sheet = vex_css::parse_stylesheet("div { display: flex; }");
let styles = vex_css::compute_styles(&document, &[sheet], viewport);

// Layout
let tree = vex_layout::layout_document(&document, &styles, viewport);
let dump = vex_layout::debug_dump(&tree);
let stacking = vex_layout::build_stacking_order(&tree);

// Display List + Rendering
let dl = vex_render::build_display_list(&tree, &styles, &document, viewport_size);
let mut renderer = vex_render::renderer::Renderer::new(&device, &queue, format);
renderer.prepare(&device, &queue, &dl, vp_w, vp_h);
renderer.render(&mut encoder, &view);

// Screenshot (offscreen)
vex_render::screenshot::save_screenshot(&dl, 1280, 720, Path::new("out.png")).unwrap();
let diff = vex_render::screenshot::pixel_diff(&pixels_a, &pixels_b, 5);
```

---

## Build Requirements

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.93.1 stable | MSVC toolchain (`x86_64-pc-windows-msvc`) |
| Zig | 0.15.2 | `winget install zig.zig` |
| MSVC Build Tools | 14.44 | VS 2022 BuildTools |

### Build Steps

```bash
cd zig && zig build && cd ..   # 1. Zig static libs
cargo build -p vex-app          # 2. Rust (links Zig .lib via build.rs)
cargo run -p vex-app            # 3. Run
```

Zig→Rust link is driven by `crates/vex-render/build.rs` → `zig/zig-out/lib/`.

---

## Hard-Won Build Fixes (Don't Undo These)

### Zig 0.15 + MSVC Linker

Three things in `zig/build.zig` that **must stay**:
1. **`.abi = .msvc`** — Forces MSVC ABI. Without this → MinGW → `___chkstk_ms` unresolved.
2. **`.stack_protector = false`** — Prevents `__stack_chk_fail` symbols missing in MSVC CRT.
3. **`.stack_check = false`** — Same category.

### Zig 0.15 API vs Older Tutorials
- `addStaticLibrary` → `addLibrary(.{ .linkage = .static, ... })`
- Root module: `b.createModule(.{ .root_source_file = ... })`
- `export fn` implies C ABI — no `callconv(.C)` needed
- Win32: `.winapi` (lowercase)
- `Allocator.VTable` requires `remap` field → use `noRemap`

### raw-window-handle + wgpu 23
- Window uses `AtomicU32` for width/height → `Send+Sync`
- wgpu 23: `Instance::new()` takes owned `InstanceDescriptor`
- wgpu 23: `request_device()` takes two args

---

## Key Files

```
Cargo.toml                              — Workspace root (16 members)
PLAN.md                                 — 11-phase master plan (695 lines)
TASKS.md                                — Task breakdown with ✅/⬜ status
SESSION_LOG.md                          — This file
docs/ARCHITECTURE.md                    — Layer diagram + crate deps

zig/build.zig                           — Zig build (MSVC ABI, static libs)
zig/platform/{root,event,window}.zig    — Win32 windowing
zig/alloc/{root,arena,pool,frame}.zig   — Custom allocators

crates/vex-core/src/                    — geometry, color, id, error, vex_url
crates/vex-net/src/                     — client, tls, dns, cookies, decompress, types
crates/vex-privacy/src/                 — tracking, adblock, https, headers
crates/vex-dom/src/                     — document, node, serialize, queries
crates/vex-html/src/                    — parser (html5ever TreeSink)
crates/vex-css/src/                     — tokenizer, parser, selectors, cascade, values
crates/vex-layout/src/                  — block, inline, flex, text, positioned, stacking, tree_builder, box_model
crates/vex-render/src/                  — display_list, painter, renderer, gpu, platform_ffi, platform, event
crates/vex-render/src/shaders/rect.wgsl — WGSL rect shader (instanced quads)
crates/vex-app/src/main.rs              — Entry point (window + GPU renderer + demo display list)

crates/vex-js/src/api/eme.rs            — EME detection (P9.4.1)
crates/vex-media/src/                   — sync, media_element, media_loading, video_render, controls, pip, dash, hls, abr
crates/vex-browser/src/webview_fallback.rs — WebView2 DRM fallback (P9.4.2)
crates/vex-browser/src/drm_overlay.rs   — WebView overlay positioning (P9.4.3)
zig/media/ffmpeg.zig                    — FFmpeg runtime bindings (P9.1.1)
zig/media/hw_decode.zig                 — Hardware decode probing (P9.1.2)
zig/media/audio_output.zig              — WASAPI audio output (P9.1.3)

crates/vex-html/tests/live_page_test.rs — P3.7.1 e2e (network, #[ignore])
crates/vex-html/tests/parse_bench.rs    — P3.7.2 parse benchmarks
crates/vex-layout/tests/layout_bench.rs — P5.6.2 layout benchmarks
```

---

## What's Next — Phase 10: Browser Chrome & Extensions

### Planned
- **P10.1 — Extension Platform**: Extension manifest parsing, content scripts, background pages
- **P10.2 — DevTools**: Basic inspector, console panel, network panel
- **P10.3 — Settings**: Preferences UI, about:settings page
- **P10.4 — History/Bookmarks**: Full history store, bookmark manager

### Completed ✅
- **P6.1 — Display List Generation**: `display_list.rs` (8 command types), `painter.rs` (walks layout tree, emits FillRect/DrawBorder/DrawText, viewport culling, opacity/clip layers)
- **P6.2 — GPU Backend**: `renderer.rs` (wgpu pipeline, instanced rect + text + image rendering, batching, alpha blending), `shaders/rect.wgsl`, `shaders/text.wgsl`, `shaders/image.wgsl`
- **P6.3 — Glyph Atlas**: 2048×2048 R8Unorm, shelf packing, cosmic-text rasterization, LRU eviction
- **P6.4 — Image Pipeline**: Decode PNG/JPEG/WebP/GIF/BMP, 4096×4096 RGBA atlas, shelf packing
- **P6.5 — Scroll State**: `ScrollState` with clamped offsets, mouse wheel integration
- **P6.6 — Full Pipeline**: `welcome.html` embedded, parsed through HTML→DOM→CSS→Layout→DisplayList→GPU
- **App integration**: Browser chrome (tab bar + address bar), FPS counter, screenshot support

---

## Phase 7 — JavaScript Engine (Boa) ✅

### Completed
- **P7.1 — JS Context**: Boa 0.19 engine integration, `JsEngine` wrapper with execution timeout, `JsValue` conversion (string/number/bool/null/undefined/object/array)
- **P7.2 — Console API**: `console.log/warn/error/info/debug/assert/count/time/timeEnd/clear/dir/table` — all captured in `ConsoleOutput` buffer
- **P7.3 — Timer API**: `setTimeout/clearTimeout/setInterval/clearInterval` — timer registry with elapsed tracking
- **P7.4 — Fetch API**: `fetch()` returns `Promise<Response>` — request/response types, JSON/text body
- **P7.5 — DOM Bindings**: `window`, `document`, `navigator` globals. `document.getElementById/querySelector/createElement/createTextNode`. Element property access (innerHTML, textContent, style, className, id, tagName). `Node.appendChild/removeChild`
- **P7.6 — Script Execution**: Inline `<script>` execution, defer/async flags, script extraction from DOM
- **P7.7 — Form Handling**: Input element value get/set, form `elements` collection, submit event handling

**Tests:** 75 vex-js tests passing, 1 ignored (network)

---

## Phase 8 — Storage & Security ✅

### Completed
- **P8.1 — Storage APIs**: `localStorage/sessionStorage` with get/set/remove/clear/key/length, 5MB per-origin quota enforcement, `StorageArea` persistence
- **P8.2 — IndexedDB**: Object store model, key-value operations (put/get/delete/clear), cursor iteration, transaction support (readonly/readwrite)
- **P8.3 — Cookie Storage**: Persistent cookie jar with secure/httpOnly/sameSite flags, expiry handling, domain/path matching
- **P8.4 — Content Security Policy**: CSP header parsing, directive evaluation (script-src, style-src, img-src, connect-src, default-src), nonce/hash source matching, violation reporting
- **P8.5 — CORS**: Preflight request generation, Access-Control-* header parsing, origin matching, simple vs preflighted request classification
- **P8.6 — Subresource Integrity**: SHA-256/384/512 hash computation, `integrity` attribute verification, multi-hash support

**Tests:** 57 vex-storage tests, 32 vex-security tests passing

---

## Phase 9 — Media Pipeline + DRM Hybrid ✅

### Completed
- **P9.1 — Audio/Video Decode (Zig)**:
  - `zig/media/ffmpeg.zig` — Runtime-loaded ffmpeg bindings (`FfmpegLib`, `MediaContext`, `VideoFrame`, `AudioFrame`), exported C API (`vex_media_open/read_packet/decode_video/decode_audio/close`)
  - `zig/media/hw_decode.zig` — DXVA2/D3D11VA hardware decode probing (`HwDecoderType`, `HwDecodeContext`), exported C API (`vex_media_hw_init/probe/decode`)
  - `zig/media/audio_output.zig` — WASAPI audio output (`AudioHandle`, ring buffer, `AudioFormat`), exported C API (`vex_audio_open/write/pause/resume/close/get_latency`)
  - `crates/vex-media/src/sync.rs` — Clock-based A/V synchronization (`MediaClock`, `SyncDecision`, `FrameSynchroniser`, 20ms drift threshold)
- **P9.2 — Media Element Integration**:
  - `media_element.rs` — `MediaElement` state machine (`ReadyState`, `MediaEvent`, play/pause/seek/volume/mute/playbackRate)
  - `media_loading.rs` — Media source loading pipeline (`MediaFormat` detection from magic bytes, `MediaLoader`, `LoadState`)
  - `video_render.rs` — Video frame → GPU texture upload → display list (`DecodedFrame`, `VideoSurface`, letterbox/pillarbox computation)
  - `controls.rs` — Media player overlay controls (play/pause, seek bar, volume, fullscreen, auto-hide 3s, hit testing)
  - `pip.rs` — Picture-in-Picture mode (`PipState`, `PipGeometry`, drag support)
- **P9.3 — Streaming Protocols**:
  - `dash.rs` — DASH MPD XML parser (`DashManifest`, `AdaptationSet`, `Representation`, ISO 8601 duration parsing)
  - `hls.rs` — HLS M3U8 parser (`MasterPlaylist`, `VariantStream`, `MediaPlaylist`, `MediaSegment`, encryption info)
  - `abr.rs` — Adaptive bitrate selection (`AbrController`, EWMA throughput estimation, 10s hysteresis, buffer-based switching)
- **P9.4 — DRM WebView Fallback**:
  - `crates/vex-js/src/api/eme.rs` — EME detection (`EmeState`, `KeySystem` enum, `navigator.requestMediaKeySystemAccess()` interception)
  - `crates/vex-browser/src/webview_fallback.rs` — WebView2 DRM fallback (`WebViewFallback`, `WebViewConfig`, `WebViewState`, `CookieSync`)
  - `crates/vex-browser/src/drm_overlay.rs` — Seamless WebView2 overlay positioning (`DrmOverlayManager`, scroll offset, tab visibility)
  - `crates/vex-browser/tests/drm_fallback_test.rs` — Integration test (EME trigger → WebView2 creation → overlay → cleanup)

**Tests:** 102 vex-media, 75 vex-js (+7 EME), 131 vex-browser (+4 DRM fallback) — all passing

---

## Skipped / Deferred

| Item | Why |
|------|-----|
| P1.1.1 VexString (interned) | Not needed yet — deferred to DOM optimization |
| P1.2.7 Zig test binary | Rust main.rs exercises platform end-to-end |
| P1.3.4 Stats allocator | Nice-to-have, not blocking |
| `criterion` benchmarks | Not in workspace deps — using `std::time::Instant` timing tests instead |

---

## License

**MPL-2.0** (Mozilla Public License 2.0) — same family as Firefox/Servo. Changed from Proprietary in this session.

- `LICENSE` file: Full MPL-2.0 text
- `Cargo.toml` workspace: `license = "MPL-2.0"`
- All 104 source files: `SPDX-License-Identifier: MPL-2.0`, `Copyright (c) Vigo Contributors`
- Style guides updated (`docs/RUST_STYLE.md`, `docs/ZIG_STYLE.md`)
