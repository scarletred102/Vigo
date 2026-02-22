# Product Requirements Document (PRD) — Vigo v1.0

**Product Name:** Vigo  
**Document Version:** 1.0  
**Status:** Draft — Ready for engineering handoff  
**Owner:** Founder / Product  
**Last Updated:** [Insert Date]

---

## 0. Executive summary
Vigo is a desktop-only browser (Windows, macOS, Linux) focused on **streaming supremacy, performance discipline, and privacy by default**. Version 1.0 (V1) ships a stable, production-ready desktop browser built on Chromium (Blink + V8) with distinct Vigo Browser Core enhancements (media orchestration, Rust adblock engine, memory discipline, and independent sync). V1 targets streaming quality parity with mainstream players (maximize permitted resolution), strong privacy defaults, and extension compatibility.

**Primary goals for V1:**
- Deliver the highest legally-available video quality on major streaming services for supported hardware.
- Demonstrate measurable memory/CPU improvements vs stock Chromium on representative workloads.
- Provide a secure, opt-in sync system and a privacy-first default configuration.

**V1 Non-goals:** mobile clients, custom JS/HTML engine, full enterprise feature set, built-in VPN (premium feature later).

---

## 1. Personas & Use Cases

### Personas
1. **Streaming Enthusiast (Sam)** — consumes 4K/HDR content, expects full quality and stable playback. Uses 2–6 large display setups. Sensitive to buffering and playback quality.
2. **Power User / Veteran (Alex)** — uses many extensions and 20+ tabs, cares about resource efficiency and keyboard-driven workflows.
3. **Privacy-Conscious Professional (Priya)** — needs anti-tracking, passkeys, and secure sync without vendor telemetry.

### Primary Use Cases (V1)
- Watch 4K Netflix/Prime/Disney+/YouTube when hardware permits.  
- Play videos on niche players (anime sites) with modern HTML5 pipeline.  
- Install and run Chrome-compatible extensions.  
- Store and unlock passwords using Windows Hello/Touch ID and optional cloud sync (encrypted).  
- Block ads/trackers by default and allow per-site exceptions.

---

## 2. Scope (MUST / SHOULD / NICE-to-have)

### MUST (V1 launch)
- Chromium-based core with Vigo Browser Core overlay.  
- Built-in Rust adblock & tracker protection with EasyList/EasyPrivacy support.  
- Media Orchestration Layer supporting: EME/Widevine integration, hardware-accelerated decoding for H.264/VP9/AV1 where available, DASH/HLS playback support, basic ABR strategy, and PiP.  
- Windows Hello and macOS Touch ID integration for credential unlocking and WebAuthn passkeys.  
- Default privacy mode: third-party cookies blocked, anti-fingerprinting enabled (configurable), DoH enabled.  
- Chrome Web Store extension compatibility and extension permission UI.  
- Memory discipline: tab suspension & working set trimming heuristics.  
- Auto-update mechanism with signed releases; reproducible build process documented.  
- Opt-in encrypted sync (bookmarks, passwords, history, settings) with E2E encryption.  
- Cross-platform installers (Windows MSI/EXE, macOS signed .dmg/.pkg, Linux .deb/.rpm).  
- Developer tools parity (basic DevTools available).  

### SHOULD (post-initial ramp; aim first 2–3 releases)
- Multi-CDN fallback discovery heuristics / lightweight CDN selection.  
- Offline video downloads for DRM-compatible content (limited pilot).  
- Built-in privacy report dashboard showing blocked trackers & resource savings.  
- Vertical tabs and advanced tab grouping.  

### NICE-TO-HAVE (future / premium)
- Built-in premium VPN / Tor toggle.  
- Advanced subtitle engine with AI-assisted sync.  
- Deep HDR calibration controls and passthrough audio mapping.  

---

## 3. Functional Requirements (detailed)

### 3.1 Core Browser
**FR-CORE-001:** Vigo must launch on Windows, macOS, and major Linux distros and display the home page within 2.5s on a modern desktop (cold start measurement).  
**Acceptance criteria:** cold start measured on benchmark suite ≤ 2.5s (documented environment).  

**FR-CORE-002:** Tabs shall be isolated by site and executed in separate renderer processes by default.  
**Acceptance criteria:** a renderer crash only kills that tab and not the entire browser; site isolation enabled in default configuration.  

**FR-CORE-003:** Address bar (omnibox) should support URL suggestions, history, and keyword search providers.  
**Acceptance criteria:** omnibox returns relevant results and supports custom search engine configuration.


### 3.2 Media & Streaming
**FR-MEDIA-001:** Must support HTML5 video playback (MP4/H.264, WebM/VP9, AV1) with GPU-accelerated decode where drivers allow.  
**Acceptance criteria:** video decode uses hardware pipeline on supported platforms (verified via telemetry during test runs).  

**FR-MEDIA-002:** EME & Widevine integration for DRM playback (Widevine L1 prioritized where hardware allows).  
**Acceptance criteria:** Netflix/Prime/Disney+ playback reaches permitted max resolution for target system when DRM + hardware conditions satisfied; documented test matrix.  

**FR-MEDIA-003:** Implement ABR algorithm (hybrid throughput+buffer) with fallback to conservative mode on unstable networks.  
**Acceptance criteria:** ABR reduces rebuffers by X% vs baseline (initial target X=30% on test harness).  

**FR-MEDIA-004:** PiP (picture-in-picture), native media keys support, and casting (Chromecast basics) must be available.  
**Acceptance criteria:** PiP toggle works on major players; media keys control playback.


### 3.3 Privacy & Adblocking
**FR-PRIV-001:** Built-in adblocker must block known ad/track domains by default; lists must be updatable.  
**Acceptance criteria:** On top 100 news sites sample, blocker removes at least 80% of known trackers (benchmarked).  

**FR-PRIV-002:** Default privacy mode shall block third-party cookies and fingerprinting vectors; users can set site exceptions.  
**Acceptance criteria:** Third-party cookies blocked for sample cross-site tests; fingerprinting scripts mitigated where possible.  

**FR-PRIV-003:** DoH must be enabled by default with a selectable default DoH provider (privacy-first).  
**Acceptance criteria:** DNS queries observed to resolve via DoH in test environment.


### 3.4 Security & Auth
**FR-AUTH-001:** Integrate Windows Hello and macOS Touch ID for unlocking the password vault and WebAuthn passkey operations.  
**Acceptance criteria:** Creating or using a passkey prompts Windows Hello/Touch ID, which completes authentication and allows site login.  

**FR-AUTH-002:** Implement secure encrypted password vault (AES-256 or stronger) with OS-backed key storage.  
**Acceptance criteria:** Vault is encrypted at rest; key material leverages platform keystore.


### 3.5 Extensions & Ecosystem
**FR-EXT-001:** Support Chrome Web Store extensions and implement runtime permission tooling (grant/revoke).  
**Acceptance criteria:** Install sample popular extensions; extension permissions UI shows and allows per-site permission adjustments.  

**FR-EXT-002:** Extensions must be sandboxed; risky APIs require runtime user consent.  
**Acceptance criteria:** Extension cannot access privileged browser internals without explicit user consent.


### 3.6 Sync & Profile Management
**FR-SYNC-001:** Provide optional end-to-end encrypted sync for bookmarks, passwords, history, and settings.  
**Acceptance criteria:** User can create an encrypted account or self-hosted endpoint; sync data is stored encrypted and decrypted client-side only.  

**FR-SYNC-002:** Profile import: allow users to import bookmarks/passwords/history from Chrome/Firefox.  
**Acceptance criteria:** Import completes for baseline datasets with minimal data loss.


## 4. Non-Functional Requirements (NFRs)

### Performance
- **NFR-PERF-001:** Memory target: median memory usage for 10 idle tabs ≤ 400MB on defined benchmark machine.  
- **NFR-PERF-002:** Cold start ≤ 2.5s; warm start < 500ms.  
- **NFR-PERF-003:** 4K playback CPU usage < baseline Chromium by at least 10% (where hardware decode not available).  

### Reliability
- **NFR-REL-001:** Crash-free sessions ≥ 99.9% in field telemetry (opt-in).  
- **NFR-REL-002:** Harmful site navigation detection with safe-browsing feed latency < 1s.  

### Security & Privacy
- **NFR-SEC-001:** Supply-chain: reproducible builds and code signing for release artifacts.  
- **NFR-SEC-002:** Telemetry is opt-in, privacy-preserving aggregation enforced for any telemetry collected.  

### Accessibility & Internationalization
- **NFR-A11Y-001:** WCAG 2.1 AA compliance on key browser UI screens.  
- **NFR-I18N-001:** Support English (US) at launch; plan for CJK + major EU locales by v1.2.


## 5. Acceptance Criteria & Test Cases (Representative)

- **AC-01 (Streaming):** On benchmark Windows 11 machine with Intel GPU and Widevine L1 path, Netflix plays at the maximum available resolution permitted; no pixelation and <1% rebuffer events during a 30-minute play session.  
- **AC-02 (Memory):** On benchmark machine, 10 idle tabs memory median must be ≤ 400MB.  
- **AC-03 (Privacy):** On sampled tracker test page, default Vigo blocks >80% of trackers and third-party cookies.  
- **AC-04 (Auth):** Creating a WebAuthn passkey uses Windows Hello and allows login without visible password fields on sites that support passkeys.  
- **AC-05 (Extensions):** Install three representative Chrome extensions (adblocker extension, password manager, and productivity extension) and confirm runtime permission UI and isolation.

---

## 6. User Flows (Top-level)

1. **First run / Onboarding:** Import options (Chrome/Firefox), privacy mode explanation, enable/disable telemetry, account creation for sync (optional), quick tour for media features.  
2. **Playback flow:** Navigate to streaming site → DRM handshake → ABR engages → hardware decode used → user toggles quality or PiP.  
3. **Extension flow:** Install from Chrome Web Store → runtime permission prompt → user can view/modify extension permissions in site settings.  
4. **Credential flow:** User saves password → stored in vault → user later logs in using Windows Hello.

---

## 7. Dependencies & Integrations
- **Chromium baseline version:** baseline branch (TBD) — decide and lock before implementation.  
- **DRM vendors:** Widevine (mandatory), PlayReady (investigate for Windows HDR paths), vendor contracts required.  
- **Codec licensing:** HEVC patents review and policy (HEVC optional; AV1 preferred where available).  
- **DoH provider:** default provider (privacy-first) selection (e.g., Cloudflare, NextDNS partner).  
- **Telemetry & Safe-browsing:** partner or self-managed threat feed.

---

## 8. Security & Compliance Requirements
- Reproducible builds + signed artifacts.  
- SBOM generation for each release.  
- Bug bounty program at public launch window.  
- GDPR/CCPA compliance for any stored user data.  
- Legal review for DRM and codec licensing prior to any streaming claims in marketing.

---

## 9. Release Plan & Milestones (Suggested)
- **Phase 0 (4–6 weeks):** Tech spike — Widevine integration proof-of-concept on Windows; baseline Chromium fork and build.  
- **Phase 1 (3 months):** Core browser shell, omnibox, tabing, basic DevTools, adblock Rust prototype.  
- **Phase 2 (3 months):** Media Orchestration Layer, ABR prototype, PiP, hardware decode integration.  
- **Phase 3 (2 months):** Windows Hello/WebAuthn integration, password vault, sync prototype.  
- **Phase 4 (2 months):** QA & security audits, platform packaging, beta launch (canary/beta channels).  
- **Phase 5 (1 month):** Public launch & measurement.

Total estimated time to public beta: ~10–12 months (engineering team size dependent).

---

## 10. Metrics & Success Criteria (KPIs)
- 4K/Max-resolution success rate on supported hardware (target ≥ 90%).  
- Playback rebuffer rate (target ≤ 1 rebuffer per hour per active session).  
- Memory footprint median for 10 tabs (target ≤ 400MB).  
- Crash rate (target < 0.1 crashes per 1000 sessions).  
- Extension compatibility rate (target: top 100 Chrome extensions run without major issues).  
- User opt-in telemetry rate (target ≥ 10% post-onboarding for product improvement — opt-in only).

---

## 11. Out-of-Scope (V1)
- Mobile clients (iOS/Android).  
- Full enterprise management console and MDM features.  
- Built-in VPN/Tor (premium).  
- In-house-written JS engine or rendering engine.  
- AI assistant or browser-embedded LLM features in V1.

---

## 12. Risks & Mitigations (Summary)
- **DRM licensing failure:** Secure vendor engagement early; provide fallback messaging.  
- **Upstream Chromium divergence pain:** Appoint integration team and strict rebase cadence.  
- **Memory/performance not meeting targets:** Prioritize Rust subsystems and tab consolidation.  
- **Extension security abuse:** Implement extension signing and runtime permission gating; initial curated gallery.  

---

## Appendix
- Glossary (EME, Widevine, ABR, DoH, SBOM).  
- Benchmarking hardware profiles and CI test matrix (to be filled by engineering).  
- Stakeholder sign-off matrix.

---

**End of PRD v1.0**



