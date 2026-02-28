// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// vex_alloc — Custom allocators: arena, pool, frame.
// Exports C ABI for Rust FFI consumption.

const std = @import("std");
pub const arena = @import("arena.zig");
pub const pool = @import("pool.zig");
pub const frame = @import("frame.zig");
pub const stats = @import("stats.zig");

const Arena = arena.Arena;

// ── C ABI Exports ─────────────────────────────────────────────────

export fn vex_arena_create(capacity: usize) ?*anyopaque {
    const backing = std.heap.page_allocator;
    const a = backing.create(Arena) catch return null;
    a.* = Arena.init(backing, capacity) catch {
        backing.destroy(a);
        return null;
    };
    return @ptrCast(a);
}

export fn vex_arena_alloc(handle: *anyopaque, len: usize) ?[*]u8 {
    const a: *Arena = @ptrCast(@alignCast(handle));
    var alloc = a.allocator();
    return (alloc.alloc(u8, len) catch return null).ptr;
}

export fn vex_arena_reset(handle: *anyopaque) void {
    const a: *Arena = @ptrCast(@alignCast(handle));
    a.reset();
}

export fn vex_arena_destroy(handle: *anyopaque) void {
    const a: *Arena = @ptrCast(@alignCast(handle));
    const backing = a.backing;
    a.deinit();
    backing.destroy(a);
}

// ── Tests ─────────────────────────────────────────────────────────

test "arena module" {
    _ = arena;
}

test "pool module" {
    _ = pool;
}

test "frame module" {
    _ = frame;
}

test "stats module" {
    _ = stats;
}
