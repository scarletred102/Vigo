# Vigo / Vex Engine — Session Log

> **Read this first each session.** Single source of truth for what's done, what's working, and what's next.

---

## Current State

**Phase 0 ✅ — Phase 1 ✅ — Phase 2 ✅ — Phase 3 ✅ — Phase 4 (CSS) ✅ — Phase 5 (Layout) ✅ — Phase 6 next**

The browser opens a 1280×720 window with a GPU-rendered dark background (wgpu/Vulkan). The full pipeline works: fetch any HTTPS page → parse HTML → build DOM → parse CSS → compute styles → lay out boxes (block, inline, flex, positioned) → build stacking order. Privacy filters strip trackers, block ads, enforce HTTPS.

```bash
cargo run -p vex-app                          # opens window
cargo test --workspace                        # 341 tests (6 ignored)
cargo clippy --workspace --all-targets        # 11 pre-existing style warnings (vex-layout field init)
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
| vex-privacy | 31 | tracking, adblock, HTTPS-only, headers |
| vex-render | 1 | doc test |
| **Total** | **341 pass, 6 ignored** | 0 failures |
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
| `vex-render` | **Compiles ✅** | Zig FFI bridge (platform_ffi.rs), safe Window wrapper (platform.rs), Event enum (event.rs), wgpu GPU context (gpu.rs) |
| `vex-app` | **Runs ✅** | Entry point binary. Creates window, inits GPU, runs event loop, clears to dark background |
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
crates/vex-render/src/                  — platform_ffi, platform, event, gpu
crates/vex-app/src/main.rs              — Entry point

crates/vex-html/tests/live_page_test.rs — P3.7.1 e2e (network, #[ignore])
crates/vex-html/tests/parse_bench.rs    — P3.7.2 parse benchmarks
crates/vex-layout/tests/layout_bench.rs — P5.6.2 layout benchmarks
```

---

## What's Next — Phase 6: GPU Rendering Pipeline

Per `PLAN.md` / `TASKS.md`:

### P6A — Display List Generation (Rust, vex-render)
- Walk layout tree → flat command buffer (FillRect, DrawText, DrawImage, PushClip/PopClip, PushOpacity/PopOpacity)
- Display list optimization (cull off-screen, merge adjacent rects)

### P6B — GPU Compositor (Zig, zig/compositor/)
- wgpu integration, WGSL shaders (rect fill, text rendering, image display)
- Glyph atlas management, texture atlas, frame scheduling (vsync)

### P6C — Text Rasterization (Zig, zig/text/)
- Glyph outlines → atlas bitmaps, subpixel AA, LRU cache

### P6D — Image Pipeline
- Decode PNG/JPEG/WebP/GIF/SVG, async loading, GPU texture upload

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
