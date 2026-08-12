// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});

    const target = b.standardTargetOptions(.{});

    // ── Static libraries ──────────────────────────────────────────

    const lib_configs = .{
        .{ "vex_platform", "platform/root.zig" },
        .{ "vex_compositor", "compositor/root.zig" },
        .{ "vex_media_zig", "media/root.zig" },
        .{ "vex_text", "text/root.zig" },
        .{ "vex_alloc", "alloc/root.zig" },
    };

    inline for (lib_configs) |cfg| {
        const lib = b.addLibrary(.{
            .linkage = .static,
            .name = cfg[0],
            .root_module = b.createModule(.{
                .root_source_file = b.path(cfg[1]),
                .target = target,
                .optimize = optimize,
                .link_libc = true,
                .stack_protector = false,
                .stack_check = false,
            }),
        });
        b.installArtifact(lib);
    }

    // ── Tests ─────────────────────────────────────────────────────

    const test_step = b.step("test", "Run all Zig unit tests");

    inline for (lib_configs) |cfg| {
        const unit_tests = b.addTest(.{
            .root_module = b.createModule(.{
                .root_source_file = b.path(cfg[1]),
                .target = target,
                .optimize = optimize,
                .link_libc = true,
                .stack_protector = false,
                .stack_check = false,
            }),
        });
        const run_unit_tests = b.addRunArtifact(unit_tests);
        test_step.dependOn(&run_unit_tests.step);
    }
}
