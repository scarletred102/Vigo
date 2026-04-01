# Plan: Vigo Engine — Ground-Up Browser in Rust + Zig

## TL;DR

Build **Vigo Engine** — a fully custom browser engine in Rust (~70%) + Zig (~30%), from scratch. No Chromium. No C++. Multi-year timeline across 10 phases. Leverage battle-tested Rust crates (html5ever, cssparser, wgpu, rustls, hyper, Boa) as foundational libraries while building custom layout, rendering, and browser chrome. For DRM streaming, use an embedded webview fallback (Chromium CEF or WebView2) until custom EME path is negotiated. Clean slate — existing Chromium overlay code is archived, new monorepo starts fresh.

**Codename:** Vigo Engine (or "Vex" — Vigo Engine X)

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│                    VIGO BROWSER CHROME                    │  Rust + Zig
│        (Tabs, Omnibar, Settings, Extensions, UI)         │
└────────────────────────────┬─────────────────────────────┘
                             │
┌────────────────────────────┴─────────────────────────────┐
│                     VIGO ENGINE CORE                      │
├────────────────┬────────────────┬────────────────────────┤
│  DOM + Events  │  Style System  │  Layout Engine         │  Rust
│  (html5ever)   │  (cssparser)   │  (Block/Inline/Flex)   │
├────────────────┴────────────────┴────────────────────────┤
│              GPU RENDERING PIPELINE                       │  Zig + wgpu
│  (Display Lists → Compositor → Rasterizer → Screen)      │
├──────────────────────────────────────────────────────────┤
│              JAVASCRIPT ENGINE                            │  Rust (Boa)
│  (Parser → Bytecode → Interpreter → [future: JIT in Zig])│
├──────────────────────────────────────────────────────────┤
│              NETWORK STACK                                │  Rust
│  (HTTP/1.1+2+3, TLS 1.3, DNS/DoH, WebSocket, Fetch)     │
├──────────────────────────────────────────────────────────┤
│              MEDIA PIPELINE                               │  Zig + Rust
│  (ffmpeg/dav1d bindings, HW accel, audio/video sync)     │
├──────────────────────────────────────────────────────────┤
│              STORAGE + SECURITY                           │  Rust
│  (SQLite, Cookies, CSP, CORS, SOP, Sandboxing)           │
├──────────────────────────────────────────────────────────┤
│              PLATFORM LAYER                               │  Zig
│  (Windowing, Input, OS Integration, Memory Allocators)    │
└──────────────────────────────────────────────────────────┘
         │                                        │
    [Custom Engine]                     [DRM Webview Fallback]
    (all web content)                   (Netflix, Disney+, etc.)
```

---

## Language Split: Rust vs Zig

### Rust (~70% of codebase)
- HTML/CSS parsing (html5ever, cssparser, selectors)
- DOM tree, event dispatch, mutation observers
- Layout engine (block, inline, flex, grid)
- CSS cascade, specificity, computed styles
- JavaScript runtime (Boa engine + Web API bindings)
- Network stack (hyper, rustls, quinn)
- Security (CSP, CORS, SOP enforcement)
- Crypto & privacy (adblock, credential vault, sync encryption)
- Browser chrome logic (tab management, navigation, history, bookmarks)

### Zig (~30% of codebase)
- GPU compositor & display list painter (SIMD-optimized)
- Platform abstraction (custom windowing, input handling)
- Custom memory allocators (arena, pool, frame allocators)
- Media codec integration (ffmpeg/dav1d/libvpx FFI, hardware decode)
- Text rasterization hot paths (glyph cache, SIMD antialiasing)
- Future: JIT backend for JS engine (if Boa interpreter too slow)

### Interop Pattern
- Zig compiles to static libraries with C ABI exports
- Rust calls Zig via `extern "C"` FFI (unsafe blocks, well-documented)
- Build system: Cargo workspace for Rust, Zig build system for Zig libs, top-level orchestrator script

---

## Phase 0 — Project Scaffold & Build System (Week 1-2)

**Goal:** Monorepo structure, build system, CI, coding standards.

1. Create new `vigo-engine/` monorepo with the following structure:
   ```
   vigo-engine/
   ├── Cargo.toml              # Rust workspace root
   ├── build.zig               # Zig build system root
   ├── build.rs                # Cargo build script (links Zig libs)
   ├── justfile                # Task runner (like Make but modern)
   ├── README.md
   ├── LICENSE
   ├── .github/
   │   └── workflows/
   │       └── ci.yml          # Rust + Zig CI
   ├── crates/                 # All Rust crates
   │   ├── vex-core/           # Core types, error handling, string types
   │   ├── vex-net/            # Network stack
   │   ├── vex-dom/            # DOM tree + events
   │   ├── vex-html/           # HTML parser (wraps html5ever)
   │   ├── vex-css/            # CSS parser + cascade (wraps cssparser)
   │   ├── vex-layout/         # Layout engine
   │   ├── vex-js/             # JS engine (wraps Boa + Web APIs)
   │   ├── vex-render/         # Rendering pipeline (Rust side)
   │   ├── vex-media/          # Media pipeline (Rust orchestration)
   │   ├── vex-storage/        # Cookies, localStorage, IndexedDB
   │   ├── vex-security/       # CSP, CORS, SOP, sandboxing
   │   ├── vex-privacy/        # Adblock, anti-fingerprint, DoH
   │   ├── vex-crypto/         # Encryption, key derivation
   │   ├── vex-sync/           # E2E encrypted sync client
   │   ├── vex-browser/        # Browser chrome (tabs, nav, bookmarks)
   │   └── vex-app/            # Main binary entry point
   ├── zig/                    # All Zig modules
   │   ├── platform/           # Windowing, input, OS abstraction
   │   ├── compositor/         # GPU compositor, display list painter
   │   ├── media/              # Codec bindings, HW decode
   │   ├── text/               # Glyph rasterization, SIMD shaping
   │   └── alloc/              # Custom allocators
   ├── tests/                  # Integration tests
   ├── benches/                # Criterion benchmarks
   ├── docs/                   # Architecture docs (carry from old repo)
   └── tools/                  # Dev tools, scripts
   ```

2. Set up Cargo workspace with all crate stubs (lib.rs with license header)
3. Set up Zig build system (`build.zig`) that compiles Zig modules to static libs
4. Create `build.rs` in vex-render and vex-media that links Zig static libs
5. Set up `justfile` with commands: `build`, `test`, `bench`, `lint`, `fmt`, `run`
6. Create GitHub Actions CI: Rust (check, test, clippy, fmt) + Zig (build, test)
7. Define coding standards documents for Rust and Zig
8. Archive old `vigo-core/` Chromium overlay (move to `archive/` branch)

**Verification:** `cargo check` passes on workspace, `zig build` compiles all modules, CI green.

---

## Phase 1 — Core Types & Platform Layer (Weeks 3-6)

**Goal:** Shared types, error handling, windowing, and event loop.

### Phase 1A: Core Types (Rust) — Week 3
1. `vex-core` crate: define shared types
   - `VexString` — interned string type for DOM attributes/tag names (small-string optimization)
   - `VexUrl` — URL parser (use `url` crate as foundation)
   - `VexError` — unified error enum with `thiserror`
   - `Rect`, `Point`, `Size`, `Color` — geometry primitives
   - `VexId` — arena-allocated ID type for DOM nodes
   - Platform detection (`cfg` target_os flags)

### Phase 1B: Platform Layer (Zig) — Weeks 3-4 *parallel with 1A*
2. `zig/platform/` — windowing and input abstraction
   - Window creation (Win32 API on Windows, Cocoa on macOS, X11/Wayland on Linux)
   - Event loop (message pump, resize, close, focus)
   - Keyboard/mouse input events
   - DPI/scaling detection
   - Expose C ABI: `vex_platform_create_window()`, `vex_platform_poll_events()`, etc.

### Phase 1C: Custom Allocators (Zig) — Week 4 *parallel with 1B*
3. `zig/alloc/` — memory allocators
   - Arena allocator (for per-frame/per-parse temporary allocations)
   - Pool allocator (for DOM nodes — fixed-size blocks)
   - Frame allocator (for rendering — reset each frame)
   - Statistics/tracking (memory usage reporting)

### Phase 1D: Rust-Zig Bridge (Week 5-6)
4. Create `vex-render` crate with FFI bindings to Zig platform layer
5. Implement basic event loop in Rust that:
   - Creates a window via Zig platform layer
   - Receives events (keyboard, mouse, resize)
   - Renders a solid color background
   - Handles graceful shutdown
6. Wire up wgpu for GPU-accelerated rendering through the Zig-created window surface

**Verification:**
- Run binary → window appears with solid background
- Keyboard/mouse events log to console
- Memory allocator tests pass in Zig
- Cross-platform: test on Windows (primary), plan for macOS/Linux

---

## Phase 2 — Network Stack (Weeks 7-12)

**Goal:** Fetch web pages over HTTP/HTTPS.

1. `vex-net` crate built on `hyper` + `rustls` + `quinn`
   - HTTP/1.1 and HTTP/2 client (via hyper)
   - TLS 1.3 (via rustls — pure Rust, no OpenSSL)
   - DNS resolver with built-in DoH (DNS-over-HTTPS) support
   - HTTP/3 (QUIC) via quinn (optional, feature-gated)
   - Connection pooling and keep-alive
   - Cookie jar (RFC 6265 compliant)
   - Redirect following (with limit)
   - Request/response streaming (for large resources)
   - Content-Encoding decompression (gzip, brotli, zstd)
   - Resource cache (HTTP cache semantics — ETag, Cache-Control, Last-Modified)
   - Fetch API abstraction (Request/Response types matching web spec)

2. `vex-privacy` integration (built into network layer from day 1):
   - Ad/tracker domain blocking (rewrite adblock engine for new architecture)
   - Tracking parameter stripping (utm_*, fbclid, etc.)
   - Referrer policy enforcement
   - HTTPS-only mode (with fallback option)
   - Header sanitization (strip tracking headers)

**Verification:**
- Fetch `https://example.com` → get HTML string
- Fetch with TLS 1.3 → verify certificate chain
- DoH resolution → verify DNS queries encrypted
- Adblock rules → blocked domains return error
- HTTP cache → second request returns cached response
- Benchmark: 500+ requests/sec to localhost test server

---

## Phase 3 — HTML Parser + DOM (Weeks 13-20)

**Goal:** Parse HTML into a live DOM tree.

### Phase 3A: HTML Parser — Weeks 13-16
1. `vex-html` crate wrapping `html5ever`
   - WHATWG-compliant HTML tokenizer + tree builder
   - Custom `TreeSink` implementation that builds `vex-dom` nodes
   - Error recovery (malformed HTML handling per spec)
   - Support `<template>`, `<script>`, `<style>` elements
   - Incremental parsing (parse as bytes arrive from network)

### Phase 3B: DOM — Weeks 15-20 *overlaps with 3A*
2. `vex-dom` crate — Document Object Model
   - Arena-allocated node storage (using Zig pool allocator via FFI, or Rust-side arena)
   - Node types: Document, Element, Text, Comment, DocumentFragment
   - Parent/child/sibling traversal (NodeList)
   - Attribute get/set/remove
   - `getElementById`, `querySelector`, `querySelectorAll` (using `selectors` crate)
   - Event system:
     - EventTarget trait (addEventListener, removeEventListener, dispatchEvent)
     - Event bubbling and capture phases
     - Standard events: click, mousedown/up/move, keydown/up, focus/blur, load, DOMContentLoaded
   - MutationObserver (simplified)
   - `innerHTML`, `textContent`, `outerHTML` (serialize back to HTML)

**Verification:**
- Parse `https://example.com` → DOM tree with correct structure
- `querySelector("h1")` → returns correct element
- Event dispatch: click event bubbles from child to parent
- Round-trip: parse HTML → serialize back → parse again → identical tree
- Benchmark: parse 100KB HTML in <5ms

---

## Phase 4 — CSS Parser + Style System (Weeks 21-28)

**Goal:** Parse CSS, resolve cascade, compute styles per element.

### Phase 4A: CSS Parser — Weeks 21-24
1. `vex-css` crate wrapping `cssparser` and `selectors`
   - CSS tokenizer + parser (via cssparser)
   - Stylesheet parsing (rules, selectors, declarations)
   - Support for:
     - Type, class, ID, attribute selectors
     - Combinators (descendant, child, sibling)
     - Pseudo-classes (:hover, :focus, :active, :first-child, :nth-child, :not)
     - Pseudo-elements (::before, ::after — stub)
     - Media queries (screen/print, width/height)
   - `<style>` element support
   - `<link rel="stylesheet">` loading (via vex-net)
   - Inline `style` attribute parsing

### Phase 4B: Cascade + Computed Styles — Weeks 25-28
2. Style resolution engine:
   - Specificity calculation
   - Cascade resolution (origin: user-agent, author, inline)
   - Inheritance (inherited properties propagate down tree)
   - Computed value resolution:
     - Length units (px, em, rem, %, vw, vh)
     - Color values (hex, rgb, hsl, named)
     - Display modes (block, inline, flex, grid, none)
     - Box model (margin, padding, border, width, height)
     - Positioning (static, relative, absolute, fixed, sticky)
     - Typography (font-family, font-size, font-weight, line-height, text-align)
   - Default user-agent stylesheet (minimal but correct)

**Verification:**
- Parse CSS `div.foo > p { color: red; }` → correct rule + selector
- Specificity: inline > #id > .class > element
- Computed styles: `font-size: 2em` on child of `font-size: 16px` → 32px
- Media queries: `@media (max-width: 800px)` → applies when window < 800px
- Benchmark: resolve styles for 1000-element DOM in <10ms

---

## Phase 5 — Layout Engine (Weeks 29-40)

**Goal:** Position and size every element on screen.

### Phase 5A: Block Layout — Weeks 29-33
1. `vex-layout` crate
   - Box model implementation (content box, padding box, border box, margin box)
   - Block formatting context (BFC)
   - Normal flow: block boxes stack vertically
   - Width/height resolution (auto, fixed, percentage)
   - Margin collapsing
   - `display: none` — skip layout entirely
   - Generate layout tree (LayoutBox) from styled DOM

### Phase 5B: Inline Layout + Text — Weeks 33-36
2. Inline formatting context:
   - Text splitting into line boxes
   - Word wrapping (break-word, overflow-wrap)
   - Text alignment (left, center, right, justify)
   - Font metrics integration (via cosmic-text):
     - Font loading (system fonts + @font-face — later)
     - Text shaping (via HarfBuzz through cosmic-text)
     - Line height calculation
     - Baseline alignment
   - Inline elements mixed with text

### Phase 5C: Flex Layout — Weeks 37-39
3. Flexbox algorithm (CSS Flexbox Level 1):
   - flex-direction, flex-wrap
   - justify-content, align-items, align-self
   - flex-grow, flex-shrink, flex-basis
   - Order property

### Phase 5D: Positioned Elements — Weeks 39-40
4. Positioned elements:
   - `position: relative` — offset from normal flow position
   - `position: absolute` — relative to nearest positioned ancestor
   - `position: fixed` — relative to viewport
   - Z-index stacking

**Deferred to later phases:** CSS Grid, Table layout, Floats (complex, diminishing returns)

**Verification:**
- Block layout: nested divs with margins → correct positions
- Inline layout: paragraph text wraps at container width
- Flex layout: 3-column layout with `justify-content: space-between` renders correctly
- Positioned: absolute-positioned element inside relative container → correct offset
- Visual regression: render known HTML → compare output PNG against reference
- Benchmark: layout 5000-element page in <16ms (60fps budget)

---

## Phase 6 — GPU Rendering Pipeline (Weeks 41-52)

**Goal:** Paint layout boxes to screen with GPU acceleration.

### Phase 6A: Display List Generation (Rust) — Weeks 41-44
1. `vex-render` crate
   - Walk layout tree → generate display list (flat command buffer):
     - FillRect (backgrounds, borders)
     - DrawText (positioned glyph runs)
     - DrawImage (decoded image data)
     - PushClip / PopClip (overflow clipping)
     - PushOpacity / PopOpacity (opacity layers)
   - Display list optimization (culling off-screen items, merging adjacent rects)

### Phase 6B: GPU Compositor (Zig) — Weeks 44-48
2. `zig/compositor/` — GPU rendering backend
   - wgpu integration (Zig calling wgpu C API, or Rust calling wgpu with Zig for hot paths)
   - Shader programs (WGSL):
     - Rectangle fill shader (backgrounds, borders, rounded corners)
     - Text rendering shader (alpha-tested glyph atlas)
     - Image display shader (texture sampling)
   - Glyph atlas management (cache rendered glyphs in GPU texture)
   - Texture atlas for images
   - Frame scheduling (requestAnimationFrame-style vsync)

### Phase 6C: Text Rasterization (Zig) — Weeks 48-50 *parallel with 6B*
3. `zig/text/` — glyph rasterization
   - Interface with cosmic-text/swash for glyph outlines
   - Rasterize glyphs into atlas bitmaps
   - Subpixel antialiasing (ClearType on Windows, LCD on Linux, system on macOS)
   - Glyph cache with LRU eviction

### Phase 6D: Image Pipeline — Weeks 50-52
4. Image loading and display:
   - Decode PNG, JPEG, WebP, GIF, SVG (via `image` crate + `resvg` for SVG)
   - Async image loading (don't block layout)
   - Image resizing / downscaling for display
   - Upload decoded images to GPU textures

**Verification:**
- Render `https://example.com` → visually recognizable page in window
- Text renders clearly with proper antialiasing
- Images display inline with text
- 60fps scrolling on 1080p page with 100 elements
- Visual diff: screenshot comparison against Firefox/Chrome rendering

---

## Phase 7 — JavaScript Engine (Weeks 53-68)

**Goal:** Execute JavaScript, manipulate DOM, handle events.

### Phase 7A: Boa Integration — Weeks 53-58
1. `vex-js` crate wrapping Boa engine
   - Embed `boa_engine` as JS runtime
   - Create execution context per document
   - Implement Web APIs as Boa native functions:
     - `document.getElementById()`, `document.querySelector()`
     - `document.createElement()`, `element.appendChild()`
     - `element.addEventListener()`, `element.removeEventListener()`
     - `element.style.*` (read/write computed styles)
     - `element.classList.*`
     - `element.innerHTML`, `element.textContent`
     - `console.log/warn/error`
     - `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`
     - `fetch()` (wired to vex-net)
     - `JSON.parse/stringify` (Boa built-in)
     - `Promise` (Boa built-in)
     - `window.location`, `window.history`
   - Script execution lifecycle:
     - `<script>` blocking execution
     - `<script defer>` and `<script async>`
     - `DOMContentLoaded` and `load` events

### Phase 7B: DOM Bindings — Weeks 58-63
2. Bridge Boa JS objects ↔ vex-dom Rust objects:
   - JS object proxies wrapping DOM nodes
   - Property access on elements maps to DOM attribute read/write
   - Event handlers registered via JS fire through Rust event system
   - GC integration (prevent DOM nodes from being collected while JS references exist)

### Phase 7C: Forms & Input — Weeks 63-68
3. Form and input handling:
   - `<input>` (text, password, checkbox, radio, submit)
   - `<textarea>`, `<select>`, `<button>`
   - Form submission (GET/POST)
   - Input events (input, change, submit)
   - Text cursor rendering and editing in input fields
   - Clipboard integration (Ctrl+C/V)

**Verification:**
- `<script>document.getElementById('x').textContent = 'hello'</script>` → updates DOM
- `<button onclick="alert('hi')">` → fires handler
- `setTimeout(() => { ... }, 1000)` → executes after 1 second
- `fetch('/api')` → returns response, `.then()` works
- Form submit → sends POST request with form data
- Benchmark: execute 1000 DOM manipulations in <50ms

---

## Phase 8 — Browser Chrome (Weeks 69-80)

**Goal:** Build the browser UI (tabs, navigation, settings).

### Phase 8A: Tab System — Weeks 69-72
1. `vex-browser` crate — browser-level state management
   - Tab model: create, close, switch, reorder, duplicate
   - Each tab owns: URL, DOM, layout tree, JS context, render state
   - Tab bar UI (rendered via Zig compositor — custom drawn, not HTML)
   - Tab isolation (separate DOM/JS contexts)
   - Lazy tab loading (restore session without loading all tabs)

### Phase 8B: Navigation — Weeks 72-75
2. Address bar + navigation:
   - URL input with autocomplete (history-based)
   - Forward/back navigation (history stack per tab)
   - Page loading states (connecting, loading, complete)
   - Progress indicator
   - HTTPS indicator (lock icon for TLS)
   - Error pages (DNS failure, connection refused, certificate error)

### Phase 8C: Core Browser Features — Weeks 75-80
3. Essential browser features:
   - Bookmarks (add, remove, folders, bookmark bar)
   - History (browsing history with search)
   - Downloads (download manager with progress)
   - Find in page (Ctrl+F text search)
   - Print (basic — generate PDF or call OS print dialog)
   - Settings page (privacy, appearance, search engine, about)
   - Keyboard shortcuts (Ctrl+T, Ctrl+W, Ctrl+L, Ctrl+R, etc.)
   - Context menus (right-click: open link in new tab, copy, etc.)
   - Zoom (per-tab zoom level)

### Phase 8D: UI Rendering — *parallel with 8A-8C*
4. Browser chrome rendering (Zig compositor):
   - Tab bar rendering (custom GPU-drawn)
   - Address bar rendering
   - Button/icon rendering
   - Dropdown menus
   - Scrollbar rendering (custom, not OS native)
   - Theming (light/dark mode, accent colors)

**Verification:**
- Open 10 tabs → switch between them → each has independent state
- Navigate to URL → page loads → back button works
- Bookmark a page → appears in bookmark bar
- Find in page → highlights matches
- Ctrl+T, Ctrl+W, Ctrl+L all work

---

## Phase 9 — Media Pipeline + DRM Hybrid (Weeks 81-92)

**Goal:** Play video/audio, with Chromium webview fallback for DRM content.

### Phase 9A: Custom Media Pipeline (Zig + Rust) — Weeks 81-86
1. `vex-media` (Rust) + `zig/media/` (Zig)
   - `<video>` and `<audio>` element support
   - Codec support via ffmpeg/dav1d bindings in Zig:
     - H.264, VP9, AV1 (video)
     - AAC, Opus, MP3, FLAC (audio)
   - Hardware-accelerated decode:
     - DXVA2/D3D11VA on Windows (Zig FFI to Win32 APIs)
     - VideoToolbox on macOS
     - VAAPI/V4L2 on Linux
   - Audio/video synchronization
   - Adaptive bitrate (ABR) for DASH/HLS streams
   - Media controls UI (play, pause, seek, volume, fullscreen)
   - Picture-in-Picture (PiP)
   - Media session API (system media keys)

### Phase 9B: DRM Webview Fallback — Weeks 86-90
2. Hybrid DRM architecture:
   - Detect DRM-protected content (EME API request from page JS)
   - For DRM content → launch embedded webview:
     - Windows: WebView2 (Chromium-based, ships with Windows 11)
     - macOS: WKWebView (WebKit-based, supports FairPlay)
     - Linux: CEF (Chromium Embedded Framework) or WebKitGTK
   - Webview renders in a seamless overlay within Vigo's tab
   - Browser chrome stays Vigo, only the content area uses webview
   - Pass cookies/auth tokens from Vigo session to webview
   - Transition: custom engine ↔ webview should be seamless to user

### Phase 9C: WebRTC Basics — Weeks 90-92
3. Basic WebRTC support (for video calls):
   - Stub — integrate existing Rust WebRTC library (webrtc-rs) or defer

**Verification:**
- `<video src="test.mp4">` → plays with controls
- YouTube video (non-DRM) → plays in custom engine
- Netflix → detects DRM → seamless webview fallback → plays
- Hardware decode active (check via media internals page)
- PiP mode → video floats above other windows

---

## Phase 10 — Security, Storage & Polish (Weeks 93-110)

**Goal:** Production-grade security, web storage APIs, and web compat push.

### Phase 10A: Security — Weeks 93-100
1. `vex-security` crate:
   - Same-Origin Policy enforcement (DOM access, XHR, fetch)
   - CORS (preflight, headers, credentials)
   - Content Security Policy (CSP Level 2)
   - X-Frame-Options / frame-ancestors
   - Mixed content blocking (HTTPS pages can't load HTTP resources)
   - Certificate validation (via rustls + webpki)
   - Process sandboxing:
     - Each tab in separate OS process
     - Restrict syscalls (seccomp on Linux, sandbox profiles on macOS, job objects on Windows)
   - Address space layout randomization (ASLR) verification
   - `vex-crypto` crate: encryption for stored data (passwords, bookmarks sync)
   - `vex-privacy` hardening: canvas noise injection, WebGL masking, font enumeration restriction

### Phase 10B: Storage — Weeks 100-104
2. `vex-storage` crate:
   - Cookies (RFC 6265: domain, path, SameSite, Secure, HttpOnly)
   - localStorage (per-origin, 5MB limit, SQLite backend)
   - sessionStorage
   - IndexedDB (simplified — SQLite-backed key-value with indexes)
   - Cache API (for service workers — future, stub now)
   - Credential storage (passwords, passkeys, integrated with OS keychain)

### Phase 10C: Web Compatibility Push — Weeks 104-110
3. Run Web Platform Tests (WPT) and fix failures:
   - HTML parsing edge cases
   - CSS property support gaps
   - DOM API missing methods
   - Flexbox edge cases
   - Event handling quirks
   - Target: pass 60% of WPT on first pass, iterate to 80%

**Verification:**
- CSP: `<script>` from disallowed origin → blocked
- CORS: cross-origin fetch without proper headers → blocked
- localStorage: set value → close browser → reopen → value persists
- Process isolation: crash renderer → browser stays alive
- WPT dashboard: track pass rate improvement over time

---

## Phase 11 — Extensions & DevTools (Weeks 111-126)

**Goal:** Developer tools and custom extension platform.

### Phase 11A: DevTools — Weeks 111-118
1. Built-in developer tools (rendered as custom Vigo UI, not a web page):
   - Elements inspector (DOM tree view, select element, computed styles)
   - Console (JS console with log/warn/error, REPL)
   - Network inspector (request/response list, timing, headers, body)
   - Sources (view page source, JS debugging — breakpoints if possible)
   - Performance (basic profiling — layout time, paint time, script time)

### Phase 11B: Extension Platform — Weeks 118-126
2. Custom extension API (NOT Chrome MV3 — Vigo's own):
   - Extension manifest format (JSON-based, permissions model)
   - Content scripts (inject JS/CSS into pages)
   - Background scripts (persistent or event-driven)
   - Browser action (toolbar button + popup)
   - APIs: tabs, bookmarks, history, storage, notifications, contextMenus
   - Extension permission system (user grants per-API access)
   - Extension store infrastructure (self-hosted or marketplace)
   - Consider: MV3 compatibility shim for popular Chrome extensions (later)

**Verification:**
- DevTools: inspect any page → see DOM tree + styles
- Console: type JS → executes in page context
- Network: see all requests with timing
- Extension: install test extension → injects content script → modifies page

---

## DRM Hybrid — Detailed Architecture

Since streaming is important but full DRM in a custom engine is near-impossible initially:

**Strategy:** Vigo Engine renders ALL web content natively. When a page requests EME (Encrypted Media Extensions), Vigo detects this and seamlessly hands off the media element to an embedded webview.

**Detection Flow:**
1. Page JS calls `navigator.requestMediaKeySystemAccess('com.widevine.alpha', ...)`
2. Vigo's EME implementation intercepts this
3. Instead of failing, Vigo:
   - Creates an embedded webview (WebView2/WKWebView) sized to the `<video>` element's layout rect
   - Navigates the webview to the current page URL
   - Overlays the webview on top of the video element area
   - Hides Vigo's native rendering for that region
4. User sees seamless playback — browser chrome is still Vigo
5. When video ends or user navigates away → destroy webview, return to native rendering

**Long-term goal:** Negotiate Widevine CDM license directly (like Firefox did), eliminating webview fallback.

---

## Key Dependencies (Rust Crates)

| Crate | Version | Purpose |
|-------|---------|---------|
| html5ever | 0.38+ | HTML parsing |
| cssparser | 0.36+ | CSS parsing |
| selectors | 0.35+ | CSS selector matching |
| wgpu | 28+ | GPU abstraction (Vulkan/Metal/DX12) |
| winit | 0.30+ | Cross-platform windowing (may replace with Zig platform) |
| rustls | 0.23+ | TLS 1.3 |
| hyper | 1.8+ | HTTP client |
| quinn | 0.11+ | QUIC / HTTP/3 |
| boa_engine | 0.21+ | JavaScript engine |
| cosmic-text | 0.18+ | Text shaping + layout |
| image | 0.25+ | Image decoding |
| url | latest | URL parsing |
| thiserror | latest | Error handling |
| tokio | 1.x | Async runtime |
| serde | 1.x | Serialization |
| rusqlite | latest | SQLite for storage |

## Key Zig Dependencies

| Library | Purpose |
|---------|---------|
| ffmpeg (C) | Media codec decode |
| dav1d (C) | AV1 decode |
| harfbuzz (C) | Text shaping (if not using cosmic-text) |
| freetype (C) | Font rasterization (if not using swash) |
| System APIs | Win32, Cocoa, X11/Wayland |

---

## Decisions

1. **Clean slate** — No code carries over from Chromium overlay. Architectural docs may be referenced. Old repo archived.
2. **Rust + Zig hybrid** — Rust for safety-critical high-level code (~70%), Zig for perf-critical low-level code (~30%)
3. **No C++ anywhere** — Zig handles all C interop needs (ffmpeg, system APIs)
4. **Boa for JS** — Interpreter-based initially. If too slow, add Zig JIT backend in Phase 11+
5. **Custom extension API** — Not Chrome MV3. Vigo's own API. Compatibility shim for popular extensions considered later.
6. **DRM hybrid** — Native engine for all content, embedded webview fallback for DRM only
7. **Process-per-tab sandbox** — From Phase 10, not day 1 (single-process MVP first, add isolation later)
8. **No mobile** — Desktop only (Windows, macOS, Linux) for the entire timeline
9. **Naming** — Engine codename "Vex" (Vigo Engine X). Crate prefix `vex-`. Zig modules unprefixed.

## Excluded (Not in scope for this plan)
- Mobile (iOS, Android)
- Chrome extension MV3 full compat (maybe later as shim)
- Full CSS Grid (Phase 5 deferred, add later)
- Service Workers (complex, future phase)
- WebAssembly (Boa doesn't include WASM yet; add later)
- WebGL/WebGPU API (for web pages — add later, focus on engine GPU first)
- PDF viewer
- Printing (basic only)

## Further Considerations
1. **Windowing: winit vs custom Zig** — winit is mature and cross-platform but adds a Rust dependency for something Zig could do natively. Recommendation: start with winit for speed, replace with Zig platform layer when ready.
2. **JS Engine: Boa vs embedded V8/SpiderMonkey** — Boa is pure Rust but slower (no JIT). V8 is fast but C++. Recommendation: start with Boa, benchmark real-world sites, add Zig JIT backend if needed.
3. **Build orchestration** — Cargo + Zig build system need a top-level orchestrator. Recommendation: `just` (justfile) calling both systems, with a build.rs in Rust crates that triggers Zig compilation.
