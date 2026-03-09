# Vigo Browser

> A privacy-first web browser built from scratch.
> Powered by the **Vex** engine — no Chromium, no Gecko, no WebKit.

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

## What Is This?

Vigo is an open-source web browser with its own rendering engine, written in **Rust** (~70%) and **Zig** (~30%). Every layer — from HTML parsing to GPU compositing — is built from the ground up. No forking, no embedding, no shortcuts.

**Why?** The web deserves more than three engines. Vigo is an independent alternative built on modern languages with privacy as a core design principle, not a bolt-on.

## Current Status

Vigo is now a working end-to-end browser prototype. The current codebase includes the full page
pipeline — networking, HTML parsing, DOM construction, CSS cascade, layout, display-list
generation, GPU rendering, JavaScript execution, browser chrome, storage, security policies,
privacy protections, DevTools, and extension loading.

The long-form implementation plan is split across the phase docs and the final browser assembly
checklist. At the time of writing, the repository marks all 11 phases as structurally complete, and
the final integration checklist in [FINAL_TASKS.md](FINAL_TASKS.md) is fully checked off.

| Phase | Status | What |
|------:|--------|------|
| 0 | ✅ Done | Project scaffold, build system, CI |
| 1 | ✅ Done | Core types, Zig platform layer, wgpu window |
| 2 | ✅ Done | HTTP stack, TLS 1.3, DNS/DoH, cookies, caching, decompression |
| 3 | ✅ Done | HTML5 parser, DOM tree, selectors, events |
| 4 | ✅ Done | CSS parser, cascade, computed styles |
| 5 | ✅ Done | Block, inline, flex, positioned layout, stacking |
| 6 | ✅ Done | GPU rendering pipeline, display lists, shaders, atlases |
| 7 | ✅ Done | JavaScript runtime and browser APIs via Boa |
| 8 | ✅ Done | Browser chrome, tabs, navigation, bookmarks, find, zoom |
| 9 | ✅ Done | Media pipeline and browser integration |
| 10 | ✅ Done | Security, storage, privacy hardening, web compatibility |
| 11 | ✅ Done | Extensions, DevTools, and process-model wiring |

See [PLAN.md](PLAN.md) for the full architecture plan, [TASKS.md](TASKS.md) for the broader task
breakdown, and [FINAL_TASKS.md](FINAL_TASKS.md) for the final browser wiring checklist.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  vex-app (binary)                    │
├──────────┬──────────┬───────────┬───────────────────┤
│ vex-net  │ vex-html │ vex-css   │ vex-layout        │
│ HTTP/2   │ html5ever│ cascade   │ block/inline/flex  │
│ TLS 1.3  │ TreeSink │ selectors │ positioned/stacking│
│ DNS/DoH  │          │ computed  │ text (cosmic-text) │
├──────────┤ vex-dom  ├───────────┼───────────────────┤
│vex-privacy│ arena   │ vex-render│ vex-js            │
│ adblock  │ queries  │ wgpu GPU  │ Boa runtime       │
│ tracking │ events   │ Zig FFI   │ browser APIs      │
├──────────┴──────────┴───────────┴───────────────────┤
│              vex-core (types, geometry, errors)       │
├─────────────────────────────────────────────────────┤
│          Zig: platform · compositor · alloc · text   │
│          Win32 windowing · arena/pool/frame allocs   │
└─────────────────────────────────────────────────────┘
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full crate dependency graph.

## Project Structure

```
crates/              16 Rust crates
  vex-core/          Shared types: geometry, color, URL, errors
  vex-net/           HTTP client, TLS 1.3, DNS/DoH, cookies
  vex-dom/           Arena-allocated DOM tree, selectors, events
  vex-html/          HTML5 parser (html5ever TreeSink)
  vex-css/           CSS tokenizer, parser, cascade, computed styles
  vex-layout/        Block, inline, flex, positioned, stacking
  vex-render/        GPU context (wgpu), Zig FFI bridge
  vex-privacy/       Ad blocking, tracker stripping, HTTPS-only
  vex-js/            JavaScript runtime and browser APIs (Boa)
  vex-media/         Audio/video pipeline and codec integration
  vex-storage/       Cookies, localStorage, sessionStorage, IndexedDB
  vex-security/      SOP, CORS, CSP, and related policy enforcement
  vex-crypto/        ChaCha20, Argon2, Ed25519
  vex-sync/          E2E encrypted sync support
  vex-browser/       Tabs, navigation, UI, DevTools, extensions
  vex-app/           Binary entry point

zig/                 5 Zig modules → static libs → C ABI → Rust FFI
  platform/          Native windowing (Win32) + event loop
  alloc/             Arena, pool, frame allocators (22 tests)
  compositor/        GPU compositor support
  media/             Codec/media FFI support
  text/              Text rasterization support

sync-server/         Go + SQLite zero-knowledge sync backend
docs/                Architecture, coding standards, FFI conventions
specs/               Product specs (PRD, engine strategy, security, etc.)
```

## Building

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.75+ | [rustup.rs](https://rustup.rs) |
| Zig | 0.15+ | [ziglang.org](https://ziglang.org/download/) or `winget install zig.zig` |
| MSVC Build Tools | 2022 | [VS Build Tools](https://visualstudio.microsoft.com/downloads/) (Windows) |

### Build & Run

```bash
# 1. Build Zig static libraries
cd zig && zig build && cd ..

# 2. Build Rust workspace (links Zig libs automatically)
cargo build -p vex-app

# 3. Run — opens a 1280×720 GPU-accelerated window
cargo run -p vex-app
```

### Test

```bash
# All tests (341 passing)
cargo test --workspace

# Zig tests (22 passing)
cd zig && zig build test

# Network-dependent tests (requires internet)
cargo test --workspace -- --ignored

# Clippy
cargo clippy --workspace --all-targets
```

## Contributing

Contributions are welcome! Vigo is licensed under the [Mozilla Public License 2.0](LICENSE), the same license family used by Firefox and Servo.

### Quick Start

1. Fork the repo and create a branch
2. Build and run the tests (see above)
3. Make your changes — follow the style guides in [`docs/`](docs/)
4. Every `.rs` file should start with:
   ```rust
   // Copyright (c) Vigo Contributors
   // SPDX-License-Identifier: MPL-2.0
   ```
5. Open a PR against `main`

### Areas Where Help Is Needed

- **Stabilization**: Exercise the browser against more real-world sites and report rendering,
  layout, or JavaScript compatibility issues
- **Cross-platform work**: Improve macOS and Linux platform support alongside the Windows-first
  path
- **Performance**: Profile parsing, layout, rendering, and startup hot paths
- **Testing**: Expand integration and end-to-end coverage for browser workflows
- **Polish**: Improve browser UX, accessibility, and developer ergonomics

## Design Principles

- **Privacy first** — Ad blocking, tracker stripping, and HTTPS enforcement are built into the network stack, not extensions.
- **No monoculture** — An independent engine. No Blink, no Gecko, no WebKit.
- **Modern languages** — Rust for safety + Zig for performance-critical paths. No C/C++.
- **Clean code** — Readable, well-tested, not over-engineered. Easy to contribute to.

## Analogous Projects

| Project | Language | Notes |
|---------|----------|-------|
| [Servo](https://servo.org/) | Rust | Mozilla/Linux Foundation browser engine |
| [Ladybird](https://ladybird.org/) | C++ | From-scratch engine (SerenityOS) |
| [Boa](https://boajs.dev/) | Rust | JavaScript engine (used by Vigo) |
| [Vigo](https://github.com/scarletred102/Vigo) | Rust + Zig | This project |

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).

```
This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
```
