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
    use std::time::Instant;
    use vex_browser::settings::BrowserSettings;
    use vex_render::renderer::Renderer;
    use vex_render::scroll::ScrollState;
    use vex_render::RenderPrivacyConfig;
    use vex_render::{Event, GpuContext, Window};

    tracing_subscriber::fmt().with_env_filter("info").init();

    tracing::info!(
        "{} Engine v{} — Starting Vigo Browser",
        engine_name(),
        engine_version()
    );

    let window = Window::new("Vigo Browser", 1280, 720).expect("failed to create window");
    tracing::info!("Window created ({}×{})", window.width(), window.height());

    let mut gpu = GpuContext::new(&window).expect("failed to init GPU");
    tracing::info!("GPU ready");

    // Browser settings (including randomized privacy configs).
    let settings = BrowserSettings::default();
    let privacy = RenderPrivacyConfig {
        canvas: settings.canvas_fingerprint.clone(),
        fonts: settings.font_restriction.clone(),
        webgl: settings.webgl_mask.clone(),
    };
    tracing::info!(
        canvas_seed = settings.canvas_fingerprint.session_seed,
        font_restrict = settings.font_restriction.enabled,
        webgl_mask = settings.webgl_mask.enabled,
        "Privacy config initialized"
    );

    let mut renderer = Renderer::new(&gpu.device, &gpu.queue, gpu.config.format);
    renderer.set_privacy_config(privacy);
    renderer.set_clear_color(0.08, 0.08, 0.12);

    let mut vp_w = gpu.config.width as f32;
    let mut vp_h = gpu.config.height as f32;

    // Parse the welcome page once up-front.
    let page_dl = build_page_display_list(vp_w, vp_h);
    // Content height is large enough to allow scrolling.
    let content_height = 1400.0_f32;

    // Scroll state for the content area (below chrome).
    let mut scroll = ScrollState::new(vp_w, vp_h);
    scroll.set_content_size(vp_w, content_height);

    // FPS tracking.
    let mut frame_count: u32 = 0;
    let mut fps_timer = Instant::now();
    let mut last_fps: u32 = 0;

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
                    vp_w = width as f32;
                    vp_h = height as f32;
                    scroll.set_viewport_size(vp_w, vp_h);
                }
                Event::MouseScroll { dy, .. } => {
                    scroll.scroll_by(0.0, -dy * 40.0);
                }
                Event::KeyDown { keycode, .. } => {
                    tracing::debug!("Key down: {keycode}");
                }
                _ => {}
            }
        }

        // Compose the final display list: chrome (fixed) + page content (scrolled).
        let dl = compose_frame(vp_w, vp_h, &scroll, &page_dl);

        // Upload to GPU.
        renderer.prepare(&gpu.device, &gpu.queue, &dl, vp_w, vp_h);

        // Render.
        let output = match gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("get texture: {e}");
                continue;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vex-frame"),
            });

        renderer.render(&mut encoder, &view);

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // FPS counter — update title every second.
        frame_count += 1;
        let elapsed = fps_timer.elapsed();
        if elapsed.as_secs() >= 1 {
            last_fps = frame_count;
            frame_count = 0;
            fps_timer = Instant::now();
            tracing::debug!("FPS: {last_fps}");
        }
        let _ = last_fps; // Will be shown in title bar once Window supports set_title.

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// Run a full HTML → DOM → CSS → Layout → Display List pipeline on the
/// built-in welcome page and return the resulting display list.
#[cfg(target_os = "windows")]
fn build_page_display_list(vp_w: f32, vp_h: f32) -> vex_render::display_list::DisplayList {
    use vex_core::Size;

    let html = include_str!("welcome.html");
    let document = vex_html::parse_html(html);

    // Extract <style> blocks from the DOM.
    let style_ids = document.get_elements_by_tag_name("style");
    let mut stylesheets = Vec::new();
    for id in &style_ids {
        let css_text = document.text_content(*id);
        if !css_text.is_empty() {
            stylesheets.push(vex_css::parse_stylesheet(&css_text));
        }
    }

    let viewport = Size::new(vp_w, vp_h);
    let styles = vex_css::compute_styles(&document, &stylesheets, viewport);
    let layout_root = vex_layout::layout_document(&document, &styles, viewport);
    vex_render::build_display_list(&layout_root, &styles, &document, viewport)
}

/// Compose a final frame: fixed browser chrome + scrolled page content.
#[cfg(target_os = "windows")]
fn compose_frame(
    vp_w: f32,
    _vp_h: f32,
    scroll: &vex_render::scroll::ScrollState,
    page_dl: &vex_render::display_list::DisplayList,
) -> vex_render::display_list::DisplayList {
    use vex_browser::ui::chrome;
    use vex_core::color::Color;
    use vex_core::geometry::{Point, Rect};
    use vex_render::display_list::{DisplayCommand, DisplayList};

    let chrome_height = chrome::chrome_height(false);
    let sy = scroll.offset_y;

    let mut dl = DisplayList::with_capacity(16 + page_dl.commands().len());

    // === Fixed browser chrome ===

    // Tab bar.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(0.0, 0.0, vp_w, chrome::TAB_BAR_HEIGHT),
        color: Color::rgb(38, 38, 46),
    });
    // Active tab.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(4.0, 4.0, 180.0, 32.0),
        color: Color::rgb(52, 52, 64),
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(16.0, 12.0),
        text: "Vigo Browser".into(),
        color: Color::rgb(210, 210, 220),
        font_size: 13.0,
        line_height: 18.0,
    });

    // Navigation bar.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(0.0, chrome::TAB_BAR_HEIGHT, vp_w, chrome::NAV_BAR_HEIGHT),
        color: Color::rgb(30, 30, 38),
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(60.0, chrome::TAB_BAR_HEIGHT + 8.0, vp_w - 120.0, 28.0),
        color: Color::rgb(44, 44, 56),
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(72.0, chrome::TAB_BAR_HEIGHT + 13.0),
        text: "vex://welcome".into(),
        color: Color::rgb(160, 160, 180),
        font_size: 14.0,
        line_height: 20.0,
    });

    // Accent line.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            0.0,
            chrome_height - chrome::ACCENT_LINE_HEIGHT,
            vp_w,
            chrome::ACCENT_LINE_HEIGHT,
        ),
        color: Color::rgb(76, 86, 200),
    });

    // === Scrolled page content ===
    // Offset every command from the pipeline by (chrome_height - scroll).
    for cmd in page_dl.commands() {
        dl.push(offset_command(cmd, 0.0, chrome_height - sy));
    }

    dl
}

/// Offset a display command by (dx, dy).
#[cfg(target_os = "windows")]
fn offset_command(
    cmd: &vex_render::display_list::DisplayCommand,
    dx: f32,
    dy: f32,
) -> vex_render::display_list::DisplayCommand {
    use vex_core::geometry::{Point, Rect};
    use vex_render::display_list::DisplayCommand;

    match cmd {
        DisplayCommand::FillRect { rect, color } => DisplayCommand::FillRect {
            rect: Rect::new(
                rect.origin.x + dx,
                rect.origin.y + dy,
                rect.size.width,
                rect.size.height,
            ),
            color: *color,
        },
        DisplayCommand::DrawText {
            position,
            text,
            color,
            font_size,
            line_height,
        } => DisplayCommand::DrawText {
            position: Point::new(position.x + dx, position.y + dy),
            text: text.clone(),
            color: *color,
            font_size: *font_size,
            line_height: *line_height,
        },
        DisplayCommand::DrawBorder {
            rect,
            widths,
            colors,
            styles,
        } => DisplayCommand::DrawBorder {
            rect: Rect::new(
                rect.origin.x + dx,
                rect.origin.y + dy,
                rect.size.width,
                rect.size.height,
            ),
            widths: *widths,
            colors: *colors,
            styles: *styles,
        },
        DisplayCommand::DrawImage { rect, image_id } => DisplayCommand::DrawImage {
            rect: Rect::new(
                rect.origin.x + dx,
                rect.origin.y + dy,
                rect.size.width,
                rect.size.height,
            ),
            image_id: *image_id,
        },
        DisplayCommand::PushClip { rect } => DisplayCommand::PushClip {
            rect: Rect::new(
                rect.origin.x + dx,
                rect.origin.y + dy,
                rect.size.width,
                rect.size.height,
            ),
        },
        // Commands without position stay unchanged.
        other => other.clone(),
    }
}
