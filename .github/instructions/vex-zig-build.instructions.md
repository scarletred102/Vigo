---
description: "Use when writing or modifying Zig code, build.zig, or Rust FFI bindings. Covers Zig 0.15 build API, MSVC ABI targeting, stack protection settings, and Rust↔Zig interop conventions for the Vex engine."
applyTo: "zig/**"
---

# Vex Engine — Zig Build & FFI Conventions

## Target: Windows MSVC ABI (Required)

Always resolve the target with `.abi = .msvc`. Omitting this causes Zig to emit MinGW symbols (`___chkstk_ms`) that the MSVC linker cannot resolve.

```zig
const target = b.resolveTargetQuery(.{ .abi = .msvc });
```

## Stack Protection Must Be Disabled

Zig's stack-check symbols (`__stack_chk_fail`, etc.) are not present in MSVC's C runtime. Disable both flags on every Zig library that will be linked into a Rust binary:

```zig
lib.root_module.stack_protector = false;
lib.root_module.stack_check = false;
```

## Zig 0.15 Build API

Use the 0.15-era `addLibrary` API, **not** the old `addStaticLibrary`:

```zig
// ✅ Correct (Zig 0.15)
const lib = b.addLibrary(.{
    .name = "vex_platform",
    .linkage = .static,
    .root_module = b.createModule(.{
        .root_source_file = b.path("platform/root.zig"),
        .target = target,
        .optimize = optimize,
    }),
});

// ❌ Wrong (old API, removed in 0.15)
// const lib = b.addStaticLibrary(.{ ... });
```

## Exported Functions

- `export fn` implies C calling convention — **do not add `callconv(.C)`** redundantly.
- Naming: `vex_{module}_{action}` (e.g., `vex_platform_create_window`).
- Win32 callbacks that need `WINAPI` calling convention: use `callconv(.winapi)`, not `.C`.

```zig
// C ABI export
export fn vex_platform_init() c_int { ... }

// Win32 callback
fn windowProc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) callconv(.winapi) LRESULT { ... }
```

## Return Convention

All C-exported functions return `c_int`:
- `0` = success  
- Negative = error code  
- Struct / pointer outputs via `*Out` parameters  

```zig
pub const VEX_OK: c_int          =  0;
pub const VEX_ERR_INVALID: c_int = -1;
pub const VEX_ERR_OOM: c_int     = -2;
pub const VEX_ERR_IO: c_int      = -3;
pub const VEX_ERR_PLATFORM: c_int = -4;
```

## Shared Struct Layout

Use `extern struct` in Zig and `#[repr(C)]` in Rust. Field types must match exactly.

```zig
// Zig
pub const WindowConfig = extern struct {
    title: [*:0]const u16, // null-terminated UTF-16
    width: u32,
    height: u32,
    resizable: u8,         // 0 = false, 1 = true
};
```

```rust
// Rust
#[repr(C)]
pub struct WindowConfigC {
    pub title:     *const u16,
    pub width:     u32,
    pub height:    u32,
    pub resizable: u8,
}
```

## Memory Ownership

- **Caller allocates**: caller provides buffer + size, callee fills it. Used for small struct outputs.  
- **Callee allocates**: callee returns pointer, caller calls matching `_destroy`. Used for handles.  
- Every `create` has a matching `destroy` — never leak.

## Allocators

- Always use explicit allocators. No hidden allocations.
- Use custom allocators from `zig/alloc/` for engine work.
- System allocator only for initialization and tests.

## Rust-Side `extern "C"` Bindings

```rust
#[link(name = "vex_platform", kind = "static")]
extern "C" {
    // SAFETY: single-threaded during init; config pointer valid for call duration
    pub fn vex_platform_init() -> c_int;
}
```

Always include a `// SAFETY:` comment on `extern "C"` call sites.
