# Vigo Browser

> A privacy-first web browser built from scratch.
> Powered by the **Vex** engine — no Chromium, no Gecko, no WebKit.

## What Is This?

Vigo is a custom web browser with its own rendering engine, written in **Rust** (~70%) and **Zig** (~30%). Every layer — from HTML parsing to GPU compositing — is built from the ground up.

## Architecture

```
Browser Chrome  →  Engine Core  →  GPU Pipeline  →  JavaScript
    ↕                  ↕               ↕               ↕
 Network Stack    DOM + CSS + Layout   wgpu         Boa Engine
    ↕                                  ↕
 Media Pipeline                   Platform Layer
    ↕                             (Win32/Cocoa/X11)
 Storage & Security
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full diagram and crate dependency graph.

## Project Structure

```
crates/              # 16 Rust crates
  vex-core/          # Shared types: geometry, color, URL, errors
  vex-net/           # HTTP client, TLS, DNS, caching
  vex-dom/           # DOM tree, events, selectors
  vex-html/          # HTML5 parser (html5ever)
  vex-css/           # CSS parser, cascade, computed styles
  vex-layout/        # Block, inline, flex, positioned layout
  vex-js/            # JavaScript engine (Boa) + Web APIs
  vex-render/        # GPU rendering (wgpu), display lists
  vex-media/         # Audio/video, streaming, DRM fallback
  vex-storage/       # SQLite: cookies, localStorage, IndexedDB
  vex-security/      # SOP, CORS, CSP, sandboxing
  vex-privacy/       # Ad blocking, tracker stripping, HTTPS-only
  vex-crypto/        # ChaCha20, Argon2, Ed25519, X25519
  vex-sync/          # E2E encrypted sync client
  vex-browser/       # Tab management, navigation, UI chrome
  vex-app/           # Binary entry point

zig/                 # 5 Zig modules (compiled to static libs)
  platform/          # Native windowing + event loop
  compositor/        # GPU compositor hot path
  media/             # ffmpeg/dav1d codec FFI
  text/              # SIMD text rasterization
  alloc/             # Arena, pool, frame allocators

sync-server/         # Go + SQLite zero-knowledge sync backend
docs/                # Architecture, coding standards, FFI docs
Docs/                # Product specs (PRD, security, media, etc.)
reference/           # Old Rust crates preserved for porting
```

## Building

### Prerequisites

- **Rust** 1.75+ (install via [rustup](https://rustup.rs))
- **Zig** 0.13+ (install from [ziglang.org](https://ziglang.org/download/))

### Build

```bash
# Zig modules (static libraries)
cd zig && zig build && cd ..

# Rust workspace
cargo build --workspace

# Run
cargo run -p vex-app
```

### Test

```bash
cargo test --workspace
cd zig && zig build test
```

### Lint

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Status

- [x] Phase 0 — Project scaffold & build system
- [ ] Phase 1 — Core types & platform layer
- [ ] Phase 2 — Network stack
- [ ] Phase 3 — HTML parser + DOM
- [ ] Phase 4 — CSS parser + style system
- [ ] Phase 5 — Layout engine
- [ ] Phase 6 — GPU rendering pipeline
- [ ] Phase 7 — JavaScript engine
- [ ] Phase 8 — Browser chrome
- [ ] Phase 9 — Media + DRM hybrid
- [ ] Phase 10 — Security, storage & web compat
- [ ] Phase 11 — Extensions & DevTools

See [PLAN.md](PLAN.md) for the architecture plan and [TASKS.md](TASKS.md) for the full task breakdown.

## License

Proprietary. Copyright (c) Vigo Team. All rights reserved.
