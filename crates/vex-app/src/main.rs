// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Vigo Browser — powered by the Vex engine.

use vex_core::{engine_name, engine_version};
use vex_render::{Event, GpuContext, Window};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!(
        "{} Engine v{} — Starting Vigo Browser",
        engine_name(),
        engine_version()
    );

    let window = Window::new("Vigo Browser", 1280, 720).expect("failed to create window");
    tracing::info!("Window created ({}×{})", window.width(), window.height());

    let mut gpu = GpuContext::new(&window).expect("failed to init GPU");
    tracing::info!("GPU ready");

    // Dark background (Vigo brand colour).
    let (bg_r, bg_g, bg_b) = (0.08, 0.08, 0.12);

    loop {
        while let Some(ev) = window.poll_event() {
            match ev {
                Event::WindowClose => {
                    tracing::info!("Window close — exiting");
                    return;
                }
                Event::WindowResize { width, height } => {
                    tracing::info!("Resized to {width}×{height}");
                    gpu.resize(width, height);
                }
                Event::KeyDown { keycode, .. } => {
                    tracing::debug!("Key down: {keycode}");
                }
                _ => {}
            }
        }

        // Clear to background colour.
        if let Err(e) = gpu.render_clear(bg_r, bg_g, bg_b) {
            tracing::error!("Render error: {e}");
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
