# Performance Architecture Document (PAD) — Vigo v1.0

**Product:** Vigo  
**Document Version:** 1.0  
**Status:** Architecture Locked — Required for Implementation  
**Owner:** Performance Engineering Lead  
**Last Updated:** [Insert Date]

---

# 1. Purpose

This document specifies Vigo’s Performance Architecture: goals, constraints, process orchestration, memory management heuristics, tab lifecycle policies, GPU and media resource management, telemetry signals for decision-making, performance KPIs, and the CI benchmarking matrix. It is the source of truth for all performance-related implementation and tuning across the product stack.

---

# 2. Objectives & Design Principles

## 2.1 Objectives
- Minimize resident memory footprint for concurrent tab workloads.
- Reduce CPU utilization during heavy media playback (prefer hardware decode).  
- Preserve interactive latency (tab switch, URL input, UI responsiveness).  
- Guarantee predictable behavior on a broad range of hardware (low-end → high-end).  
- Provide measurable and repeatable benchmarks for every release.

## 2.2 Design Principles
- **Predictive Resource Management:** Use telemetry-derived heuristics to predict resource pressure and act pre-emptively.  
- **Graceful Degradation:** When resources are constrained, reduce non-critical subsystems first (background tabs, prefetchers).  
- **Determinism in Policies:** Avoid ad-hoc heuristics; make policies reproducible and tunable via feature flags.  
- **Memory Safety:** Prefer Rust for memory-sensitive subsystems to reduce leaks & crashes.  
- **User Transparency:** Provide UI indicators when the browser suspends tabs or frees resources.

---

# 3. System Model & Components

### Layers
1. **Browser Core (Controller):** Global decision-maker; enforces policies and orchestrates subsystems.  
2. **Process Manager:** Tracks renderer/GPU/media/extension processes, resource footprints, and lifecycle.  
3. **Memory Reclaimer:** Implements working-set trimming, heap compaction triggers, and OS-level hints.  
4. **Tab Lifecycle Manager:** Tab states (active, background-active, suspended, frozen, discarded).  
5. **Media Resource Manager:** Handles hardware decoder pools, buffer limits, and priority assignment.  
6. **Telemetry & Signal Bus:** Low-latency channel for metrics used by decision engines.

---

# 4. Tab & Process Lifecycle (State Machine)

## 4.1 States
- **Active:** Foreground tab with recent input/interaction. Full resources.  
- **Background-active:** Background tab with active audio/long-running JS. Reduced scheduling priority.  
- **Idle:** Background tab with no activity; eligible for light-weight throttling.  
- **Suspended (Soft):** Script timers paused, paint subtree kept, memory reduced via discarding caches. Quick resume.  
- **Frozen (Hard):** Renderer state serialized to disk; process may be terminated. Resume requires deserialization.  
- **Discarded:** Tab removed from memory fully; session stored; resume requires full reload.

## 4.2 Transition Triggers
- User input → Active  
- Media playback / audio → Background-active  
- Idle timer + memory pressure → Suspended  
- High memory pressure + long idle → Frozen / Discarded  

## 4.3 Policies
- Prefer **Suspended** over **Frozen** for tabs with non-zero chance of imminent reuse (heuristics based on activity recency and user behavior patterns).  
- Reserve fast-resume cache for pinned tabs and media-heavy tabs.  
- Do not discard tabs with ongoing downloads, WebRTC, or media capture.

---

# 5. Memory Management Strategy

## 5.1 Working Set Trimming
- Periodically invoke allocator shrink-to-fit on renderer processes flagged by the Memory Reclaimer.  
- Free caches (image, script, font) aggressively in background/idle tabs.  

## 5.2 Garbage Collection Cooperation
- Coordinate with V8: advise low-memory GC heuristics for background renderers.  
- Use V8 flags to reduce memory retention for inactive isolates.

## 5.3 Cross-Process Sharing
- Use shared memory for immutable resources (fonts, large cache blobs) with copy-on-write semantics to reduce duplication.  

## 5.4 Disk-backed Snapshotting
- For **Frozen** state, serialize minimal renderer state and truncate process memory.  
- Snapshot format versioned and validated; encrypted when necessary.

---

# 6. CPU & Scheduler Policies

- Background renderers get reduced CPU scheduling priority (OS-level hints).  
- Media processes and GPU tasks have higher priority to maintain smooth playback.  
- Use cooperative throttling for JS timers (budgeting model for background scripts).  

---

# 7. GPU & Media Resource Management

## 7.1 Hardware Decoder Pooling
- Maintain a pool of hardware decoder slots per system, tracked by the Media Resource Manager.  
- Prioritize decoders for foreground playback; opportunistically demote or transition background decoders to software decode when necessary.  

## 7.2 GPU Memory Budgeting
- Enforce per-process GPU memory budgets, reclaiming textures from background tabs first.  
- Support low-memory GPU devices by forcing lower-resolution surfaces for inactive tabs.  

## 7.3 HDR & Color Management Overhead Minimization
- Only enable full HDR pipeline when an active playback requires it; otherwise prefer SDR to avoid GPU/color pipeline overhead.

---

# 8. Network & Prefetch Controls

- Prefetching (DNS, resources, speculative connections) limited by global budget; budgets shrink under memory/CPU pressure.  
- Background prefetch paused when on battery or metered networks (heuristics).  
- Parallel segment fetching limited to avoid saturating the network for other tabs.

---

# 9. Heuristics & ML Signals (Optional)

## 9.1 Heuristics Inputs
- Tab last-visited timestamp  
- Interaction frequency  
- Time-of-day patterns  
- User device class (low/medium/high spec)

## 9.2 ML Opportunity (V1.2+)
- Train models to predict the probability of a tab being reused in the next N minutes to improve suspend/discard decisions.  
- Use on-device models with privacy-preserving training and periodic updates.

---

# 10. Telemetry Signals & Privacy

Telemetry (opt-in only) used for policy tuning:  
- Process RSS and private bytes  
- Tab resume times  
- Suspension/destruction rates  
- Per-tab CPU/GPU usage during playback  

Telemetry aggregated locally and anonymized prior to upload. Users can opt-out any time.

---

# 11. Performance KPIs & Benchmarks

### KPIs
- Cold start ≤ 2.5s  
- Warm start ≤ 0.5s  
- Median memory for 10 idle tabs ≤ 400MB  
- 4K playback CPU delta ≤ -10% vs baseline Chromium when software decode used  
- Tab resume latency from Suspended state ≤ 350ms (target)  

### Benchmark Suite
- Synthetic tab growth test (open 1→100 tabs with mixed content)  
- Media playback stress (3 concurrent 1080p/1 concurrent 4K)  
- Long-duration leak test (24-hour idle tab set)  
- Real-world site crawl test (top 200 sites)  

CI must run weekly benchmarks; regressions trigger gating policies.

---

# 12. Testing & Validation Matrix

- Unit tests for Memory Reclaimer and Tab Lifecycle Manager.  
- Integration tests with V8 flags for background GC.  
- Cross-platform performance runs on Windows x86/x64/ARM, macOS Intel/ARM, Linux Ubuntu (multiple kernels).  
- Hardware GPU matrix tests (NVIDIA, AMD, Intel, Apple Silicon).  

---

# 13. Implementation Roadmap

**Sprint 0 (4 weeks):** Implement Process Manager skeleton, telemetry bus, and basic tab states.  
**Sprint 1 (6 weeks):** Memory Reclaimer, basic suspended state with quick-resume cache.  
**Sprint 2 (6 weeks):** Hardware decoder pool & media prioritization hooks.  
**Sprint 3 (6 weeks):** Disk-backed frozen state & snapshotting; V8 GC tuning.  
**Sprint 4 (4 weeks):** Performance profiling, CI benchmark integration, and regression gates.

---

# 14. Operational Concerns & Rollback Strategies

- Feature flags for all major policies; canary flag rollout for policy changes.  
- Automatic rollback if memory or crash KPIs degrade beyond thresholds.  
- Debugging hooks to fetch snapshots and logs from affected user sessions (with consent).

---

# 15. Appendix

- Glossary: RSS, Working Set, V8 Isolate, CDM  
- Sample configuration knobs (suspend timeout, discard thresholds)  
- Suggested open-source tools for benchmarking (e.g., tsung, browsertime)

---

**End of Performance Architecture Document (PAD) v1.0**

