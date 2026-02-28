# FFI Conventions — Rust↔Zig Interop

## Overview

Zig code compiles into static libraries. Rust calls Zig via `extern "C"`.
All shared types use C ABI layout.

## Naming Convention

All exported functions follow: `vex_{module}_{action}`

```
vex_platform_init()
vex_platform_create_window(config)
vex_compositor_submit_frame(display_list)
vex_media_decode_packet(ctx, packet)
vex_alloc_arena_create(size)
```

## Return Convention

```c
// Success: return 0
// Error: return negative error code
typedef int VexResult;

#define VEX_OK              0
#define VEX_ERR_INVALID    -1
#define VEX_ERR_OOM        -2
#define VEX_ERR_IO         -3
#define VEX_ERR_PLATFORM   -4
#define VEX_ERR_UNSUPPORTED -5
```

## Struct Layout

Shared structs use `extern struct` in Zig and `#[repr(C)]` in Rust.

```zig
// Zig side
pub const WindowConfig = extern struct {
    title: [*:0]const u16,  // null-terminated UTF-16 (Win32 wide string)
    width: u32,
    height: u32,
    resizable: u8,  // 0 = false, 1 = true
};
```

```rust
// Rust side
#[repr(C)]
pub struct WindowConfigC {
    pub title: *const u16,  // null-terminated UTF-16 pointer
    pub width: u32,
    pub height: u32,
    pub resizable: u8,
}
```

## Memory Ownership Rules

### Rule 1: Caller Allocates

Caller provides a buffer pointer + size. Callee writes into it.
Used for: event polling, small struct outputs.

```zig
export fn vex_platform_poll_event(out_event: *EventC) callconv(.C) bool
```

### Rule 2: Callee Allocates

Callee allocates and returns a pointer. Caller must call the matching `_destroy` function.
Used for: window handles, contexts, large buffers.

```zig
export fn vex_platform_create_window(config: *const WindowConfigC) callconv(.C) ?*anyopaque
export fn vex_platform_destroy_window(handle: *anyopaque) callconv(.C) void
```

### Rule 3: Never Mix Allocators

- Zig-allocated memory: freed by Zig `_destroy`/`_free` functions.
- Rust-allocated memory: freed by Rust `Drop`.
- Never free Zig memory from Rust or vice versa.

## String Convention

- Strings passed Zig→Rust: null-terminated `[*:0]const u8` / `*const c_char`.
- Strings passed Rust→Zig (Win32 wide): null-terminated UTF-16 `[*:0]const u16` / `*const u16`.
- Strings passed Rust→Zig (general): null-terminated `CString` pointer.
- Lifetime: valid only for duration of the function call unless documented otherwise.

## Build Integration

Zig builds produce static libs in `zig/zig-out/lib/`.
Rust `build.rs` files link them:

```rust
// crates/vex-render/build.rs
fn main() {
    let zig_out = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/../zig/zig-out/lib", d))
        .unwrap();
    println!("cargo:rustc-link-search=native={}", zig_out);
    println!("cargo:rustc-link-lib=static=vex_platform");
    println!("cargo:rerun-if-changed=../../zig/platform/root.zig");
}
```
