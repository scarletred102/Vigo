# Media & DRM Architecture Specification — Vigo v1.0

**Product:** Vigo  
**Document Version:** 1.0  
**Status:** Architecture Locked — Ready for Engineering  
**Owner:** Media Systems Lead  
**Last Updated:** [Insert Date]

---

# 1. Purpose

This document defines the complete media playback and DRM architecture for Vigo v1.0. It specifies:

- DRM integration strategy
- Hardware decode pathways
- Adaptive bitrate (ABR) logic
- Media process isolation
- HDR & audio handling
- Subtitle rendering
- Performance targets
- Platform-specific implementations

This specification governs all streaming-related development.

---

# 2. Architectural Overview

Vigo uses Chromium’s media pipeline as a base but introduces a Media Orchestration Layer (MOL) above it.

Architecture:

+----------------------------------------------------+
| Vigo Media Orchestration Layer (MOL)              |
| - ABR Controller                                   |
| - Buffer Heuristics Engine                         |
| - Hardware Decode Prioritizer                      |
| - DRM Policy Manager                               |
| - HDR & Audio Manager                              |
| - Subtitle Rendering Engine                        |
+----------------------------------------------------+
| Chromium Media Pipeline                            |
| - EME                                              |
| - CDM (Widevine)                                   |
| - Demuxers                                         |
| - Decoders                                         |
| - Renderer                                         |
+----------------------------------------------------+
| OS / GPU Hardware Layer                            |
+----------------------------------------------------+

---

# 3. DRM Architecture

## 3.1 DRM Standards Supported (V1)

Mandatory:
- Widevine (Primary DRM integration)

Investigative / Platform-Conditional:
- PlayReady (Windows enhancement path, evaluation phase)

V1 Scope Decision:
Widevine integration is mandatory. PlayReady optional depending on licensing and feasibility.

---

## 3.2 Widevine Integration Model

- Widevine CDM integrated via Chromium EME interface.
- Support Widevine L1 where hardware and OS secure path permit.
- Fallback to L3 if hardware secure path unavailable.

Secure Path Requirements:
- Encrypted media buffers must remain protected in memory.
- Hardware-backed key storage when OS supports it.

Acceptance Criteria:
- On supported hardware, Netflix/Prime/Disney+ reach maximum allowed resolution.
- CDM handshake time < 1 second under stable network.

---

## 3.3 DRM Policy Manager (Vigo Layer)

Responsibilities:
- Detect DRM level (L1/L3)
- Expose playback capability matrix to ABR engine
- Enforce resolution caps when DRM security requirements unmet
- Prevent software fallback when hardware secure path required

---

# 4. Hardware Acceleration Architecture

## 4.1 Codec Support Matrix (V1)

Required Codecs:
- H.264
- VP9
- AV1
- HEVC (platform dependent; licensing review required)

---

## 4.2 Platform Decode Paths

### Windows
- DXVA2 / D3D11 Video Decoder
- DirectComposition pipeline
- HDR10 passthrough if GPU supports

### macOS
- VideoToolbox hardware decode
- Metal-backed rendering
- HDR via macOS color pipeline

### Linux
- VAAPI decode
- Vulkan/OpenGL rendering
- Driver compatibility matrix required

---

## 4.3 Hardware Decode Prioritizer

Algorithm:
1. Check codec support
2. Check driver compatibility
3. Check DRM security requirements
4. Prefer hardware decode
5. If unavailable, fallback to optimized software decode

Constraint:
Software decode must not exceed 75% sustained CPU utilization during 4K playback.

---

# 5. Adaptive Bitrate (ABR) Strategy

## 5.1 ABR Model

Hybrid Strategy:
- Throughput-based estimation
- Buffer-based fallback control

Primary Signals:
- Recent segment download speed
- Current buffer depth
- Frame drop rate
- CPU utilization

---

## 5.2 ABR Modes

Default Mode:
- Aggressive upgrade when stable bandwidth
- Conservative downgrade when instability detected

Optional User Modes:
- Max Quality Mode
- Data Saver Mode

---

## 5.3 Performance Targets

- Rebuffer events ≤ 1 per hour of playback
- Quality oscillation limited to ≤ 3 switches per 10 minutes under stable network

---

# 6. Buffering & Network Heuristics

Buffer Targets:
- Minimum: 10 seconds
- Optimal: 20–30 seconds

Network Behavior:
- Parallel segment fetch allowed if server supports
- HTTP/3 preferred
- Fallback to HTTP/2 automatically

CDN Failover:
- Detect repeated segment timeouts
- Trigger CDN host re-resolution

---

# 7. HDR & Audio Pipeline

## 7.1 HDR Handling

Supported:
- HDR10 (where OS & display allow)

Requirements:
- Detect display HDR capability
- Pass HDR metadata correctly through GPU pipeline
- Disable HDR if OS reports incompatibility

---

## 7.2 Audio Handling

- Multi-track audio support
- Stereo & 5.1 passthrough where available
- Sync drift correction ≤ 50ms

---

# 8. Subtitle Engine

Features:
- WebVTT support
- SRT support (for HTML5 players that allow injection)
- Styling override capability
- Manual subtitle offset control

Performance Requirement:
Subtitle rendering must not introduce frame delay.

---

# 9. Media Process Isolation

Media must run in isolated GPU / media processes.

Requirements:
- Renderer crash must not terminate media process.
- CDM must remain sandboxed.
- Memory buffers protected.

---

# 10. Performance Benchmarks

V1 Targets:
- 4K playback CPU usage < stock Chromium baseline by ≥10% when hardware decode unavailable.
- Memory overhead per active video ≤ 150MB incremental.
- Playback startup latency ≤ 2 seconds on broadband (>50Mbps).

---

# 11. Testing Matrix

Mandatory Test Scenarios:

1. Netflix 4K HDR playback (Windows + macOS)
2. YouTube AV1 4K playback
3. Prime Video DRM validation
4. Anime HTML5 custom player playback
5. Network throttling 5Mbps → 50Mbps ramp test
6. 30-minute continuous playback stability test

CI must automate playback validation where possible.

---

# 12. Security Requirements

- DRM keys never exposed to renderer process.
- Media buffers encrypted until decode stage.
- No persistent storage of decrypted segments.
- CDM updated alongside Chromium security patches.

---

# 13. Telemetry (Opt-In Only)

If user consents:
- Playback resolution success rate
- Rebuffer count
- CPU usage metrics

Telemetry must be aggregated and anonymized.

---

# 14. Risks & Mitigation

Risk: DRM license restrictions
Mitigation: Early vendor engagement and legal review.

Risk: GPU driver incompatibility
Mitigation: Maintain hardware compatibility matrix and fallback logic.

Risk: HDR misconfiguration
Mitigation: OS-level HDR validation before enabling.

---

# 15. Out of Scope (V1)

- Dolby Vision
- Offline DRM downloads (pilot only post-V1)
- In-house CDM development
- Custom video codec development

---

# 16. Final Statement

The Vigo Media & DRM architecture is designed to:

- Achieve maximum legally permitted streaming quality
- Maintain strict process isolation
- Optimize CPU and memory efficiency
- Preserve DRM security guarantees

This document is binding for V1 media implementation.

---

**End of Media & DRM Architecture Specification v1.0**
