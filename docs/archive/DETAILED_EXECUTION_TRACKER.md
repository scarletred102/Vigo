# Vigo — Detailed Execution Tracker (Servo/Vex Path)

**Last updated:** 2026-03-31  
**Owner:** Vigo core engineering  
**Direction locked:**

- No Chromium dependency
- Servo-style architecture boundaries (embedder ↔ engine)
- Windows-first implementation now
- Cross-platform rollout deferred (not blocked, just sequenced later)

## Execution mode (parallel lanes)

We are executing in parallel, not serial:

- **Lane 1:** Stability + KPI gating (Workstream A)
- **Lane 2:** Core browser capability work (Workstreams B/C/D/E)
- **Lane 3:** Manual product checkpoints (run/use/feedback loop)

---

## How to read this file

- `[ ]` = not started
- `[/]` = in progress
- `[x]` = finished and validated

Each work item has:

1. **Goal** (what outcome we need)
2. **Implementation tasks** (code-level steps)
3. **Validation** (tests/bench/proof required before marking done)

---

## Workstream A — NFR/KPI Proof + Benchmark Gating (**CURRENT SPRINT**)

### Goal

Create objective quality gates for performance and stability, so future features are measured against hard thresholds instead of subjective regressions.

### A.1 Bootstrap KPI gate infrastructure

- [x] **A-001** Add machine-readable KPI thresholds (`benchmarks/kpi_thresholds.json`).
- [x] **A-002** Add deterministic KPI gate runner (`scripts/kpi-gate.ps1`) that:
  - runs benchmark tests,
  - extracts measured values,
  - compares to thresholds,
  - writes JSON report artifact,
  - fails with clear diagnostics when thresholds are exceeded.
- [x] **A-003** Wire KPI gate into CI (`.github/workflows/ci.yml`) as dedicated Windows job.

### A.2 Expand KPI coverage (next)

- [x] **A-004** Add cold-start KPI capture in `vex-app` (time to first frame).
- [x] **A-005** Add process memory KPI capture (working set snapshots) for 10 idle tabs.
- [x] **A-006** Add crash-free session accounting + session health report.
- [ ] **A-007** Enforce KPI regression budget policy in CI (hard fail beyond budget).

### A.3 Validation checklist

- [x] KPI gate script runs locally.
- [x] KPI report artifact generated at `target/kpi-report.json`.
- [x] CI contains dedicated KPI job.
- [x] Cold-start KPI snapshot writer implemented and unit-tested in `vex-app`.
- [x] Windows working-set memory snapshot writer implemented and unit-tested.
- [x] Crash-free session counters persisted to `session_health.json`.
- [ ] KPI job enforces cold start, memory, crash-free metrics (pending A-007 tightening).

---

## Workstream B — WebAuthn + Platform Authenticators

### Goal

Implement WebAuthn/passkey flows with platform authenticators:

- Windows Hello first
- Touch ID abstraction prepared for later

### Tasks

- [x] **B-001** Create `vex-browser::webauthn` core types + request/response models.
- [ ] **B-002** Add `PlatformAuthenticator` trait and Windows Hello provider.
- [ ] **B-003** Add browser-side credential broker (embedder command/message path).
- [ ] **B-004** Wire JS API surface `navigator.credentials.create/get`.
- [ ] **B-005** Add secure credential persistence + RP ID validation.
- [ ] **B-006** Add end-to-end tests for registration + assertion flow.

### Validation

- [ ] Passkey create/assert against test RP on Windows.
- [ ] User cancel/error paths mapped to spec-like DOMException semantics.
- [ ] No raw private key export path.

---

## Workstream C — Chrome-Extension Fidelity (without Chromium)

### Goal

Reach high-fidelity compatibility for top extension categories while keeping strict isolation.

### Tasks

- [/] **C-001** Upgrade manifest compatibility toward MV3 shape.
- [x] **C-002** Implement runtime messaging primitives.
- [ ] **C-003** Implement host permissions + runtime grant/revoke workflow.
- [ ] **C-004** Add declarative network rules subset for adblock-class extensions.
- [ ] **C-005** Isolate extension workers/process boundaries.
- [ ] **C-006** Extension compatibility harness for representative top extensions.

---

## Manual Checkpoint Loop (run + use + assess)

- [x] **CP-001** Add checkpoint launcher script: `scripts/checkpoint-run-browser.ps1`.
- [ ] **CP-002** User-run checkpoint completed and feedback captured.

### How to run checkpoint

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\checkpoint-run-browser.ps1
```

Optional dry run (no GUI launch):

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\checkpoint-run-browser.ps1 -NoRun -SkipBuild
```

### Validation

- [ ] Adblocker / password manager / productivity extension pass matrix.
- [ ] Permission UI supports per-site overrides.
- [ ] Extension cannot access privileged browser-core internals.

---

## Workstream D — Media/DRM Matrix

### Goal

Deliver verified media behavior across ABR, PiP, media keys, casting, and DRM policy pathways.

### Tasks

- [ ] **D-001** Harden ABR telemetry and quality-switch reporting.
- [ ] **D-002** Integrate PiP controls with browser UI lifecycle.
- [ ] **D-003** Implement media-key handling bridge to `MediaSession` actions.
- [ ] **D-004** Add casting discovery/control skeleton (Windows-first path).
- [ ] **D-005** Build DRM policy matrix executor (Widevine/PlayReady/ClearKey detection paths).
- [ ] **D-006** Expand playback test matrix and CI automation hooks.

### Validation

- [ ] ABR target proof (rebuffer regression report vs baseline).
- [ ] PiP + media keys pass manual + automated checks.
- [ ] DRM matrix report generated for supported/unsupported environments.

---

## Workstream E — Sync Commitments (E2E + self-host + recovery)

### Goal

Complete zero-knowledge sync lifecycle beyond basic encrypted payload transport.

### Tasks

- [ ] **E-001** Add device onboarding flow (first device + additional device).
- [ ] **E-002** Add explicit self-host endpoint config + validation.
- [ ] **E-003** Implement recovery token/passphrase flow.
- [ ] **E-004** Implement key rotation and device revocation.
- [ ] **E-005** Add conflict-resolution policy with deterministic merge behavior.
- [ ] **E-006** Build sync integration tests against local sync-server.

### Validation

- [ ] Two-device encrypted sync works end-to-end.
- [ ] Self-host works without cloud dependency.
- [ ] Recovery flow tested and documented with user warnings.

---

## Finished Work Log

### 2026-03-31

- [x] Created this detailed tracker file.
- [x] Implemented KPI threshold source: `benchmarks/kpi_thresholds.json`.
- [x] Implemented KPI gate runner: `scripts/kpi-gate.ps1`.
- [x] Added CI job: `kpi-gate` in `.github/workflows/ci.yml`.
- [x] Implemented cold-start KPI snapshot capture in `crates/vex-app/src/main.rs`.
- [x] Added `writes_kpi_snapshot_json` unit test (4/4 vex-app tests passing).
- [x] Implemented Windows working-set KPI capture (`GetProcessMemoryInfo`) in `crates/vex-app/src/main.rs`.
- [x] Added `reads_current_process_working_set` unit test (5/5 vex-app tests passing).
- [x] Implemented session health persistence and crash-free accounting (`session_health.json`).
- [x] Added `session_health_roundtrip` unit test (6/6 vex-app tests passing).
- [x] Added WebAuthn core scaffolding module: `crates/vex-browser/src/webauthn.rs`.
- [x] Added extension runtime messaging hub: `crates/vex-browser/src/extensions/messaging.rs`.
- [x] Extended extension manifest compatibility for MV2/MV3 background entrypoints.
- [x] Added manual checkpoint script: `scripts/checkpoint-run-browser.ps1`.
- [x] Wired runtime messaging through loader/app loop (`vigo.runtime.sendMessage` request routing + active-tab dispatch).
- [x] Replaced extension content-script "match-only" logging with actual runtime injection + per-tab dedupe.
- [x] Applied Servo-shell-inspired chrome palette across tab/nav bars and app chrome overlays.

---

## Current active focus

**Active items:** Workstream A.2 (A-007), Workstream C (C-001), Workstream B (B-002)  
**Next concrete step:** wire runtime WebAuthn provider abstraction (Windows Hello) while tightening KPI gate thresholds from baseline to budgeted targets.
