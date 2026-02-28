// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// Pool allocator — fixed-size block allocation with free list.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub fn Pool(comptime block_size: usize) type {
    return struct {
        const Self = @This();
        const BLOCK = if (block_size < @sizeOf(usize)) @sizeOf(usize) else block_size;

        buf: []u8,
        free_head: ?usize = null,
        count: usize,
        in_use: usize = 0,
        backing: Allocator,

        pub fn init(backing: Allocator, block_count: usize) !Self {
            const total = BLOCK * block_count;
            const buf = try backing.alloc(u8, total);

            // Build free list through the blocks.
            var head: ?usize = null;
            var i: usize = block_count;
            while (i > 0) {
                i -= 1;
                const offset = i * BLOCK;
                const ptr: *usize = @ptrCast(@alignCast(buf.ptr + offset));
                ptr.* = head orelse std.math.maxInt(usize);
                head = offset;
            }

            return .{ .buf = buf, .free_head = head, .count = block_count, .backing = backing };
        }

        pub fn deinit(self: *Self) void {
            self.backing.free(self.buf);
            self.* = undefined;
        }

        pub fn alloc(self: *Self) ?[*]u8 {
            const offset = self.free_head orelse return null;
            const ptr: *usize = @ptrCast(@alignCast(self.buf.ptr + offset));
            const next = ptr.*;
            self.free_head = if (next == std.math.maxInt(usize)) null else next;
            self.in_use += 1;
            return self.buf.ptr + offset;
        }

        pub fn free(self: *Self, ptr: [*]u8) void {
            const offset = @intFromPtr(ptr) - @intFromPtr(self.buf.ptr);
            const slot: *usize = @ptrCast(@alignCast(ptr));
            slot.* = self.free_head orelse std.math.maxInt(usize);
            self.free_head = offset;
            self.in_use -= 1;
        }
    };
}

// ── Tests ─────────────────────────────────────────────────────────

const testing = std.testing;

test "pool: alloc and free" {
    var pool = try Pool(64).init(testing.allocator, 4);
    defer pool.deinit();

    const a = pool.alloc().?;
    const b = pool.alloc().?;
    try testing.expectEqual(@as(usize, 2), pool.in_use);

    pool.free(a);
    try testing.expectEqual(@as(usize, 1), pool.in_use);

    // Re-alloc reuses a's slot.
    const c = pool.alloc().?;
    try testing.expectEqual(a, c);
    pool.free(b);
    pool.free(c);
}

test "pool: exhaustion" {
    var pool = try Pool(32).init(testing.allocator, 2);
    defer pool.deinit();

    _ = pool.alloc();
    _ = pool.alloc();
    try testing.expectEqual(@as(?[*]u8, null), pool.alloc());
}
