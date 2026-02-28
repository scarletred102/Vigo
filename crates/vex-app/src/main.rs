// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Vigo Browser — powered by the Vex engine.

use vex_core::{engine_name, engine_version};

fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("Vigo Browser: Windows is the only supported platform right now.");
        return;
    }

    #[cfg(target_os = "windows")]
    run();
}

#[cfg(target_os = "windows")]
fn run() {
    use vex_render::{Event, GpuContext, Window};

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
