# Vigo Engine (Vex) — Master Task Breakdown

> Every phase. Every task. Every deliverable. No ambiguity.
>
> **Convention:** Each task is numbered `P{phase}.{sub}.{task}`. Status markers: `⬜` not started, `🔶` in progress, `✅` done.
> Dependencies are noted explicitly. Tasks within the same sub-phase can run in parallel unless stated otherwise.

---

## Phase 0 — Project Scaffold & Build System

**Timeline:** Weeks 1–2  
**Goal:** Monorepo structure, dual build system (Cargo + Zig), CI pipeline, coding standards.  
**Entry criteria:** None — this is the starting point.  
**Exit criteria:** `cargo check` passes on entire workspace. `zig build` compiles all Zig modules. CI pipeline green. All crate stubs exist with license headers.

---

### P0.1 — Repository Initialization

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.1.1 | Create `remake/vigo-engine/` directory as the new monorepo root. | Empty directory with `.gitignore` | ✅ |
| P0.1.2 | Create `LICENSE` file — proprietary license text (same as old repo). | `LICENSE` | ✅ |
| P0.1.3 | Create `README.md` with project name "Vigo Engine", codename "Vex", one-paragraph description, build instructions placeholder, and architecture diagram from PLAN.md. | `README.md` | ✅ |
| P0.1.4 | Create `.gitignore` covering: Rust (`target/`, `Cargo.lock` for libs), Zig (`zig-cache/`, `zig-out/`), OS files (`.DS_Store`, `Thumbs.db`), IDE files (`.vscode/`, `.idea/`). | `.gitignore` | ✅ |
| P0.1.5 | Create `.editorconfig` — UTF-8, LF line endings, 4-space indent for Rust, 4-space indent for Zig, 2-space for TOML/YAML/JSON. | `.editorconfig` | ✅ |

---

### P0.2 — Cargo Workspace Setup

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.2.1 | Create root `Cargo.toml` as a Cargo workspace. Set `resolver = "2"`. Define `members` array listing all 16 crates under `crates/*`. Define shared `[workspace.dependencies]` for common deps (thiserror, serde, tokio, tracing, url). Define `[workspace.package]` with shared metadata (version = "0.1.0", edition = "2024", license, authors, rust-version = "1.85"). | `Cargo.toml` | ✅ |
| P0.2.2 | Create `crates/vex-core/Cargo.toml` with `[package]` inheriting workspace metadata. Add deps: `thiserror`, `url`, `serde` (with derive feature), `smallvec`. Create `crates/vex-core/src/lib.rs` with license header and `//! Vex core types` doc comment. | `crates/vex-core/` | ✅ |
| P0.2.3 | Create `crates/vex-net/Cargo.toml`. Deps: `hyper` (client, http1, http2 features), `hyper-util`, `hyper-rustls`, `rustls`, `tokio` (full), `quinn`, `trust-dns-resolver`, `http`, `bytes`, `flate2`, `brotli`, `zstd`, `vex-core` (path). Create `src/lib.rs` stub. | `crates/vex-net/` | ✅ |
| P0.2.4 | Create `crates/vex-dom/Cargo.toml`. Deps: `vex-core` (path), `smallvec`, `serde`. Create `src/lib.rs` stub. | `crates/vex-dom/` | ✅ |
| P0.2.5 | Create `crates/vex-html/Cargo.toml`. Deps: `html5ever`, `markup5ever`, `tendril`, `vex-core` (path), `vex-dom` (path). Create `src/lib.rs` stub. | `crates/vex-html/` | ✅ |
| P0.2.6 | Create `crates/vex-css/Cargo.toml`. Deps: `cssparser`, `selectors`, `vex-core` (path), `vex-dom` (path). Create `src/lib.rs` stub. | `crates/vex-css/` | ✅ |
| P0.2.7 | Create `crates/vex-layout/Cargo.toml`. Deps: `vex-core` (path), `vex-dom` (path), `vex-css` (path), `cosmic-text`. Create `src/lib.rs` stub. | `crates/vex-layout/` | ✅ |
| P0.2.8 | Create `crates/vex-js/Cargo.toml`. Deps: `boa_engine`, `boa_gc`, `vex-core` (path), `vex-dom` (path), `vex-net` (path). Create `src/lib.rs` stub. | `crates/vex-js/` | ✅ |
| P0.2.9 | Create `crates/vex-render/Cargo.toml`. Deps: `wgpu`, `winit`, `raw-window-handle`, `vex-core` (path), `vex-layout` (path), `image`, `resvg`. Create `src/lib.rs` stub. Has `build.rs` placeholder for future Zig linking. | `crates/vex-render/` | ✅ |
| P0.2.10 | Create `crates/vex-media/Cargo.toml`. Deps: `vex-core` (path), `vex-render` (path), `tokio`. Create `src/lib.rs` stub. Has `build.rs` placeholder for future Zig linking. | `crates/vex-media/` | ✅ |
| P0.2.11 | Create `crates/vex-storage/Cargo.toml`. Deps: `rusqlite` (bundled feature), `serde`, `serde_json`, `vex-core` (path). Create `src/lib.rs` stub. | `crates/vex-storage/` | ✅ |
| P0.2.12 | Create `crates/vex-security/Cargo.toml`. Deps: `vex-core` (path), `vex-net` (path), `url`. Create `src/lib.rs` stub. | `crates/vex-security/` | ✅ |
| P0.2.13 | Create `crates/vex-privacy/Cargo.toml`. Deps: `vex-core` (path), `vex-net` (path), `regex`, `aho-corasick`. Create `src/lib.rs` stub. | `crates/vex-privacy/` | ✅ |
| P0.2.14 | Create `crates/vex-crypto/Cargo.toml`. Deps: `chacha20poly1305`, `argon2`, `ed25519-dalek`, `x25519-dalek`, `rand`, `zeroize`, `vex-core` (path). Create `src/lib.rs` stub. | `crates/vex-crypto/` | ✅ |
| P0.2.15 | Create `crates/vex-sync/Cargo.toml`. Deps: `vex-core` (path), `vex-crypto` (path), `vex-net` (path), `serde`, `serde_json`, `tokio`. Create `src/lib.rs` stub. | `crates/vex-sync/` | ✅ |
| P0.2.16 | Create `crates/vex-browser/Cargo.toml`. Deps: `vex-core` (path), `vex-dom` (path), `vex-html` (path), `vex-css` (path), `vex-layout` (path), `vex-js` (path), `vex-render` (path), `vex-net` (path), `vex-storage` (path), `vex-privacy` (path). Create `src/lib.rs` stub. | `crates/vex-browser/` | ✅ |
| P0.2.17 | Create `crates/vex-app/Cargo.toml`. This is the binary crate (`[[bin]]` target named `vigo`). Deps: `vex-browser` (path), `vex-render` (path), `tokio` (rt-multi-thread, macros), `tracing`, `tracing-subscriber`. Create `src/main.rs` with `fn main()` that prints "Vigo Engine v0.1.0 — Vex". | `crates/vex-app/` | ✅ |
| P0.2.18 | Run `cargo check` on the entire workspace. Fix any dependency resolution or syntax errors until it passes clean. | Green `cargo check` | ✅ |

---

### P0.3 — Zig Build System Setup

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.3.1 | Create `zig/build.zig` — top-level Zig build file. Define 5 static library targets: `vex_platform`, `vex_compositor`, `vex_media_zig`, `vex_text`, `vex_alloc`. Each exports a `.a`/`.lib` to `zig-out/lib/`. Set optimization mode to `.ReleaseSafe` for release, `.Debug` for debug. Add `zig build test` step that runs all Zig tests. | `zig/build.zig` | ✅ |
| P0.3.2 | Create `zig/platform/root.zig` — stub file exporting one C ABI function: `export fn vex_platform_init() callconv(.C) c_int { return 0; }`. This is the "hello world" of the platform layer. | `zig/platform/root.zig` | ✅ |
| P0.3.3 | Create `zig/compositor/root.zig` — stub exporting `export fn vex_compositor_init() callconv(.C) c_int { return 0; }`. | `zig/compositor/root.zig` | ✅ |
| P0.3.4 | Create `zig/media/root.zig` — stub exporting `export fn vex_media_init() callconv(.C) c_int { return 0; }`. | `zig/media/root.zig` | ✅ |
| P0.3.5 | Create `zig/text/root.zig` — stub exporting `export fn vex_text_init() callconv(.C) c_int { return 0; }`. | `zig/text/root.zig` | ✅ |
| P0.3.6 | Create `zig/alloc/root.zig` — stub exporting `export fn vex_alloc_init() callconv(.C) c_int { return 0; }`. | `zig/alloc/root.zig` | ✅ |
| P0.3.7 | Run `zig build` from `zig/` directory. Verify all 5 static libraries are produced in `zig-out/lib/`. Fix any build errors. | Green `zig build` | ✅ |

---

### P0.4 — Rust↔Zig Build Integration

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.4.1 | Create `crates/vex-render/build.rs` — Cargo build script that: (a) determines the Zig output directory relative to the workspace root, (b) calls `println!("cargo:rustc-link-search=native={zig_out_dir}")`, (c) calls `println!("cargo:rustc-link-lib=static=vex_platform")` and `println!("cargo:rustc-link-lib=static=vex_compositor")`, (d) calls `println!("cargo:rerun-if-changed=../../zig/platform/root.zig")`. | `crates/vex-render/build.rs` | ✅ |
| P0.4.2 | Create `crates/vex-media/build.rs` — similar to above but links `vex_media_zig` and `vex_text`. | `crates/vex-media/build.rs` | ⬜ |
| P0.4.3 | Create `crates/vex-render/src/ffi.rs` — Rust `extern "C"` declarations matching the Zig stubs: `extern "C" { fn vex_platform_init() -> i32; fn vex_compositor_init() -> i32; }`. Add `mod ffi;` to `lib.rs`. | `crates/vex-render/src/ffi.rs` | ✅ |
| P0.4.4 | Verify the full build chain works: run `zig build` first, then `cargo check` — the Rust crates that depend on Zig libs should find the .lib/.a files and resolve the extern symbols. | Green build chain | ✅ |

---

### P0.5 — Task Runner & CI

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.5.1 | Create `justfile` (for the `just` command runner) in the monorepo root with these recipes: `build` (zig build && cargo build), `test` (zig build test && cargo test), `check` (cargo check && cargo clippy), `fmt` (cargo fmt --check && zig fmt check), `run` (zig build && cargo run -p vex-app), `bench` (cargo bench), `clean` (cargo clean && rm -rf zig/zig-out zig/zig-cache). | `justfile` | ✅ |
| P0.5.2 | Create `.github/workflows/ci.yml` — GitHub Actions workflow triggered on push/PR. Jobs: (1) `rust` — install Rust stable, run `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`. (2) `zig` — install Zig 0.13+, run `cd zig && zig build && zig build test`. Matrix: os = [ubuntu-latest, windows-latest, macos-latest]. | `.github/workflows/ci.yml` | ✅ |
| P0.5.3 | Create `rust-toolchain.toml` specifying the Rust version channel (stable) and components (rustfmt, clippy). | `rust-toolchain.toml` | ✅ |

---

### P0.6 — Coding Standards & Documentation

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.6.1 | Create `docs/RUST_STYLE.md` — Rust coding standards: license header on every file, error handling with `thiserror` (no `.unwrap()` in lib code), `tracing` for logging (not `println!`), doc comments on all public items, `#[must_use]` on fallible functions, `unsafe` blocks require `// SAFETY:` comment. | `docs/RUST_STYLE.md` | ✅ |
| P0.6.2 | Create `docs/ZIG_STYLE.md` — Zig coding standards: license header, all exported functions use C calling convention, snake_case naming, explicit allocators (no hidden allocations), all test functions named `test_<description>`, error sets documented. | `docs/ZIG_STYLE.md` | ✅ |
| P0.6.3 | Create `docs/ARCHITECTURE.md` — copy the architecture diagram and language split section from PLAN.md. Add crate dependency graph (text-based). | `docs/ARCHITECTURE.md` | ✅ |
| P0.6.4 | Create `docs/FFI_CONVENTIONS.md` — document the Rust↔Zig interop pattern: how Zig exports C ABI, how Rust declares externs, naming convention (`vex_<module>_<function>`), error return conventions (0 = success, negative = error code), memory ownership rules (caller allocates / callee allocates with free function). | `docs/FFI_CONVENTIONS.md` | ✅ |

---

### P0.7 — Scaffold Verification

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P0.7.1 | Run `just build` — must complete without errors. | Green build | ✅ |
| P0.7.2 | Run `just test` — all crate stubs pass (trivially — no tests yet, but no failures). | Green tests | ✅ |
| P0.7.3 | Run `just run` — must print "Vigo Engine v0.1.0 — Vex" to stdout. | Working binary | ✅ |
| P0.7.4 | Verify file count: 16 Rust crates × (Cargo.toml + src/lib.rs or src/main.rs) = ~35 files + zig stubs + docs + CI = ~55+ files total. List all files, confirm nothing missing. | File manifest | ✅ |

---
---

## Phase 1 — Core Types & Platform Layer

**Timeline:** Weeks 3–6  
**Goal:** Shared primitive types for the entire engine. A window on screen driven by Zig platform layer. A working Rust↔Zig event loop. GPU surface initialized.  
**Entry criteria:** Phase 0 complete (`cargo check` + `zig build` pass).  
**Exit criteria:** Running `cargo run -p vex-app` opens a native window with a colored background. Mouse/keyboard events print to `tracing` output. All unit tests pass.

---

### P1.1 — Core Types (`vex-core`)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P1.1.1 | **VexString** — Create `crates/vex-core/src/string.rs`. Implement an interned string type backed by a global `HashSet<&'static str>` (or use the `string_cache` crate). Must support: `From<&str>`, `From<String>`, `PartialEq`, `Eq`, `Hash`, `Clone` (cheap — it's just an index/pointer), `Display`, `Debug`, `Serialize`/`Deserialize`. Write 5 tests: creation, equality, hashing, display, clone-is-cheap (assert pointer equality). | `src/string.rs` + tests | ✅ |
| P1.1.2 | **VexUrl** — Create `src/url.rs`. Thin wrapper around the `url::Url` crate. Add methods: `parse(input: &str) -> Result<Self>`, `origin() -> String`, `scheme() -> &str`, `host() -> Option<&str>`, `path() -> &str`, `query_pairs() -> impl Iterator`, `is_https() -> bool`, `join(relative: &str) -> Result<Self>`. Write 8 tests: valid URL, invalid URL, origin extraction, relative URL join, HTTPS detection, query parsing, scheme access, empty input. | `src/url.rs` + tests | ✅ |
| P1.1.3 | **Geometry primitives** — Create `src/geometry.rs`. Define: `Point { x: f32, y: f32 }`, `Size { width: f32, height: f32 }`, `Rect { origin: Point, size: Size }`, `Insets { top: f32, right: f32, bottom: f32, left: f32 }` (for margins/padding). Implement `Rect::contains(point)`, `Rect::intersects(other)`, `Rect::union(other)`, `Rect::offset(dx, dy)`, `Rect::inset(insets)`. All types derive `Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize`. Write 10 tests covering each method. | `src/geometry.rs` + tests | ✅ |
| P1.1.4 | **Color** — Create `src/color.rs`. Define `Color { r: u8, g: u8, b: u8, a: u8 }`. Constructors: `Color::rgba(r, g, b, a)`, `Color::rgb(r, g, b)` (a=255), `Color::from_hex("#rrggbb")`, `Color::from_hex("#rrggbbaa")`, `Color::from_css_name("red")` (support the 17 CSS named colors + "transparent"). Method: `to_f32_array() -> [f32; 4]` (for GPU shader uniforms). Write 8 tests: hex parsing, named colors, transparent, f32 conversion, invalid hex. | `src/color.rs` + tests | ✅ |
| P1.1.5 | **VexId** — Create `src/id.rs`. A `#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)] struct VexId(u32)` used as an arena index for DOM nodes. Implement `VexId::new(index: u32)`, `VexId::index() -> u32`, `Display` (prints `"node#123"`). An `IdAllocator` struct that hands out sequential IDs and can recycle freed ones (free list). Write 5 tests: sequential allocation, recycle, display format. | `src/id.rs` + tests | ✅ |
| P1.1.6 | **VexError** — Create `src/error.rs`. Define a unified error enum using `thiserror::Error`. Variants: `Network(String)`, `Parse(String)`, `Css(String)`, `Layout(String)`, `Js(String)`, `Io(#[from] std::io::Error)`, `Url(#[from] url::ParseError)`, `Platform(String)`, `Storage(String)`, `Internal(String)`. Define `pub type VexResult<T> = Result<T, VexError>`. Write 3 tests: error creation, display, from-conversion. | `src/error.rs` + tests | ✅ |
| P1.1.7 | **Lib re-exports** — Update `src/lib.rs` to declare all modules (`mod string; mod url; mod geometry; mod color; mod id; mod error;`) and re-export all public types at crate root. Add crate-level doc comment explaining vex-core's role. | Updated `src/lib.rs` | ✅ |
| P1.1.8 | Run `cargo test -p vex-core` — all tests pass. Run `cargo clippy -p vex-core` — no warnings. | Green tests + clippy | ✅ |

---

### P1.2 — Zig Platform Layer (`zig/platform/`)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P1.2.1 | **Event types** | `event.zig` + tests | ✅ |
| P1.2.2 | **Window handle** | `window.zig` | ✅ |
| P1.2.3 | **Event loop** | Event loop in `window.zig` | ✅ |
| P1.2.4 | **DPI detection** | DPI function | ✅ |
| P1.2.5 | **Raw window handle** | `get_raw_handle` function | ✅ |
| P1.2.6 | **C ABI exports** | `root.zig` exports | ✅ |
| P1.2.7 | **Platform test binary** | `test_window.zig` | ⬜ |
| P1.2.8 | Run `zig build` + `zig build test` — platform library compiles, tests pass. | Green Zig build | ✅ |

---

### P1.3 — Zig Custom Allocators (`zig/alloc/`)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P1.3.1 | **Arena allocator** | `arena.zig` + tests | ✅ |
| P1.3.2 | **Pool allocator** | `pool.zig` + tests | ✅ |
| P1.3.3 | **Frame allocator** | `frame.zig` + tests | ✅ |
| P1.3.4 | **Statistics tracker** — Create `zig/alloc/stats.zig`. A wrapper allocator that tracks: `total_allocated`, `total_freed`, `current_usage`, `peak_usage`, `allocation_count`. Wraps any inner allocator. Write tests: allocate/free, check stats. | `stats.zig` + tests | ✅ |
| P1.3.5 | **C ABI exports** | `root.zig` exports | ✅ |
| P1.3.6 | Run `zig build test` — all allocator tests pass. | Green tests | ✅ |

---

### P1.4 — Rust↔Zig Bridge & Window

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P1.4.1 | **FFI type definitions** | `platform_ffi.rs` | ✅ |
| P1.4.2 | **Safe Rust wrappers** | `platform.rs` | ✅ |
| P1.4.3 | **Event enum (Rust)** | `event.rs` | ✅ |
| P1.4.4 | **wgpu surface creation** | `gpu.rs` | ✅ |
| P1.4.5 | **Render loop** | render_frame method | ✅ |
| P1.4.6 | **Main event loop** | Updated `main.rs` | ✅ |
| P1.4.7 | **Build integration test** — Window opens, GPU renders, mouse/keyboard events logged, close exits cleanly. | Working window demo | ✅ |

---
---

## Phase 2 — Network Stack

**Timeline:** Weeks 7–12  
**Goal:** HTTP/1.1+2 client with TLS 1.3, DNS/DoH, caching, compression, privacy filters.  
**Entry criteria:** Phase 1 complete (window + GPU surface working).  
**Exit criteria:** `vex-net` can fetch any HTTPS page and return the HTML body. Privacy filters strip tracking params. Unit + integration tests pass.

---

### P2.1 — HTTP Client Foundation

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.1.1 | **TLS configuration** — Create `crates/vex-net/src/tls.rs`. Build a `rustls::ClientConfig` with: Mozilla root certificates (via `webpki-roots` crate), TLS 1.3 only (disable 1.2 by default, allow as fallback behind feature flag), ALPN protocols `["h2", "http/1.1"]`. Wrap in `Arc<ClientConfig>` for sharing. Write 2 tests: config creation succeeds, ALPN protocols set correctly. | `tls.rs` + tests | ✅ |
| P2.1.2 | **HTTP client struct** — Create `crates/vex-net/src/client.rs`. Define `HttpClient` struct holding: `hyper_util::client::legacy::Client` with `hyper_rustls::HttpsConnector`, connection pool config (max idle: 100, idle timeout: 90s), default headers (User-Agent: "Vigo/0.1 Vex"). Constructor: `HttpClient::new() -> VexResult<Self>`. | `client.rs` | ✅ |
| P2.1.3 | **Request/Response types** — Create `crates/vex-net/src/types.rs`. Define `Request { url: VexUrl, method: Method, headers: HeaderMap, body: Option<Vec<u8>> }` and `Response { status: u16, headers: HeaderMap, body: Vec<u8>, url: VexUrl, was_cached: bool }`. Enum `Method { Get, Post, Put, Delete, Head, Options }`. | `types.rs` | ✅ |
| P2.1.4 | **Fetch method** — In `client.rs`, implement `async fn fetch(&self, request: Request) -> VexResult<Response>`. Build a `hyper::Request` from vex `Request`, execute via the client, read the full body into `Vec<u8>`, wrap in vex `Response`. Handle connection errors, timeouts (default: 30s), and status codes. | `fetch` method | ✅ |
| P2.1.5 | **Integration test: fetch example.com** — Create `crates/vex-net/tests/fetch_test.rs`. Test `HttpClient::new().fetch(Request::get("https://example.com"))` returns status 200 and body contains `<html`. Requires network — mark with `#[ignore]` for CI but run locally. | Integration test | ✅ |

---

### P2.2 — DNS & DoH

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.2.1 | **DNS resolver** — Create `crates/vex-net/src/dns.rs`. Wrap `trust-dns-resolver::TokioAsyncResolver`. Constructor creates resolver with system config. Method: `async fn resolve(&self, host: &str) -> VexResult<Vec<IpAddr>>`. | `dns.rs` | ✅ |
| P2.2.2 | **DoH support** — Extend `dns.rs`. Add `DnsMode` enum: `System`, `DoH { server_url: String }`. When `DoH` mode is selected, configure `trust-dns-resolver` with HTTPS upstream (e.g., `https://1.1.1.1/dns-query` for Cloudflare). Write test: resolve `example.com` via DoH → returns IP addresses. | DoH mode | ✅ |
| P2.2.3 | **Wire DNS into HTTP client** — Modify `HttpClient` to accept a `DnsMode` config. When DoH is enabled, create a custom `tower::Service` that resolves via DoH before connecting. | DNS integration | ⬜ |

---

### P2.3 — Content Handling

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.3.1 | **Decompression** — Create `crates/vex-net/src/decompress.rs`. Detect `Content-Encoding` header. Support: `gzip` (via `flate2`), `br` (via `brotli`), `zstd` (via `zstd`), `identity` (passthrough). Function: `decompress(encoding: &str, bytes: &[u8]) -> VexResult<Vec<u8>>`. Write 3 tests with pre-compressed payloads. | `decompress.rs` + tests | ✅ |
| P2.3.2 | **Redirect following** — In `client.rs`, implement redirect following in `fetch()`. Follow 3xx responses up to 10 redirects. Handle 301, 302, 303, 307, 308. On 303, change method to GET. Track redirect chain. Write 2 tests using a test HTTP server (or mock). | Redirect handling | ✅ |
| P2.3.3 | **Cookie jar** — Create `crates/vex-net/src/cookies.rs`. Implement `CookieJar` struct: `insert(url, set_cookie_header) -> ()`, `get_cookies(url) -> String` (returns `Cookie` header value). Respect: `Domain`, `Path`, `Secure`, `HttpOnly`, `SameSite`, `Expires`/`Max-Age`. Thread-safe (`Arc<RwLock<...>>`). Write 10 tests: basic set/get, domain matching, path matching, secure flag, expiration, SameSite. | `cookies.rs` + tests | ✅ |
| P2.3.4 | **Wire cookies into client** — Modify `fetch()` to: before request, call `cookie_jar.get_cookies(url)` and add `Cookie` header; after response, call `cookie_jar.insert(url, set_cookie_header)` for each `Set-Cookie` header. | Cookie integration | ✅ |

---

### P2.4 — HTTP Cache

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.4.1 | **Cache storage** — Create `crates/vex-net/src/cache.rs`. Define `HttpCache` struct backed by an in-memory `HashMap<VexUrl, CachedResponse>`. `CachedResponse` stores: body, headers, `ETag`, `Last-Modified`, `Cache-Control` directives (max-age, no-cache, no-store), insertion timestamp. | `cache.rs` | ✅ |
| P2.4.2 | **Cache-Control parsing** — In `cache.rs`, implement `parse_cache_control(header: &str) -> CacheDirectives`. Parse: `max-age=N`, `no-cache`, `no-store`, `must-revalidate`, `public`, `private`. Write 5 tests. | Cache-Control parser | ✅ |
| P2.4.3 | **Cache integration** — Modify `fetch()`: before sending request, check cache. If cached and fresh (within max-age), return cached response with `was_cached: true`. If stale, add `If-None-Match` (ETag) or `If-Modified-Since` header. If server returns 304, return cached body. If `no-store`, skip cache entirely. Write 4 tests with mock server. | Cache logic | ✅ |

---

### P2.5 — Privacy Integration

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.5.1 | **Tracking parameter stripper** — Create `crates/vex-privacy/src/tracking.rs`. Define list of 30+ tracking parameters: `utm_source`, `utm_medium`, `utm_campaign`, `utm_term`, `utm_content`, `fbclid`, `gclid`, `msclkid`, `twclid`, `dclid`, `mc_cid`, `mc_eid`, `_ga`, `_gl`, etc. Function: `strip_tracking_params(url: &mut VexUrl)` — removes matching query parameters. Write 8 tests: each param type, multiple params, non-tracking params preserved. | `tracking.rs` + tests | ✅ |
| P2.5.2 | **Domain blocklist** — Create `crates/vex-privacy/src/adblock.rs`. Implement `AdblockEngine` struct holding a `HashSet<String>` of blocked domains. Method: `is_blocked(url: &VexUrl) -> bool` — checks host against blocklist, including subdomain matching (if `ads.example.com` is blocked, so is `foo.ads.example.com`). Load from a text file (one domain per line). Write 6 tests: exact match, subdomain, non-match, empty host. | `adblock.rs` + tests | ✅ |
| P2.5.3 | **HTTPS-only mode** — Create `crates/vex-privacy/src/https.rs`. Function: `enforce_https(url: &mut VexUrl) -> bool` — upgrades `http://` to `https://`. Returns false if already HTTPS. Exempts: localhost, 127.0.0.1, .local, .onion. Write 5 tests. | `https.rs` + tests | ✅ |
| P2.5.4 | **Header sanitization** — Create `crates/vex-privacy/src/headers.rs`. Function: `sanitize_headers(headers: &mut HeaderMap)` — removes headers: `X-Client-Data`, `Sec-Browsing-Topics`, `Attribution-Reporting-*`. Enforces strict referrer: if cross-origin, reduce to origin-only. Write 4 tests. | `headers.rs` + tests | ✅ |
| P2.5.5 | **Privacy middleware** — Create `crates/vex-privacy/src/middleware.rs`. A `PrivacyLayer` struct that wraps all the above. Method: `process_request(&self, request: &mut Request)` — calls strip_tracking_params, enforce_https, sanitize_headers, check adblock (return error if blocked). Expose in `lib.rs`. | `middleware.rs` | ✅ |
| P2.5.6 | **Wire privacy into vex-net** — Modify `HttpClient::fetch()` to accept an optional `&PrivacyLayer` and call `process_request()` before sending. | Privacy in fetch | ✅ |

---

### P2.6 — Network Verification

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P2.6.1 | Run `cargo test -p vex-net -p vex-privacy` — all unit tests pass. | Green unit tests | ✅ |
| P2.6.2 | Integration test: fetch 10 different HTTPS sites, verify all return 200 and HTML body. | Integration test | ⬜ |
| P2.6.3 | Integration test: fetch a URL with `?utm_source=test&q=hello` → verify `utm_source` stripped, `q` preserved. | Privacy test | ⬜ |
| P2.6.4 | Benchmark: time 100 sequential fetches of `https://example.com` (cached). Target: <500ms total. | Benchmark | ⬜ |

---
---

## Phase 3 — HTML Parser + DOM

**Timeline:** Weeks 13–20  
**Goal:** Parse any HTML page into an in-memory DOM tree. Query, traverse, and serialize it.  
**Entry criteria:** Phase 2 complete (can fetch HTML from the web).  
**Exit criteria:** Can fetch a page via `vex-net`, parse via `vex-html`, build a `vex-dom` tree, query it with `querySelector`, and serialize it back. All tests pass.

---

### P3.1 — DOM Node Arena

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.1.1 | **Node storage** — Create `crates/vex-dom/src/arena.rs`. Implement `NodeArena` — a `Vec<Node>` indexed by `VexId`. Methods: `alloc(node_data: NodeData) -> VexId`, `get(id: VexId) -> &Node`, `get_mut(id: VexId) -> &mut Node`. Node is never removed (arena model — freed when whole document is dropped). | `arena.rs` | ✅ |
| P3.1.2 | **Node structure** — Create `crates/vex-dom/src/node.rs`. Define `Node { id: VexId, parent: Option<VexId>, first_child: Option<VexId>, last_child: Option<VexId>, next_sibling: Option<VexId>, prev_sibling: Option<VexId>, data: NodeData }`. Enum `NodeData { Document, Element(ElementData), Text(String), Comment(String), DocumentFragment }`. `ElementData { tag_name: VexString, attributes: Vec<Attribute>, namespace: Namespace }`. `Attribute { name: VexString, value: String }`. `Namespace` enum: `Html, Svg, MathMl`. | `node.rs` | ✅ |
| P3.1.3 | **Tree manipulation** — Create `crates/vex-dom/src/tree.rs`. Functions that operate on `NodeArena`: `append_child(arena, parent_id, child_id)` — sets up parent/child/sibling links, `insert_before(arena, parent_id, child_id, reference_id)`, `remove_child(arena, parent_id, child_id)` — unlinks from sibling chain (node stays in arena, just disconnected). Write 8 tests: append, insert_before, remove, verify all links. | `tree.rs` + tests | ✅ |
| P3.1.4 | **Traversal** — Create `crates/vex-dom/src/traversal.rs`. Implement iterators: `ChildrenIter` (iterates first_child → next_sibling chain), `DescendantsIter` (depth-first pre-order traversal of subtree), `AncestorsIter` (walks parent chain to root). Each takes `&NodeArena` and a starting `VexId`. Write 5 tests: children count, descendant order, ancestor chain. | `traversal.rs` + tests | ✅ |

---

### P3.2 — Document Object

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.2.1 | **Document struct** — Create `crates/vex-dom/src/document.rs`. `Document { arena: NodeArena, root: VexId }`. The root node is always `NodeData::Document`. Methods: `Document::new() -> Self` (creates arena + root), `Document::root_element() -> Option<VexId>` (first Element child of root — the `<html>` element), `Document::create_element(tag: &str) -> VexId`, `Document::create_text(text: &str) -> VexId`, `Document::create_comment(text: &str) -> VexId`. | `document.rs` | ✅ |
| P3.2.2 | **Element access** — In `document.rs`, add: `get_element_by_id(id: &str) -> Option<VexId>` — linear scan of all elements checking `id` attribute. `get_elements_by_tag_name(tag: &str) -> Vec<VexId>` — linear scan. `get_elements_by_class_name(class: &str) -> Vec<VexId>` — check `class` attribute (space-separated). Write 5 tests. | Element query methods | ✅ |
| P3.2.3 | **Attribute access** — Create `crates/vex-dom/src/attributes.rs`. Helper functions: `get_attribute(arena, id, name) -> Option<&str>`, `set_attribute(arena, id, name, value)`, `remove_attribute(arena, id, name) -> bool`, `has_attribute(arena, id, name) -> bool`, `has_class(arena, id, class_name) -> bool` (checks space-separated class list). Write 6 tests. | `attributes.rs` + tests | ✅ |
| P3.2.4 | **Text content** — In `document.rs` or a new `src/text_content.rs`, implement: `text_content(arena, id) -> String` (concatenate all descendant Text nodes), `inner_html(arena, id) -> String` (serialize children to HTML string), `outer_html(arena, id) -> String` (serialize element + children). Write 4 tests. | Text/HTML accessors | ✅ |

---

### P3.3 — HTML Parser (html5ever Integration)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.3.1 | **TreeSink implementation** — Create `crates/vex-html/src/sink.rs`. Implement `html5ever::tree_builder::TreeSink` for a custom `VexSink` struct that wraps a `Document`. Required methods: `get_document() -> VexId`, `elem_name(target) -> ExpandedName`, `create_element(name, attrs, flags) -> VexId`, `create_comment(text) -> VexId`, `append(parent, child)`, `append_before_sibling(sibling, child)`, `append_doctype_to_document(name, public, system)`, `remove_from_parent(target)`, `reparent_children(node, new_parent)`, `get_template_contents(target) -> VexId`, `same_node(x, y) -> bool`, `set_quirks_mode(mode)`, `mark_script_already_started(node)`, `parse_error(msg)`. Each maps to `vex-dom` tree operations. | `sink.rs` | ✅ |
| P3.3.2 | **Parser wrapper** — Create `crates/vex-html/src/parser.rs`. Function: `parse_html(input: &str) -> Document`. Creates `VexSink`, creates `html5ever::parse_document(sink, ParseOpts::default())`, feeds entire input, returns the `Document`. Also: `parse_html_fragment(input: &str, context_tag: &str) -> Document` for innerHTML parsing. | `parser.rs` | ✅ |
| P3.3.3 | **Incremental parsing** — In `parser.rs`, implement `HtmlParser` struct with `fn feed(&mut self, chunk: &[u8])` and `fn finish(self) -> Document`. Allows parsing as bytes arrive from network. Uses `html5ever::tendril::TendrilSink::process()`. | Incremental parser | ✅ |
| P3.3.4 | **Script/style extraction** — After parsing, walk the DOM. For each `<script>` element: extract text content (inline script) or `src` attribute (external). For each `<style>` element: extract text content. For each `<link rel="stylesheet">`: extract `href`. Return these as `Vec<ScriptInfo>` and `Vec<StyleInfo>`. This is needed later for CSS and JS phases. | Script/style extraction | ✅ |
| P3.3.5 | **Parse tests** — Write 15 unit tests in `crates/vex-html/tests/`: (1) empty document, (2) basic `<html><head><body>`, (3) nested divs, (4) attributes preserved, (5) text nodes, (6) comments, (7) self-closing tags (`<br>`, `<img>`), (8) malformed HTML (missing close tags — verify auto-correction), (9) entities (`&amp;` → `&`), (10) `<template>` content, (11) `<script>` content not parsed as HTML, (12) multiple classes, (13) id attribute, (14) deeply nested structure (100 levels), (15) real-world HTML snippet (paste a chunk of example.com source). | 15 tests | ✅ |

---

### P3.4 — querySelector (Selectors Integration)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.4.1 | **SelectorImpl trait** — Create `crates/vex-dom/src/selector_impl.rs`. Implement the `selectors::SelectorImpl` trait for a `VexSelectorImpl` struct. Define associated types: `AttrValue = String`, `Identifier = VexString`, `LocalName = VexString`, `NamespaceUrl = VexString`, `BooleanAttribute = String`, `NonTSPseudoClass` (empty enum for now), `PseudoElement` (empty enum). | `selector_impl.rs` | ✅ |
| P3.4.2 | **Element trait** — Create `crates/vex-dom/src/selector_element.rs`. Implement `selectors::Element` for a `VexElement<'a>` wrapper that borrows `&'a NodeArena` and holds `VexId`. Required methods: `opaque() -> OpaqueElement`, `parent_element()`, `parent_node()`, `prev_sibling_element()`, `next_sibling_element()`, `first_element_child()`, `is_html_element_in_html_document()`, `has_local_name(name)`, `has_namespace(ns)`, `is_part()`, `has_id(id, case)`, `has_class(name, case)`, `attr_matches(ns, local_name, operation)`, `match_pseudo_class(pc)`, `match_non_ts_pseudo_class(pc)`. Each reads from the arena. | `selector_element.rs` | ✅ |
| P3.4.3 | **querySelector/querySelectorAll** — In `document.rs`, implement: `query_selector(arena, root_id, selector_str) -> Option<VexId>` and `query_selector_all(arena, root_id, selector_str) -> Vec<VexId>`. Use `selectors::parser::SelectorList::parse()` and `selectors::matching::matches_selector()`. Iterate descendants, return first match / all matches. | Query methods | ✅ |
| P3.4.4 | **Selector tests** — Write 10 tests: `div` (type), `.class`, `#id`, `div.foo`, `div > p` (child combinator), `div p` (descendant), `p + p` (adjacent sibling), `[href]` (attribute), `div.a.b` (multiple classes), complex chain `#main > .content p.text`. | 10 tests | ✅ |

---

### P3.5 — Event System

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.5.1 | **Event types** — Create `crates/vex-dom/src/events/event_type.rs`. Define `EventType` enum: `Click`, `MouseDown`, `MouseUp`, `MouseMove`, `KeyDown`, `KeyUp`, `Focus`, `Blur`, `Input`, `Change`, `Submit`, `Load`, `DOMContentLoaded`, `Scroll`, `Resize`, `Custom(String)`. Each has `fn name(&self) -> &str` returning the JS-compatible name (e.g., `Click` → `"click"`). | `event_type.rs` | ✅ |
| P3.5.2 | **Event object** — Create `crates/vex-dom/src/events/event.rs`. Define `Event { event_type: EventType, target: VexId, current_target: Option<VexId>, phase: EventPhase, bubbles: bool, cancelable: bool, default_prevented: bool, propagation_stopped: bool }`. `EventPhase` enum: `Capturing`, `AtTarget`, `Bubbling`. Methods: `prevent_default()`, `stop_propagation()`, `stop_immediate_propagation()`. | `event.rs` | ✅ |
| P3.5.3 | **Listener storage** — Create `crates/vex-dom/src/events/listeners.rs`. `EventListenerMap` — a `HashMap<VexId, HashMap<EventType, Vec<EventListener>>>`. `EventListener { callback_id: u64, capture: bool }`. The `callback_id` is an opaque handle that will be resolved to a JS function in Phase 7. Methods: `add_listener(node, event_type, callback_id, capture)`, `remove_listener(node, event_type, callback_id)`, `get_listeners(node, event_type) -> &[EventListener]`. Write 5 tests. | `listeners.rs` + tests | ✅ |
| P3.5.4 | **Event dispatch** — Create `crates/vex-dom/src/events/dispatch.rs`. Function: `dispatch_event(arena, listeners, event) -> bool` (returns whether default was prevented). Algorithm: (1) Build path from target to root (list of ancestor VexIds). (2) Capture phase: walk root → target, fire listeners with `capture: true`. (3) At-target phase: fire all listeners for target. (4) Bubbling phase (if `event.bubbles`): walk target → root, fire listeners with `capture: false`. At each step, check `propagation_stopped`. Return `event.default_prevented`. Write 6 tests: basic dispatch, bubbling, capture, stopPropagation, preventDefault, non-bubbling event. | `dispatch.rs` + tests | ✅ |

---

### P3.6 — Serialization

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.6.1 | **HTML serializer** — Create `crates/vex-dom/src/serialize.rs`. Function: `serialize_to_html(arena, node_id) -> String`. Walk subtree depth-first. For elements: output `<tag attr="val">...children...</tag>`. For text: escape `<`, `>`, `&`. For void elements (`br`, `img`, `hr`, `input`, `meta`, `link`): self-closing, no end tag. For comments: `<!--text-->`. Write 5 tests including round-trip (parse → serialize → compare). | `serialize.rs` + tests | ✅ |

---

### P3.7 — Integration

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P3.7.1 | **End-to-end test** — `tests/live_page_test.rs` in vex-html. Fetches `https://example.com` via `vex-net`, parses with `vex-html`, queries `h1` → verifies "Example Domain", queries `p` → verifies count, serializes back. Also fetches httpbin.org/html. `#[ignore]` (requires network). | E2E test | ✅ |
| P3.7.2 | **Benchmark** — `tests/parse_bench.rs` in vex-html. Parses 100KB synthetic HTML (10 iterations, avg <50ms debug), 200-level nested HTML, 500 elements × 7 attrs. No criterion — uses `std::time::Instant`. | Benchmark | ✅ |

---
---

## Phase 4 — CSS Parser + Style System

**Timeline:** Weeks 21–28  
**Goal:** Parse CSS, match selectors to DOM elements, resolve the cascade, compute final styles.  
**Entry criteria:** Phase 3 complete (DOM tree built from HTML).  
**Exit criteria:** Given a DOM tree + stylesheets, the style system produces a `ComputedStyle` for every element. Tests pass for specificity, inheritance, and unit resolution.

---

### P4.1 — CSS Value Types

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P4.1.1 | **Length type** — Create `crates/vex-css/src/values/length.rs`. Enum `LengthValue { Px(f32), Em(f32), Rem(f32), Percent(f32), Vw(f32), Vh(f32), Auto, Zero }`. Method: `resolve(font_size: f32, root_font_size: f32, viewport: Size) -> f32` (converts any unit to pixels). Write 8 tests for each unit type conversions. | `length.rs` + tests | ✅ |
| P4.1.2 | **Color value** — Create `crates/vex-css/src/values/color.rs`. Enum `ColorValue { Rgba(Color), CurrentColor, Inherit, Transparent }`. Parsing function: `parse_color(input: &str) -> Option<ColorValue>` — handles `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r,g,b)`, `rgba(r,g,b,a)`, `hsl(h,s,l)`, `hsla(h,s,l,a)`, named colors (148 CSS named colors), `transparent`, `currentcolor`, `inherit`. Write 12 tests. | `color.rs` + tests | ✅ |
| P4.1.3 | **Display value** — Create `crates/vex-css/src/values/display.rs`. Enum `Display { Block, Inline, InlineBlock, Flex, InlineFlex, Grid, InlineGrid, None, Contents, Table, TableRow, TableCell, ListItem }`. Parse from string. | `display.rs` | ✅ |
| P4.1.4 | **Position value** — Enum `Position { Static, Relative, Absolute, Fixed, Sticky }`. | `position.rs` | ✅ |
| P4.1.5 | **Font values** — `FontFamily(Vec<String>)`, `FontWeight` (100-900 + named), `FontStyle` (normal/italic/oblique), `FontSize` (length or keyword like `small`, `medium`, `large`). | `font.rs` | ✅ |
| P4.1.6 | **Box model values** — Struct `BoxSide<T> { top: T, right: T, bottom: T, left: T }` — used for margin, padding, border-width. `BorderStyle` enum (none, solid, dashed, dotted, etc.). | `box_model.rs` | ✅ |
| P4.1.7 | **Flexbox values** — `FlexDirection`, `FlexWrap`, `JustifyContent`, `AlignItems`, `AlignSelf`, `AlignContent`. Each as enum. | `flex.rs` | ✅ |
| P4.1.8 | **Text values** — `TextAlign` (left/center/right/justify), `TextDecoration`, `WhiteSpace`, `Overflow` (visible/hidden/scroll/auto), `VerticalAlign`. | `text.rs` | ✅ |

---

### P4.2 — CSS Parsing

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P4.2.1 | **Property enum** — Create `crates/vex-css/src/properties.rs`. Enum `Property` with ~50 variants covering the most common CSS properties: `Display(Display)`, `Position(Position)`, `Width(LengthValue)`, `Height(LengthValue)`, `MarginTop(LengthValue)` ... (all 4 sides), `PaddingTop(LengthValue)` ... (all 4 sides), `BorderTopWidth(LengthValue)` ..., `BorderTopStyle(BorderStyle)` ..., `BorderTopColor(ColorValue)` ..., `Color(ColorValue)`, `BackgroundColor(ColorValue)`, `FontFamily(FontFamily)`, `FontSize(LengthValue)`, `FontWeight(FontWeight)`, `FontStyle(FontStyle)`, `LineHeight(LengthValue)`, `TextAlign(TextAlign)`, `TextDecoration(TextDecoration)`, `Opacity(f32)`, `Overflow(Overflow)`, `ZIndex(i32)`, `FlexDirection(FlexDirection)`, `FlexWrap(FlexWrap)`, `JustifyContent(JustifyContent)`, `AlignItems(AlignItems)`, `FlexGrow(f32)`, `FlexShrink(f32)`, `FlexBasis(LengthValue)`, `Top/Right/Bottom/Left(LengthValue)`, `Visibility(Visibility)`, `Cursor(Cursor)`, `BoxSizing(BoxSizing)`. | `properties.rs` | ✅ |
| P4.2.2 | **Declaration parser** — Create `crates/vex-css/src/parser/declaration.rs`. Use `cssparser::DeclarationParser` trait. Implement `parse_value(name, input) -> Result<Property>`. For each property name string, delegate to the appropriate value parser. Handle shorthand expansion: `margin: 10px` → 4 separate margin properties, `border: 1px solid black` → 12 properties (width/style/color × 4 sides), `font` shorthand, `background` shorthand. Write 10 tests for each shorthand. | `declaration.rs` + tests | ✅ |
| P4.2.3 | **Rule parser** — Create `crates/vex-css/src/parser/rule.rs`. Use `cssparser::QualifiedRuleParser` trait. Parse a qualified rule: selector list + declaration block. Returns `CssRule { selectors: SelectorList, declarations: Vec<Property> }`. | `rule.rs` | ✅ |
| P4.2.4 | **At-rule parser** — Create `crates/vex-css/src/parser/at_rule.rs`. Use `cssparser::AtRuleParser` trait. Handle `@media` (with condition), `@import` (extract URL), `@font-face` (stub). Ignore unknown at-rules. | `at_rule.rs` | ✅ |
| P4.2.5 | **Stylesheet parser** — Create `crates/vex-css/src/parser/stylesheet.rs`. Top-level function: `parse_stylesheet(css: &str) -> Stylesheet`. `Stylesheet { rules: Vec<CssRule>, media_rules: Vec<MediaRule> }`. Uses `cssparser::StyleSheetParser` with the above parsers. | `stylesheet.rs` | ✅ |
| P4.2.6 | **Inline style parser** — Function: `parse_inline_style(style_attr: &str) -> Vec<Property>`. Parses the content of a `style=""` attribute (just declarations, no selector). | Inline style parser | ✅ |
| P4.2.7 | **Parse tests** — 10 tests: basic rule, multiple selectors, shorthand, media query, inline style, comments, empty stylesheet, `!important` flag, pseudo-classes in selectors, real-world CSS snippet. | 10 tests | ✅ |

---

### P4.3 — Specificity & Cascade

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P4.3.1 | **Specificity calculation** — Create `crates/vex-css/src/cascade/specificity.rs`. The `selectors` crate computes specificity internally. Extract it: `specificity(selector: &Selector) -> (u32, u32, u32)` (ID, class, type counts). Write 5 tests: `#id` = (1,0,0), `.class` = (0,1,0), `div` = (0,0,1), `#id .class div` = (1,1,1), `div.foo > p.bar` = (0,2,2). | `specificity.rs` + tests | ✅ |
| P4.3.2 | **Cascade origin** — Create `crates/vex-css/src/cascade/origin.rs`. Enum `Origin { UserAgent, Author, AuthorImportant, Inline, InlineImportant }`. Ordering: UserAgent < Author < Inline < AuthorImportant < InlineImportant. | `origin.rs` | ✅ |
| P4.3.3 | **Declaration matching** — Create `crates/vex-css/src/cascade/matching.rs`. Function: `collect_matching_declarations(element_id, arena, stylesheets) -> Vec<(Property, Specificity, Origin)>`. For each stylesheet, for each rule, test if any selector matches the element (using the selector Element trait from P3.4.2). Collect all matching declarations with their specificity and origin. | `matching.rs` | ✅ |
| P4.3.4 | **Cascade resolution** — Create `crates/vex-css/src/cascade/resolve.rs`. Function: `resolve_cascade(declarations: Vec<(Property, Specificity, Origin)>) -> Vec<Property>`. Sort by: (1) origin, (2) specificity, (3) source order. For each property, the last (highest priority) wins. Return the "winning" declaration for each property. Write 5 tests: origin ordering, specificity ordering, `!important` override, source order tiebreak. | `resolve.rs` + tests | ✅ |

---

### P4.4 — Computed Styles

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P4.4.1 | **ComputedStyle struct** — Create `crates/vex-css/src/computed.rs`. Define `ComputedStyle` with a field for every supported property, all resolved to absolute pixel values or concrete enum values. Fields: `display: Display`, `position: Position`, `width: f32` (pixels or `f32::NAN` for auto), `height: f32`, `margin_top/right/bottom/left: f32`, `padding_top/right/bottom/left: f32`, `border_top/right/bottom/left_width: f32`, `border_top_style: BorderStyle` ..., `color: Color`, `background_color: Color`, `font_family: Vec<String>`, `font_size: f32` (pixels), `font_weight: u16`, `line_height: f32`, `text_align: TextAlign`, `opacity: f32`, `z_index: i32`, `flex_direction: FlexDirection`, etc. Default constructor: `ComputedStyle::default()` with browser default values (display: inline, font-size: 16px, color: black, etc.). | `computed.rs` | ✅ |
| P4.4.2 | **Inheritance** — Create `crates/vex-css/src/cascade/inheritance.rs`. Define which properties inherit: `color`, `font-family`, `font-size`, `font-weight`, `font-style`, `line-height`, `text-align`, `text-decoration`, `visibility`, `cursor`, `white-space`. Function: `apply_inheritance(child_style: &mut ComputedStyle, parent_style: &ComputedStyle)` — for each inherited property, if the child's value is `Inherit` or unset, copy from parent. Write 3 tests: color inherits, margin does not inherit, explicit value overrides inheritance. | `inheritance.rs` + tests | ✅ |
| P4.4.3 | **Value resolution** — Create `crates/vex-css/src/cascade/value_resolution.rs`. Function: `resolve_value(property: &Property, parent_font_size: f32, root_font_size: f32, viewport: Size) -> resolved value`. Converts `em` → `px` (relative to parent font size), `rem` → `px` (relative to root font size), `%` → `px` (relative to containing block), `vw/vh` → `px` (relative to viewport). Write 8 tests for each unit type. | `value_resolution.rs` + tests | ✅ |
| P4.4.4 | **Style computation pipeline** — Create `crates/vex-css/src/cascade/compute.rs`. Function: `compute_styles(document: &Document, stylesheets: &[Stylesheet]) -> HashMap<VexId, ComputedStyle>`. Algorithm: (1) Collect user-agent stylesheet defaults. (2) For each element (tree order), collect matching declarations, resolve cascade, apply inheritance from parent's computed style, resolve all values to pixels/absolute. Store in map. This is the core style engine. Write 3 tests: basic style application, inheritance chain, cascade override. | `compute.rs` + tests | ✅ |
| P4.4.5 | **User-agent stylesheet** — Create `crates/vex-css/src/ua_stylesheet.rs`. Hardcode a minimal default stylesheet: `html { display: block } body { display: block; margin: 8px } div { display: block } p { display: block; margin-top: 1em; margin-bottom: 1em } h1 { display: block; font-size: 2em; font-weight: bold; margin: 0.67em 0 } h2 { font-size: 1.5em; ... } ... a { color: blue; text-decoration: underline } ul, ol { padding-left: 40px } li { display: list-item } table { display: table } ...`. Cover the 30 most common HTML elements. | `ua_stylesheet.rs` | ✅ |

---

### P4.5 — Media Queries

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P4.5.1 | **Media query evaluator** — Create `crates/vex-css/src/media.rs`. `MediaCondition` enum: `Width(MinMax, f32)`, `Height(MinMax, f32)`, `Screen`, `Print`, `All`, `And(Vec<MediaCondition>)`, `Or(Vec<MediaCondition>)`, `Not(Box<MediaCondition>)`. Function: `evaluate_media(condition: &MediaCondition, viewport: Size) -> bool`. Write 5 tests: min-width, max-width, screen type, AND combo, NOT. | `media.rs` + tests | ✅ |
| P4.5.2 | **Filter stylesheets by media** — When computing styles, filter `@media` rules based on current viewport. Only include matching rules. | Media filtering | ✅ |

---
---

## Phase 5 — Layout Engine

**Timeline:** Weeks 29–40  
**Goal:** Convert styled DOM into positioned boxes with concrete coordinates.  
**Entry criteria:** Phase 4 complete (computed styles for every element).  
**Exit criteria:** Given styled DOM, layout produces a tree of `LayoutBox` with `x, y, width, height` for every element. Block, inline, flex, and positioned layouts work. Benchmarks meet 60fps budget.

---

### P5.1 — Layout Tree Construction

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.1.1 | **LayoutBox struct** — Create `crates/vex-layout/src/box_model.rs`. Define `LayoutBox { node_id: VexId, box_type: BoxType, dimensions: Dimensions, children: Vec<LayoutBox> }`. `BoxType { Block, Inline, InlineBlock, Flex, Anonymous }`. `Dimensions { content: Rect, padding: Insets, border: Insets, margin: Insets }`. Methods: `padding_box() -> Rect`, `border_box() -> Rect`, `margin_box() -> Rect` (each expands outward). Write 3 tests. | `box_model.rs` + tests | ✅ |
| P5.1.2 | **Tree builder** — Create `crates/vex-layout/src/tree_builder.rs`. Function: `build_layout_tree(document: &Document, styles: &HashMap<VexId, ComputedStyle>) -> LayoutBox`. Walk the DOM tree. For each element: skip `display: none`. Map `display: block` → `BoxType::Block`, `display: inline` → `BoxType::Inline`, `display: flex` → `BoxType::Flex`, etc. For text nodes, create anonymous inline boxes. If a block box has mixed block+inline children, wrap the inline runs in anonymous block boxes (per CSS spec). Write 5 tests: pure block, pure inline, mixed, display:none skipped, nested. | `tree_builder.rs` + tests | ✅ |

---

### P5.2 — Block Layout

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.2.1 | **Width calculation** — Create `crates/vex-layout/src/block.rs`. Function: `calculate_block_width(box: &mut LayoutBox, containing_width: f32, style: &ComputedStyle)`. Algorithm: if `width` is specified, use it. Else, `width = containing_width - margin_left - margin_right - padding_left - padding_right - border_left - border_right`. Handle `auto` margins (for centering). Handle `box-sizing: border-box`. Write 5 tests: fixed width, auto width, auto margins (centering), border-box. | Width calculation + tests | ✅ |
| P5.2.2 | **Height calculation** — In `block.rs`, function: `calculate_block_height(box: &mut LayoutBox, style: &ComputedStyle)`. If `height` is specified, use it. Else, height = sum of children's margin boxes. Handle `min-height`/`max-height`. | Height calculation | ✅ |
| P5.2.3 | **Position children** — In `block.rs`, function: `layout_block(box: &mut LayoutBox, containing: Rect, styles)`. Position children vertically: each child starts at `y = previous_child.margin_box().bottom`. Apply margin collapsing between adjacent block siblings (larger margin wins, not additive). Write 4 tests: stacking, margin collapsing, nested blocks. | Block positioning + tests | ✅ |
| P5.2.4 | **Overflow** — Handle `overflow: hidden` by setting a clip rect on the LayoutBox. `overflow: scroll` adds scrollable area (track content height vs box height). `overflow: visible` has no clip. Store `clip_rect: Option<Rect>` and `scroll_offset: Point` on LayoutBox. | Overflow handling | ✅ |

---

### P5.3 — Inline Layout + Text

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.3.1 | **Font system init** — Create `crates/vex-layout/src/text.rs`. Initialize `cosmic_text::FontSystem` (loads system fonts). Create `SwashCache` for glyph rasterization. Wrap in a shared `TextEngine` struct. Method: `measure_text(text: &str, font_family: &[String], font_size: f32, max_width: f32) -> TextLayout` where `TextLayout { lines: Vec<TextLine>, total_height: f32 }`, `TextLine { glyphs: Vec<GlyphInfo>, width: f32, baseline: f32 }`. | `text.rs` | ✅ |
| P5.3.2 | **Line breaking** — In `text.rs`, implement word-wrap line breaking. Use `cosmic_text::Buffer` to shape text with a set width. Extract line breaks. Handle `word-break: break-word` and `overflow-wrap: break-word`. Write 3 tests: normal wrapping, single long word, multiple lines. | Line breaking | ✅ |
| P5.3.3 | **Inline box layout** — Create `crates/vex-layout/src/inline.rs`. Function: `layout_inline(box: &mut LayoutBox, containing_width: f32, text_engine: &TextEngine, styles)`. For inline elements: flow left-to-right, wrap to next line when exceeding `containing_width`. For text nodes: call `measure_text`, position each line. For inline-block: layout as block, then place inline. Handle `text-align` at the line level (left/center/right/justify). Write 4 tests. | `inline.rs` + tests | ✅ |

---

### P5.4 — Flexbox Layout

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.4.1 | **Flex container** — Create `crates/vex-layout/src/flex.rs`. Function: `layout_flex(box: &mut LayoutBox, containing: Rect, styles)`. Implement CSS Flexbox Level 1 algorithm: (1) Determine main axis from `flex-direction`. (2) Resolve flex item sizes: collect `flex-basis`, `flex-grow`, `flex-shrink` for each child. (3) Calculate free space on main axis. (4) Distribute free space: grow items with `flex-grow > 0`, shrink items if overflow with `flex-shrink > 0`. (5) Handle `flex-wrap: wrap` — when items overflow, start new flex line. | Flex core algorithm | ✅ |
| P5.4.2 | **Flex alignment** — In `flex.rs`, implement: `justify-content` (flex-start, flex-end, center, space-between, space-around, space-evenly) — distributes space on main axis. `align-items` (stretch, flex-start, flex-end, center, baseline) — positions items on cross axis. `align-self` — per-item override. Write 6 tests: each justify-content value, stretch vs center, wrap. | Flex alignment + tests | ✅ |
| P5.4.3 | **Flex order** — In `flex.rs`, sort flex items by `order` property before layout (default order: 0, preserve source order for equal values). Write 1 test. | Flex order | ✅ |

---

### P5.5 — Positioned Layout

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.5.1 | **Relative positioning** — Create `crates/vex-layout/src/positioned.rs`. After normal flow layout, for `position: relative` elements: offset by `top`/`left`/`right`/`bottom` from normal position. Don't affect siblings. Write 2 tests. | Relative positioning + tests | ✅ |
| P5.5.2 | **Absolute positioning** — For `position: absolute`: remove from normal flow. Find nearest ancestor with `position` ≠ `static` (the "containing block"). Position relative to that containing block using `top`/`left`/`right`/`bottom`. If both `left` and `right` set, compute width. Write 3 tests. | Absolute positioning + tests | ✅ |
| P5.5.3 | **Fixed positioning** — For `position: fixed`: position relative to viewport. Store separately so it doesn't scroll with content. Write 1 test. | Fixed positioning | ✅ |
| P5.5.4 | **Z-index stacking** — Create `crates/vex-layout/src/stacking.rs`. Build stacking contexts: elements with `position` ≠ `static` and `z-index` ≠ `auto` create stacking contexts. Sort children by z-index within each context. Output: `Vec<StackingLayer>` ordered back-to-front for the painter. Write 3 tests. | `stacking.rs` + tests | ✅ |

---

### P5.6 — Layout Pipeline & Verification

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P5.6.1 | **Layout pipeline** — Create `crates/vex-layout/src/lib.rs` pipeline function: `layout(document, styles, viewport_size, text_engine) -> LayoutBox`. Build tree → block layout → inline layout → flex layout → positioned layout → stacking order. | Pipeline function | ✅ |
| P5.6.2 | **Benchmark** — `tests/layout_bench.rs` in vex-layout. 5000-element layout (`#[ignore]`, release target <16ms), 200-element style+layout, 200-level deep nesting. No criterion — uses `std::time::Instant`. | Benchmark | ✅ |
| P5.6.3 | **Visual dump** — Implement `dump_layout_tree(box, indent) -> String` that prints a text representation of each box's position and size. Useful for debugging. | Debug dump | ✅ |

---
---

## Phase 6 — GPU Rendering Pipeline

**Timeline:** Weeks 41–52  
**Goal:** Paint layout boxes to pixels on screen via GPU. Text, backgrounds, borders, images visible.  
**Entry criteria:** Phase 5 complete (layout tree with positions/sizes).  
**Exit criteria:** Running the browser loads a page and displays it visually in the window. Text is readable with antialiasing. Images display. 60fps achieved on simple pages.

---

### P6.1 — Display List

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.1.1 | **Display commands** — Create `crates/vex-render/src/display_list.rs`. Enum `DisplayCommand { FillRect { rect: Rect, color: Color, border_radius: f32 }, DrawBorder { rect: Rect, widths: Insets, colors: [Color; 4], styles: [BorderStyle; 4] }, DrawText { position: Point, glyphs: Vec<GlyphInstance>, color: Color, font_size: f32 }, DrawImage { rect: Rect, image_id: ImageId }, PushClip { rect: Rect }, PopClip, PushOpacity { opacity: f32 }, PopOpacity }`. `GlyphInstance { glyph_id: u32, x: f32, y: f32 }`. `DisplayList = Vec<DisplayCommand>`. | `display_list.rs` | ✅ |
| P6.1.2 | **Display list builder** — Create `crates/vex-render/src/painter.rs`. Function: `build_display_list(layout_root: &LayoutBox, styles: &HashMap<VexId, ComputedStyle>) -> DisplayList`. Walk layout tree in paint order (respecting stacking contexts). For each box: (1) draw background-color as FillRect, (2) draw borders as DrawBorder, (3) for text nodes, emit DrawText with positioned glyphs, (4) for `<img>` elements, emit DrawImage, (5) for `overflow: hidden`, emit PushClip/PopClip around children, (6) for `opacity < 1.0`, emit PushOpacity/PopOpacity. Write 3 tests: simple rect, text, nested clips. | `painter.rs` + tests | ✅ |
| P6.1.3 | **Culling optimisation** — In `painter.rs`, before emitting a command, check if the box's rect intersects the viewport rect. If entirely outside, skip it and all children. This prevents off-screen elements from generating GPU work. | Viewport culling | ✅ |

---

### P6.2 — GPU Backend (wgpu)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.2.1 | **Renderer struct** — Create `crates/vex-render/src/renderer.rs`. `Renderer { device: wgpu::Device, queue: wgpu::Queue, surface: wgpu::Surface, rect_pipeline: wgpu::RenderPipeline, text_pipeline: wgpu::RenderPipeline, image_pipeline: wgpu::RenderPipeline, glyph_atlas: GlyphAtlas, image_atlas: ImageAtlas }`. Constructor takes `GpuContext` from Phase 1. | `renderer.rs` | ✅ |
| P6.2.2 | **Rectangle shader** — Create `crates/vex-render/src/shaders/rect.wgsl`. Vertex shader: takes `position: vec2<f32>` + `rect: vec4<f32>` (x, y, w, h) + `color: vec4<f32>` + `border_radius: f32`. Outputs screen-space position + color. Fragment shader: outputs color, apply SDF for border-radius (smooth rounded corners). Create matching `wgpu::RenderPipeline`. | `rect.wgsl` + pipeline | ✅ |
| P6.2.3 | **Text shader** — Create `crates/vex-render/src/shaders/text.wgsl`. Vertex shader: takes glyph position + atlas UV coords. Fragment shader: samples glyph atlas texture, applies alpha test, multiplies by text color. Create matching pipeline. Supports subpixel rendering (sample R/G/B atlas channels separately). | `text.wgsl` + pipeline | ✅ |
| P6.2.4 | **Image shader** — Create `crates/vex-render/src/shaders/image.wgsl`. Vertex shader: quad with texture coords. Fragment shader: sample image texture. Create matching pipeline. | `image.wgsl` + pipeline | ✅ |
| P6.2.5 | **Render frame** — In `renderer.rs`, implement `fn render(&mut self, display_list: &DisplayList, viewport: Size)`. Algorithm: (1) Get surface texture. (2) Create command encoder. (3) Begin render pass with background clear. (4) Walk display list: batch FillRect commands into rect pipeline draw call, batch DrawText into text pipeline draw call, batch DrawImage into image pipeline draw call. (5) Handle PushClip/PopClip via scissor rects. (6) Handle PushOpacity via alpha blending state. (7) Submit + present. | Frame rendering | ✅ |
| P6.2.6 | **Batching** — In renderer, implement draw call batching: accumulate all FillRect commands into one vertex buffer, all DrawText into one vertex buffer, etc. Submit one draw call per pipeline per frame (minimize state changes). Target: <10 draw calls per frame for a typical page. | Batching | ✅ |

---

### P6.3 — Glyph Atlas

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.3.1 | **Atlas texture** — Create `crates/vex-render/src/glyph_atlas.rs`. `GlyphAtlas` manages a large GPU texture (e.g., 2048×2048 RGBA). Packs rasterized glyphs into the texture using a shelf-based packing algorithm (rows of varying height). Method: `get_or_rasterize(glyph_id, font_size, font_family) -> GlyphUV` — returns UV coordinates in the atlas. If glyph not cached, rasterize via `cosmic_text::SwashCache` and upload to texture. | `glyph_atlas.rs` | ✅ |
| P6.3.2 | **LRU eviction** — When atlas is full, evict least-recently-used glyphs. Track usage per glyph with a counter bumped each frame. When atlas hits 90% capacity, evict bottom 20% by usage. | LRU eviction | ✅ |
| P6.3.3 | **Subpixel rendering** — Rasterize glyphs at subpixel offsets (quantize to 4 subpixel positions per pixel). Store R/G/B channels separately in atlas for ClearType-style rendering. | Subpixel support | ⬜ |

---

### P6.4 — Image Pipeline

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.4.1 | **Image decode** — Create `crates/vex-render/src/image_decode.rs`. Function: `decode_image(bytes: &[u8]) -> VexResult<DecodedImage>`. `DecodedImage { width: u32, height: u32, pixels: Vec<u8> (RGBA) }`. Use `image` crate to handle PNG, JPEG, WebP, GIF (first frame), BMP. Use `resvg` for SVG. Write 4 tests with embedded test images. | `image_decode.rs` + tests | ✅ |
| P6.4.2 | **Image atlas** — Create `crates/vex-render/src/image_atlas.rs`. Like glyph atlas but for decoded images. Large texture (4096×4096), shelf-packed. Method: `upload(decoded: &DecodedImage) -> ImageUV`. LRU eviction when full. | `image_atlas.rs` | ✅ |
| P6.4.3 | **Async image loading** — In the browser pipeline (later wired in Phase 8), images load asynchronously: start fetch via `vex-net`, decode on background thread via `tokio::spawn_blocking`, upload to atlas, trigger re-render. Pages render immediately with placeholder boxes (sized by `width`/`height` attributes), then fill in as images arrive. | Async loading pattern | ⬜ |

---

### P6.5 — Scrolling

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.5.1 | **Scroll state** — Create `crates/vex-render/src/scroll.rs`. `ScrollState { offset_x: f32, offset_y: f32, content_height: f32, viewport_height: f32 }`. Methods: `scroll_by(dx, dy)` (clamped to content bounds), `scroll_to(x, y)`, `can_scroll_down/up() -> bool`. | `scroll.rs` | ✅ |
| P6.5.2 | **Scroll integration** — In the render pipeline, apply scroll offset as a translation transform to the display list. On `MouseScroll` event, update scroll state and trigger re-render. Handle smooth scrolling (animate offset over time). | Scroll integration | ✅ |

---

### P6.6 — Full Pipeline Test

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P6.6.1 | **Pipeline integration** — Wire the full pipeline in `vex-app/main.rs`: (1) Create window + GPU context. (2) Fetch `https://example.com`. (3) Parse HTML → DOM. (4) Parse CSS (inline + `<style>` elements). (5) Compute styles. (6) Layout with viewport size. (7) Build display list. (8) Render to screen. (9) Handle scroll/resize events → re-layout/re-render. | Full pipeline | ✅ |
| P6.6.2 | **Screenshot comparison** — Add ability to render to an offscreen texture and save as PNG. Compare against reference screenshots of known pages. | Visual testing | ✅ |
| P6.6.3 | **FPS counter** — Display frame time / FPS in window title bar. Target: 60fps on a simple page (< 500 elements). | FPS measurement | ✅ |

---
---

## Phase 7 — JavaScript Engine

**Timeline:** Weeks 53–68  
**Goal:** Execute JavaScript, manipulate DOM via Web APIs, handle events & forms.  
**Entry criteria:** Phase 6 complete (pages render visually).  
**Exit criteria:** Pages with inline `<script>` tags execute. DOM manipulation works. onclick handlers fire. Forms submit. fetch() API works.

---

### P7.1 — Boa Runtime Setup

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P7.1.1 | **JS context** — Create `crates/vex-js/src/context.rs`. `JsRuntime` struct wrapping `boa_engine::Context`. Constructor creates context with default built-ins. Method: `execute(script: &str) -> VexResult<JsValue>` — parses and executes JS string, returns result or error. Write 3 tests: arithmetic, string concat, error handling. | `context.rs` + tests | ✅ |
| P7.1.2 | **Console API** — Create `crates/vex-js/src/api/console.rs`. Register `console` global object with methods: `log(...)`, `warn(...)`, `error(...)`, `info(...)`, `debug(...)`. Each formats arguments and sends to `tracing`. Register on context at construction. Write 3 tests: console.log format, multiple args, non-string args. | `console.rs` + tests | ✅ |
| P7.1.3 | **Timer API** — Create `crates/vex-js/src/api/timers.rs`. Register global functions: `setTimeout(callback, delay)`, `setInterval(callback, delay)`, `clearTimeout(id)`, `clearInterval(id)`. Store pending timers in a `BTreeMap<Instant, TimerEntry>`. In the event loop, check for expired timers and invoke callbacks. Write 3 tests: setTimeout fires, clearTimeout cancels, setInterval repeats. | `timers.rs` + tests | ✅ |
| P7.1.4 | **Fetch API** — Create `crates/vex-js/src/api/fetch.rs`. Register global `fetch(url, options?)` function that returns a Promise. Internally calls `vex-net::HttpClient::fetch()`. The Promise resolves with a `Response` object that has `.text()`, `.json()`, `.status`, `.ok`, `.headers`. Write 2 tests: fetch returns text, fetch returns JSON. | `fetch.rs` + tests | ✅ |

---

### P7.2 — DOM Bindings

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P7.2.1 | **Window object** — Create `crates/vex-js/src/api/window.rs`. Register `window` global with properties: `location` (object with `href`, `origin`, `pathname`, `search`, `hash`, `assign(url)`, `reload()`), `history` (object with `back()`, `forward()`, `pushState(state, title, url)`), `navigator` (object with `userAgent`), `innerWidth`, `innerHeight`. | `window.rs` | ✅ |
| P7.2.2 | **Document object** — Create `crates/vex-js/src/api/document.rs`. Register `document` global with methods: `getElementById(id)`, `querySelector(sel)`, `querySelectorAll(sel)`, `createElement(tag)`, `createTextNode(text)`, `createDocumentFragment()`. Each returns a JS object proxy wrapping a `VexId`. The proxy delegates property access to the Rust DOM. | `document.rs` | ✅ |
| P7.2.3 | **Element proxy** — Create `crates/vex-js/src/api/element.rs`. When a DOM element is exposed to JS, create a proxy object with: properties (`tagName`, `id`, `className`, `innerHTML`, `textContent`, `children`, `parentElement`, `style`), methods (`getAttribute(name)`, `setAttribute(name, value)`, `removeAttribute(name)`, `appendChild(child)`, `removeChild(child)`, `insertBefore(newNode, refNode)`, `addEventListener(type, callback, options?)`, `removeEventListener(type, callback)`, `classList.add/remove/toggle/contains`). Each property/method reads/writes the Rust DOM via the shared `Document`. | `element.rs` | ✅ |
| P7.2.4 | **Style proxy** — Create `crates/vex-js/src/api/style_proxy.rs`. The `element.style` property returns an object where getting/setting properties (e.g., `el.style.color = 'red'`) reads/writes the element's inline style attribute. Triggers re-style/re-layout. | `style_proxy.rs` | ✅ |
| P7.2.5 | **Event bridge** — Create `crates/vex-js/src/api/events.rs`. When `addEventListener` is called from JS, store the callback as a `JsValue` (Boa GC-rooted) and register it in the DOM event listener map with a callback ID. When `dispatch_event` fires in the Rust DOM and reaches a JS callback ID, invoke the JS callback with an `Event` proxy object (containing `type`, `target`, `currentTarget`, `preventDefault()`, `stopPropagation()`). Write 3 tests: click handler fires, event properties accessible, preventDefault works. | `events.rs` + tests | ✅ |
| P7.2.6 | **GC rooting** — Ensure DOM nodes referenced by JS are not dropped. When a JS proxy references a `VexId`, add it to a `GcRootSet<VexId>`. The DOM arena never frees nodes in the root set. When JS GC collects the proxy, remove from root set. | GC integration | ✅ |

---

### P7.3 — Script Execution Lifecycle

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P7.3.1 | **Script discovery** — After HTML parsing, collect all `<script>` elements. Classify: inline (has text content, no `src`) vs external (has `src` attribute). Record `defer` and `async` flags. | Script collection | ✅ |
| P7.3.2 | **Blocking scripts** — For inline scripts and external scripts without `defer`/`async`: fetch external source (via `vex-net`), execute synchronously (block parsing/rendering until complete). Execute in document order. | Blocking execution | ✅ |
| P7.3.3 | **Defer scripts** — For `<script defer>`: fetch in parallel during parsing. Execute all in document order after DOM is fully built, before `DOMContentLoaded`. | Defer execution | ✅ |
| P7.3.4 | **Async scripts** — For `<script async>`: fetch in parallel. Execute as soon as downloaded, regardless of document order. | Async execution | ✅ |
| P7.3.5 | **DOMContentLoaded & load events** — Fire `DOMContentLoaded` after DOM + all defer scripts are done. Fire `load` after all resources (images, styles, deferred scripts) are loaded. | Lifecycle events | ✅ |

---

### P7.4 — Forms & Input

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P7.4.1 | **Input element model** — Create `crates/vex-dom/src/forms.rs`. Define `InputState { value: String, selection_start: usize, selection_end: usize, checked: bool }`. Attach to `<input>`, `<textarea>`, `<select>` elements. | `forms.rs` | ✅ |
| P7.4.2 | **Text input editing** — Handle keyboard events on focused input: insert character at cursor, delete/backspace, arrow keys to move cursor, Ctrl+A select all, Ctrl+C/V copy/paste (via platform clipboard API). Fire `input` and `change` DOM events. | Text editing | ✅ |
| P7.4.3 | **Form submission** — On `<form>` submit (button click or Enter key): collect all `<input>` values within the form. Build URL-encoded body (for POST) or query string (for GET). Navigate to form `action` URL with the data. Fire `submit` DOM event (cancellable). | Form submission | ✅ |
| P7.4.4 | **Input rendering** — In the layout/render pipeline: `<input type="text">` renders as a box with text + cursor. `<input type="checkbox">` renders as a checkmark box. `<input type="radio">` renders as a circle. `<button>` renders as a styled box with text content. `<select>` renders as a dropdown (stub — button with popup). | Input rendering | ✅ |

---
---

## Phase 8 — Browser Chrome

**Timeline:** Weeks 69–80  
**Goal:** Full browser shell — tabs, address bar, navigation, bookmarks, history, settings.  
**Entry criteria:** Phase 7 complete (pages render + JS executes).  
**Exit criteria:** Multi-tab browsing works. Type URL → navigate. Back/forward. Bookmarks save/restore. Find in page works.

---

### P8.1 — Tab System

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P8.1.1 | **Tab model** — Create `crates/vex-browser/src/tab.rs`. `Tab { id: TabId, url: VexUrl, title: String, document: Option<Document>, styles: Option<HashMap<VexId, ComputedStyle>>, layout: Option<LayoutBox>, js_runtime: Option<JsRuntime>, scroll: ScrollState, loading: LoadingState, favicon: Option<ImageId> }`. `LoadingState` enum: `Idle`, `Connecting`, `Loading { progress: f32 }`, `Complete`. Methods: `load_url(url)`, `reload()`, `stop()`. | `tab.rs` | ⬜ |
| P8.1.2 | **Tab manager** — Create `crates/vex-browser/src/tab_manager.rs`. `TabManager { tabs: Vec<Tab>, active_tab: usize }`. Methods: `new_tab(url) -> TabId`, `close_tab(id)`, `switch_to(id)`, `active_tab() -> &Tab`, `move_tab(from, to)`, `duplicate_tab(id)`, `tab_count() -> usize`. Write 5 tests. | `tab_manager.rs` + tests | ⬜ |
| P8.1.3 | **Page load pipeline per tab** — In `tab.rs`, implement `Tab::load_url(url)`: (1) Set loading state to `Connecting`. (2) Fetch URL via `vex-net`. (3) Set loading state to `Loading`. (4) Parse HTML → DOM. (5) Collect and parse CSS. (6) Compute styles. (7) Layout. (8) Execute scripts. (9) Set loading state to `Complete`. (10) Extract `<title>` for tab title. All async with progress updates. | Tab load pipeline | ⬜ |
| P8.1.4 | **Session state** — Create `crates/vex-browser/src/session.rs`. On browser close, serialize open tabs (URLs, scroll positions, active tab index) to JSON file in user data directory. On browser start, restore from file. Lazy tab loading: only reload the active tab immediately, others load when switched to. | Session persistence | ⬜ |

---

### P8.2 — Navigation

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P8.2.1 | **History stack** — Create `crates/vex-browser/src/navigation.rs`. Per-tab `NavigationHistory { entries: Vec<HistoryEntry>, current_index: usize }`. `HistoryEntry { url: VexUrl, title: String, scroll_position: Point }`. Methods: `push(url)`, `back() -> Option<&HistoryEntry>`, `forward() -> Option<&HistoryEntry>`, `can_go_back() -> bool`, `can_go_forward() -> bool`. Write 5 tests. | `navigation.rs` + tests | ⬜ |
| P8.2.2 | **Link clicking** — When user clicks an `<a href="...">` element: resolve the href relative to current URL, call `tab.load_url(resolved)`. For `target="_blank"`, open in new tab. For `javascript:` hrefs, execute the JS. | Link handling | ⬜ |
| P8.2.3 | **Error pages** — Define error page templates (simple HTML strings): DNS failure ("We can't find that page"), connection refused, TLS error ("Your connection is not private"), 404, 500. Render as a normal DOM document in the tab. | Error pages | ⬜ |

---

### P8.3 — Browser UI (Chrome)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P8.3.1 | **UI layout model** — Create `crates/vex-browser/src/ui/layout.rs`. Define browser chrome regions: `TabBar { height: 36px, top of window }`, `NavigationBar { height: 40px, below tab bar }` containing back/forward/reload buttons + address bar + menu button, `ContentArea { fills remaining space }`. These are NOT DOM elements — they're custom-drawn UI widgets. | UI layout model | ⬜ |
| P8.3.2 | **Tab bar rendering** — Create `crates/vex-browser/src/ui/tab_bar.rs`. Draw tab bar using `DisplayCommand::FillRect` and `DrawText`. Each tab: rounded-top rectangle, title text (truncated with ellipsis), close button (X). Active tab: highlighted color. Hover state. "+" button for new tab. Tabs resize to fit width. | Tab bar rendering | ⬜ |
| P8.3.3 | **Navigation bar rendering** — Create `crates/vex-browser/src/ui/nav_bar.rs`. Draw: back arrow button (grayed out if can't go back), forward arrow, reload/stop button, address bar (text input showing current URL), HTTPS lock icon (green for valid cert). | Nav bar rendering | ⬜ |
| P8.3.4 | **Address bar input** — Implement text input in the address bar. On focus (Ctrl+L or click): select all text. On typing: filter text. On Enter: navigate to URL (add `https://` if no scheme). On Escape: cancel editing, restore original URL. | Address bar input | ⬜ |
| P8.3.5 | **Keyboard shortcuts** — Create `crates/vex-browser/src/ui/shortcuts.rs`. Map: `Ctrl+T` → new tab, `Ctrl+W` → close tab, `Ctrl+L` → focus address bar, `Ctrl+R` / `F5` → reload, `Ctrl+Shift+T` → reopen last closed tab, `Ctrl+Tab` / `Ctrl+Shift+Tab` → next/prev tab, `Alt+Left` → back, `Alt+Right` → forward, `Ctrl+F` → find in page, `Ctrl+D` → bookmark, `F11` → fullscreen, `Ctrl++` / `Ctrl+-` → zoom, `Ctrl+0` → reset zoom. | Shortcuts | ⬜ |
| P8.3.6 | **Context menu** — Create `crates/vex-browser/src/ui/context_menu.rs`. On right-click: show a custom-drawn menu with options: "Open Link in New Tab" (if on a link), "Copy Link Address", "Copy", "Paste", "Select All", "Inspect Element" (stub for DevTools), "View Page Source". Hit-test the click position against DOM to determine context. | Context menu | ⬜ |

---

### P8.4 — Bookmarks & History

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P8.4.1 | **Bookmark storage** — Create `crates/vex-browser/src/bookmarks.rs`. `Bookmark { url: VexUrl, title: String, folder: String, created: DateTime }`. Stored in SQLite via `vex-storage`. Methods: `add(url, title, folder)`, `remove(url)`, `list(folder) -> Vec<Bookmark>`, `search(query) -> Vec<Bookmark>`. Write 4 tests. | `bookmarks.rs` + tests | ⬜ |
| P8.4.2 | **Bookmark bar** — Render bookmarks bar below nav bar (optional, togglable). Show bookmarks in root folder as clickable buttons. Folders as dropdown menus. | Bookmark bar UI | ⬜ |
| P8.4.3 | **History storage** — Create `crates/vex-browser/src/history.rs`. `HistoryItem { url: VexUrl, title: String, visited_at: DateTime, visit_count: u32 }`. Stored in SQLite. Methods: `record_visit(url, title)`, `search(query, limit) -> Vec<HistoryItem>`, `get_recent(limit) -> Vec<HistoryItem>`, `clear_all()`, `clear_range(from, to)`. Write 4 tests. | `history.rs` + tests | ⬜ |
| P8.4.4 | **Find in page** — Create `crates/vex-browser/src/find.rs`. On `Ctrl+F`: show find bar (text input at top of content area). On typing: search all text nodes in DOM for match (case-insensitive). Highlight all matches (yellow background via overlay DisplayCommands). Navigate between matches with Enter/Shift+Enter. Show "N of M" counter. On Escape: close find bar. | Find in page | ⬜ |
| P8.4.5 | **Downloads** — Create `crates/vex-browser/src/downloads.rs`. When a navigation results in a non-HTML Content-Type (or `Content-Disposition: attachment`): show download dialog (file name, size). Save to user's downloads directory. Track progress. Show download shelf at bottom of window. | Downloads | ⬜ |
| P8.4.6 | **Zoom** — Create `crates/vex-browser/src/zoom.rs`. Per-tab zoom level (default: 100%). `Ctrl++` → +10%, `Ctrl+-` → -10%, `Ctrl+0` → reset. Apply zoom as a scale factor to the layout viewport (smaller viewport = larger content). Persist zoom per origin. | Zoom | ⬜ |

---

### P8.5 — Settings

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P8.5.1 | **Settings storage** — Create `crates/vex-browser/src/settings.rs`. Key-value store backed by SQLite or JSON file. Categories: Privacy (adblock on/off, DoH server, HTTPS-only, tracking protection level), Appearance (theme: light/dark/system, font size, zoom default), Search (default engine URL template), General (homepage, on-startup behavior, download location). | `settings.rs` | ⬜ |
| P8.5.2 | **Settings UI** — A special internal page (`vigo://settings`) rendered using the same browser chrome UI system (custom-drawn, not HTML). Sections with toggles, dropdowns, text inputs. Changes persist immediately. | Settings page | ⬜ |

---
---

## Phase 9 — Media Pipeline + DRM Hybrid

**Timeline:** Weeks 81–92  
**Goal:** Video/audio playback with hardware acceleration. DRM via embedded webview.  
**Entry criteria:** Phase 8 complete (browsing works).  
**Exit criteria:** `<video>` and `<audio>` play. YouTube works (non-DRM). Netflix via webview fallback.

---

### P9.1 — Audio/Video Decode (Zig)

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P9.1.1 | **ffmpeg bindings** — In `zig/media/ffmpeg.zig`, create Zig bindings for ffmpeg's `libavcodec`, `libavformat`, `libswresample`, `libswscale`. Functions: `open_file(path) -> MediaContext`, `read_packet(ctx) -> Packet`, `decode_video(ctx, packet) -> VideoFrame`, `decode_audio(ctx, packet) -> AudioFrame`, `close(ctx)`. `VideoFrame { width, height, pixels: [*]u8, format: PixelFormat }`. `AudioFrame { samples: [*]f32, channels: u32, sample_rate: u32 }`. | `ffmpeg.zig` | ⬜ |
| P9.1.2 | **Hardware decode** — In `zig/media/hw_decode.zig`, detect available hardware decoders. On Windows: probe DXVA2/D3D11VA. Initialize hardware decoder context. When decoding, prefer hardware path, fall back to software. Export: `vex_media_hw_init() -> bool`, `vex_media_hw_decode(packet) -> VideoFrame`. | `hw_decode.zig` | ⬜ |
| P9.1.3 | **Audio output** — In `zig/media/audio_output.zig`, open platform audio device. On Windows: use WASAPI (via win32 API). Write audio samples to output buffer. Handle sample rate conversion. Export: `vex_audio_open(sample_rate, channels) -> AudioHandle`, `vex_audio_write(handle, samples, count)`, `vex_audio_close(handle)`. | `audio_output.zig` | ⬜ |
| P9.1.4 | **A/V sync** — Create `crates/vex-media/src/sync.rs`. Clock-based synchronization: master clock from audio (audio drives timing). Video frames presented at their PTS (presentation timestamp) relative to audio clock. If video is behind, skip frame. If ahead, wait. Target: <20ms audio-video drift. | `sync.rs` | ⬜ |

---

### P9.2 — Media Element Integration

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P9.2.1 | **HTMLMediaElement** — Create `crates/vex-media/src/media_element.rs`. Model `<video>` and `<audio>` elements. Properties: `src`, `currentTime`, `duration`, `paused`, `volume`, `muted`, `playbackRate`, `readyState`. Methods: `play()`, `pause()`, `seek(time)`. Events: `play`, `pause`, `timeupdate`, `ended`, `canplay`, `error`. | `media_element.rs` | ⬜ |
| P9.2.2 | **Media source loading** — When `<video src="...">` is encountered: fetch the URL via `vex-net`. Detect format (MP4, WebM, etc.) from Content-Type or file extension. Open with ffmpeg bindings. Start decode loop on background thread. | Media loading | ⬜ |
| P9.2.3 | **Video rendering** — Decoded video frames → upload to GPU texture → render as `DrawImage` in the display list at the `<video>` element's layout position. Update texture each frame at playback rate. | Video rendering | ⬜ |
| P9.2.4 | **Media controls UI** — Overlay controls on video element: play/pause button, seek bar, time display, volume slider, fullscreen button. Custom-drawn (not HTML). Show on hover, auto-hide after 3 seconds. | Media controls | ⬜ |
| P9.2.5 | **Picture-in-Picture** — On PiP activation: detach video from tab layout. Render in a separate always-on-top floating window (create via Zig platform layer). Show mini controls. On PiP exit: re-attach to tab. | PiP | ⬜ |

---

### P9.3 — Streaming Protocols

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P9.3.1 | **DASH parser** — Create `crates/vex-media/src/dash.rs`. Parse DASH MPD (Media Presentation Description) XML. Extract: adaptation sets, representations (quality levels), segment URLs, duration. | `dash.rs` | ⬜ |
| P9.3.2 | **HLS parser** — Create `crates/vex-media/src/hls.rs`. Parse HLS M3U8 playlists. Extract: variant streams (quality levels), segment URLs, duration, encryption info. | `hls.rs` | ⬜ |
| P9.3.3 | **ABR algorithm** — Create `crates/vex-media/src/abr.rs`. Adaptive bitrate: monitor download throughput and buffer level. Switch between quality levels: if buffer low, switch down; if bandwidth high and buffer healthy, switch up. Hysteresis to prevent oscillation (don't switch more than once per 10 seconds). Write 4 tests with simulated throughput. | `abr.rs` + tests | ⬜ |

---

### P9.4 — DRM Webview Fallback

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P9.4.1 | **EME detection** — In `vex-js`, when JS calls `navigator.requestMediaKeySystemAccess(...)`, intercept and set a flag: `drm_requested = true`. Store the key system ID (`com.widevine.alpha`, `com.apple.fps`, etc.). | EME intercept | ⬜ |
| P9.4.2 | **WebView2 integration (Windows)** — Create `crates/vex-browser/src/webview_fallback.rs`. On Windows: use `webview2` Rust crate (or raw COM interop). Create a WebView2 controller parented to the Vigo window. Size it to match the `<video>` element's layout rect. Navigate it to the current page URL. Handle cookies sync (copy cookies from Vigo's jar to WebView2's). | WebView2 integration | ⬜ |
| P9.4.3 | **Seamless overlay** — Position the webview exactly over the video area. When the page scrolls, reposition the webview. When the tab switches, hide the webview. On navigation away, destroy the webview and return to native rendering. | Webview overlay | ⬜ |
| P9.4.4 | **Fallback test** — Test: navigate to Netflix. EME triggers webview. Video plays in the webview area. Browser chrome remains Vigo. | E2E DRM test | ⬜ |

---
---

## Phase 10 — Security, Storage & Web Compat

**Timeline:** Weeks 93–110  
**Goal:** Same-Origin Policy, CORS, CSP, process sandboxing, web storage APIs, Web Platform Test compliance.  
**Entry criteria:** Phase 9 complete (media plays).  
**Exit criteria:** Security policies enforced. Storage APIs work. WPT pass rate ≥60%.

---

### P10.1 — Same-Origin Policy & CORS

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.1.1 | **Origin model** — Create `crates/vex-security/src/origin.rs`. `Origin { scheme: String, host: String, port: u16 }`. Derive from URL. `same_origin(a, b) -> bool`. `is_opaque()` for `data:`, `file:`, `about:` URLs. Write 5 tests. | `origin.rs` + tests | ⬜ |
| P10.1.2 | **SOP enforcement** — In DOM: prevent JS in one origin from accessing DOM of another origin (e.g., cross-origin iframes). In fetch: flag cross-origin requests. In storage: scope cookies/localStorage to origin. | SOP enforcement | ⬜ |
| P10.1.3 | **CORS preflight** — In `vex-net`, for cross-origin fetches with non-simple methods/headers: send OPTIONS preflight. Check `Access-Control-Allow-Origin`, `Allow-Methods`, `Allow-Headers`. Block if preflight fails. Apply `Access-Control-Expose-Headers` to response. Handle `credentials: 'include'`. Write 5 tests. | CORS in fetch | ⬜ |

---

### P10.2 — Content Security Policy

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.2.1 | **CSP parser** — Create `crates/vex-security/src/csp.rs`. Parse `Content-Security-Policy` header. Extract directives: `default-src`, `script-src`, `style-src`, `img-src`, `connect-src`, `font-src`, `frame-src`, `media-src`. Parse source lists: `'self'`, `'none'`, `'unsafe-inline'`, `'unsafe-eval'`, `https:`, `data:`, specific hosts, nonces, hashes. Write 5 tests. | `csp.rs` + tests | ⬜ |
| P10.2.2 | **CSP enforcement** — Before loading any sub-resource (script, image, CSS, fetch), check against active CSP policy. Block if not allowed. Before executing inline `<script>`, check if `'unsafe-inline'` or matching nonce/hash. Log violations. Write 3 tests. | CSP enforcement | ⬜ |

---

### P10.3 — Process Sandboxing

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.3.1 | **Process model** — Create `crates/vex-browser/src/process.rs`. Each tab runs in a separate OS process. Browser process (main) manages tabs, UI, networking. Renderer process handles DOM, layout, JS for one tab. IPC via named pipes (Windows) or unix sockets. Define message types: `LoadUrl`, `NavigationComplete`, `RenderFrame`, `InputEvent`, `JsCallback`. | Process model | ⬜ |
| P10.3.2 | **Windows sandbox** — For renderer processes on Windows: create with restricted token (remove admin groups), assign to a Job Object with limits (memory: 512MB, CPU: 60%), set UI restrictions (no clipboard, no desktop access). Use `CreateProcessAsUser` with restricted token. | Windows sandbox | ⬜ |
| P10.3.3 | **Crash isolation** — If a renderer process crashes: browser process detects it, shows "This page has crashed" error page in the tab, allows reload. Other tabs unaffected. Write integration test: intentionally crash renderer, verify browser stays alive. | Crash isolation | ⬜ |

---

### P10.4 — Web Storage APIs

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.4.1 | **Cookie storage** — Create `crates/vex-storage/src/cookies.rs`. Persist cookies to SQLite. Table: `cookies(domain, path, name, value, secure, http_only, same_site, expires)`. Functions: `save(cookie)`, `load(domain, path) -> Vec<Cookie>`, `delete_expired()`, `clear_all()`. Wire to `vex-net` cookie jar. Write 5 tests. | Cookie persistence | ⬜ |
| P10.4.2 | **localStorage** — Create `crates/vex-storage/src/local_storage.rs`. Per-origin key-value store. SQLite table: `local_storage(origin, key, value)`. Methods: `get_item(origin, key) -> Option<String>`, `set_item(origin, key, value)`, `remove_item(origin, key)`, `clear(origin)`, `length(origin) -> usize`. 5MB limit per origin. Expose to JS via `window.localStorage`. Write 5 tests. | localStorage | ⬜ |
| P10.4.3 | **sessionStorage** — Same API as localStorage but in-memory only (no SQLite). Cleared when tab closes. Keyed to (origin, tab_id). Expose to JS via `window.sessionStorage`. | sessionStorage | ⬜ |
| P10.4.4 | **IndexedDB (simplified)** — Create `crates/vex-storage/src/indexed_db.rs`. SQLite-backed key-value store with indexes. API: `open(name, version)`, object stores with `add`, `get`, `put`, `delete`, indexes with `get_by_index`. Transaction model (begin/commit/abort). This is a simplified subset — enough for most sites. Expose to JS. | IndexedDB | ⬜ |

---

### P10.5 — Privacy Hardening

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.5.1 | **Canvas fingerprint protection** — When JS reads canvas pixel data (`getImageData`, `toDataURL`, `toBlob`), inject random noise (±1 on each color channel). Consistent per origin per session (so the site can't detect the noise by comparing reads). | Canvas protection | ⬜ |
| P10.5.2 | **WebGL masking** — Override `UNMASKED_RENDERER_WEBGL` and `UNMASKED_VENDOR_WEBGL` to return generic strings ("Vex GPU" / "Vex"). Limit `getExtension` to a standard subset. | WebGL masking | ⬜ |
| P10.5.3 | **Font enumeration restriction** — When JS queries system fonts (via CSS font loading API or canvas text measurement), return a limited standard set (12 common web-safe fonts) instead of the full system font list. | Font restriction | ⬜ |

---

### P10.6 — Web Platform Tests

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P10.6.1 | **WPT runner** — Create `tools/wpt_runner.rs`. Checkout Web Platform Tests repo. For each test: load the HTML in Vigo engine, execute, check test assertions. Report pass/fail/error. | WPT runner | ⬜ |
| P10.6.2 | **WPT triage** — Run WPT suite. Categorize failures by subsystem (HTML, CSS, DOM, JS, Fetch). Prioritize fixes for most-impacted categories. Target: 60% pass on first run, iterate to 80%. | WPT dashboard | ⬜ |

---
---

## Phase 11 — Extensions & DevTools

**Timeline:** Weeks 111–126  
**Goal:** Built-in developer tools. Custom extension platform.  
**Entry criteria:** Phase 10 complete (security, storage, web compat).  
**Exit criteria:** DevTools inspect DOM/styles/network. Extensions install AND modify page content.

---

### P11.1 — DevTools: Elements Panel

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P11.1.1 | **DevTools window** — Create `crates/vex-browser/src/devtools/mod.rs`. Open DevTools as a panel docked to bottom or right of content area (or undocked in separate window). Toggle via `F12` or `Ctrl+Shift+I`. Custom-rendered UI (not HTML). | DevTools framework | ⬜ |
| P11.1.2 | **DOM tree view** — Render DOM as collapsible tree. Each element shows `<tag class="..." id="...">`. Click to select. Selected element highlighted on page (overlay blue box). Expand/collapse children. | DOM tree | ⬜ |
| P11.1.3 | **Computed styles panel** — When an element is selected in DOM tree, show its computed styles in a side panel. Group by category (Box Model visual, then Dimensions, Margin, Padding, Border, Typography, Colors, Layout). Show inherited-from indicators. | Styles panel | ⬜ |
| P11.1.4 | **Box model visualizer** — When hovering over an element in DOM tree or page: show colored overlay (margin = orange, border = yellow, padding = green, content = blue) on the page. | Box model overlay | ⬜ |

---

### P11.2 — DevTools: Console

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P11.2.1 | **Console panel** — Scrollable list of log messages. Each message: timestamp, level (log/warn/error), formatted content. Capture output from `console.log/warn/error` calls. | Console log | ⬜ |
| P11.2.2 | **REPL** — Text input at bottom of console. On Enter: execute as JS in page context via `vex-js`. Display result. Support multi-line input (Shift+Enter). Up/down arrow for history. | REPL | ⬜ |
| P11.2.3 | **Object inspector** — When a JS object is logged or returned, show expandable tree view of properties. For DOM nodes, show as `<tag>` link (click to select in Elements). For arrays, show length + indices. | Object inspector | ⬜ |

---

### P11.3 — DevTools: Network

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P11.3.1 | **Request log** — Capture all network requests made by the page. Store: URL, method, status, content-type, size, timing (DNS, connect, TLS, first byte, download). Display as sortable table. | Request log | ⬜ |
| P11.3.2 | **Request detail** — Click a request: show headers (request + response), body (formatted: JSON pretty-print, HTML syntax highlight), timing waterfall bar. | Request detail | ⬜ |
| P11.3.3 | **Filtering** — Filter by: type (XHR, JS, CSS, Image, Font, Media), status (2xx, 3xx, 4xx, 5xx), search text. Clear button. | Filtering | ⬜ |

---

### P11.4 — DevTools: Sources & Performance

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P11.4.1 | **Page source viewer** — Show page HTML source with syntax highlighting. For external scripts/styles: show their content in tabs. | Source viewer | ⬜ |
| P11.4.2 | **Performance panel** — Show frame timing: parse time, style time, layout time, paint time, JS execution time. Display as stacked bar chart per frame. Highlight slow frames (>16ms). | Performance panel | ⬜ |

---

### P11.5 — Extension Platform

| Task | Description | Deliverable | Status |
|------|-------------|-------------|--------|
| P11.5.1 | **Extension manifest** — Create `docs/EXTENSION_API.md`. Define manifest format: `{ "name": "...", "version": "1.0", "permissions": ["tabs", "storage", ...], "content_scripts": [{ "matches": ["*://*.example.com/*"], "js": ["content.js"] }], "background": { "service_worker": "background.js" }, "browser_action": { "default_popup": "popup.html", "default_icon": "icon.png" } }`. | Manifest spec | ⬜ |
| P11.5.2 | **Extension loader** — Create `crates/vex-browser/src/extensions/loader.rs`. Load extensions from `~/.vigo/extensions/` directory. Parse manifest.json. Validate permissions. Register content scripts and background scripts. | Extension loader | ⬜ |
| P11.5.3 | **Content script injection** — When a page loads and matches a content script's `matches` pattern: inject the script's JS into the page's JS context (but in an isolated world — separate global object, shared DOM access). | Content script injection | ⬜ |
| P11.5.4 | **Background scripts** — Run extension background scripts in a separate `JsRuntime` (not tied to any tab). Provide extension APIs: `vigo.tabs.query()`, `vigo.tabs.create()`, `vigo.storage.local.get/set()`, `vigo.notifications.create()`. | Background scripts | ⬜ |
| P11.5.5 | **Browser action** — Show extension icons in toolbar. On click: show popup (HTML rendered by Vigo engine in small floating window). Pass messages between popup ↔ background script via `vigo.runtime.sendMessage()`. | Browser action | ⬜ |
| P11.5.6 | **Extension permissions** — On install: show permission dialog listing requested permissions. User must approve. Store granted permissions. Enforce: extension can only call APIs it has permission for. | Permission system | ⬜ |
| P11.5.7 | **Test extension** — Create a sample extension "Vigo Dark Mode": content script that adds `filter: invert(1)` to `<html>`. Verify it loads and works. | Test extension | ⬜ |

---
---

## Summary

| Phase | Tasks | Est. Duration |
|-------|-------|---------------|
| **Phase 0** — Scaffold | 30 tasks | Weeks 1–2 |
| **Phase 1** — Core + Platform | 30 tasks | Weeks 3–6 |
| **Phase 2** — Network | 24 tasks | Weeks 7–12 |
| **Phase 3** — HTML + DOM | 25 tasks | Weeks 13–20 |
| **Phase 4** — CSS + Style | 22 tasks | Weeks 21–28 |
| **Phase 5** — Layout | 18 tasks | Weeks 29–40 |
| **Phase 6** — GPU Rendering | 18 tasks | Weeks 41–52 |
| **Phase 7** — JavaScript | 20 tasks | Weeks 53–68 |
| **Phase 8** — Browser Chrome | 23 tasks | Weeks 69–80 |
| **Phase 9** — Media + DRM | 15 tasks | Weeks 81–92 |
| **Phase 10** — Security + Storage | 16 tasks | Weeks 93–110 |
| **Phase 11** — Extensions + DevTools | 18 tasks | Weeks 111–126 |
| **TOTAL** | **~259 tasks** | **~126 weeks** |
