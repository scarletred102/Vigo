# Plan: Vigo Browser — Full Implementation Blueprint

**TL;DR**: Vigo is a **proprietary**, desktop-only Chromium fork — privacy-first, streaming-optimized, performance-hardened, built to play **everything**. Currently at documentation-only stage (zero code). Solo developer. Free during beta, **one-time purchase** at GA. Self-hosted sync server. This plan provides the complete implementation roadmap from repository scaffold to public beta, organized into **7 phases over ~16–20 months** (solo-adjusted). Architecture follows Brave's proven overlay pattern (not a direct chromium/src fork). Rust for security-critical modules, E2E encrypted sync with zero-knowledge self-hosted server, Widevine DRM + universal codec support for streaming supremacy.

### Decided
| Question | Answer |
|----------|--------|
| Licensing | **Full proprietary** — closed source, all rights reserved |
| Monetization | **Free at beta → one-time paid purchase** at GA (no subscription) |
| Sync hosting | **Self-hosted** sync server (user's own infra or Vigo-provided) |
| Team | **Solo developer** — timeline extended accordingly |
| HEVC | **Include in V1** — license via platform decoders (OS-bundled), no separate patent pool needed for HW-only decode |
| Media goal | **Play everything** — every codec, container, protocol a user might encounter |

---

## Phase 0: Foundation & Scaffolding (Weeks 1–6)

### 0.1 Repository Architecture
- Create **two-repo structure** following Brave's proven pattern:
  - `vigo-browser` — build orchestration, npm scripts, CI/CD, installer configs
  - `vigo-core` — all Vigo-specific code, mounted at `src/vigo/` inside Chromium source tree
- `vigo-core/` internal structure:
  ```
  vigo-core/
  ├── browser/           # Browser-process Vigo features
  ├── chromium_src/       # File-level overrides (shadow pattern)
  ├── components/         # Vigo-specific components
  │   ├── adblock/        # Rust adblock engine
  │   ├── media_orchestration/  # MOL
  │   ├── privacy_engine/       # Anti-tracking, fingerprint resistance
  │   ├── sync/                 # E2E encrypted sync client
  │   └── credential_vault/     # Password/passkey management
  ├── patches/            # Minimal .patch files for upstream Chromium
  ├── build/              # GN build configuration
  ├── app/                # Branding, icons, localization
  ├── third_party/        # Vendored deps (libsodium, adblock lists)
  ├── rust/               # Rust workspace root
  │   ├── vigo_adblock/
  │   ├── vigo_crypto/
  │   └── vigo_filter/
  ├── installer/          # Platform-specific installers
  │   ├── win/            # MSI/EXE (WiX)
  │   ├── mac/            # .dmg/.pkg
  │   └── linux/          # .deb/.rpm
  ├── test/               # Test suites
  └── docs/               # Developer docs (move current Docs/ here)
  ```

### 0.2 Chromium Baseline Selection
- **Decision needed**: Select Chromium version (recommend latest stable, currently ~M132+)
- Pin version in `vigo-core/package.json`
- Use `depot_tools` for checkout (`fetch chromium`)
- Configure GN args:
  - `is_chrome_branded = false`
  - `proprietary_codecs = true` (requires ffmpeg licensing)
  - `ffmpeg_branding = "Chrome"` (for H.264/AAC)
  - Custom `google_api_key`, `google_default_client_id`, `google_default_client_secret`
  - Custom user data directory paths (`%LOCALAPPDATA%\Vigo\User Data`, `~/Library/Application Support/Vigo`, `~/.config/vigo`)

### 0.3 Build System Setup
- GN + Ninja (Chromium's native build system — do NOT fight it)
- npm scripts for orchestration (`npm run init`, `npm run sync`, `npm run build`, `npm run apply_patches`)
- `chromium_src/` override mechanism for surgical upstream modifications
- Set up `ccache`/`sccache` for incremental builds (full build = 2-6 hours)
- **Minimum build machine**: 16GB RAM, 100GB+ free disk, 8+ cores

### 0.4 CI/CD Pipeline
- GitHub Actions or dedicated build servers (Chromium builds are resource-intensive)
- Windows, macOS, Linux matrix
- Component builds for CI speed; full release builds for nightly/weekly
- Linting: C++ (clang-tidy), Rust (clippy), TypeScript (eslint)
- Automated patch verification on Chromium version bump

### 0.5 Widevine PoC
- **CRITICAL PATH**: Contact Google/Widevine to initiate license agreement IMMEDIATELY (takes weeks)
- Build proof-of-concept: Chromium + Widevine CDM loading + EME test page
- Verify L1/L3 detection, CDM handshake < 1s target
- Test against Netflix, YouTube, Disney+ DRM-protected content
- Document: CDM binary distribution strategy (separate download vs. bundled)

### 0.6 Google API Keys
- Register Google Cloud project for Vigo
- Obtain API keys for: Safe Browsing, Geolocation (or replace with alternatives)
- Decision: which Google services to keep vs. replace with Vigo alternatives
- Note: `chrome.identity.getAuthToken` extension API will NOT work — plan polyfill if needed

### 0.7 Proprietary Build & Licensing Setup
- **Closed-source repository** — private GitHub repo, no public forks
- Proprietary license header in all source files
- EULA / Terms of Service for beta distribution
- Plan one-time purchase pricing research (competitive: ~$29–49 range, look at Vivaldi/Sidekick/Arc models)
- Payment infrastructure evaluation: Gumroad, Paddle, FastSpring, or Stripe direct
- License key / activation system design (simple: key → server validation → unlock)
- Beta → paid transition UX: existing beta users get grace period or loyalty discount

### 0.8 Documentation Gaps to Resolve Before Coding
- [ ] Select Chromium baseline version
- [x] ~~Decide HEVC licensing~~ → **Include V1** (HW decode via OS platform decoders, no separate patent license needed)
- [ ] Choose DoH default provider (Cloudflare 1.1.1.1 recommended)
- [ ] Define Argon2id parameters (memory: 64MB, time: 3 iterations, parallelism: 4 — benchmark on target HW)
- [ ] Resolve dual key hierarchy: local credential vault (AES-256/DPAPI) vs. sync K_root
- [ ] Define sync conflict resolution: LWW for settings, CRDT for bookmarks, merge for history
- [ ] Specify anti-fingerprinting scope (canvas, WebGL, AudioContext, fonts, UA)
- [ ] One-time purchase price point
- [ ] Payment processor selection

---

## Phase 1: Browser Shell, Fundamentals & Core Privacy (Weeks 7–22)

> **Solo-dev note**: This phase is extended from 12 to 16 weeks. The browser fundamentals section is critical — Vigo must do everything a normal browser does before it can do anything special.

### 1.0 Browser Fundamentals — "The Default Things" (*depends on 0.2, 0.3*)
Everything users expect from any browser on day one:

**Navigation & Browsing**
- Forward / Back / Reload / Stop / Home
- Omnibox: URL entry + search (unified bar)
- Search engine selection (default: DuckDuckGo, configurable: Google, Bing, Ecosia, custom)
- Page zoom (ctrl+/-, ctrl+0 reset)
- Full-screen mode (F11)
- Find-in-page (Ctrl+F)
- View page source, inspect element
- Print to PDF / printer (Ctrl+P)
- Page save (Ctrl+S) — complete page, HTML only, MHTML

**Tab & Window Management**
- Multi-tab with drag-and-drop reordering
- Tab duplication, pinning, muting
- Tab groups with color labels
- New window / incognito (private) window
- Restore closed tabs / sessions (Ctrl+Shift+T)
- Session restore on crash or restart
- Multi-window across monitors

**Bookmarks & History**
- Bookmark bar, bookmark manager, folders
- Import bookmarks from Chrome, Firefox, Edge, Safari
- Full browsing history with search & date filtering
- Clear browsing data (history, cache, cookies, passwords — granular controls)

**Downloads**
- Download manager with pause/resume/cancel
- Download location chooser (per-download or default folder)
- Dangerous file warnings
- Download progress in toolbar

**Settings & Preferences**
- Appearance: theme (light/dark/system), font size, page zoom default
- Default browser registration (OS-level)
- Startup behavior: new tab / continue where left off / specific pages
- Language & spell-check (system dictionaries)
- Proxy configuration (system/manual/PAC)
- Accessibility: high contrast, caret browsing, screen reader support
- Keyboard shortcut viewer/customization

**Developer Tools**
- Full Chromium DevTools (Elements, Console, Network, Performance, Application, etc.)
- Device emulation / responsive design mode
- JavaScript console

**Standards & Compatibility**
- Full HTML5, CSS3, ES2024+ support (inherited from Chromium)
- Web Components, Shadow DOM, Custom Elements
- WebAssembly (Wasm)
- WebGL 2.0 + WebGPU
- Service Workers, Web Workers
- IndexedDB, LocalStorage, SessionStorage
- WebSockets, WebTransport
- Geolocation API (with Vigo's own API key)
- Notifications API
- Clipboard API
- Drag & Drop API
- File System Access API
- Web Share API
- Payment Request API
- Credential Management API

**PDF & Documents**
- Built-in PDF viewer (Chromium's PDFium)
- PDF form filling, text selection, search
- Print to PDF

### 1.1 Browser Shell & Branding (*depends on 0.2, 0.3*)
- Custom branding: window title, about page, user agent string
- Custom new tab page (minimal, no Google services) — speed dial, recent/frequent sites, search bar
- Omnibox with search engine selection (default: DuckDuckGo or user choice)
- Tab bar with Vigo styling
- Settings page scaffold (organized: General, Privacy, Media, Sync, Extensions, Advanced)
- Keyboard shortcut framework
- About Vigo page (version, license status, credits)

### 1.2 Google Service Removal (*depends on 0.2*)
- Strip Google telemetry endpoints via `chromium_src/` overrides
- Remove Google Sync integration
- Remove Google account sign-in
- Replace or disable: Chrome Reporting, RLZ, Variations, etc.
- Keep: Safe Browsing API (with Vigo's own API key), or replace with Brave-style lists

### 1.3 Rust Adblock Engine (*parallel with 1.1*)
- Build using `rust_static_library` GN template (not cargo directly)
- FFI bridge: C++ ↔ Rust via Chromium's rust-ffi patterns
- EasyList + EasyPrivacy filter lists (auto-updating)
- `declarativeNetRequest`-compatible for extension interop
- Performance target: < 1ms per URL check
- Reference: Brave's `adblock-rust` for proven patterns

### 1.4 Privacy Engine (*depends on 1.2*)
- 3rd-party cookie blocking (default on)
- Anti-fingerprinting suite:
  - Canvas noise injection
  - WebGL renderer/vendor masking
  - AudioContext fingerprint resistance
  - Font enumeration restriction
  - User-Agent normalization
  - Client Hints reduction
- DNS-over-HTTPS (default provider: Cloudflare, configurable)
- HTTPS-first mode
- Referrer policy: strict-origin-when-cross-origin
- Tracking parameter stripping (UTM, fbclid, etc.)

### 1.5 Tab Management Foundation (*parallel with 1.4*)
- Tab lifecycle state machine (6 states per PAD):
  Active → Background-active → Idle → Suspended → Frozen → Discarded
- Process Manager: track per-renderer resource usage
- Basic tab suspension for idle tabs (threshold: 5 min idle)
- Working-set trimming for background renderers
- UI indicators for suspended/frozen tabs

---

## Phase 2: Universal Media Engine — "Play Everything" (Weeks 16–30)

> **Solo-dev note**: Extended for comprehensive codec/format coverage. This is a key differentiator — Vigo plays everything users throw at it.

### 2.0 Complete Codec & Format Matrix

Vigo must handle every media format users encounter on the web and locally.

**Video Codecs — ALL enabled**
| Codec | Source | HW Decode | Notes |
|-------|--------|-----------|-------|
| H.264 / AVC | ffmpeg (proprietary build) | DXVA2/VTB/VAAPI | Universal baseline, required for 90%+ of web video |
| H.265 / HEVC | ffmpeg + OS platform decoders | D3D11/VTB/VAAPI | 4K streaming, efficient compression. Use OS HW decoders to avoid patent pool fees |
| VP8 | libvpx (built into Chromium) | Limited | Legacy WebM |
| VP9 | libvpx (built into Chromium) | DXVA2/VTB/VAAPI | YouTube primary codec, widely HW-accelerated |
| AV1 | libaom + dav1d (preferred SW) | D3D11 (Intel 11+, NVIDIA 30+, AMD 6000+) | Next-gen open codec, YouTube/Netflix adopting |
| MPEG-2 | ffmpeg | DXVA2/VAAPI | Legacy DVD content, some IPTV |
| MPEG-4 Part 2 | ffmpeg | Limited | Legacy DivX/Xvid web content |
| Theora | libtheora (Chromium) | None (SW only) | Legacy open-source codec |
| WMV / VC-1 | ffmpeg | DXVA2 (Windows) | Legacy Windows Media content |

**Audio Codecs — ALL enabled**
| Codec | Source | Notes |
|-------|--------|-------|
| AAC (LC, HE, HE-v2) | ffmpeg (proprietary) | Primary web audio |
| MP3 (MPEG-1 Layer 3) | ffmpeg | Universal legacy |
| Opus | libopus (Chromium) | Modern web audio, WebRTC |
| Vorbis | libvorbis (Chromium) | WebM audio |
| FLAC | libflac (Chromium) | Lossless audio |
| PCM / WAV | Built-in | Uncompressed audio |
| AC-3 (Dolby Digital) | ffmpeg | 5.1 surround, streaming |
| E-AC-3 (Dolby Digital Plus) | ffmpeg | Enhanced surround, Netflix/streaming |
| DTS | ffmpeg | Surround sound (some streaming) |
| WMA | ffmpeg | Legacy Windows Media Audio |
| ALAC | ffmpeg | Apple Lossless |
| AMR-NB/WB | ffmpeg | Voice recordings |

**Container Formats — ALL supported**
| Container | Extensions | Notes |
|-----------|-----------|-------|
| MP4 / M4V / M4A | .mp4, .m4v, .m4a | Primary web container |
| WebM | .webm | Google/YouTube standard |
| MKV (Matroska) | .mkv, .mka | Power-user container, multi-track |
| AVI | .avi | Legacy but still common |
| MOV (QuickTime) | .mov | Apple ecosystem |
| FLV | .flv | Legacy Flash video (still on some sites) |
| OGG / OGV | .ogg, .ogv | Open-source container |
| MPEG-TS | .ts, .mts | Transport stream, IPTV, broadcast |
| MPEG-PS | .mpg, .mpeg | Program stream, legacy |
| 3GP / 3G2 | .3gp, .3g2 | Mobile video, user uploads |
| WMV / ASF | .wmv, .asf | Windows Media |
| WAV | .wav | Uncompressed audio container |

**Streaming Protocols**
| Protocol | Support | Notes |
|----------|---------|-------|
| HLS (HTTP Live Streaming) | Via hls.js or native | Apple standard, Netflix/Disney+ |
| DASH (Dynamic Adaptive Streaming) | Via Shaka Player / native | YouTube, most modern ABR |
| MSE (Media Source Extensions) | Native Chromium | Enables JS-driven playback |
| Progressive HTTP | Native | Direct file download + play |
| WebRTC | Native Chromium | Real-time video/audio (calls, live) |
| RTSP/RTP | Via ffmpeg bridge | IP cameras, some live streams |
| HTTP/3 (QUIC) | Native Chromium | Modern transport |

**Image Formats — ALL supported**
| Format | Source | Notes |
|--------|--------|-------|
| JPEG / JPG | Chromium native | Universal |
| PNG | Chromium native | Lossless with transparency |
| GIF | Chromium native | Animated images |
| WebP | Chromium native | Modern web images |
| AVIF | Chromium native | Next-gen, AV1-based image |
| SVG | Chromium native | Vector graphics |
| ICO / CUR | Chromium native | Favicons, cursors |
| BMP | Chromium native | Legacy |
| JPEG XL | Enable via flag / patch | Next-gen (Chromium removed it, re-enable) |
| APNG | Chromium native | Animated PNG |
| TIFF | ffmpeg / custom | Occasional web use |

**DRM Systems**
| DRM | Status | Coverage |
|-----|--------|----------|
| Widevine (L1/L3) | **Mandatory V1** | Netflix, Disney+, Hulu, HBO Max, YouTube Premium, Spotify |
| PlayReady | **V1.1** (Windows only) | Microsoft ecosystem, some 4K Windows content |
| FairPlay | ❌ Not possible | Apple-only, not available on Chromium |
| ClearKey | Native Chromium | Testing / low-security content |

### 2.1 ffmpeg Proprietary Build (*Phase 0 dependency*)
- Build ffmpeg with **all codecs enabled** (`--enable-nonfree --enable-gpl --enable-libx264 --enable-libx265 --enable-libfdk-aac`)
- GN args: `proprietary_codecs = true`, `ffmpeg_branding = "Chrome"`
- Chromium's `//third_party/ffmpeg` with extended codec configuration
- Since Vigo is **proprietary / paid software**, GPL/LGPL ffmpeg linking is permissible under commercial license or dynamic linking
- Compile platform-specific optimizations (SIMD, NEON)

### 2.2 Hardware Decode Pipeline (*depends on 0.5*)
- Windows: DXVA2/D3D11 Video Decoder → DirectComposition (H.264, HEVC, VP9, AV1)
- macOS: VideoToolbox → Metal rendering (H.264, HEVC, VP9; AV1 on Apple Silicon M3+)
- Linux: VAAPI → Vulkan/OpenGL (H.264, HEVC, VP9, AV1 driver-dependent)
- 5-step HW decode prioritization algorithm (per Media spec)
- Software decode fallback via ffmpeg + dav1d (AV1) with CPU cap ≤ 75% at 4K
- **Codec capability detection**: runtime GPU/driver probe → populate decode capability matrix → inform ABR

### 2.3 Media Orchestration Layer (*depends on 2.2*)
- ABR Controller: hybrid throughput + buffer-based
  - Signals: segment speed, buffer depth, frame drops, CPU
  - Modes: Max Quality, Balanced, Data Saver
- Buffer Heuristics Engine: min 10s, optimal 20-30s
- DRM Policy Manager: L1/L3 detection, resolution cap enforcement
- Quality oscillation control: ≤ 3 switches / 10 min
- Rebuffer target: ≤ 1/hour
- **Codec negotiation**: prefer HW-decoded codec when ABR offers multiple (e.g., prefer VP9 HW over H.264 SW at same quality)

### 2.4 HDR & Spatial Audio (*parallel with 2.3*)
- HDR10 passthrough (Windows DirectComposition, macOS color pipeline)
- HDR10+ detection (passthrough where OS supports)
- Dolby Vision **profile 5/8** — software-decodable profiles (no Dolby license for HW path in V1)
- Multi-track audio selection UI
- 5.1 / 7.1 surround passthrough (HDMI/SPDIF bitstream)
- Dolby Digital / Dolby Digital Plus passthrough
- Audio sync drift ≤ 50ms

### 2.5 Picture-in-Picture (*depends on 2.3*)
- Always-on-top mini player
- Playback controls overlay (play/pause, skip, volume, seek bar)
- Resize/reposition freely
- Multi-monitor support
- PiP for any `<video>` element (right-click → "Picture in Picture")

### 2.6 Subtitle Engine (*parallel with 2.5*)
- **Formats**: WebVTT, SRT, SSA/ASS, TTML/DFXP, SUB/IDX (VobSub)
- Style overrides (font family, size, color, outline, background, position)
- Manual timing offset (±10s in 100ms steps)
- Zero frame-delay rendering
- Multi-subtitle track selection UI
- Embedded subtitle extraction from MKV/MP4 containers

### 2.7 Local Media Playback
- Drag-and-drop local files into browser → play with full MOL pipeline
- `file://` protocol video/audio playback with all codecs
- Playlist from folder (basic: play all files in directory)
- Media info overlay (codec, resolution, bitrate, duration — toggle with keyboard shortcut)

### 2.8 JPEG XL Re-enablement
- Chromium removed JPEG XL support (M110) — **Vigo re-enables it**
- Patch Chromium's image decode pipeline to restore `image/jxl` MIME handling
- Differentiator: Vigo supports the format others dropped

### 2.9 Media Testing Matrix (Expanded)
**Streaming DRM**
- Netflix 4K HDR (Widevine L1)
- YouTube 4K AV1 + VP9 hardware decode
- Amazon Prime Video DRM (Widevine)
- Disney+ 4K HDR (Widevine)
- HBO Max / Paramount+ (Widevine)
- Twitch live streams (low-latency HLS)
- Crunchyroll / Funimation (anime, Widevine)

**Codec Coverage**
- H.264 1080p progressive (most common web video)
- HEVC 4K file playback (local .mkv)
- VP9 4K YouTube
- AV1 hardware decode (on supported GPU)
- AV1 software decode via dav1d (on older GPU)
- MPEG-2 legacy content
- WMV/VC-1 legacy content

**Container & Format**
- MKV with multi-audio + multi-subtitle tracks
- MP4 with embedded chapters
- WebM VP9+Opus
- AVI DivX/Xvid legacy
- FLV (legacy Flash video files)
- OGG Theora+Vorbis
- MPEG-TS transport stream

**Audio**
- FLAC lossless playback
- Opus in WebM container
- AC-3 / E-AC-3 5.1 passthrough
- MP3 progressive playback
- AAC-HE in M4A container

**Protocols**
- HLS live stream
- DASH VOD stream
- WebRTC video call (Google Meet, Jitsi)
- Progressive HTTP download+play

**Stress & Stability**
- Network ramp-up/ramp-down (throttle 50Mbps → 2Mbps → 50Mbps)
- 30-minute continuous 4K stability test
- 3×1080p + 1×4K concurrent tabs
- Rapid seek stress (seek every 2s for 5 minutes)

---

## Phase 3: Credential Vault, Sync & Extensions (Weeks 24–38)

### 3.1 Credential Vault (*depends on 1.4*)
- AES-256 encryption at rest
- Master key derivation:
  - Windows: DPAPI + Windows Hello
  - macOS: Keychain + Touch ID
  - Linux: GNOME Keyring / KWallet
- WebAuthn passkey support
- Credential buffer zeroing after use
- Non-pageable memory for sensitive data
- Auto-fill integration

### 3.2 E2E Encrypted Sync — Crypto Layer (*depends on 3.1*)
- Cryptographic primitives (use libsodium via `rust_static_library`):
  - X25519 key agreement
  - Ed25519 signatures
  - XChaCha20-Poly1305 AEAD (primary) / AES-256-GCM (fallback)
  - HKDF-SHA256 key derivation
  - Argon2id password KDF
- Key hierarchy: K_root → HKDF → K_bookmarks, K_passwords, K_history, K_settings, K_tabs
- Per-record encryption: HKDF(K_collection, record_id) → K_record
- Per-device key wrapping: K_root encrypted with each device's DK_pub via hybrid ECDH + AEAD

### 3.3 E2E Encrypted Sync — Client (*depends on 3.2*)
- Device onboarding flows:
  1. Passphrase-based (Argon2id → unwrap K_root)
  2. Passkey/Platform Authenticator (recommended)
  3. QR/Direct Device Pairing (ephemeral ECDH)
- Sync data types: bookmarks, passwords, history, settings, open tabs
- Conflict resolution:
  - Bookmarks: tree CRDT (add/move/delete)
  - Passwords: last-write-wins with timestamp
  - History: append-only merge
  - Settings: last-write-wins
- Recovery flows: passphrase, optional escrowed token, multi-device approval
- Key rotation: client-side K_root rotation, rewrap for all devices

### 3.4 E2E Encrypted Sync — Self-Hosted Server (*parallel with 3.3*)
- REST/JSON API over HTTPS (9 endpoints per Sync spec)
- Zero-knowledge: store only encrypted blobs
- **Self-hosted first**: ship as Docker container + docker-compose for easy deployment
- KV store backend: **SQLite for single-user / PostgreSQL for multi-user**
- Setup wizard: `docker run -p 8443:8443 vigo/sync-server` with auto-TLS (Let's Encrypt)
- Per-user rate limiting
- Device registration/revocation
- Grace period for offline devices during key rotation
- Admin dashboard (basic web UI): connected devices, sync status, storage usage
- Backup/restore: encrypted database export/import
- **Solo-dev priority**: get single-user self-hosted working first, scale later

### 3.5 Extension Platform (*parallel with 3.1*)
- Chrome Web Store compatibility (Manifest V3)
- Three-tier permission model: Basic → Sensitive → Power
- Extension signing infrastructure
- Per-origin + time-scoped permission grants
- Extension sandboxing (isolated processes)
- Rate limiting: network requests, CPU (15% single-core cap), memory
- Developer tools: DevTools extension inspector

### 3.6 Unified Key Architecture Decision
- **Resolve**: local vault key (OS keystore) vs. sync K_root relationship
- **Recommended approach**: vault uses OS keystore for local encryption; sync adds a second encryption layer with K_passwords for transport. Passwords are decrypted from vault, re-encrypted with K_passwords for sync. On receiving device, decrypted from K_passwords, re-encrypted with local vault key.

---

## Phase 4: Performance Hardening & Polish (Weeks 32–42)

### 4.1 Memory Optimization (*depends on 1.5*)
- V8 GC cooperation (low-memory hints for background isolates)
- Cross-process shared memory (fonts, large cache blobs) with COW
- Disk-backed snapshots for frozen tabs (encrypted, versioned)
- Target: 10 idle tabs ≤ 400MB median
- **Media memory**: limit per-video incremental memory to ≤ 150MB (critical with universal codec support)

### 4.2 CPU & Scheduler Optimization (*parallel with 4.1*)
- OS-level CPU priority: reduced for background, elevated for media/GPU
- Cooperative JS timer throttling (budget model)
- Background tab script execution limits
- **Software decode CPU governance**: enforce ≤75% cap, auto-downgrade quality if exceeded

### 4.3 GPU Resource Management (*depends on 2.1*)
- Hardware decoder pool tracking
- Per-process GPU memory budgets
- HDR pipeline activation only when needed

### 4.4 Startup Optimization
- Cold start ≤ 2.5s target
- Warm start < 500ms target
- Profile: identify and defer non-critical initialization
- Lazy-load extension host, sync client, media subsystems

### 4.5 CI Benchmark Suite
- Synthetic: 1→100 tab growth (mixed content)
- Media stress: 3×1080p + 1×4K concurrent
- Leak test: 24-hour idle tab set
- Real-world: top 200 sites crawl
- Weekly regression runs with gates

---

## Phase 5: Security, Licensing & Beta (Weeks 38–50)

### 5.1 Supply Chain Security
- Reproducible/deterministic builds
- SBOM generation per release
- Code signing:
  - Windows: Authenticode certificate
  - macOS: Apple Developer ID + notarization
- Binary hash verification

### 5.2 Auto-Update System
- Signed update manifests
- TLS-pinned update channel
- Differential updates (reduce download size)
- Windows: consider Omaha-based or custom
- macOS: Sparkle framework or custom
- Linux: APT/YUM repo + AppImage updates

### 5.3 Security Audit
- Third-party security audit (engage firm by Week 28)
- Focus areas: sync crypto, credential vault, sandbox escapes, extension isolation
- Fuzz testing: media boundaries, renderer IPC, extension APIs
- Penetration testing: sync server, update channel

### 5.4 Installer Creation
- Windows: MSI + EXE (WiX toolset), default browser registration
- macOS: .dmg + .pkg, App Store consideration
- Linux: .deb (Ubuntu/Debian), .rpm (Fedora/RHEL), AppImage, Flatpak consideration

### 5.5 Beta Program (FREE)
- **Free public beta** — no license key required during beta period
- Beta watermark / banner: "Beta — Free Preview. Vigo will be a one-time purchase at launch."
- Telemetry: opt-in, aggregated, anonymized
- Crash reporting: custom (not Google's)
- Feedback channel: in-browser reporting
- Beta duration: ~3–6 months based on stability
- Collect user feedback for pricing validation

### 5.7 License & Payment System
- One-time purchase model — **no subscriptions, no recurring fees**
- License key generation: cryptographically signed keys (Ed25519 signature over user_id + purchase_date + version)
- Offline validation: key validated locally without phoning home (privacy-first)
- Online activation optional: register key with Vigo server for device management
- Payment processor: **Paddle or FastSpring** (handle global tax compliance for solo dev)
- Pricing research: competitive analysis vs. Sidekick (~$5/mo), Wavebox (~$7/mo), Arc (free)
- Recommended price point: **$29–39 one-time** (lifetime updates for major version, e.g., Vigo 1.x)
- Major version upgrades (2.0, 3.0): paid upgrade at discount for existing users
- Grace period: beta users get **6 months free** after GA, then must purchase
- Refund policy: 30-day money-back guarantee

### 5.6 Critical CVE Patch Pipeline
- Chromium upstream monitoring
- 72-hour SLA for critical CVEs
- Automated patch extraction + build + deploy

---

## Phase 6: Public Beta Launch (Weeks 48–54)

### 6.1 Beta Launch (FREE)
- Landing page + download site (simple: hero, features, download button)
- Documentation: user guide, privacy policy, terms of service, EULA
- **No payment required** — beta is 100% free
- Announce: "Vigo will be a one-time purchase at launch. Beta testers get a loyalty discount."
- Distribution: direct download from vigo-browser.com (no app stores yet)
- Bug reporting: in-browser feedback button + GitHub Issues (public issue tracker, private source)

### 6.2 Beta → GA Transition (Weeks 54–62)
- Stabilize based on beta feedback
- Implement license key system
- Set up payment processing
- Launch pricing page
- Beta user migration: generate free/discounted keys for active beta testers
- **GA Launch**: Vigo 1.0 — one-time purchase, free updates within 1.x

### 6.3 V1.1 Planning
- Developer portal for extensions
- Vigo Extension Store (curated, signed)
- PlayReady DRM (Windows 4K enhancement)
- ML-based tab suspension predictions

### 6.4 V1.2+ Roadmap
- Post-quantum sync (PQXDH + Triple Ratchet migration)
- On-device ML for resource management
- Dolby Vision full HW decode (with license)
- Dolby Atmos spatial audio
- Mobile exploration (if revenue supports it)
- Managed sync hosting option (Vigo-hosted, paid add-on)
- Firefox WebExtension compatibility layer

---

## Relevant Files (Current)
- `Docs/vigo_prd_v_1.md` — Master requirements, personas, feature tiers, NFRs, release timeline
- `Docs/vigo_engine_strategy_document_v_1.md` — Chromium rationale, differentiation layers, non-goals
- `Docs/vigo_extension_platform_specification_v_1.md` — Extension compat, permissions, signing, developer tools
- `Docs/vigo_media_drm_architecture_spec_v_1.md` — MOL architecture, DRM, ABR, HDR, codecs, test matrix
- `Docs/vigo_performance_architecture_document_v_1.md` — Tab lifecycle, memory, CPU, GPU, benchmarks
- `Docs/vigo_security_design_document_v_1.md` — Threat model, sandbox, vault, supply chain, patch SLA
- `Docs/vigo_sync_and_cryptography_architecture_spec_v_1.md` — Zero-knowledge sync, crypto primitives, key hierarchy, device flows
- `Docs/chat.md` — Original brainstorm, gap analysis

## Verification Strategy

### Automated
1. **Build verification**: CI builds pass on Windows first, macOS/Linux follow
2. **Unit tests**: C++ gtest + Rust `#[test]` for all components
3. **Integration tests**: Extension compatibility (top 20 extensions, manually)
4. **Media test matrix**: Full codec coverage (see Phase 2.9 — 35+ test scenarios)
5. **Benchmark suite**: 4 scenarios with regression gates (tab growth, media stress, leak, site crawl)
6. **Security fuzz testing**: libFuzzer for media/IPC/extension boundaries
7. **Crypto test vectors**: NIST/RFC test vectors for all cryptographic operations
8. **Sync roundtrip tests**: Multi-device sync correctness for all data types
9. **Codec verification**: Automated playback test for every codec/container combination in the matrix

### Manual
1. **DRM playback**: Netflix, Disney+, YouTube Premium, Amazon Prime, HBO Max, Crunchyroll (real accounts)
2. **Codec playback**: H.264, HEVC, VP9, AV1, MPEG-2, WMV in MKV/MP4/AVI/WebM/FLV containers
3. **Audio playback**: FLAC, Opus, AC-3, E-AC-3, AAC-HE, MP3, DTS passthrough
4. **Extension compatibility**: uBlock Origin, Bitwarden, Dark Reader, React DevTools, Grammarly
5. **Local file playback**: drag-and-drop MKV, MP4, AVI, MOV, FLV files
6. **Browser fundamentals**: print, find-in-page, bookmarks import, download manager, PDF viewer
7. **Privacy audit**: EFF Cover Your Tracks, BrowserLeaks.com
8. **Accessibility**: Screen reader (NVDA, VoiceOver), keyboard navigation
9. **Self-hosted sync**: Docker deployment, multi-device sync roundtrip, device revocation
10. **License system**: Key generation, activation, offline validation (pre-GA)

## Key Decisions Made
| Decision | Choice | Rationale |
|----------|--------|----------|
| **Licensing** | Full proprietary, closed source | Solo dev, commercial product, protect IP |
| **Monetization** | Free beta → one-time paid (~$29–39) | Sustainable without subscriptions, user-friendly |
| **Sync hosting** | Self-hosted (Docker) | Privacy-first, no cloud dependency, user controls data |
| **Team** | Solo developer | Timeline adjusted to ~16–20 months |
| **Architecture** | Brave-style overlay (NOT direct fork) | Proven by 384+ contributors, minimizes merge conflicts |
| **Build system** | GN + Ninja with npm orchestration | Chromium-native, don't fight the build system |
| **Rust integration** | `rust_static_library` GN template | First-class Chromium support since M119 |
| **DRM** | Widevine mandatory, PlayReady V1.1 | Industry standard, free license |
| **HEVC** | Include V1 (OS HW decoders) | Users expect it, no separate patent fee for HW-only decode |
| **Media** | Play everything (all codecs/containers) | Key differentiator, proprietary license enables it |
| **Sync crypto** | X25519 + XChaCha20-Poly1305 + Argon2id | Battle-tested primitives via libsodium |
| **Conflict resolution** | CRDT bookmarks, LWW settings/passwords, append history | Per-type strategy matches data semantics |
| **Key architecture** | Independent vault + sync layers, bridge via decrypt-reencrypt | Clean separation |
| **JPEG XL** | Re-enabled | Differentiator, Chromium dropped it |

## Open Decisions (Remaining)
1. **Chromium version**: Latest stable vs. specific pinned version
2. **DoH provider**: Cloudflare (recommended), NextDNS, or configurable-only
3. **One-time purchase price**: $29 vs $39 vs $49 (needs market testing during beta)
4. **Payment processor**: Paddle vs FastSpring vs Stripe (tax compliance matters for solo)
5. **ffmpeg licensing approach**: Dynamic linking (LGPL-safe) vs static (requires commercial ffmpeg license or GPL acceptance for proprietary build)

## Solo Developer Strategy

> Building a Chromium-based browser solo is an extreme undertaking. This section is the survival guide.

### Time Allocation (weekly, ~50–60hr weeks)
| Activity | Hours/Week | Notes |
|----------|-----------|-------|
| Core development | 30–35 | The actual building |
| Chromium upstream tracking | 5–8 | Patch maintenance, security updates |
| Build system / CI | 3–5 | Keeping builds green |
| Testing | 5–8 | Manual + automated |
| Documentation | 2–3 | User-facing + internal |
| Admin / licensing / legal | 2–3 | Widevine, payments, EULA |

### Ruthless Prioritization Rules
1. **Ship the browser shell first** — if it can't browse, nothing else matters
2. **Media second** — this is the #1 differentiator
3. **Privacy third** — this is the #2 differentiator
4. **Sync fourth** — nice to have, not launch-critical for beta
5. **Extensions fifth** — Chrome Web Store "just works" with Chromium base, defer custom store
6. **Performance polish last** — optimize only after features work

### Shortcuts for Solo Dev
- Use Chromium's built-in extension support as-is for V1 (skip custom permission UI until V1.1)
- Skip extension signing infrastructure for beta (enforce at GA)
- Use SQLite for sync server (not Cassandra) — scale when you have users
- Skip cross-platform initially: **build Windows first**, then macOS, then Linux
- Use GitHub Actions free tier for CI (limited but free)
- Defer macOS notarization until beta testers actually need it
- Defer Linux packaging until beta demand warrants it

### Adjusted Timeline (Solo)
| Phase | Original | Solo-Adjusted | Delta |
|-------|----------|--------------|-------|
| Phase 0: Foundation | 6 wks | 8 wks | +2 (build system learning curve) |
| Phase 1: Shell + Privacy | 12 wks | 16 wks | +4 (browser fundamentals) |
| Phase 2: Media Engine | 12 wks | 14 wks | +2 (codec breadth) |
| Phase 3: Vault + Sync | 12 wks | 14 wks | +2 (solo complexity) |
| Phase 4: Performance | 10 wks | 10 wks | +0 (can overlap) |
| Phase 5: Security + Beta | 8 wks | 12 wks | +4 (licensing system, no team) |
| Phase 6: Beta Launch | 6 wks | 6 wks | +0 |
| Phase 7: GA Launch | — | 8 wks | New: payment + license system |
| **Total** | **~10–12 mo** | **~16–20 mo** | **+6–8 mo** |

## Risk Register
| Risk | Severity | Mitigation |
|------|----------|------------|
| **Solo burnout** | 🔴 Critical | Set sustainable pace, take breaks, automate relentlessly |
| Widevine license delay | 🔴 Critical | Initiate contact Week 1, PoC is Phase 0 gate |
| Sync crypto implementation bugs | 🔴 Critical | Use libsodium (battle-tested), NIST test vectors, defer external audit to pre-GA |
| ffmpeg GPL/licensing conflict | 🟠 High | Use dynamic linking or obtain commercial ffmpeg license |
| Chromium upgrade patch drift | 🟠 High | Minimize patches, prefer chromium_src overrides |
| Build infrastructure costs | 🟠 High | Windows-first, defer other platforms, use sccache |
| One-time purchase conversion rate | 🟠 High | Validate pricing during beta, offer beta-tester discounts |
| Single point of failure (you) | 🟠 High | Document everything, reproducible builds, no bus factor |
| Extension compatibility breakage | 🟡 Medium | Test top-20 extensions manually per release |
| macOS notarization rejections | 🟡 Medium | Defer macOS to Phase 5, test early |
| HEVC decode inconsistency across GPUs | 🟡 Medium | Runtime capability detection, graceful SW fallback |
