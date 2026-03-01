# Vigo / Vex Engine — Session Log

> **Read this first each session.** Single source of truth for what's done, what's working, and what's next.

---

## Current State

**Phase 0 ✅ — Phase 1 ✅ — Phase 2 ✅ — Phase 3 ✅ — Phase 4 (CSS) ✅ — Phase 5 (Layout) ✅ — Phase 6 (GPU Rendering) ✅**

The full rendering pipeline is operational: parse HTML → build DOM → extract `<style>` → parse CSS → compute styles → lay out boxes → build display list → GPU render with text, rectangles, and borders. The app uses `include_str!("welcome.html")` to embed a styled welcome page that goes through the complete pipeline. Browser chrome (tab bar, address bar) is composited as a fixed overlay. Scroll offsets page content beneath chrome. Screenshot capture renders to offscreen textures for visual regression testing.

```bash
cargo run -p vex-app                          # opens window + renders welcome.html via full pipeline
cargo test --workspace                        # 382 tests (13 ignored)
cargo clippy --workspace --all-targets        # 0 warnings
cd zig && zig build test                      # 22 Zig tests
```

---

## Test Summary

| Crate | Tests | Notes |
|-------|------:|-------|
| vex-core | 39 | geometry, color, id, url, error |
| vex-css | 89 | parser, selectors, cascade, computed styles |
| vex-dom | 68 | arena tree, queries, serialize, iterators |
| vex-html | 11 | html5ever parsing, fragments, malformed HTML |
| vex-html (parse tests) | 17 | round-trip integration tests |
| vex-html (parse bench) | 3 | 100KB, nested, attribute-heavy benchmarks |
| vex-html (live page) | 2 | `#[ignore]` — network-required e2e tests |
| vex-layout | 43 | block, inline, flex, positioned, stacking, text |
| vex-layout (bench) | 2+1 | 200-element, deep nesting; 5000-element `#[ignore]` |
| vex-net | 33 | client, cookies, dns, decompress, types |
| vex-net (fetch) | 4 | `#[ignore]` — live HTTPS integration tests |
| vex-privacy | 32 | tracking, adblock, HTTPS-only, headers |
| vex-render | 47 | display list, painter, renderer, glyph atlas, image atlas, screenshot, scroll |
| **Total** | **382 pass, 13 ignored** | 0 failures |
| Zig | 22 | arena, pool, frame allocators |

---

## What's Built

### Rust Crates (16 total, under `crates/`)

| Crate | Status | What it does |
|-------|--------|-------------|
| `vex-core` | **39 tests ✅** | Error types, geometry (Point/Size/Rect/Insets), Color (hex/css/named), VexId (arena index + allocator), VexUrl (url::Url wrapper) |
| `vex-net` | **37 tests ✅** | HTTP/1.1+2 client (hyper+rustls), TLS 1.3, DNS/DoH (hickory), gzip/br/zstd decompression, cookie jar (domain/path/secure/expiry), redirect following (301-308), Request/Response/Method types |
| `vex-privacy` | **31 tests ✅** | Tracking param stripper (50+ params), domain adblock engine (subdomain matching), HTTPS-only mode, header sanitization (X-Client-Data, Sec-Browsing-Topics, Attribution-Reporting), cross-origin referrer reduction |
| `vex-dom` | **68 tests ✅** | Arena-allocated DOM tree (Document/Element/Text/Comment/Doctype), tree manipulation (append/insert/remove/reparent), depth-first/children/ancestor iterators, attribute map, query selectors (getElementById, getElementsByTagName/ClassName, querySelector/All), text_content, HTML serializer |
| `vex-html` | **33 tests ✅** | html5ever TreeSink integration, full-document and fragment parsing, handles malformed HTML/entities/void elements/script raw text, live-page e2e test (example.com + httpbin.org), parse benchmarks (100KB, nested, attribute-heavy) |
| `vex-css` | **89 tests ✅** | Tokenizer, parser (selectors + declarations), specificity, cascade engine, style computation (compute_styles → HashMap<VexId, ComputedStyle>), all CSS value types (length/color/display/position/overflow/flex), UA defaults, inheritance |
| `vex-layout` | **48 tests ✅** | Block layout (width calc, margin collapsing, overflow clip), inline layout (line boxes, text-align), flex layout (grow/shrink, justify-content 6 values, align-items 5 values, wrap), text measurement (cosmic-text), positioned elements (relative/absolute/fixed), stacking contexts, layout pipeline (layout_document), debug dump, performance benchmarks |
| `vex-render` | **47 tests ✅** | Display list (FillRect/DrawBorder/DrawText/DrawImage/PushClip/PopClip/PushOpacity/PopOpacity), painter (walks layout tree → display list, viewport culling, visibility/opacity), GPU renderer (rect + text pipelines, WGSL shaders rect/text/image, instanced drawing, batching, alpha blending), glyph atlas (shelf packing, cosmic-text rasterizer, LRU eviction), image atlas (4096×4096, shelf packing), image decoder (PNG/JPEG/WebP/GIF/BMP), screenshot (offscreen render to PNG, pixel_diff comparison), scroll state, Zig FFI (platform_ffi.rs), Window wrapper, Event enum, wgpu GPU context |
| `vex-app` | **Runs ✅** | Entry point binary. Parses embedded `welcome.html` through full pipeline (HTML→DOM→CSS→Layout→Display List), composites with browser chrome (tab bar, address bar, accent line), renders via Renderer with text + rect pipelines, scroll support, FPS counter |
| Others | Stubs | `vex-js`, `vex-media`, `vex-storage`, `vex-security`, `vex-crypto`, `vex-sync`, `vex-browser` |

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

crates/vex-html/tests/live_page_test.rs — P3.7.1 e2e (network, #[ignore])
crates/vex-html/tests/parse_bench.rs    — P3.7.2 parse benchmarks
crates/vex-layout/tests/layout_bench.rs — P5.6.2 layout benchmarks
```

---

## What's Next — Phase 6: GPU Rendering Pipeline (continued)

### Completed ✅
- **P6.1 — Display List Generation**: `display_list.rs` (8 command types), `painter.rs` (walks layout tree, emits FillRect/DrawBorder/DrawText, viewport culling, opacity/clip layers)
- **P6.2 — GPU Backend (partial)**: `renderer.rs` (wgpu pipeline, instanced rect rendering, batching, alpha blending), `shaders/rect.wgsl` (vertex quad generation, pixel→NDC transform, per-instance color)
- **App integration**: `main.rs` renders a demo mock browser chrome (10+ colored rectangles)

### Remaining
- **P6.2.3** Text shader (text.wgsl) — glyph atlas sampling, subpixel AA
- **P6.2.4** Image shader (image.wgsl) — texture sampling
- **P6.3** Glyph atlas — rasterize glyphs into GPU texture, shelf-based packing, LRU eviction
- **P6.4** Image pipeline — decode PNG/JPEG/WebP/SVG, async loading, GPU upload
- **P6.5** Scroll state — smooth scrolling, event integration
- **P6.6** Full pipeline test — wire HTML→DOM→CSS→Layout→Render for real pages

**Target:** Render `https://example.com` visually in the window. 60fps scrolling.

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
