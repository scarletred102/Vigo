# Zig Coding Standards — Vex Engine

## File Header

Every `.zig` file must start with:

```zig
// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
```

## Exported Functions

- All functions exported to Rust must use C calling convention: `callconv(.C)`
- Use `export fn` for C ABI exports.
- Naming: `vex_{module}_{function}` (e.g., `vex_platform_create_window`).

## Error Returns

- C ABI functions return `c_int` (aliased as `VexResult`).
- `0` = success.
- Negative values = error codes (document each).
- Struct/pointer outputs via out-parameters.

## Memory Ownership

- **Caller allocates**: caller provides buffer + size, callee fills it.
- **Callee allocates**: callee returns pointer, caller must call matching `_free` function.
- Document ownership in function doc comment.
- Never leak — every `create` has a matching `destroy`.

## Allocators

- Always use explicit allocators. No hidden allocations.
- Use the custom allocators in `zig/alloc/` for engine work.
- System allocator only for initialization and tests.

## Naming

- Functions: `snake_case`
- Types: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE` or `snake_case` (Zig convention)
- Files: `snake_case.zig`
- C-exported names: `vex_{module}_{action}`

## Testing

- All test functions: `test "description of test"` format.
- Use `std.testing.expectEqual`, `std.testing.expect`, etc.
- Tests run via `zig build test`.

## C Interop Structs

- Use `extern struct` with explicit field layout for C ABI.
- Match exact field types with Rust `#[repr(C)]` counterparts.
- Document byte layout in comments.

## Platform Abstraction

- Use `@import("builtin").os.tag` for platform branching.
- Unimplemented platforms: `@compileError("not implemented for this platform")`.
- Windows-first, then macOS, then Linux.
