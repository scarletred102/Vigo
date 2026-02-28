// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0
//
// stats.zig — Allocator wrapper that tracks allocation statistics.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Allocation statistics.
pub const Stats = struct {
    total_allocated: usize,
    total_freed: usize,
    current_usage: usize,
    peak_usage: usize,
    allocation_count: usize,
    free_count: usize,
};

/// A wrapper allocator that tracks allocation statistics.
/// Delegates all actual allocations to an inner allocator.
pub const StatsAllocator = struct {
    inner: Allocator,
    stats: Stats,

    pub fn init(inner: Allocator) StatsAllocator {
        return .{
            .inner = inner,
            .stats = .{
                .total_allocated = 0,
                .total_freed = 0,
                .current_usage = 0,
                .peak_usage = 0,
                .allocation_count = 0,
                .free_count = 0,
            },
        };
    }

    pub fn getStats(self: *const StatsAllocator) Stats {
        return self.stats;
    }

    pub fn allocator(self: *StatsAllocator) Allocator {
        return .{
            .ptr = @ptrCast(self),
            .vtable = &vtable,
        };
    }

    const vtable: Allocator.VTable = .{
        .alloc = statsAlloc,
        .resize = statsResize,
        .free = statsFree,
        .remap = noRemap,
    };

    fn noRemap(
        _: *anyopaque,
        _: []u8,
        _: std.mem.Alignment,
        _: usize,
        _: usize,
    ) ?[*]u8 {
        return null;
    }

    fn statsAlloc(
        ctx: *anyopaque,
        len: usize,
        alignment: std.mem.Alignment,
        ret_addr: usize,
    ) ?[*]u8 {
        const self: *StatsAllocator = @ptrCast(@alignCast(ctx));
        const result = self.inner.rawAlloc(len, alignment, ret_addr);
        if (result != null) {
            self.stats.total_allocated += len;
            self.stats.current_usage += len;
            self.stats.allocation_count += 1;
            if (self.stats.current_usage > self.stats.peak_usage) {
                self.stats.peak_usage = self.stats.current_usage;
            }
        }
        return result;
    }

    fn statsResize(
        ctx: *anyopaque,
        buf: []u8,
        alignment: std.mem.Alignment,
        new_len: usize,
        ret_addr: usize,
    ) bool {
        const self: *StatsAllocator = @ptrCast(@alignCast(ctx));
        const old_len = buf.len;
        if (self.inner.rawResize(buf, alignment, new_len, ret_addr)) {
            if (new_len > old_len) {
                const delta = new_len - old_len;
                self.stats.total_allocated += delta;
                self.stats.current_usage += delta;
            } else {
                const delta = old_len - new_len;
                self.stats.total_freed += delta;
                self.stats.current_usage -= delta;
            }
            if (self.stats.current_usage > self.stats.peak_usage) {
                self.stats.peak_usage = self.stats.current_usage;
            }
            return true;
        }
        return false;
    }

    fn statsFree(
        ctx: *anyopaque,
        buf: []u8,
        alignment: std.mem.Alignment,
        ret_addr: usize,
    ) void {
        const self: *StatsAllocator = @ptrCast(@alignCast(ctx));
        self.stats.total_freed += buf.len;
        self.stats.current_usage -= buf.len;
        self.stats.free_count += 1;
        self.inner.rawFree(buf, alignment, ret_addr);
    }
};

// ── Tests ─────────────────────────────────────────────────────────

test "stats_track_alloc_and_free" {
    var sa = StatsAllocator.init(std.testing.allocator);
    var alloc = sa.allocator();

    const slice = try alloc.alloc(u8, 100);
    const s1 = sa.getStats();
    try std.testing.expectEqual(s1.total_allocated, 100);
    try std.testing.expectEqual(s1.current_usage, 100);
    try std.testing.expectEqual(s1.allocation_count, 1);

    alloc.free(slice);
    const s2 = sa.getStats();
    try std.testing.expectEqual(s2.total_freed, 100);
    try std.testing.expectEqual(s2.current_usage, 0);
    try std.testing.expectEqual(s2.free_count, 1);
    try std.testing.expectEqual(s2.peak_usage, 100);
}

test "stats_peak_usage" {
    var sa = StatsAllocator.init(std.testing.allocator);
    var alloc = sa.allocator();

    const a = try alloc.alloc(u8, 50);
    const b = try alloc.alloc(u8, 75);
    // peak should be 125
    alloc.free(a);
    // current = 75, peak still 125
    const stats = sa.getStats();
    try std.testing.expectEqual(stats.peak_usage, 125);
    try std.testing.expectEqual(stats.current_usage, 75);

    alloc.free(b);
}
