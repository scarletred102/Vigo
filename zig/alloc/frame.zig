// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary
//
// Frame allocator — double-buffered arena for per-frame transient data.

const std = @import("std");
const Arena = @import("arena.zig").Arena;
const Allocator = std.mem.Allocator;

pub const FrameAllocator = struct {
    arenas: [2]Arena,
    current: u1 = 0,

    pub fn init(backing: Allocator, capacity: usize) !FrameAllocator {
        return .{
            .arenas = .{
                try Arena.init(backing, capacity),
                try Arena.init(backing, capacity),
            },
        };
    }

    pub fn deinit(self: *FrameAllocator) void {
        self.arenas[0].deinit();
        self.arenas[1].deinit();
    }

    /// Start a new frame — resets the current arena.
    pub fn beginFrame(self: *FrameAllocator) void {
        self.arenas[self.current].reset();
    }

    /// Swap buffers.
    pub fn endFrame(self: *FrameAllocator) void {
        self.current ^= 1;
    }

    /// Allocator for this frame's transient data.
    pub fn allocator(self: *FrameAllocator) Allocator {
        return self.arenas[self.current].allocator();
    }
};

// ── Tests ─────────────────────────────────────────────────────────

const testing = std.testing;

test "frame allocator: double buffer" {
    var fa = try FrameAllocator.init(testing.allocator, 1024);
    defer fa.deinit();

    // Frame 1: allocate.
    fa.beginFrame();
    const a = try fa.allocator().alloc(u8, 64);
    a[0] = 42;
    fa.endFrame();

    // Frame 2: previous frame data still accessible (in the other arena).
    fa.beginFrame();
    try testing.expectEqual(@as(u8, 42), a[0]);
    fa.endFrame();

    // Frame 3: that arena gets reset.
    fa.beginFrame();
    // 'a' is now invalid — arena was reset. We just verify no crash.
    fa.endFrame();
}
