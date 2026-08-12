// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Build script — tells Cargo where to find Zig media static libraries.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(path) => path,
        Err(error) => {
            println!("cargo:warning=missing CARGO_MANIFEST_DIR: {error}");
            return;
        }
    };

    let workspace_root = match PathBuf::from(&manifest_dir)
        .parent() // crates/
        .and_then(|path| path.parent())
    {
        Some(path) => path.to_path_buf(),
        None => {
            println!("cargo:warning=could not determine workspace root from {manifest_dir}");
            return;
        }
    };

    let zig_lib_dir = workspace_root.join("zig").join("zig-out").join("lib");

    println!("cargo:rustc-link-search=native={}", zig_lib_dir.display());
    println!("cargo:rerun-if-changed={}", zig_lib_dir.display());

    if zig_lib_dir.exists() {
        let libs = ["vex_media_zig", "vex_text"];
        for lib in &libs {
            let src = zig_lib_dir.join(format!("{lib}.lib"));
            if src.exists() {
                let dst_a = zig_lib_dir.join(format!("lib{lib}.a"));
                let dst_lib = zig_lib_dir.join(format!("lib{lib}.lib"));
                let _ = std::fs::copy(&src, &dst_a);
                let _ = std::fs::copy(&src, &dst_lib);
            }
        }

        println!("cargo:rustc-link-lib=static=vex_media_zig");
        println!("cargo:rustc-link-lib=static=vex_text");
    } else {
        println!(
            "cargo:warning=Zig libraries not found at {zig_lib_dir:?}. Run `cd zig && zig build` first."
        );
    }

    println!("cargo:rerun-if-changed=../../zig/media/root.zig");
    println!("cargo:rerun-if-changed=../../zig/text/root.zig");
}
