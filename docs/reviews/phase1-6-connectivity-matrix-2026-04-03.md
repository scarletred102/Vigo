# Phase 1-6 Connectivity Matrix (2026-04-03)

This matrix verifies intended connections between network/parser/style/layout/render and JS runtime paths.

| Connection | Intended flow | Verified implementation state |
|---|---|---|
| Phase 1 -> Phase 6 | Network stack supports JS-driven fetch/script retrieval | ✅ `vex-js::fetch` uses `vex-net::HttpClient`; tab resource fetch paths integrated |
| Phase 2 -> Phase 6 | JS APIs mutate canonical shared DOM | ✅ `SharedDocument` bridge used by `document`/`element` APIs |
| Phase 6 -> Phase 3 | JS DOM/style mutations trigger style recompute | ✅ runtime dirty-node drain + relayout style recomputation path |
| Phase 6 -> Phase 4 | JS changes invalidate layout and trigger reflow | ✅ dirty nodes mark reflow plan and relayout in app loop |
| Phase 6 -> Phase 5 | Updated layout regenerates display list and render input | ✅ relayout -> display list -> renderer/compositor diagnostics |
| Phase 6 API Surface | Browser-like frame/timer APIs available | ✅ rAF/cancelAnimationFrame added; `window.*` parity improved |
| Script lifecycle | async/defer/blocking behavior reasonably sequenced | ✅ deferred -> DOMContentLoaded -> async -> load sequencing implemented |

## Validation commands

```powershell
cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app
cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings
```

Both passed during this audit pass.
