# Phase 1-7 Connectivity Matrix (2026-04-03)

This matrix verifies intended data/control flow across network, DOM, style, layout, render/compositor, JS runtime, and interactivity bridge/event-loop semantics.

| Connection | Intended flow | Verified implementation state |
|---|---|---|
| Phase 1 -> Phase 6/7 | Network stack supports JS fetch + external script/resource retrieval | ✅ `vex-js::fetch` and tab resource/script loading use `vex-net` |
| Phase 2 -> Phase 6/7 | JS APIs mutate canonical shared DOM | ✅ `SharedDocument` is common source for JS APIs and engine pipelines |
| Phase 6/7 -> Phase 3 | JS mutations trigger style recomputation | ✅ dirty-node invalidation feeds relayout style recompute |
| Phase 6/7 -> Phase 4 | JS changes trigger reflow/layout updates | ✅ app loop marks dirty nodes and relayouts |
| Phase 6/7 -> Phase 5 | Updated layout regenerates display list and render/compositor input | ✅ display-list + renderer/compositor diagnostics update in frame loop |
| Phase 6/7 memory bindings | JS proxies tracked against DOM roots | ✅ runtime-owned `GcRootSet` roots document/proxy nodes |
| Phase 7 event loop | Timers, rAF, microtasks integrated in runtime | ✅ `setTimeout`/`setInterval`/rAF + `queueMicrotask` with runtime flushing |
| Phase 7 dispatch semantics | Microtasks flush after script/events/lifecycle/timers | ✅ integrated in `JsRuntime` execution and dispatch paths |

## Validation commands

```powershell
cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app
cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app --all-targets -- -D warnings
```

Both passed during this Phase 7 completion pass.
