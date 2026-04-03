# Phase 1-9 Connectivity Matrix (2026-04-03)

This matrix verifies end-to-end wiring from network and parser/runtime layers through security/privacy filters and process-isolation orchestration.

| Connection | Intended flow | Verified implementation state |
|---|---|---|
| Phase 1 -> Phase 9 privacy edge | Outbound requests are filtered before network fetch | ✅ top-level navigation uses privacy middleware via `fetch_filtered` |
| Phase 1 -> Phase 9 security edge | Subresource loads enforce CSP/CORS | ✅ script/style prefetch uses `secure_fetch` + `SecurityContext` |
| Phase 1 -> Phase 8/9 cookie continuity | Response cookie headers persist and remain JS-visible | ✅ `Set-Cookie` persisted in tab load path; `document.cookie` remains wired |
| Phase 2/6/7 -> Phase 9 | DOM/JS-driven resource fetch paths are policy-enforced | ✅ external script/style resource requests pass through security checks |
| Phase 8 -> Phase 9 | Storage/media integrations remain stable under security/privacy controls | ✅ phase8 and phase9 full-stack tests pass together |
| Phase 9 process bridge | Tab lifecycle maps to renderer process lifecycle + IPC updates | ✅ app loop synchronizes spawn/terminate/sandbox and IPC `LoadUrl` routing |

## Validation commands

```powershell
cargo test -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media -p vex-security -p vex-privacy
cargo clippy -p vex-net -p vex-html -p vex-dom -p vex-css -p vex-layout -p vex-render -p vex-js -p vex-browser -p vex-app -p vex-storage -p vex-media -p vex-security -p vex-privacy --all-targets -- -D warnings
```

Both passed during this Phase 9 completion pass.
