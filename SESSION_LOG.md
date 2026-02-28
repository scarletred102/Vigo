# Vigo / Vex Engine — Session Log

> **Read this first each session.** It's the single source of truth for what's done, what's working, and what's next.

---

## Current State (2026-02-28)

**Phase 0 ✅ — Phase 1 ✅ — Phase 2 ✅ — Phase 3 (core done) ✅ — Phase 4 next**

The browser opens a 1280×720 window with a GPU-rendered dark background (wgpu/Vulkan). Events work. The network stack can fetch any HTTPS page with TLS 1.3, follow redirects, decompress gzip/brotli/zstd, and manage cookies. Privacy filters strip tracking params, block ad domains, enforce HTTPS, and sanitize headers. **Any HTML can be parsed into a full DOM tree**, queried (by id/tag/class), traversed, mutated, and serialized back to HTML.

```
cargo run -p vex-app
cargo test -p vex-core -p vex-net -p vex-privacy -p vex-dom -p vex-html  # 130 tests
```

This opens the window. Close it normally to exit.

---

## What's Built

### Rust Crates (16 total, under `crates/`)

| Crate | Status | What it does |
|-------|--------|-------------|
| `vex-core` | **31 tests ✅** | Error types, geometry (Point/Size/Rect/Insets), Color (hex/css/named), VexId (arena index + allocator), VexUrl (wrapper around `url::Url`) |
| `vex-render` | **Compiles ✅** | Zig FFI bridge (`platform_ffi.rs`), safe Window wrapper (`platform.rs`), Event enum (`event.rs`), wgpu GPU context (`gpu.rs`) |
| `vex-app` | **Runs ✅** | Entry point binary. Creates window, inits GPU, runs event loop, clears to dark background |
| `vex-net` | **27 tests ✅** | HTTP/1.1+2 client (hyper+rustls), TLS 1.3, DNS/DoH (hickory), gzip/br/zstd decompression, cookie jar, redirect following (301-308), types (Request/Response/Method) |
| `vex-privacy` | **25 tests ✅** | Tracking param stripper (50+ params), domain adblock engine, HTTPS-only mode, header sanitization, cross-origin referrer reduction |
| `vex-dom` | **29 tests ✅** | Arena-allocated DOM tree (Node, Element, Text, Comment, Doctype), tree manipulation (append/insert/remove/reparent), depth-first/children/ancestor iterators, attribute helpers, `getElementById/getElementsByTagName/getElementsByClassName`, `text_content`, HTML serializer |
| `vex-html` | **18 tests ✅** | html5ever `TreeSink` integration, full-document parsing, fragment parsing, handles malformed HTML, entities, void elements, `<script>` raw text, deeply nested structures |
| Everything else | Stubs | `vex-css`, `vex-layout`, `vex-js`, `vex-media`, `vex-storage`, `vex-security`, `vex-crypto`, `vex-sync`, `vex-browser` |

### Zig Modules (5, under `zig/`)

| Module | Status | What it does |
|--------|--------|-------------|
| `platform` | **Working ✅** | Win32 windowing — `CreateWindowExW`, `PeekMessageW`, WndProc translating `WM_*` to EventC structs. DPI via `GetDpiForWindow`. Raw handle extraction (HWND + HINSTANCE). |
| `alloc` | **11 tests ✅** | Arena (bump allocator), Pool (fixed-size block allocator), Frame (double-buffered arena for per-frame allocations). C ABI exports for arena. |
| `compositor` | Stub | Empty init/shutdown exports |
| `media` | Stub | Empty init/shutdown exports |
| `text` | Stub | Empty init/shutdown exports |

### Test Commands

```bash
# All Rust tests (130 passing: 31 core + 23 net unit + 4 net integration + 25 privacy + 29 dom + 17 html integration + 1 html doctest)
cargo test -p vex-core -p vex-net -p vex-privacy -p vex-dom -p vex-html

# Zig tests (11 passing)
cd zig && zig build test

# Clippy (0 warnings)
cargo clippy -p vex-core -p vex-net -p vex-privacy -p vex-dom -p vex-html -p vex-render -p vex-app -- -Dwarnings
```

---

## Build Requirements

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.93.1 stable | MSVC toolchain (`x86_64-pc-windows-msvc`) |
| Zig | 0.15.2 | Installed via `winget install zig.zig` |
| MSVC Build Tools | 14.44 | VS 2022 BuildTools |

### Build Steps (from workspace root)

```bash
# 1. Build Zig static libs first
cd zig && zig build && cd ..

# 2. Build Rust (Cargo links the Zig libs automatically)
cargo build -p vex-app

# 3. Run
cargo run -p vex-app
```

The Zig→Rust link is driven by `crates/vex-render/build.rs` which points at `zig/zig-out/lib/`.

---

## Hard-Won Build Fixes (Don't Undo These)

### Zig 0.15 + MSVC Linker Compatibility

Three things in `zig/build.zig` that **must stay** or linking breaks:

1. **`.abi = .msvc`** — Forces MSVC calling convention. Without this Zig targets MinGW and emits `___chkstk_ms` which MSVC's `link.exe` can't resolve.

2. **`.stack_protector = false`** — Zig inserts `__stack_chk_fail` / `__stack_chk_guard` calls. These symbols don't exist in MSVC's C runtime.

3. **`.stack_check = false`** — Same category. Prevents stack boundary check symbols that MSVC doesn't provide.

These are set in both lib and test `createModule()` calls.

### Zig 0.15 API Changes (vs older tutorials)

- `addStaticLibrary` → `addLibrary(.{ .linkage = .static, ... })`
- Root module created via `b.createModule(.{ .root_source_file = ... })`
- `export fn` already implies C ABI — no `callconv(.C)` needed
- Win32 calling convention: `.winapi` (lowercase, not `WINAPI`)
- `Allocator.VTable` requires `remap` field → use `Allocator.VTable.noRemap`

### raw-window-handle + wgpu

- Window uses `AtomicU32` for width/height (not `Cell`) so it's `Send+Sync`
- `unsafe impl Send for Window {}` / `unsafe impl Sync for Window {}` — required by wgpu's surface creation
- wgpu 23: `Instance::new()` takes owned `InstanceDescriptor` (not reference)
- wgpu 23: `request_device()` takes two args (descriptor + optional trace path)

---

## Key Files

```
Cargo.toml                          — Workspace root (16 members)
PLAN.md                             — 11-phase master plan (695 lines)
TASKS.md                            — Task breakdown with ✅/⬜ status
docs/ARCHITECTURE.md                — Layer diagram + crate dependency graph
docs/FFI_CONVENTIONS.md             — Zig↔Rust naming/layout conventions

zig/build.zig                       — Zig build (MSVC ABI, static libs)
zig/platform/{root,event,window}.zig — Win32 windowing
zig/alloc/{root,arena,pool,frame}.zig — Custom allocators

crates/vex-core/src/lib.rs          — Core types re-exports
crates/vex-render/build.rs          — Links Zig .lib files into Rust
crates/vex-render/src/platform_ffi.rs — #[repr(C)] FFI types
crates/vex-render/src/platform.rs   — Safe Window wrapper + raw-window-handle
crates/vex-render/src/gpu.rs        — wgpu context (surface, device, queue)
crates/vex-render/src/event.rs      — Rust Event enum
crates/vex-app/src/main.rs          — Entry point
```

---

## What's Next — Phase 2: Network Stack

Per `TASKS.md`, the next work is:

### P2.1 — HTTP Client Foundation
- TLS config (rustls, TLS 1.3, ALPN)
- `HttpClient` struct (hyper + hyper-rustls)
- Request/Response types
- `async fn fetch()` method
- Integration test: fetch example.com

### P2.2 — DNS & DoH
- System DNS via trust-dns-resolver
- DoH mode (Cloudflare/Google)
- Wire into HTTP client

### P2.3 — Content Handling
- Decompression (gzip/brotli/zstd)
- Redirect following (3xx, up to 10 hops)
- Cookie jar (domain/path/secure/httponly/samesite)

### P2.4 — Privacy Filters
- URL param stripping (utm_*, fbclid, etc.)
- Referrer policy enforcement
- Tracker blocking (basic filter list)

**Entry criteria:** Phase 1 complete ✅  
**Exit criteria:** `vex-net` can fetch any HTTPS page and return HTML. Privacy filters strip tracking params. Tests pass.

---

## Skipped / Deferred

| Item | Why |
|------|-----|
| P1.1.1 VexString (interned strings) | Deferred — not needed yet, can add when DOM work starts |
| P1.2.7 Zig test binary | Low value — the Rust main.rs already exercises the platform layer end-to-end |
| P1.3.4 Stats allocator | Nice-to-have, not blocking anything |
| `winit` dependency | Removed — we use our own Zig windowing layer instead |
