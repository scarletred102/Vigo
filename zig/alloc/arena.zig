// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// Arena allocator — bump-pointer with bulk reset.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Arena = struct {
    buf: []u8,
    offset: usize = 0,
    backing: Allocator,

    pub fn init(backing: Allocator, capacity: usize) !Arena {
        const buf = try backing.alloc(u8, capacity);
        return .{ .buf = buf, .backing = backing };
    }

    pub fn deinit(self: *Arena) void {
        self.backing.free(self.buf);
        self.* = undefined;
    }

    /// Reclaim all memory at once (no individual frees).
    pub fn reset(self: *Arena) void {
        self.offset = 0;
    }

    pub fn usable(self: *const Arena) usize {
        return self.buf.len - self.offset;
    }

    pub fn allocator(self: *Arena) Allocator {
        return .{
            .ptr = @ptrCast(self),
            .vtable = &vtable,
        };
    }

    const vtable = Allocator.VTable{
        .alloc = arenaAlloc,
        .resize = Allocator.noResize,
        .remap = Allocator.noRemap,
        .free = Allocator.noFree,
    };

    fn arenaAlloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, _: usize) ?[*]u8 {
        const self: *Arena = @ptrCast(@alignCast(ctx));
        const align_val = alignment.toByteUnits();
        const aligned = std.mem.alignForward(usize, self.offset, align_val);
        if (aligned + len > self.buf.len) return null;
        const ptr = self.buf.ptr + aligned;
        self.offset = aligned + len;
        return ptr;
    }
};

// ── Tests ─────────────────────────────────────────────────────────

const testing = std.testing;

test "arena: alloc and reset" {
    var arena = try Arena.init(testing.allocator, 4096);
    defer arena.deinit();

    const alloc = arena.allocator();
    const slice = try alloc.alloc(u8, 100);
    try testing.expectEqual(@as(usize, 100), slice.len);
    try testing.expect(arena.usable() < 4096);

    arena.reset();
    try testing.expectEqual(@as(usize, 4096), arena.usable());
}

test "arena: exhaustion returns error" {
    var arena = try Arena.init(testing.allocator, 64);
    defer arena.deinit();

    const alloc = arena.allocator();
    const result = alloc.alloc(u8, 128);
    try testing.expect(if (result) |_| false else |_| true);
}
