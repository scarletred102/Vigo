# Rust Coding Standards — Vex Engine

## File Header

Every `.rs` file must start with:

```rust
// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
```

## Error Handling

- Use `thiserror` for error enums in library crates.
- Use `anyhow` only in `vex-app` (the binary crate).
- **Never** use `.unwrap()` or `.expect()` in library code. Use `?` or explicit error handling.
- Test code may use `.unwrap()`.

## Logging

- Use `tracing` macros (`tracing::info!`, `tracing::debug!`, `tracing::warn!`, `tracing::error!`).
- Never use `println!` or `eprintln!` in library code (only in `main.rs`).
- Debug-level logs for per-frame/per-event data.
- Info-level for lifecycle events (startup, shutdown, page load).
- Warn for recoverable oddities.
- Error for things that need attention.

## Documentation

- All public items must have `///` doc comments.
- Module-level `//!` doc comment in every `lib.rs` and `mod.rs`.
- Use `# Examples` sections for non-obvious APIs.

## Safety

- Every `unsafe` block must have a `// SAFETY:` comment explaining why it's sound.
- Minimize `unsafe` — prefer safe abstractions.
- All FFI (`extern "C"`) functions must have safety documentation.

## Annotations

- `#[must_use]` on functions returning `Result` or important values.
- `#[inline]` only on hot-path functions (<5 lines, called per-frame).
- `#[cfg(test)]` for test modules.

## Naming

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Crate names: `vex-{name}` (kebab-case)
- Module files: `snake_case.rs`

## Testing

- Unit tests in `#[cfg(test)] mod tests` at bottom of each file.
- Integration tests in `tests/` directory of each crate.
- Test function names: `test_{what}_{condition}_{expected}` or descriptive names.
- Use `assert_eq!`, `assert_ne!`, `assert!` with messages.

## Dependencies

- All deps declared in `[workspace.dependencies]` and inherited via `dep.workspace = true`.
- No wildcard versions. Pin to major version minimum.
- Prefer crates from the workspace over external when possible.

## Formatting

- `cargo fmt` enforced in CI.
- Line width: 100 chars (default rustfmt).
- Imports: group by std → external → internal, separated by blank lines.
