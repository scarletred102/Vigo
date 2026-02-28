// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Vigo Browser — powered by the Vex engine.

use vex_core::{engine_name, engine_version};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!(
        "{} Engine v{} — Starting Vigo Browser",
        engine_name(),
        engine_version()
    );

    println!("Vigo Engine v{} — {}", engine_version(), engine_name());
}
