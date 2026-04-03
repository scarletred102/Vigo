# Phase 3/4/5 Integration Audit — 2026-04-03

Status: ✅ Completed

Objective: verify CSS (Phase 3), Layout (Phase 4), and Render/Compositor (Phase 5) are not only individually green, but also correctly connected in runtime behavior.

---

## Findings and fixes

### 1) Dynamic style correctness gap (Phase 3 ↔ 4)
**Issue:** `Tab::relayout` reused stale computed styles and did not recompute from source stylesheets, so dynamic pseudo-class state (`:focus`, `:checked`) and viewport-sensitive style changes could lag.

**Fixes:**
- `Tab` now stores parsed stylesheets (`stylesheets: Vec<Stylesheet>`).
- `Tab::relayout` now recomputes styles with `compute_styles(&doc, &self.stylesheets, viewport)` before reflow/layout.
- `Tab::relayout` now uses `reflow_document` with `ReflowPlan` support.

### 2) DOM state propagation gap for interaction pseudo-classes
**Issue:** focus and checked states were not consistently projected into `ElementState` flags, weakening selector-state behavior and form visual fidelity.

**Fixes (event pipeline):**
- `process_focus_change` now updates `ElementState::FOCUS` + `FOCUS_WITHIN` chains.
- `process_click` now includes default checkbox/radio click behavior and updates `ElementState::CHECKED`.
- input/change events are emitted for default control toggles.

### 3) Runtime relayout trigger gap (focus/form state)
**Issue:** click/focus state changes could alter style but not always trigger relayout/repaint.

**Fixes:**
- content click handling now marks target nodes dirty for reflow and relayouts when state-changing clicks occur.
- key-driven input edits now mark focused node dirty before relayout and refresh content height bounds.

### 4) Smooth scrolling connection gap (Phase 5 UX path)
**Issue:** app loop still used immediate clamped scrolling and never ticked smooth/kinetic scroll state.

**Fixes:**
- mouse wheel path now uses `scroll_by_smooth`.
- main loop ticks scroll animation each frame via `tab.tick_scroll(dt)`.

### 5) Compositor/damage/tile subsystems not exercised by host loop
**Issue:** newly added Phase-5 modules existed but were not integrated into frame orchestration.

**Fixes:**
- app render loop now computes per-frame damage (`compute_damage`), marks dirty tiles (`TileGrid`), and derives layer stats (`build_layers` + `cull_fully_occluded`).
- diagnostics are emitted alongside FPS telemetry.

### 6) API connectivity fix
- exported `build_display_list_with_images` at `vex-render` crate root for browser pipeline usage.

---

## Additional quality fixes during strict-gate pass
- Resolved strict clippy blockers in dependency paths encountered during connected-stack checks:
  - `vex-js/src/api/extensions.rs` (`cloned_ref_to_slice_refs`)
  - `vex-browser/src/notifications.rs` test initializers (`field_reassign_with_default`)

---

## Validation

All gates passed after integration fixes:

- ✅ `cargo test -p vex-browser -p vex-app -p vex-css -p vex-layout -p vex-render`
- ✅ `cargo clippy -p vex-browser -p vex-app -p vex-css -p vex-layout -p vex-render --all-targets -- -D warnings`
- ✅ `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-browser -p vex-app`
- ✅ `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-browser -p vex-app --all-targets -- -D warnings`

---

## Exit statement

The Phase 3/4/5 pipeline is now validated as connected end-to-end:

- style state updates ->
- reflow/layout update ->
- display list regeneration ->
- render/compositor diagnostics + smooth-scroll runtime behavior.
