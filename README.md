# Vigo Browser

> An experimental, privacy-oriented browser built from scratch in Rust and Zig.
> Powered by the Vex engine — not Chromium, Gecko, or WebKit.

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)

## Status

Vigo is pre-release software, currently validated on Windows. It is an independent engine project, not a production-browser replacement yet.

The current build can:

- load HTTP(S) pages through the Vex fetch, HTML, CSS, layout, JavaScript, and GPU-rendering pipeline;
- open and navigate tabs with back/forward/reload controls and address-bar search;
- persist browsing history and bookmarks, and display them in built-in pages;
- download direct file URLs and `Content-Disposition: attachment` responses to the profile-owned downloads folder;
- submit basic URL-encoded HTML forms over GET and POST;
- present native JavaScript dialogs and permission-backed text clipboard access;
- provide find-in-page, browser settings, DevTools scaffolding, and extension loading; and
- build the native window/platform layer from Zig.

Some important boundaries are still incomplete:

- Downloads are basic: there is no destination chooser, pause/resume support, or reputation/malware scanning.
- Forms currently cover URL-encoded GET/POST controls; multipart file uploads and full select/validation semantics are incomplete.
- Renderer-process management currently uses a single-process sandbox policy hook; it is not a hardened process sandbox.
- The WebView/DRM fallback is scaffolded rather than a complete integration.
- Web-platform compatibility is incomplete, so many sites will not behave like they do in mature browsers.

JavaScript dialogs are asynchronous in the current engine: use `await alert(...)`,
`await confirm(...)`, and `await prompt(...)`. Clipboard access uses
`await navigator.clipboard.readText()` and `await navigator.clipboard.writeText(...)`,
and is gated per HTTP(S) origin. This differs from the synchronous dialog behavior
of mature browser engines.

The current limitations above define the public release boundary.

## Architecture

```text
vex-app                         Native browser shell
  ├── vex-browser               Tabs, history, bookmarks, UI, DevTools, extensions
  ├── vex-net                   HTTP, TLS, DNS, cookies, cache, privacy filtering
  ├── vex-html / vex-dom        HTML parser and DOM
  ├── vex-css / vex-layout      Cascade, styles, block/inline/flex layout
  ├── vex-js                    Boa JavaScript integration and browser APIs
  ├── vex-render                wgpu display-list rendering and Rust/Zig FFI
  ├── vex-security / privacy    CSP, CORS, HTTPS, tracker and ad filtering
  └── vex-storage / media       Web storage and media pipeline components

zig/                            Win32 platform layer and allocation primitives
sync-server/                    Optional Go/SQLite sync backend
```

## Build on Windows

### Prerequisites

| Tool | Version |
| --- | --- |
| Rust | Stable (the project declares Rust 1.75+) |
| Zig | 0.16.0 (locally validated) |
| MSVC Build Tools | Visual Studio 2022 or newer |

### Build and run

```powershell
cd zig
zig build
cd ..

cargo build -p vex-app
cargo run -p vex-app
```

### Validate

```powershell
cargo test --workspace

cd zig
zig build test
cd ..

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The repository includes a Windows CI workflow that runs these checks. Zig source files use LF line endings for compatibility with Windows CI.

## Contributing

Contributions are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md), use a branch based on `main`, and include focused tests with behavior changes.

Please report vulnerabilities privately according to [SECURITY.md](SECURITY.md). Community participation is covered by the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Vigo is licensed under the [Mozilla Public License 2.0](LICENSE).
