// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Build script — tells Cargo where to find Zig static libraries.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = PathBuf::from(&manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("could not find workspace root")
        .to_path_buf();

    let zig_lib_dir = workspace_root.join("zig").join("zig-out").join("lib");

    if zig_lib_dir.exists() {
        println!(
            "cargo:rustc-link-search=native={}",
            zig_lib_dir.display()
        );
        println!("cargo:rustc-link-lib=static=vex_platform");
        println!("cargo:rustc-link-lib=static=vex_compositor");
        println!("cargo:rustc-link-lib=static=vex_alloc");

        // Windows system libs required by the Zig platform layer.
        #[cfg(target_os = "windows")]
        {
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=kernel32");
        }
    } else {
        println!("cargo:warning=Zig libraries not found at {zig_lib_dir:?}. Run `cd zig && zig build` first.");
    }

    // Re-run if Zig sources change.
    println!("cargo:rerun-if-changed=../../zig/platform/root.zig");
    println!("cargo:rerun-if-changed=../../zig/platform/window.zig");
    println!("cargo:rerun-if-changed=../../zig/platform/event.zig");
    println!("cargo:rerun-if-changed=../../zig/alloc/root.zig");
    println!("cargo:rerun-if-changed=../../zig/compositor/root.zig");
}
