# Final Browser Assembly — Complete Task List

All 11 phases are complete in isolation, but tens of critical stubs, disconnected modules, and fake implementations remain. This is the complete ordered list of tasks to turn the codebase into a functional browser.

**Total: 77 tasks**

---

## Foundation & Config (Tasks 1–3)

- [x] **1. Create centralized `BrowserConfig`** — Consolidate all hardcoded constants (chrome height 85 vs 80, window size 1280×720, user-agent string, colors) into `BrowserSettings`. Fix the chrome height mismatch between `main.rs`, `drm_overlay.rs`, and `ui/chrome.rs`.
  - Files: `settings.rs`, `ui/chrome.rs`, `drm_overlay.rs`, `main.rs`

- [x] **2. Make root font-size dynamic** — `cascade/compute.rs` hardcodes `16.0_f32`. Compute from `<html>` element's style or `BrowserSettings.default_font_size`.
  - Files: `cascade/compute.rs`, `computed.rs`

- [x] **3. Fix `tab.rs` panic** — Replace `panic!()` with a guaranteed-valid static `VexUrl` for `about:blank`.
  - Files: `tab.rs`

---

## Network — Wire Existing Code (Tasks 4–8)

- [x] **4. Wire `HttpCache` into `HttpClient`** — The cache module is fully coded + tested but `HttpClient` never uses it. Add a `cache` field, check freshness before requests, send conditional headers, handle 304.
  - Files: `crates/vex-net/src/client.rs`, `cache.rs`

- [x] **5. Wire DNS resolution results into HTTP connector** — Currently resolved IPs are discarded with `let _ =`. Feed resolved addresses into the actual connection.
  - Files: `client.rs`, `dns.rs`

- [x] **6. Enforce cookie `HttpOnly` and `SameSite`** — Fields exist, are parsed, but marked `#[allow(dead_code)]`. Implement actual enforcement logic.
  - Files: `crates/vex-net/src/cookies.rs`

- [x] **7. Add `deflate` decompression** — Missing from `decompress.rs`. Also handle multiple Content-Encoding values.
  - Files: `decompress.rs`

- [x] **8. Wire security policies into fetch** — `SopPolicy`, `CorsPolicy`, `CspPolicy` are fully implemented but never called. Add security classification to the fetch pipeline.
  - Files: `client.rs`, `vex-security/src/{sop,cors,csp}.rs`

---

## Privacy — Wire Existing Code (Tasks 9–10)

- [x] **9. Randomize canvas fingerprint seed** — Defaults to `0`. Generate `rand::random::<u64>()` at browser startup.
  - Files: `crates/vex-privacy/src/canvas.rs`, browser init code

- [x] **10. Wire canvas/webgl/font restrictions to rendering** — Privacy modules exist but are never called from the render pipeline. Add hooks.
  - Files: `crates/vex-render/src/renderer.rs`, `vex-privacy/src/{canvas,webgl,fonts}.rs`

---

## JS Engine — Replace Stubs (Tasks 11–19)

- [x] **11. Wire `location.assign()` to real navigation** — Push `BrowserRequest::Navigate` to shared `RequestQueue`. Browser event loop drains queue each tick.
  - Files: `crates/vex-js/src/api/window.rs`, `browser_request.rs`, `context.rs`

- [x] **12. Wire `location.reload()`** — Pushes `BrowserRequest::Reload` to queue.
  - Files: `api/window.rs`

- [x] **13. Wire `history.back/forward/pushState`** — Pushes `Back`, `Forward`, `PushState{url}` to `RequestQueue`. Browser loop dispatches to `NavigationHistory`.
  - Files: `api/window.rs`, `browser_request.rs`

- [x] **14. Update `location` object dynamically** — `update_location(url, ctx)` parses URL and sets href/origin/protocol/hostname/port/pathname/search/hash on window.location.
  - Files: `api/window.rs`

- [x] **15. Wire `alert()`** — Pushes `BrowserRequest::Alert(msg)` to queue. Browser loop renders overlay or logs.
  - Files: `api/window.rs`, `browser_request.rs`

- [x] **16. Wire `console.*()` to DevTools** — Each console method pushes `BrowserRequest::ConsoleLog{level, message}` to queue in addition to `tracing`. 3 new queue tests.
  - Files: `api/console.rs`, `browser_request.rs`

- [x] **17. Fix fetch API efficiency** — Added `register_with_handle(handle, ctx)` and `perform_fetch_with_handle()`. JsRuntime creates one tokio runtime, passes handle to fetch. Extracted shared `fetch_async()`.
  - Files: `api/fetch.rs`, `context.rs`

- [x] **18. Wire `navigator` properties dynamically** — `NavigatorConfig{user_agent, language, cookie_enabled}` populated from settings. `register_with_config(queue, nav_config, ctx)` passes config to navigator builder. System language detection.
  - Files: `api/window.rs`, `context.rs`

- [x] **19. Implement live `element.style` proxy** — Replaced snapshot properties with `PropertyDescriptor` getter/setter closures. Writing `el.style.color = 'blue'` updates DOM `style` attribute. 2 new tests.
  - Files: `api/style_proxy.rs`

---

## Storage — Wire to JS (Tasks 20–23)

- [x] **20. Expose `localStorage` to JS** — Bind to `vex-storage::LocalStorage` (SQLite, already complete). Scope by page origin.
  - Files: new `crates/vex-js/src/api/local_storage.rs`, `api/window.rs`

- [x] **21. Expose `sessionStorage` to JS** — Bind to `vex-storage::SessionStorage` (in-memory, per origin+tab).
  - Files: new `crates/vex-js/src/api/session_storage.rs`, `api/window.rs`

- [x] **22. Expose `document.cookie` to JS** — Getter reads from `CookieStore` (excluding HttpOnly), setter parses and stores.
  - Files: `api/document.rs`, `vex-storage/src/cookies.rs`

- [x] **23. Wire IndexedDB to JS (basic)** — `indexedDB.open()`, object store CRUD. Backed by existing `IdbDatabase`.
  - Files: new `crates/vex-js/src/api/indexed_db.rs`

---

## DOM Events — Complete the Loop (Tasks 24–27)

- [x] **24. Wire event dispatch to JS callbacks** — When `dispatch_event` fires a callback_id, invoke the corresponding JS function via `JsRuntime`.
  - Files: `api/events.rs`, `vex-dom/src/events/dispatch.rs`, `context.rs`

- [x] **25. Wire click events: platform → DOM → JS** — Mouse click at (x,y) → hit-test layout tree → create Click event → dispatch → if `<a href>`, navigate.
  - Files: `main.rs`, `vex-layout` (hit-test), `vex-dom/src/events/dispatch.rs`, `links.rs`

- [x] **26. Wire keyboard events to focused inputs** — Key events → focused element → update `InputState` → fire `input`/`change` events → re-render.
  - Files: browser loop, `vex-dom/src/forms.rs`, `api/events.rs`

- [x] **27. Fire `DOMContentLoaded` and `load` lifecycle events** — After parsing + defer scripts → `DOMContentLoaded`. After all resources → `load`.
  - Files: `context.rs`, `tab.rs` pipeline

---

## Script Execution (Tasks 28–29)

- [ ] **28. Wire script execution into tab loading** — Parse `<script>` elements, fetch external sources, execute blocking/defer/async in correct order.
  - Files: `tab.rs`, `context.rs`, `vex-html/src/parser.rs`

- [ ] **29. Create shared `JsRuntime` per tab** — Each tab owns its runtime with all Web APIs registered.
  - Files: `tab.rs`, `context.rs`

---

## Browser Chrome — Wire to Main Loop (Tasks 30–40)

- [ ] **30. Rewrite `main.rs` with `TabManager` + real browser loop** — Replace hardcoded welcome-page pipeline with actual multi-tab architecture.
  - Files: `crates/vex-app/src/main.rs`

- [ ] **31. Wire tab bar clicks** — Hit-test → `TabBarAction` → `TabManager` methods.
  - Files: `main.rs`, `ui/tab_bar.rs`, `tab_manager.rs`

- [ ] **32. Wire navigation bar** — Back/Forward/Reload buttons + address bar input → navigation.
  - Files: `main.rs`, `ui/nav_bar.rs`, `navigation.rs`, `links.rs`

- [ ] **33. Wire keyboard shortcuts** — `match_shortcut()` → `BrowserAction` → actual execution.
  - Files: `main.rs`, `ui/shortcuts.rs`

- [ ] **34. Wire context menu** — Right-click → hit-test → context menu → action.
  - Files: `main.rs`, `ui/context_menu.rs`

- [ ] **35. Wire bookmarks** — Bookmark bar rendering + click navigation + Ctrl+D add/remove.
  - Files: `main.rs`, `bookmarks.rs`, `ui/chrome.rs`

- [ ] **36. Wire Find-in-page** — Ctrl+F → find bar → `FindState::search()` → highlight matches.
  - Files: `main.rs`, `find.rs`, display list overlay

- [ ] **37. Wire downloads** — Non-displayable content → `DownloadManager` → save to disk.
  - Files: `main.rs`, `downloads.rs`

- [ ] **38. Wire zoom** — Ctrl+/- → `ZoomState` → re-layout at new scale.
  - Files: `main.rs`, `zoom.rs`, `vex-layout` viewport

- [ ] **39. Wire search URL** — Replace `links.rs` stub with `BrowserSettings::search_url(query)`.
  - Files: `links.rs`, `settings.rs`

- [ ] **40. Wire session save/restore** — Save tabs on exit, restore on launch.
  - Files: `main.rs`, `session.rs`

---

## Form & Input (Tasks 41–44)

- [ ] **41. Wire text input editing to rendering** — Keyboard → `InputState` → re-render with cursor.
  - Files: browser loop, `forms.rs`, `form_painter.rs`

- [ ] **42. Fix radio button rendering** — Currently square. Add circle SDF to shader.
  - Files: `form_painter.rs`, `shaders/rect.wgsl`

- [ ] **43. Fix cursor X-position** — Replace `char_count * 0.6 * font_size` with actual text measurement.
  - Files: `form_painter.rs`, `vex-layout/src/text.rs`

- [ ] **44. Wire form submission** — Collect values → build POST body / GET query → navigate.
  - Files: `forms.rs`, `tab.rs`, `links.rs`

---

## Image Loading (Tasks 45–46)

- [ ] **45. Wire async image loading** — `<img src>` → async fetch → decode → atlas upload → re-render.
  - Files: `tab.rs`, `image_loading.rs`, `image_atlas.rs`, `painter.rs`

- [ ] **46. Basic `srcset` support** — Pick best source based on viewport width.
  - Files: `image_loading.rs`, `painter.rs`

---

## Rendering Polish (Tasks 47–49)

- [ ] **47. Implement `text-align: justify`** — Currently TODO in `inline.rs`. Distribute space between words.
  - Files: `vex-layout/src/inline.rs`

- [ ] **48. Verify opacity pipeline** — Ensure `PushOpacity/PopOpacity` works in renderer.
  - Files: `painter.rs`, `renderer.rs`

- [ ] **49. Wire `border-radius` to SDF shader** — Add rounded corner logic to `rect.wgsl`.
  - Files: `shaders/rect.wgsl`, `display_list.rs`, `painter.rs`

---

## Layout Hit-Testing (Task 50)

- [ ] **50. Implement `hit_test(layout_root, x, y) → VexId`** — Walk layout tree in reverse paint order. Needed for clicks, hover, cursor.
  - Files: new `crates/vex-layout/src/hit_test.rs`

---

## DevTools — Connect to Real Data (Tasks 51–56)

- [ ] **51. Wire Elements panel to real DOM** — Display active tab's DOM tree.
  - Files: `devtools/elements.rs`, `devtools/mod.rs`

- [ ] **52. Wire computed styles panel** — Show selected element's `ComputedStyle`.
  - Files: `devtools/elements.rs`

- [ ] **53. Wire Console panel** — Show JS `console.*()` output + REPL.
  - Files: `devtools/console.rs`, `api/console.rs`

- [ ] **54. Wire Network panel** — Instrument `HttpClient` to emit request records.
  - Files: `devtools/network.rs`, `client.rs`

- [ ] **55. Wire Performance panel** — Capture pipeline stage timing per frame.
  - Files: `devtools/performance.rs`, `main.rs`

- [ ] **56. Wire Sources panel** — Show page HTML and linked scripts.
  - Files: `devtools/sources.rs`, `tab.rs`

---

## Extensions — Wire Loading (Tasks 57–60)

- [ ] **57. Wire extension loader to startup** — Scan `~/.vigo/extensions/`, parse manifests.
  - Files: `extensions/loader.rs`, `main.rs`

- [ ] **58. Wire content script injection** — On page load match → inject script in isolated context.
  - Files: `extensions/content.rs`, `tab.rs`

- [ ] **59. Wire background scripts** — Dedicated `JsRuntime` with `vigo.*` extension APIs.
  - Files: `extensions/loader.rs`, new `extensions/background.rs`

- [ ] **60. Wire browser action buttons** — Extension icons in toolbar → popup rendering.
  - Files: `extensions/action.rs`, `main.rs`

---

## Process Model (Tasks 61–64)

- [ ] **61. Real process spawning** — Replace fake PID counter with `CreateProcessW`.
  - Files: `process.rs`

- [ ] **62. Real sandbox enforcement** — Windows Job Object + restricted token.
  - Files: `sandbox.rs`

- [ ] **63. Implement WebView2 COM init** — Actual environment → controller → view creation.
  - Files: `webview_fallback.rs`

- [ ] **64. Real WebView2 availability check** — Actual `GetAvailableCoreWebView2BrowserVersionString` API call.
  - Files: `webview_fallback.rs`

---

## Media Pipeline (Tasks 65–71)

- [ ] **65. Evaluate `zig/compositor` necessity** — wgpu is already Rust-side. May deprecate or implement.
  - Files: `zig/compositor/root.zig`

- [ ] **66. Evaluate `zig/text` necessity** — cosmic-text already works. May deprecate or implement.
  - Files: `zig/text/root.zig`

- [ ] **67. Complete `ffmpeg.zig` decode loop** — Open format → find streams → decode → return frames.
  - Files: `zig/media/ffmpeg.zig`

- [ ] **68. Complete `audio_output.zig` WASAPI** — Create device → open stream → write samples.
  - Files: `zig/media/audio_output.zig`

- [ ] **69. Wire media pipeline end-to-end** — `<video>` → fetch → decode → GPU texture → render.
  - Files: `media_element.rs`, `media_loading.rs`, `video_render.rs`, `sync.rs`, `tab.rs`

- [ ] **70. Wire media controls UI** — Play/pause/seek overlay on video elements.
  - Files: `controls.rs`, `drm_overlay.rs`

- [ ] **71. Wire HLS/DASH streaming** — Parse playlist → fetch segments → ABR quality switching.
  - Files: `hls.rs`, `dash.rs`, `abr.rs`, `media_loading.rs`

---

## Crypto & Sync (Tasks 72–73)

- [ ] **72. Implement `vex-crypto`** — ChaCha20-Poly1305, Argon2id, Ed25519, X25519 using declared deps. Zeroize secrets.
  - Files: `crates/vex-crypto/src/lib.rs` (currently empty)

- [ ] **73. Implement `vex-sync`** — Sync client connecting to Go sync-server. Bookmarks/history/settings sync.
  - Files: `crates/vex-sync/src/lib.rs` (currently empty)

---

## Final Integration (Tasks 74–77)

- [ ] **74. Complete application entry point** — Full browser event loop: platform events → hit-test → DOM events → JS → re-style → re-layout → re-render → present.
  - Files: `crates/vex-app/src/main.rs`

- [ ] **75. Create `vex://` internal pages** — `vex://settings`, `vex://history`, `vex://bookmarks`, `vex://downloads`, `vex://newtab`.
  - Files: new internal page files or generated display lists

- [ ] **76. Window title updates** — Show `"Page Title — Vigo Browser"`, update on navigation.
  - Files: `main.rs`, `zig/platform/window.zig`

- [ ] **77. Debug FPS in title** — Show `[60 FPS]` in debug builds.
  - Files: `main.rs`

---

## Key Decisions

- **Zig compositor/text may be deprecated** (#65-66) — Rust-side wgpu + cosmic-text already do the job
- **Media (#65-71) is the hardest cluster** — ffmpeg FFI + WASAPI is complex. Can be deferred after core browser works
- **Crypto + Sync (#72-73) are optional for MVP** — Browser functions without sync
- **Process isolation (#61-62) is a security feature**, not functional — single-process is fine for now

---

## Success Criteria

1. `just ci` passes clean (clippy, fmt, all tests)
2. `just run` opens window, loads `vex://welcome`, displays rendered page
3. Type URL in address bar → page loads, renders, JS executes
4. Click links → navigation works, back/forward works
5. Ctrl+T/W → tabs open/close
6. `console.log()` appears in DevTools console
7. `<input>` elements accept text input
8. `localStorage.setItem/getItem` persists across page loads
9. Images load asynchronously and appear
10. Find-in-page (Ctrl+F) highlights matches
