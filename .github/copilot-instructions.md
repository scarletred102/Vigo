# Vigo Browser — Vex Engine

Custom browser engine built from scratch in **Rust (~70%) + Zig (~30%)**.  
Codename **Vex** (Vigo Engine X). License: MPL-2.0. Windows-first, cross-platform target.

## Build & Test

Requires: Rust stable (see `rust-toolchain.toml`), Zig 0.15+, MSVC Build Tools 14.x.

```sh
just check        # fast compile check (no codegen)
just build        # debug build
just test         # all Rust tests
just lint         # cargo clippy --workspace -- -D warnings
just fmt          # cargo fmt --all
just ci           # fmt-check + lint + test (what CI runs)
just run          # launch the browser (debug)
just zig-build    # build Zig static libs (required on Windows)
```

CI runs on ubuntu/windows/macos via `.github/workflows/ci.yml`.  
**`just ci` must pass clean before any commit.**

## Architecture

8-layer stack — see `docs/ARCHITECTURE.md` for the full dependency graph.

| Crate / Module | Role |
|----------------|------|
| `vex-core` | Shared types: VexUrl, Color, Rect, VexId, VexError |
| `vex-net` | HTTP/1.1+2, TLS 1.3, DNS/DoH, cookies, cache |
| `vex-privacy` | Tracking param stripping, adblock, HTTPS-only |
| `vex-html` | html5ever-based HTML parser → DOM |
| `vex-dom` | Arena DOM tree, querySelector, events |
| `vex-css` | CSS parser, cascade, computed styles |
| `vex-layout` | Block/inline/flex/positioned layout engine |
| `vex-render` | Display list, wgpu renderer, glyph/image atlases |
| `vex-js` | JavaScript (Boa engine 0.19), DOM bindings |
| `vex-app` | Entry point — full page pipeline, window, event loop |
| `zig/platform` | Win32 windowing (CreateWindowExW) |
| `zig/alloc` | Arena, Pool, Frame allocators |
| `zig/compositor` | GPU compositor hot paths |
| `zig/text` | SIMD glyph rasterization |

Full pipeline (page render):  
`parse_html → CSS cascade → layout_document → build_display_list → renderer.prepare → renderer.render`

## Phase Status

| Phase | Status |
|-------|--------|
| 0 — Scaffold | ✅ |
| 1 — Core Types & Platform | ✅ |
| 2 — Network Stack | ✅ |
| 3 — HTML Parser + DOM | ✅ |
| 4 — CSS Parser + Style System | ✅ |
| 5 — Layout Engine | ✅ |
| 6 — GPU Rendering | ✅ |
| 7 — JavaScript (Boa) | ✅ |
| 8 — Browser Chrome | ✅ |
| 9 — Media Pipeline | ✅ |
| 10 — Storage & Security | ✅ |
| 11 — Extensions & Process Model | ✅ |
| **Final Assembly (77 wiring tasks)** | 🔲 in progress |

All 11 phases are structurally complete. Current work: **final wiring, integration, and assembly** — see `FINAL_TASKS.md`.  
**Only work on tasks listed in `FINAL_TASKS.md`. No new features beyond what is listed.**

## Core Directive

> **Clean, readable, and reusable code — do not over-engineer.**

## Non-Negotiable Rules

- Zero Clippy warnings — `cargo clippy --workspace -- -D warnings` must pass.
- No `.unwrap()` / `.expect()` in library crates — use `?`. Tests may use `.unwrap()`.
- No `println!` / `eprintln!` in library crates — use `tracing::info!` / `debug!` / `warn!` / `error!`.
- Every `unsafe` block needs a `// SAFETY:` comment.
- Every `.rs` and `.zig` file starts with the MPL-2.0 header:
  ```
  // Copyright (c) Vigo Contributors
  // SPDX-License-Identifier: MPL-2.0
  ```
- All deps declared in `[workspace.dependencies]` (root `Cargo.toml`), inherited via `workspace = true`.
  Do not add a dep without checking it doesn't duplicate an existing workspace dep.
- Error handling: `thiserror` in library crates, `anyhow` only in `vex-app`.

## Critical Zig / FFI Pitfalls

See `.github/instructions/vex-zig-build.instructions.md` for full details.

- Target query **must** include `.abi = .msvc` — without it Zig emits MinGW symbols the MSVC linker rejects.
- Every Zig library needs `stack_protector = false` and `stack_check = false`.
- Use the Zig 0.15 `addLibrary` API — not the removed `addStaticLibrary`.
- `export fn` implies C ABI — don't add `callconv(.C)` redundantly.

## Key wgpu / Render API Notes

See `.github/instructions/vex-render-apis.instructions.md` for full details.

- **wgpu 23**: use `ImageCopyTexture` / `ImageDataLayout` — `TexelCopy*` names were removed.
- **cosmic-text 0.12**: use `get_image_uncached()` to avoid borrow conflicts with `FontSystem`.
- Glyph atlas: 2048×2048 R8Unorm. Image atlas: 4096×4096 Rgba8UnormSrgb.

## Testing Conventions

- Unit tests: `#[cfg(test)] mod tests` at the bottom of each file.
- Integration tests: `tests/` directory per crate.
- Every wired-up function must have test coverage where the logic is non-trivial.
- Non-deterministic (network/filesystem) tests: `#[ignore]`.
- Current count: ~382 Rust tests, 22 Zig tests — do not reduce this.

## Detailed Conventions

| Topic | File |
|-------|------|
| **Current task checklist** | `FINAL_TASKS.md` |
| Rust & Zig style, naming, safety | `.github/instructions/vex-coding-conventions.instructions.md` |
| Zig build, MSVC ABI, FFI layout | `.github/instructions/vex-zig-build.instructions.md` |
| wgpu 23, cosmic-text, image pipeline | `.github/instructions/vex-render-apis.instructions.md` |
| Rust style reference | `docs/RUST_STYLE.md` |
| Zig style reference | `docs/ZIG_STYLE.md` |
| FFI conventions | `docs/FFI_CONVENTIONS.md` |
| Full architecture | `docs/ARCHITECTURE.md` |
