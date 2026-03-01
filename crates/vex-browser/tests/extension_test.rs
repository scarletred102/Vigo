// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Integration test: load the sample "Vigo Dark Mode" extension.

use std::path::PathBuf;

use vex_browser::extensions::loader::ExtensionLoader;
use vex_browser::extensions::permissions::Permission;

fn sample_extensions_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // repo root
    p.push("extensions");
    p
}

#[test]
fn load_dark_mode_extension() {
    let dir = sample_extensions_dir();
    let ext_dir = dir.join("vigo-dark-mode");
    assert!(
        ext_dir.exists(),
        "extensions/vigo-dark-mode should exist: {}",
        ext_dir.display()
    );

    let mut loader = ExtensionLoader::new(dir);
    let id = loader.load_extension(&ext_dir).unwrap();
    assert_eq!(id, "vigo-dark-mode");

    let ext = loader.get("vigo-dark-mode").unwrap();
    assert_eq!(ext.manifest.name, "Vigo Dark Mode");
    assert_eq!(ext.manifest.version, "1.0.0");
    assert!(ext.manifest.permissions.contains(&Permission::ActiveTab));
    assert_eq!(ext.manifest.content_scripts.len(), 1);
    assert!(ext.manifest.browser_action.is_some());
}

#[test]
fn enable_dark_mode_injects_content_script() {
    let dir = sample_extensions_dir();
    let ext_dir = dir.join("vigo-dark-mode");

    let mut loader = ExtensionLoader::new(dir);
    loader.load_extension(&ext_dir).unwrap();
    loader.enable("vigo-dark-mode");

    let scripts = loader.content_scripts();
    assert_eq!(scripts.len(), 1);
    assert!(scripts[0].source.contains("filter: invert(1)"));

    // Matches any http/https URL.
    let matches = loader.content_scripts_for_url("https://example.com/page");
    assert_eq!(matches.len(), 1);
}

#[test]
fn dark_mode_content_script_url_matching() {
    let dir = sample_extensions_dir();
    let ext_dir = dir.join("vigo-dark-mode");

    let mut loader = ExtensionLoader::new(dir);
    loader.load_extension(&ext_dir).unwrap();
    loader.enable("vigo-dark-mode");

    // Matches various HTTP URLs.
    assert_eq!(
        loader
            .content_scripts_for_url("http://localhost:8080/")
            .len(),
        1
    );
    assert_eq!(
        loader
            .content_scripts_for_url("https://www.example.com/path/page.html")
            .len(),
        1
    );

    // Does not match non-HTTP protocols.
    assert!(loader
        .content_scripts_for_url("ftp://files.example.com/")
        .is_empty());
}
