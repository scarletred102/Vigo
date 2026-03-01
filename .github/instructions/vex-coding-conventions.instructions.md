---
description: "Use when writing any Rust or Zig code for the Vex engine. Enforces the project's core coding philosophy: clean, readable, reusable, not over-engineered. Covers style, safety, testing, and Clippy."
applyTo: "crates/**/*.rs, zig/**/*.zig"
---

# Vex Engine — Coding Conventions

Core directive: **clean, readable, and reusable code — do not over-engineer.**

## Philosophy

- Solve the actual problem. Do not add abstractions "for future flexibility" unless a concrete use case exists today.
- Prefer flat structures over deep hierarchies.
- Prefer small, focused functions over large multi-purpose ones.
- If a feature is not in the current phase plan (PLAN.md / TASKS.md), do not implement it.

## Hard Rules

- **Zero Clippy warnings** — `cargo clippy --workspace -- -D warnings` must pass clean.
- **Zero `unwrap()` / `expect()` in library code** — use `?` or explicit error handling. Test code may use `.unwrap()`.
- **No `println!` / `eprintln!` in library crates** — use `tracing::info!` / `debug!` / `warn!` / `error!`.
- **Every `unsafe` block needs a `// SAFETY:` comment** explaining why it is sound.

## File Headers

Every `.rs` and `.zig` source file must begin with:

```
// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
```

## Error Handling (Rust)

- Library crates: `thiserror` for error enums.
- Binary crate (`vex-app`): `anyhow` is allowed.
- Return `Result<T, VexError>` (or a crate-local error type) — never panic.

## Naming (Rust)

| Item | Convention |
|------|-----------|
| Types / Traits / Enums | `PascalCase` |
| Functions / methods / fields | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| Crate names | `vex-{name}` (kebab-case) |

## Documentation (Rust)

- All public items: `///` doc comments.
- Every `lib.rs` / `mod.rs`: `//!` module-level doc comment.
- Add `# Examples` for non-obvious APIs.

## Testing

- Unit tests in `#[cfg(test)] mod tests` at the bottom of each file.
- Integration tests in `tests/` directory.
- Every phase deliverable must have test coverage — **no untested public functions**.
- Name tests descriptively: `test_{what}_{condition}_{expected}` or plain English.
- Network / filesystem tests that are non-deterministic: mark `#[ignore]`.

## Dependencies

- All deps declared in `[workspace.dependencies]`, inherited via `dep.workspace = true`.
- No wildcard versions. Pin to minimum major version.
- Do not add a new crate dependency without confirming it does not duplicate an existing workspace dep.

## Style

- `cargo fmt` enforced in CI (line width 100, default rustfmt settings).
- Imports: std → external → internal, separated by blank lines.
- `#[must_use]` on functions returning `Result` or important values.
- `#[inline]` only on hot-path functions (≤5 lines, called per-frame).
