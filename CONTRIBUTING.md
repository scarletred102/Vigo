# Contributing to Vigo

Thanks for helping build Vigo and the Vex engine. The project is pre-release software; changes should prioritize correctness, security, and reproducible validation over feature count.

## Before you start

- Use a focused branch based on `main`.
- Keep Rust code formatted with `cargo fmt --all`.
- Run the relevant tests, then `cargo test --workspace` for cross-crate changes.
- Run `zig build test` from `zig/` when changing Zig code or Rust/Zig FFI.
- Do not add credentials, profile data, build artifacts, or downloaded fixtures to a commit.

## Pull requests

Explain the behavior change, the affected crates, and the validation you ran. Add regression coverage for a bug fix whenever the behavior can be tested deterministically.

Small, independently reviewable pull requests are preferred. Avoid drive-by refactors mixed with a behavioral change unless the refactor is necessary for the fix.

## Code conventions

- Preserve the existing copyright and SPDX header in Rust and Zig source files.
- Use explicit error handling on network, file-system, and FFI boundaries.
- Keep platform-specific code behind the relevant `cfg` or Zig platform boundary.
- Treat privacy, security, and compatibility regressions as release blockers.

## Reporting a security issue

Please follow [SECURITY.md](SECURITY.md) instead of opening a public issue for a suspected vulnerability.

## Community expectations

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
