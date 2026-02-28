const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ── Static libraries ──────────────────────────────────────────

    const platform_lib = b.addStaticLibrary(.{
        .name = "vex_platform",
        .root_source_file = b.path("platform/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    platform_lib.linkLibC();
    b.installArtifact(platform_lib);

    const compositor_lib = b.addStaticLibrary(.{
        .name = "vex_compositor",
        .root_source_file = b.path("compositor/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    compositor_lib.linkLibC();
    b.installArtifact(compositor_lib);

    const media_lib = b.addStaticLibrary(.{
        .name = "vex_media_zig",
        .root_source_file = b.path("media/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    media_lib.linkLibC();
    b.installArtifact(media_lib);

    const text_lib = b.addStaticLibrary(.{
        .name = "vex_text",
        .root_source_file = b.path("text/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    text_lib.linkLibC();
    b.installArtifact(text_lib);

    const alloc_lib = b.addStaticLibrary(.{
        .name = "vex_alloc",
        .root_source_file = b.path("alloc/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    alloc_lib.linkLibC();
    b.installArtifact(alloc_lib);

    // ── Tests ─────────────────────────────────────────────────────

    const test_step = b.step("test", "Run all Zig unit tests");

    inline for (.{
        .{ "platform/root.zig", "platform-tests" },
        .{ "compositor/root.zig", "compositor-tests" },
        .{ "media/root.zig", "media-tests" },
        .{ "text/root.zig", "text-tests" },
        .{ "alloc/root.zig", "alloc-tests" },
    }) |entry| {
        const unit_tests = b.addTest(.{
            .root_source_file = b.path(entry[0]),
            .target = target,
            .optimize = optimize,
        });
        unit_tests.linkLibC();
        const run_unit_tests = b.addRunArtifact(unit_tests);
        test_step.dependOn(&run_unit_tests.step);
    }
}
