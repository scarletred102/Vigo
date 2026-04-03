# Phase 5 Browser Gap Matrix (2026-04-02)

This matrix captures Phase-5 rendering features commonly present in major engines and maps Vigo's delivered hardening work.

| Capability Area | Chrome / Firefox / Safari baseline | Vigo before hardening | Vigo after Phase 5 hardening |
|---|---|---|---|
| Layer promotion heuristics | Yes (opacity/transform/position/filter/animation driven) | Minimal implicit behavior | ✅ `build_layers` + `LayerReason` heuristics |
| Occlusion culling | Yes | None | ✅ `cull_fully_occluded` opaque coverage cull |
| Damage tracking | Yes | None | ✅ `compute_damage` + `merge_damage` |
| Tile-based scheduling | Yes | None | ✅ `TileGrid` dirty/visible prioritization |
| Image textured GPU pass | Yes | `DrawImage` command present, renderer path incomplete | ✅ image pipeline + image atlas upload + instances |
| Clip stack correctness | Yes | Commands existed, extraction ignored clip for many paths | ✅ clip-aware extraction for rect/text/image |
| Opacity stack correctness | Yes | Partial (rect path) | ✅ rect + text + image opacity stack handling |
| Border style fidelity | Yes | Solid-like fallback for many borders | ✅ dashed/dotted segment emission |
| Stacking order behavior | Yes | Mostly tree-order paint | ✅ z-index-aware sibling ordering |
| Native-like form paint | Yes | Separate module existed, main pipeline not integrated | ✅ form painter integrated in main painter walk |
| Smooth/inertial scrolling UX | Yes | Basic clamped scrolling only | ✅ smooth target + fling + tick + animation state |
| Crash/failure resilience for offscreen render | Yes (fallible internals) | panic-driven paths in screenshot flow | ✅ `try_render_to_pixels` fallible API |

## Notes
- This is a practical engineering matrix for Vigo's current architecture and does not claim byte-identical behavior to any single browser engine.
- Follow-up opportunities for future phases:
  - GPU clip-stack/scissor batching in render pass,
  - full retained layer trees with async raster workers,
  - advanced text effects (subpixel AA policy, LCD fallback policies),
  - picture caching and render-pass dependency graphs.
