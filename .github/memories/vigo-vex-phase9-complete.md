# Vigo Browser (Vex Engine) — Status (Phase 9 Complete)

## Project Overview
- Custom browser engine from scratch in Rust (~70%) + Zig (~30%)
- Codename: "Vex" (Vigo Engine X)
- Workspace: `L:\Code-insiders\Vigo`
- Architecture: 8-layer stack, Zig → static libs → C ABI → Rust FFI
- License: MPL-2.0, open source

## Completed Phases
- ✅ Phase 0: Foundation & Scaffold (16 Rust crates, 5 Zig modules)
- ✅ Phase 1: Core Types & Platform Layer (P1.1-P1.4)
- ✅ Phase 2: Network Stack (P2.1-P2.6)
- ✅ Phase 3: HTML Parser + DOM
- ✅ Phase 4: CSS Parser + Style Computation
- ✅ Phase 5: Layout Engine (block, inline, flex, positioned)
- ✅ Phase 6: GPU Rendering (display list, shaders, glyph atlas, image atlas, full pipeline)
- ✅ Phase 7: JavaScript (Boa 0.19, DOM bindings, Console/Timer/Fetch APIs)
- ✅ Phase 8: Storage & Security (localStorage, IndexedDB, CSP, CORS, SRI)
- ✅ Phase 9: Media Pipeline + DRM Hybrid

## Phase 9 Deliverables

### P9.1 — Audio/Video Decode (Zig)
- `zig/media/ffmpeg.zig` — Runtime-loaded ffmpeg bindings (FfmpegLib, MediaContext, VideoFrame, AudioFrame)
- `zig/media/hw_decode.zig` — DXVA2/D3D11VA hardware decode probing (HwDecoderType, HwDecodeContext)
- `zig/media/audio_output.zig` — WASAPI audio output (AudioHandle, ring buffer, AudioFormat)
- `crates/vex-media/src/sync.rs` — Clock-based A/V sync (MediaClock, SyncDecision, FrameSynchroniser, 20ms drift)

### P9.2 — Media Element Integration
- `media_element.rs` — MediaElement state machine (ReadyState, MediaEvent, play/pause/seek)
- `media_loading.rs` — Media source loading (MediaFormat detection from magic bytes, MediaLoader, LoadState)
- `video_render.rs` — Video frame → GPU texture → display list (DecodedFrame, VideoSurface, letterbox/pillarbox)
- `controls.rs` — Media player overlay controls (play/pause, seek, volume, fullscreen, auto-hide 3s)
- `pip.rs` — Picture-in-Picture (PipState, PipGeometry, drag support)

### P9.3 — Streaming Protocols
- `dash.rs` — DASH MPD XML parser (DashManifest, AdaptationSet, Representation, ISO 8601 duration)
- `hls.rs` — HLS M3U8 parser (MasterPlaylist, VariantStream, MediaPlaylist, MediaSegment, encryption)
- `abr.rs` — Adaptive bitrate (AbrController, EWMA throughput, 10s hysteresis, buffer-based switching)

### P9.4 — DRM WebView Fallback
- `crates/vex-js/src/api/eme.rs` — EME detection (EmeState, KeySystem enum, navigator.requestMediaKeySystemAccess interception)
- `crates/vex-browser/src/webview_fallback.rs` — WebView2 fallback (WebViewFallback, WebViewConfig, CookieSync)
- `crates/vex-browser/src/drm_overlay.rs` — Seamless overlay positioning (DrmOverlayManager, scroll/visibility)
- `crates/vex-browser/tests/drm_fallback_test.rs` — Integration test (EME trigger → WebView2 → overlay → cleanup)

## Test Counts: 736 Rust tests passing, 22 Zig tests, 0 failures, clippy clean (--all-targets)

## Key API Knowledge
- `Rect` fields: `rect.origin.x`, `rect.origin.y`, `rect.size.width`, `rect.size.height` (nested, NOT flat)
- EME pattern: `EmeState` wraps `Rc<RefCell<EmeInner>>`, shared via closure capture
- `build_navigator_with_eme()` creates complete navigator JsValue with EME method
- `KeySystem::parse()` not `from_str()` (clippy `should_implement_trait` lint)
- DASH XML parser: `extract_attr()` must prefix attr name with space to avoid substring matches (e.g., "bandwidth" vs "width")

## Next: Phase 10 — Browser Chrome & Extensions
