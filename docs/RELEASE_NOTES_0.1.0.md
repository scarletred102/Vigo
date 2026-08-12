# Vigo 0.1.0-experimental

## Release status

This is an unsigned experimental Windows source release. Vigo is an independent
browser engine project and is not a replacement for a mature production browser.

## Supported platform

- Windows is the supported development and validation platform for this release.

## Included browser capabilities

- HTTP(S) navigation through the Vex HTML, CSS, layout, JavaScript, and GPU
  rendering pipeline.
- Tabs, address-bar navigation, history, bookmarks, find-in-page, settings,
  and built-in browser pages.
- Native JavaScript alert, confirmation, and prompt dialogs.
- Permission-gated plain-text system clipboard access for HTTP(S) origins.
- Direct URL downloads and HTTP attachment responses written to the profile-owned
  downloads folder.
- Basic HTML form submission with URL-encoded GET and POST requests.
- A native Zig/Win32 platform layer.

## Important compatibility and security limits

- JavaScript dialog APIs are asynchronous in this engine. Pages must use
  `await alert(...)`, `await confirm(...)`, and `await prompt(...)`.
- Clipboard access uses `await navigator.clipboard.readText()` and
  `await navigator.clipboard.writeText(...)` and asks the user per HTTP(S)
  origin.
- Downloads do not yet provide a destination chooser, pause/resume, or
  reputation/malware scanning.
- Forms do not yet support multipart file uploads or complete select and
  constraint-validation semantics.
- Renderer orchestration remains a single-process policy hook, not a hardened
  process sandbox.
- WebView/DRM fallback paths are incomplete.
- Web-platform compatibility is incomplete; many real-world websites will not
  behave as they do in Chromium, Firefox, or Safari.

## Publication checks

Before merging this release into `main`, run the full Windows CI workflow and
ensure the format, check, test, KPI, and Clippy gates pass in the release
toolchain. See `.internal-docs/RELEASE_READINESS.md` for the complete checklist.
