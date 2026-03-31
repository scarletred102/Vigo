# Vex Engine Architecture

## Layer Diagram

```
┌──────────────────────────────────────────────────────────────┐
│  Browser Chrome  (vex-browser, vex-app)                      │
│  Tabs · Navigation · Bookmarks · Settings · UI               │
├──────────────────────────────────────────────────────────────┤
│  Engine Core  (vex-html, vex-dom, vex-css, vex-layout)       │
│  HTML5 Parser · DOM Tree · CSS Cascade · Layout Engine        │
├──────────────────────────────────────────────────────────────┤
│  GPU Rendering  (vex-render + zig/compositor)                │
│  Display Lists · wgpu · Glyph Atlas · Image Atlas · Scroll   │
├──────────────────────────────────────────────────────────────┤
│  JavaScript  (vex-js)                                        │
│  Boa Engine · DOM Bindings · Web APIs · Timers · Fetch       │
├──────────────────────────────────────────────────────────────┤
│  Network  (vex-net)                                          │
│  HTTP/1.1+2 · TLS 1.3 · DNS/DoH · Cache · Cookies           │
├──────────────────────────────────────────────────────────────┤
│  Media  (vex-media + zig/media)                              │
│  ffmpeg · DASH/HLS · A/V Sync · Hardware Decode · PiP        │
├──────────────────────────────────────────────────────────────┤
│  Storage & Security  (vex-storage, vex-security, vex-crypto) │
│  SQLite · CSP · CORS · SOP · Sandbox · E2E Crypto            │
├──────────────────────────────────────────────────────────────┤
│  Platform  (zig/platform, zig/alloc, zig/text)               │
│  Win32/Cocoa/X11 · Custom Allocators · SIMD Text · DPI       │
└──────────────────────────────────────────────────────────────┘
```

## Language Split

| Layer | Language | Rationale |
|-------|----------|-----------|
| Core parsing/DOM/CSS/Layout | Rust | Memory safety, rich type system, html5ever/cssparser ecosystem |
| Networking | Rust | hyper + rustls = battle-tested, async/await ergonomics |
| JavaScript | Rust | Boa engine is pure Rust, GC integrates with Rust ownership |
| GPU rendering | Rust + Zig | Rust for display list building, Zig for hot compositor paths |
| Platform windowing | Zig | Direct Win32/Cocoa/X11 calls without bindgen overhead |
| Media codecs | Zig | Direct ffmpeg/dav1d C API calls with zero-overhead wrapping |
| Custom allocators | Zig | Arena/pool/frame allocators benefit from Zig's explicit memory model |
| Text rasterization | Zig | SIMD-optimized glyph rasterization hot path |
| Browser shell | Rust | High-level orchestration, state management, UI logic |
| Crypto/Security | Rust | RustCrypto ecosystem, zeroize, constant-time operations |

## Crate Dependency Graph

```
vex-app
  └─ vex-browser
       ├─ vex-dom
       │    └─ vex-core
       ├─ vex-html
       │    ├─ vex-dom
       │    └─ vex-core
       ├─ vex-css
       │    ├─ vex-dom
       │    └─ vex-core
       ├─ vex-layout
       │    ├─ vex-css
       │    ├─ vex-dom
       │    └─ vex-core
       ├─ vex-js
       │    ├─ vex-dom
       │    ├─ vex-net
       │    └─ vex-core
       ├─ vex-render
       │    ├─ vex-layout
       │    └─ vex-core
       ├─ vex-net
       │    └─ vex-core
       ├─ vex-storage
       │    └─ vex-core
       └─ vex-privacy
            ├─ vex-net
            └─ vex-core

vex-media
  ├─ vex-render
  └─ vex-core

vex-sync
  ├─ vex-crypto
  ├─ vex-net
  └─ vex-core

vex-security
  ├─ vex-net
  └─ vex-core

vex-crypto
  └─ vex-core
```

## Rust↔Zig Interop

Zig modules compile to static libraries (`.lib` on Windows, `.a` on Unix).
Rust crates link via `build.rs` + `cargo:rustc-link-lib=static=...`.
All FFI functions use C calling convention and `#[repr(C)]` structs.
See `docs/FFI_CONVENTIONS.md` for full protocol.
