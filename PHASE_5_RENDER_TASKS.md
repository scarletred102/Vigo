# Phase 5 — Graphics, Paint & Compositing (Execution + Hardening)

Status: ✅ **Completed**
Date: 2026-04-02
Scope: `crates/vex-render`

---

## Browser-reference audit (what mature browsers already do)

From Chrome/Blink, Firefox/Gecko/WebRender, and Safari/WebKit compositing pipelines, common baseline patterns include:

1. **Layerization heuristics** for expensive/animated content (opacity, transforms, positioned overlays, filters).
2. **Damage tracking + partial repaint** to avoid full-frame redraw.
3. **Tile-based raster/compositing** for large surfaces and scroll-heavy workloads.
4. **Dedicated textured image pipelines** and atlas-backed text/image GPU upload paths.
5. **Clip/opacity stack correctness** through paint and instance extraction.
6. **Smooth/inertial scrolling behavior** users expect from modern browsers.

User-demand gaps often reported in browser UX:
- smoother scrolling behavior,
- more stable rendering under large pages,
- better failure behavior (less hard-crash/panic style paths),
- improved visual correctness (stacking, borders, forms).

---

## Implemented Phase-5 hardening

### A) Display list intelligence
- `DisplayCommand` / `DisplayList` now support `PartialEq` for diff-oriented flows.
- Added command geometry extraction: `command_bounds`.
- Added list-level geometry: `DisplayList::bounds()`.
- Added command telemetry: `DisplayListStats` + `DisplayList::stats()`.

### B) Compositor + tiling subsystems
- Added `compositor.rs`:
  - `build_layers(...)` layer promotion heuristics,
  - `LayerReason` classification,
  - `cull_fully_occluded(...)` opaque-coverage culling.
- Added `tiling.rs`:
  - `TileGrid` fixed-size tile decomposition,
  - dirty tile marking and clearing,
  - viewport-visible and dirty-visible tile selection.

### C) Damage tracking
- Added `damage.rs`:
  - `compute_damage(previous, next, viewport)`,
  - `merge_damage(...)` overlap/touch compaction.

### D) Renderer pipeline upgrades
- Added **GPU image pipeline** for `DrawImage`:
  - image instance stream + textured quad pass,
  - image atlas texture upload synchronization.
- Added renderer image APIs:
  - `upload_image(ImageId, DecodedImage)`,
  - `upload_image_bytes(ImageId, &[u8])`,
  - `cached_image_count()`.
- Improved per-command extraction correctness:
  - clip-stack aware rect/text/image collection,
  - opacity-stack aware rect/text/image alpha.
- Improved border fidelity:
  - dashed/dotted borders segmented instead of always solid rectangle fallback.

### E) Painter/runtime fidelity
- Added z-index-aware sibling paint ordering in painter traversal.
- Integrated form control painting (`form_painter`) into main display-list build flow.

### F) UX improvements users directly feel
- Enhanced `ScrollState` with smooth + kinetic model:
  - targets, velocity, friction,
  - `scroll_by_smooth`, `fling`, `tick`, `is_animating`.
- Added fallible screenshot path:
  - `try_render_to_pixels(...) -> Result<Vec<u8>, String>`
  - reduced hard-panic behavior in adapter/device/map failures.

---

## Files changed (Phase 5)

- `crates/vex-render/src/display_list.rs`
- `crates/vex-render/src/renderer.rs`
- `crates/vex-render/src/shaders/image.wgsl`
- `crates/vex-render/src/painter.rs`
- `crates/vex-render/src/scroll.rs`
- `crates/vex-render/src/screenshot.rs`
- `crates/vex-render/src/lib.rs`
- `crates/vex-render/src/compositor.rs` (new)
- `crates/vex-render/src/damage.rs` (new)
- `crates/vex-render/src/tiling.rs` (new)
- `PLAN.md`

---

## Validation

All strict gates passed:

- ✅ `cargo test -p vex-render`
- ✅ `cargo clippy -p vex-render --all-targets -- -D warnings`
- ✅ Cross-phase regression:
  - `cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render`
  - `cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render --all-targets -- -D warnings`

---

## Exit decision

Phase 5 is complete for current scope and now includes practical modern-browser rendering architecture components:
- compositing layerization,
- damage tracking,
- tile scheduling,
- image GPU pipeline,
- improved paint correctness,
- user-facing smooth scrolling behavior.

Next phase candidate: **Phase 6 (JavaScript Execution)**.
