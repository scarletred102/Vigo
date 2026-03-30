// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Vigo Browser — powered by the Vex engine.
//!
//! Entry point: creates the window, initializes GPU, sets up the tab manager,
//! and runs the main browser event loop with full toolbar wiring.

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
    use std::collections::HashMap;
    use std::time::Instant;

    use vex_browser::bookmarks::BookmarkManager;
    use vex_browser::devtools::console::ConsoleState;
    use vex_browser::devtools::performance::PerformanceState;
    use vex_browser::devtools::sources::{SourceKind, SourcesState};
    use vex_browser::devtools::DevToolsState;
    use vex_browser::downloads::DownloadManager;
    use vex_browser::embedder::{
        ConsoleLevel as EmbedderConsoleLevel, DialogRequest, EmbedderBus, EmbedderCommand,
        EmbedderMsg,
    };
    use vex_browser::event_handler;
    use vex_browser::event_handler::KeyResult;
    use vex_browser::extensions::loader::ExtensionLoader;
    use vex_browser::find::FindState;
    use vex_browser::links;
    use vex_browser::navigation::NavigationHistory;
    use vex_browser::session::SessionState;
    use vex_browser::settings::BrowserSettings;
    use vex_browser::tab::TabId;
    use vex_browser::tab_manager::TabManager;
    use vex_browser::ui::context_menu::ContextMenu;
    use vex_browser::ui::nav_bar::hit_test_nav_bar;
    use vex_browser::ui::shortcuts::match_shortcut;
    use vex_browser::ui::tab_bar::{hit_test_tab_bar, TabBarAction};
    use vex_browser::ui::toolbar::ToolbarLayout;
    use vex_browser::zoom::ZoomState;
    use vex_core::geometry::{Point, Size};
    use vex_core::VexUrl;
    use vex_js::BrowserRequest;
    use vex_render::event::MouseButton;
    use vex_render::renderer::Renderer;
    use vex_render::{Event, GpuContext, RenderPrivacyConfig, Window};

    tracing_subscriber::fmt().with_env_filter("info").init();

    tracing::info!(
        "{} Engine v{} — Starting Vigo Browser",
        engine_name(),
        engine_version()
    );

    // A-004 (KPI): measure cold start from `run()` entry to first presented frame.
    let app_start = Instant::now();

    // ── Settings ─────────────────────────────────────────────────
    let settings = BrowserSettings::default();
    let window = Window::new(
        "Vigo Browser",
        settings.window_width,
        settings.window_height,
    )
    .expect("failed to create window");
    tracing::info!("Window created ({}×{})", window.width(), window.height());

    let mut gpu = GpuContext::new(&window).expect("failed to init GPU");
    tracing::info!("GPU ready");

    let privacy = RenderPrivacyConfig {
        canvas: settings.canvas_fingerprint.clone(),
        fonts: settings.font_restriction.clone(),
        webgl: settings.webgl_mask.clone(),
    };
    tracing::info!(
        canvas_seed = settings.canvas_fingerprint.session_seed,
        "Privacy config initialized"
    );

    let mut renderer = Renderer::new(&gpu.device, &gpu.queue, gpu.config.format);
    renderer.set_privacy_config(privacy);
    renderer.set_clear_color(0.08, 0.08, 0.12);

    let mut vp_w = gpu.config.width as f32;
    let mut vp_h = gpu.config.height as f32;

    // ── Browser state ────────────────────────────────────────────
    let mut tab_mgr = TabManager::new();
    let mut nav_histories: HashMap<TabId, NavigationHistory> = HashMap::new();
    let mut zoom = ZoomState::new();
    let mut bookmarks = BookmarkManager::new();
    let mut downloads = DownloadManager::new();
    let mut find = FindState::new();
    let mut context_menu: Option<ContextMenu> = None;
    let mut embedder_bus = EmbedderBus::new();

    // Address bar editing state.
    let mut address_bar_focused = false;
    let mut address_bar_text = String::new();

    // Focused form control inside page content.
    let mut focused_node: Option<vex_core::VexId> = None;

    // Find bar state.
    let mut find_bar_visible = false;
    let mut find_bar_text = String::new();

    // ── DevTools state (Tasks 51-56) ─────────────────────────────
    let mut devtools = DevToolsState::new();
    let mut console_state = ConsoleState::new();
    let mut perf_state = PerformanceState::new();
    let mut sources_state = SourcesState::new();
    // Start recording performance immediately so DevTools has data.
    perf_state.start_recording();

    // ── Extensions (Tasks 57-60) ─────────────────────────────────
    let ext_dir = data_dir().join("extensions");
    let _ = std::fs::create_dir_all(&ext_dir);
    let mut ext_loader = ExtensionLoader::new(ext_dir);
    {
        let (loaded, errors) = ext_loader.load_all();
        for id in &loaded {
            ext_loader.approve_permissions(id);
            ext_loader.enable(id);
        }
        if !loaded.is_empty() {
            tracing::info!("Loaded {} extension(s)", loaded.len());
        }
        for e in &errors {
            tracing::warn!("Extension load error: {e}");
        }
    }

    // ── Session restore (Task 40) ────────────────────────────────
    let data = data_dir();
    let _ = std::fs::create_dir_all(&data);
    let session_path = data.join("session.json");
    let bookmarks_path = data.join("bookmarks.json");
    let kpi_path = data.join("kpi_latest.json");
    let mut cold_start_recorded = false;

    if let Ok(bm) = BookmarkManager::load(&bookmarks_path) {
        bookmarks = bm;
        tracing::info!("Loaded {} bookmarks", bookmarks.count());
    }

    if let Ok(session) = SessionState::load(&session_path) {
        if !session.tabs.is_empty() {
            session.restore_into(&mut tab_mgr);
            tracing::info!("Restored {} tabs from session", tab_mgr.tab_count());
        }
    }

    // ── Bootstrap active tab content if needed ─────────────────────────────
    {
        let needs_bootstrap = !tab_mgr.active_tab().has_document();
        if needs_bootstrap {
            let url = tab_mgr.active_tab().url.clone();
            let url_str = url.as_ref().to_string();

            if url_str.starts_with("http://") || url_str.starts_with("https://") {
                navigate_tab(&mut tab_mgr, &url, vp_w, vp_h);
            } else {
                let viewport = Size::new(vp_w, vp_h);
                let html = vex_browser::internal_pages::newtab_page();
                let tab = tab_mgr.active_tab_mut();
                tab.load_html(&html, viewport);
                tab.url = VexUrl::parse("vex://newtab").unwrap_or_else(|_| {
                    VexUrl::parse("about:blank").expect("about:blank is valid")
                });
                tab.set_content_size(vp_w, 1400.0);

                // Task 56: Populate Sources panel with the internal new-tab HTML.
                sources_state.add_source(
                    "newtab.html".to_owned(),
                    "vex://newtab".to_owned(),
                    SourceKind::Html,
                    html,
                );
            }
        }
    }

    // ── FPS tracking ─────────────────────────────────────────────
    let mut frame_count: u32 = 0;
    let mut fps_timer = Instant::now();
    let mut _last_fps: u32 = 0;

    // ── Main event loop ──────────────────────────────────────────
    loop {
        let show_bookmarks = settings.show_bookmarks_bar;
        let toolbar = ToolbarLayout::compute(vp_w, vp_h, show_bookmarks, find_bar_visible);

        while let Some(ev) = window.poll_event() {
            match ev {
                // ── Window close → save session + exit ──────────
                Event::WindowClose => {
                    let session = SessionState::from_tabs(tab_mgr.tabs(), tab_mgr.active_index());
                    if let Err(e) = session.save(&session_path) {
                        tracing::warn!("Failed to save session: {e}");
                    }
                    if let Err(e) = bookmarks.save(&bookmarks_path) {
                        tracing::warn!("Failed to save bookmarks: {e}");
                    }
                    tracing::info!("Window close — exiting");
                    return;
                }

                // ── Window resize ───────────────────────────────
                Event::WindowResize { width, height } => {
                    tracing::info!("Resized to {width}×{height}");
                    gpu.resize(width, height);
                    vp_w = width as f32;
                    vp_h = height as f32;
                    let count = tab_mgr.tab_count();
                    for i in 0..count {
                        let id = tab_mgr.tabs()[i].id;
                        if let Some(tab) = tab_mgr.tab_mut(id) {
                            tab.set_viewport(vp_w, vp_h);
                        }
                    }
                }

                // ── Scroll ──────────────────────────────────────
                Event::MouseScroll { dy, .. } => {
                    context_menu = None;
                    tab_mgr.active_tab_mut().scroll_by(0.0, -dy * 40.0);
                }

                // ── Mouse click ─────────────────────────────────
                Event::MouseButtonDown { button, x, y } => {
                    let fx = x as f32;
                    let fy = y as f32;

                    // Context menu hit first (if visible).
                    if let Some(ref menu) = context_menu {
                        if let Some(action) = menu.hit_test(fx, fy) {
                            handle_menu_action(
                                &action,
                                &mut tab_mgr,
                                &mut nav_histories,
                                vp_w,
                                vp_h,
                            );
                            context_menu = None;
                            continue;
                        }
                        context_menu = None;
                    }

                    match button {
                        MouseButton::Left => {
                            // Task 31: Tab bar clicks.
                            let tab_action = hit_test_tab_bar(
                                fx,
                                fy,
                                tab_mgr.tab_count(),
                                tab_mgr.active_index(),
                                toolbar.tab_bar,
                            );
                            match tab_action {
                                TabBarAction::SwitchTab(idx) => {
                                    let id = tab_mgr.tabs()[idx].id;
                                    tab_mgr.switch_to(id);
                                    address_bar_focused = false;
                                }
                                TabBarAction::CloseTab(idx) => {
                                    let id = tab_mgr.tabs()[idx].id;
                                    tab_mgr.close_tab(id);
                                    address_bar_focused = false;
                                }
                                TabBarAction::NewTab => {
                                    load_welcome_tab(&mut tab_mgr, vp_w, vp_h);
                                    address_bar_focused = false;
                                }
                                TabBarAction::None => {
                                    // Task 32: Nav bar clicks.
                                    let nav_action = hit_test_nav_bar(fx, fy, toolbar.nav_bar);
                                    let nav_action_after = nav_action.clone();
                                    let bookmark_hit = show_bookmarks
                                        && hit_test_bookmark_bar(
                                            fx,
                                            fy,
                                            &bookmarks,
                                            toolbar.bookmark_bar,
                                        )
                                        .is_some();
                                    let prev_focus_state = address_bar_focused;
                                    handle_nav_action(
                                        nav_action,
                                        &mut tab_mgr,
                                        &mut nav_histories,
                                        &mut address_bar_focused,
                                        &mut address_bar_text,
                                        &bookmarks,
                                        show_bookmarks,
                                        &toolbar,
                                        fx,
                                        fy,
                                        vp_w,
                                        vp_h,
                                    );

                                    // If nav handling did not consume this click,
                                    // forward to page content hit-test + DOM events.
                                    if matches!(nav_action_after, vex_browser::NavBarAction::None)
                                        && !bookmark_hit
                                    {
                                        let content = toolbar.content_area;
                                        if fx >= content.origin.x
                                            && fx <= content.origin.x + content.size.width
                                            && fy >= content.origin.y
                                            && fy <= content.origin.y + content.size.height
                                        {
                                            address_bar_focused = false;
                                            handle_content_click(
                                                &mut tab_mgr,
                                                &mut nav_histories,
                                                &mut focused_node,
                                                fx,
                                                fy,
                                                content.origin.x,
                                                content.origin.y,
                                                vp_w,
                                                vp_h,
                                            );
                                        }
                                    } else if !matches!(
                                        nav_action_after,
                                        vex_browser::NavBarAction::AddressBar
                                    ) && prev_focus_state
                                    {
                                        address_bar_focused = false;
                                    }
                                }
                            }
                        }
                        // Task 34: Right-click → context menu.
                        MouseButton::Right => {
                            let tab_id = tab_mgr.active_tab_id();
                            let can_back =
                                nav_histories.get(&tab_id).is_some_and(|h| h.can_go_back());
                            let can_fwd = nav_histories
                                .get(&tab_id)
                                .is_some_and(|h| h.can_go_forward());
                            context_menu = Some(ContextMenu::page_menu(
                                Point::new(fx, fy),
                                can_back,
                                can_fwd,
                            ));
                        }
                        _ => {}
                    }
                }

                // ── Keyboard ────────────────────────────────────
                Event::KeyDown { keycode, modifiers } => {
                    let ctrl = modifiers & 0x01 != 0;
                    let shift = modifiers & 0x02 != 0;
                    let alt = modifiers & 0x04 != 0;

                    if !address_bar_focused {
                        if let Some(focused) = focused_node {
                            let char_value = vk_to_char(keycode, shift);
                            let key_result = {
                                let tab = tab_mgr.active_tab_mut();
                                match (&tab.shared_doc, tab.runtime.as_mut()) {
                                    (Some(shared), Some(runtime)) => {
                                        Some(event_handler::process_key_down(
                                            focused, keycode, char_value, shared, runtime,
                                        ))
                                    }
                                    _ => None,
                                }
                            };

                            if let Some(result) = key_result {
                                match result {
                                    KeyResult::InputChanged => {
                                        let viewport = Size::new(vp_w, vp_h);
                                        tab_mgr.active_tab_mut().relayout(viewport);
                                        continue;
                                    }
                                    KeyResult::Handled => continue,
                                    KeyResult::NoFocus => {
                                        focused_node = None;
                                    }
                                }
                            }
                        }
                    }

                    // Address bar input handling (Task 32).
                    if address_bar_focused && !ctrl && !alt {
                        match keycode {
                            0x0D => {
                                let input = address_bar_text.clone();
                                address_bar_focused = false;
                                // Task 39: normalize_or_search uses search engine.
                                if let Ok(url) =
                                    links::normalize_or_search(&input, &settings.search_engine)
                                {
                                    let tab_id = tab_mgr.active_tab_id();
                                    let history = nav_histories.entry(tab_id).or_default();
                                    history.push(
                                        url.clone(),
                                        tab_mgr.active_tab().title.clone(),
                                        Point::new(0.0, 0.0),
                                    );
                                    navigate_tab(&mut tab_mgr, &url, vp_w, vp_h);
                                }
                            }
                            0x1B => address_bar_focused = false,
                            0x08 => {
                                address_bar_text.pop();
                            }
                            _ => {
                                if let Some(ch) = vk_to_char(keycode, shift) {
                                    address_bar_text.push(ch);
                                }
                            }
                        }
                        continue;
                    }

                    // Find bar input handling (Task 36).
                    if find_bar_visible && !ctrl && !alt {
                        match keycode {
                            0x1B => {
                                find_bar_visible = false;
                                find.clear();
                                find_bar_text.clear();
                            }
                            0x0D => {
                                if shift {
                                    find.prev_match();
                                } else {
                                    find.next_match();
                                }
                            }
                            0x08 => {
                                find_bar_text.pop();
                                if let Some(doc) = tab_mgr.active_tab().borrow_document() {
                                    find.search(&doc, &find_bar_text);
                                }
                            }
                            _ => {
                                if let Some(ch) = vk_to_char(keycode, shift) {
                                    find_bar_text.push(ch);
                                    if let Some(doc) = tab_mgr.active_tab().borrow_document() {
                                        find.search(&doc, &find_bar_text);
                                    }
                                }
                            }
                        }
                        continue;
                    }

                    // Ctrl+D → toggle bookmark (Task 35).
                    if ctrl && !shift && !alt && keycode == 0x44 {
                        let url = tab_mgr.active_tab().url.to_string();
                        let title = tab_mgr.active_tab().title.clone();
                        if bookmarks.is_bookmarked(&url) {
                            let bar = bookmarks.bar_bookmarks();
                            if let Some(b) = bar.iter().find(|b| b.url == url) {
                                let id = b.id;
                                bookmarks.remove(id);
                                tracing::info!("Bookmark removed: {url}");
                            }
                        } else {
                            bookmarks.add(&title, &url, "Bookmarks Bar");
                            tracing::info!("Bookmark added: {url}");
                        }
                        continue;
                    }

                    // Task 33: Keyboard shortcuts.
                    if let Some(shortcut) = platform_to_shortcut(keycode, modifiers) {
                        if let Some(action) = match_shortcut(&shortcut) {
                            // Task 51: F12 / Ctrl+Shift+I → toggle DevTools.
                            if matches!(action, vex_browser::ui::shortcuts::BrowserAction::DevTools)
                            {
                                devtools.toggle();
                                tracing::info!(
                                    "DevTools {}",
                                    if devtools.is_open() {
                                        "opened"
                                    } else {
                                        "closed"
                                    }
                                );
                            } else {
                                handle_browser_action(
                                    action,
                                    &mut tab_mgr,
                                    &mut nav_histories,
                                    &mut zoom,
                                    &mut find,
                                    &mut find_bar_visible,
                                    &mut find_bar_text,
                                    &mut address_bar_focused,
                                    &mut address_bar_text,
                                    &mut downloads,
                                    vp_w,
                                    vp_h,
                                );
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        // ── Drain JS request queue → Embedder bus (Servo-style) ──
        {
            let queue = tab_mgr.active_tab().request_queue.clone();
            let requests: Vec<BrowserRequest> = queue.borrow_mut().drain(..).collect();
            for req in requests {
                let active_tab_id = tab_mgr.active_tab_id();
                match req {
                    BrowserRequest::ConsoleLog { level, message } => {
                        let embedder_level = match level.as_str() {
                            "warn" => EmbedderConsoleLevel::Warn,
                            "error" => EmbedderConsoleLevel::Error,
                            "info" => EmbedderConsoleLevel::Info,
                            "debug" => EmbedderConsoleLevel::Debug,
                            _ => EmbedderConsoleLevel::Log,
                        };
                        embedder_bus.push_message(EmbedderMsg::ConsoleMessage(
                            active_tab_id,
                            embedder_level,
                            message,
                        ));
                    }
                    BrowserRequest::Alert(msg) => {
                        embedder_bus
                            .push_message(EmbedderMsg::ShowDialog(active_tab_id, DialogRequest::Alert(msg)));
                    }
                    BrowserRequest::Navigate(raw_url) => {
                        let current_url = tab_mgr.active_tab().url.clone();
                        let Some(url) = resolve_browser_request_url(&raw_url, &current_url) else {
                            tracing::warn!(
                                target: "vigo::browser_loop",
                                raw_url,
                                "ignoring invalid BrowserRequest::Navigate URL"
                            );
                            continue;
                        };
                        embedder_bus.push_command(EmbedderCommand::Navigate(active_tab_id, url));
                    }
                    BrowserRequest::Reload => {
                        embedder_bus.push_command(EmbedderCommand::Reload(active_tab_id));
                    }
                    BrowserRequest::Back => {
                        embedder_bus.push_command(EmbedderCommand::GoBack(active_tab_id));
                    }
                    BrowserRequest::Forward => {
                        embedder_bus.push_command(EmbedderCommand::GoForward(active_tab_id));
                    }
                    BrowserRequest::PushState { url } => {
                        let tab_id = active_tab_id;
                        let current_url = tab_mgr.active_tab().url.clone();

                        let next_url = match url {
                            Some(raw) => match resolve_browser_request_url(&raw, &current_url) {
                                Some(resolved) => resolved,
                                None => {
                                    tracing::warn!(
                                        target: "vigo::browser_loop",
                                        raw,
                                        "ignoring invalid BrowserRequest::PushState URL"
                                    );
                                    continue;
                                }
                            },
                            None => current_url.clone(),
                        };

                        let title = tab_mgr.active_tab().title.clone();
                        let scroll = Point::new(
                            tab_mgr.active_tab().scroll.offset_x,
                            tab_mgr.active_tab().scroll.offset_y,
                        );
                        nav_histories.entry(tab_id).or_default().push(
                            next_url.clone(),
                            title,
                            scroll,
                        );

                        if let Some(tab) = tab_mgr.tab_mut(tab_id) {
                            tab.url = next_url.clone();
                            if let Some(runtime) = tab.runtime.as_mut() {
                                vex_js::api::window::update_location(
                                    next_url.as_ref(),
                                    runtime.context_mut(),
                                );
                            }
                            tab.mark_dirty();
                        }

                        embedder_bus.push_message(EmbedderMsg::UrlChanged(tab_id, next_url));
                        push_history_changed_message(&mut embedder_bus, tab_id, &nav_histories);
                    }
                }
            }
        }

        process_embedder_bus(
            &mut embedder_bus,
            &mut tab_mgr,
            &mut nav_histories,
            &mut console_state,
            vp_w,
            vp_h,
        );

        // ── Extension content script injection (Task 58) ────────
        {
            let tab = tab_mgr.active_tab();
            let url_str = tab.url.to_string();
            let scripts = ext_loader.content_scripts_for_url(&url_str);
            if !scripts.is_empty() {
                tracing::debug!(
                    "Content scripts matched for {url_str}: {} script(s)",
                    scripts.len()
                );
                // Content script injection happens on page load in tab.load_html;
                // here we log that scripts matched for debugging.
            }
        }

        // ── Compose display list ────────────────────────────────
        let frame_start = Instant::now();
        let dl = compose_frame(
            &tab_mgr,
            &toolbar,
            &bookmarks,
            &find,
            find_bar_visible,
            &find_bar_text,
            &context_menu,
            address_bar_focused,
            &address_bar_text,
            &nav_histories,
            &settings,
            &zoom,
            vp_w,
            &devtools,
            &console_state,
            &perf_state,
            &sources_state,
        );

        // ── Tick JS timers for active tab (Task: timer queue) ────
        {
            let active_id = tab_mgr.active_tab().id;
            if let Some(tab) = tab_mgr.tab_mut(active_id) {
                tab.tick_timers();
            }
        }

        // ── Render ──────────────────────────────────────────────
        renderer.prepare(&gpu.device, &gpu.queue, &dl, vp_w, vp_h);

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

        // A-004 (KPI): write one cold-start snapshot after the first frame is presented.
        if !cold_start_recorded {
            cold_start_recorded = true;
            let cold_start_ms = app_start.elapsed().as_secs_f64() * 1000.0;
            let working_set_bytes = current_process_working_set_bytes();
            write_kpi_snapshot(&kpi_path, cold_start_ms, vp_w, vp_h, working_set_bytes);
            tracing::info!(
                cold_start_ms,
                working_set_bytes,
                path = %kpi_path.display(),
                "Captured cold-start KPI snapshot"
            );
        }

        // ── Performance recording (Task 55) ─────────────────────
        let frame_elapsed = frame_start.elapsed();
        perf_state.record_frame_timings(
            std::time::Duration::ZERO, // parse — only happens on load
            std::time::Duration::ZERO, // style — only happens on load
            std::time::Duration::ZERO, // layout — only happens on load
            frame_elapsed,             // paint — entire frame composition + GPU
            std::time::Duration::ZERO, // script — only happens on events
            std::time::Duration::ZERO, // other
        );

        // ── FPS ─────────────────────────────────────────────────
        frame_count += 1;
        if fps_timer.elapsed().as_secs() >= 1 {
            _last_fps = frame_count;
            frame_count = 0;
            fps_timer = Instant::now();
            tracing::debug!("FPS: {_last_fps}");
        }

        // ── Window title (Task 76 + 77) ────────────────────────
        {
            let page_title = &tab_mgr.active_tab().title;
            let base_title = if page_title.is_empty() {
                "Vigo Browser".to_string()
            } else {
                format!("{page_title} — Vigo Browser")
            };
            #[cfg(debug_assertions)]
            let base_title = format!("{base_title} [{_last_fps} FPS]");
            window.set_title(&base_title);
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════

/// User data directory for persistence (~/.vigo/).
#[cfg(target_os = "windows")]
fn data_dir() -> std::path::PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE") {
        std::path::PathBuf::from(home).join(".vigo")
    } else {
        std::path::PathBuf::from(".vigo")
    }
}

/// Write a lightweight KPI snapshot JSON file used by benchmark/report tooling.
#[cfg(target_os = "windows")]
fn write_kpi_snapshot(
    path: &std::path::Path,
    cold_start_ms: f64,
    viewport_w: f32,
    viewport_h: f32,
    working_set_bytes: Option<u64>,
) {
    let unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let working_set_json = working_set_bytes
        .map(|bytes| bytes.to_string())
        .unwrap_or_else(|| "null".to_string());

    let payload = format!(
        "{{\n  \"captured_at_unix_ms\": {},\n  \"cold_start_ms\": {:.2},\n  \"viewport_width\": {:.0},\n  \"viewport_height\": {:.0},\n  \"working_set_bytes\": {}\n}}\n",
        unix_ms,
        cold_start_ms,
        viewport_w,
        viewport_h,
        working_set_json
    );

    if let Err(e) = std::fs::write(path, payload) {
        tracing::warn!(path = %path.display(), "Failed to write KPI snapshot: {e}");
    }
}

/// Snapshot current process working set (resident memory) on Windows.
#[cfg(target_os = "windows")]
fn current_process_working_set_bytes() -> Option<u64> {
    use std::ffi::c_void;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }

    unsafe {
        let process = GetCurrentProcess();
        if process.is_null() {
            return None;
        }

        let mut counters = ProcessMemoryCounters {
            cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };

        let ok = GetProcessMemoryInfo(process, &mut counters, counters.cb);
        if ok == 0 {
            None
        } else {
            Some(counters.working_set_size as u64)
        }
    }
}

/// Resolve a JavaScript-requested URL against the current tab URL.
///
/// `location.assign()` and `history.pushState()` may provide absolute or
/// relative URLs. Relative URLs are resolved against `base`.
#[cfg(target_os = "windows")]
fn resolve_browser_request_url(raw: &str, base: &vex_core::VexUrl) -> Option<vex_core::VexUrl> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains("://") || trimmed.starts_with("about:") || trimmed.starts_with("vex:") {
        return vex_core::VexUrl::parse(trimmed).ok();
    }

    if let Ok(url) = base.join(trimmed) {
        return Some(url);
    }

    vex_core::VexUrl::parse(trimmed).ok()
}

/// Emit a `HistoryChanged` embedder message for a tab when history mutates.
#[cfg(target_os = "windows")]
fn push_history_changed_message(
    embedder_bus: &mut vex_browser::EmbedderBus,
    tab_id: vex_browser::tab::TabId,
    nav_histories: &std::collections::HashMap<vex_browser::tab::TabId, vex_browser::NavigationHistory>,
) {
    if let Some(history) = nav_histories.get(&tab_id) {
        let (urls, current_index) = history.snapshot();
        embedder_bus.push_message(vex_browser::EmbedderMsg::HistoryChanged(
            tab_id,
            urls,
            current_index,
        ));
    }
}

/// Process queued embedder commands and messages (Servo-style embedder bridge).
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn process_embedder_bus(
    embedder_bus: &mut vex_browser::EmbedderBus,
    tab_mgr: &mut vex_browser::TabManager,
    nav_histories: &mut std::collections::HashMap<vex_browser::tab::TabId, vex_browser::NavigationHistory>,
    console_state: &mut vex_browser::devtools::console::ConsoleState,
    vp_w: f32,
    vp_h: f32,
) {
    use vex_browser::devtools::console::LogLevel;

    // 1) Commands: embedder/UI -> engine actions.
    while let Some(command) = embedder_bus.pop_command() {
        match command {
            vex_browser::EmbedderCommand::Navigate(tab_id, url) => {
                if !tab_mgr.switch_to(tab_id) {
                    continue;
                }

                let title = tab_mgr.active_tab().title.clone();
                let scroll = vex_core::geometry::Point::new(
                    tab_mgr.active_tab().scroll.offset_x,
                    tab_mgr.active_tab().scroll.offset_y,
                );
                nav_histories
                    .entry(tab_id)
                    .or_default()
                    .push(url.clone(), title, scroll);

                navigate_tab(tab_mgr, &url, vp_w, vp_h);
                embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(tab_id, url));
                embedder_bus.push_message(vex_browser::EmbedderMsg::TitleChanged(
                    tab_id,
                    Some(tab_mgr.active_tab().title.clone()),
                ));
                embedder_bus.push_message(vex_browser::EmbedderMsg::LoadStatusChanged(
                    tab_id,
                    vex_browser::LoadStatus::Complete,
                ));
                push_history_changed_message(embedder_bus, tab_id, nav_histories);
            }
            vex_browser::EmbedderCommand::GoBack(tab_id) => {
                if !tab_mgr.switch_to(tab_id) {
                    continue;
                }
                let target = nav_histories
                    .get_mut(&tab_id)
                    .and_then(|h| h.back().map(|entry| entry.url.clone()));
                if let Some(url) = target {
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                    embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(tab_id, url));
                    push_history_changed_message(embedder_bus, tab_id, nav_histories);
                }
            }
            vex_browser::EmbedderCommand::GoForward(tab_id) => {
                if !tab_mgr.switch_to(tab_id) {
                    continue;
                }
                let target = nav_histories
                    .get_mut(&tab_id)
                    .and_then(|h| h.forward().map(|entry| entry.url.clone()));
                if let Some(url) = target {
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                    embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(tab_id, url));
                    push_history_changed_message(embedder_bus, tab_id, nav_histories);
                }
            }
            vex_browser::EmbedderCommand::Reload(tab_id) => {
                if !tab_mgr.switch_to(tab_id) {
                    continue;
                }
                reload_active_tab(tab_mgr, vp_w, vp_h);
                embedder_bus.push_message(vex_browser::EmbedderMsg::LoadStatusChanged(
                    tab_id,
                    vex_browser::LoadStatus::Complete,
                ));
            }
            vex_browser::EmbedderCommand::Stop(tab_id) => {
                if !tab_mgr.switch_to(tab_id) {
                    continue;
                }
                tab_mgr.active_tab_mut().stop();
                embedder_bus.push_message(vex_browser::EmbedderMsg::LoadStatusChanged(
                    tab_id,
                    vex_browser::LoadStatus::Complete,
                ));
            }
            vex_browser::EmbedderCommand::NewTab(url_opt) => {
                match url_opt {
                    Some(url) => {
                        let id = tab_mgr.new_tab(url.clone());
                        navigate_tab(tab_mgr, &url, vp_w, vp_h);
                        embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(id, url));
                    }
                    None => {
                        load_welcome_tab(tab_mgr, vp_w, vp_h);
                        let id = tab_mgr.active_tab_id();
                        embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(
                            id,
                            tab_mgr.active_tab().url.clone(),
                        ));
                    }
                }
            }
            vex_browser::EmbedderCommand::CloseTab(tab_id) => {
                if tab_mgr.close_tab(tab_id) {
                    embedder_bus.push_message(vex_browser::EmbedderMsg::TabClosed(tab_id));
                    let active_id = tab_mgr.active_tab_id();
                    embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(
                        active_id,
                        tab_mgr.active_tab().url.clone(),
                    ));
                }
            }
            vex_browser::EmbedderCommand::SwitchTab(tab_id) => {
                if tab_mgr.switch_to(tab_id) {
                    embedder_bus.push_message(vex_browser::EmbedderMsg::UrlChanged(
                        tab_id,
                        tab_mgr.active_tab().url.clone(),
                    ));
                    embedder_bus.push_message(vex_browser::EmbedderMsg::TitleChanged(
                        tab_id,
                        Some(tab_mgr.active_tab().title.clone()),
                    ));
                }
            }
            vex_browser::EmbedderCommand::EvaluateJs(tab_id, code) => {
                let msg = format!("EvaluateJs request for {tab_id}: {} bytes", code.len());
                console_state.log_message(LogLevel::System, &msg);
            }
            vex_browser::EmbedderCommand::SetViewport(tab_id, width, height) => {
                if let Some(tab) = tab_mgr.tab_mut(tab_id) {
                    tab.relayout(vex_core::geometry::Size::new(width, height));
                }
            }
            vex_browser::EmbedderCommand::Scroll(tab_id, dx, dy) => {
                if let Some(tab) = tab_mgr.tab_mut(tab_id) {
                    tab.scroll.scroll_by(dx, dy);
                }
            }
            vex_browser::EmbedderCommand::Shutdown => {
                embedder_bus.push_message(vex_browser::EmbedderMsg::ShutdownComplete);
            }
        }
    }

    // 2) Messages: engine -> embedder/UI reactions.
    while let Some(message) = embedder_bus.pop_message() {
        match message {
            vex_browser::EmbedderMsg::ConsoleMessage(_, level, message) => {
                let log_level = match level {
                    vex_browser::ConsoleLevel::Warn => LogLevel::Warn,
                    vex_browser::ConsoleLevel::Error => LogLevel::Error,
                    vex_browser::ConsoleLevel::Info => LogLevel::Info,
                    vex_browser::ConsoleLevel::Debug => LogLevel::Debug,
                    vex_browser::ConsoleLevel::Log => LogLevel::Log,
                };
                console_state.log_message(log_level, &message);
            }
            vex_browser::EmbedderMsg::ShowDialog(_, request) => match request {
                vex_browser::DialogRequest::Alert(msg) => {
                    console_state.log_message(LogLevel::System, &format!("alert: {msg}"));
                }
                vex_browser::DialogRequest::Confirm(msg) => {
                    console_state.log_message(LogLevel::System, &format!("confirm: {msg}"));
                }
                vex_browser::DialogRequest::Prompt(msg, default) => {
                    let default = default.unwrap_or_default();
                    console_state
                        .log_message(LogLevel::System, &format!("prompt: {msg} (default={default})"));
                }
            },
            other => {
                tracing::debug!(target: "vigo::embedder", "embedder msg: {other:?}");
            }
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        current_process_working_set_bytes, resolve_browser_request_url, write_kpi_snapshot,
    };
    use vex_core::VexUrl;

    #[test]
    fn resolves_absolute_url() {
        let base = VexUrl::parse("https://example.com/page").expect("valid URL");
        let resolved = resolve_browser_request_url("https://vigo.dev/docs", &base)
            .expect("absolute URL should parse");
        assert_eq!(resolved.as_ref(), "https://vigo.dev/docs");
    }

    #[test]
    fn resolves_relative_url_against_base() {
        let base = VexUrl::parse("https://example.com/dir/index.html").expect("valid URL");
        let resolved =
            resolve_browser_request_url("next/page", &base).expect("relative URL should resolve");
        assert_eq!(resolved.as_ref(), "https://example.com/dir/next/page");
    }

    #[test]
    fn rejects_empty_input() {
        let base = VexUrl::parse("https://example.com/").expect("valid URL");
        assert!(resolve_browser_request_url("   ", &base).is_none());
    }

    #[test]
    fn writes_kpi_snapshot_json() {
        let mut path = std::env::temp_dir();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_millis();
        path.push(format!("vigo-kpi-{ts}.json"));

        write_kpi_snapshot(&path, 123.45, 1280.0, 720.0, Some(42));

        let json = std::fs::read_to_string(&path).expect("snapshot file should exist");
        assert!(json.contains("\"cold_start_ms\": 123.45"));
        assert!(json.contains("\"viewport_width\": 1280"));
        assert!(json.contains("\"viewport_height\": 720"));
        assert!(json.contains("\"working_set_bytes\": 42"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reads_current_process_working_set() {
        let value = current_process_working_set_bytes();
        assert!(value.is_some(), "working-set lookup should succeed on Windows");
        assert!(value.expect("has value") > 0);
    }
}

/// Open a new tab with the built-in welcome page.
#[cfg(target_os = "windows")]
fn load_welcome_tab(tab_mgr: &mut vex_browser::TabManager, vp_w: f32, vp_h: f32) {
    tab_mgr.new_tab_blank();
    let viewport = vex_core::geometry::Size::new(vp_w, vp_h);
    let html = vex_browser::internal_pages::newtab_page();
    let tab = tab_mgr.active_tab_mut();
    tab.load_html(&html, viewport);
    tab.url = vex_core::VexUrl::parse("vex://newtab")
        .unwrap_or_else(|_| vex_core::VexUrl::parse("about:blank").expect("about:blank is valid"));
    tab.set_content_size(vp_w, 1400.0);
}

/// Navigate the active tab to a URL. Internal pages use the welcome HTML;
/// download URLs are routed to the download manager (Task 37).
#[cfg(target_os = "windows")]
fn navigate_tab(
    tab_mgr: &mut vex_browser::TabManager,
    url: &vex_core::VexUrl,
    vp_w: f32,
    vp_h: f32,
) {
    let url_str = url.as_ref();
    let viewport = vex_core::geometry::Size::new(vp_w, vp_h);

    // Internal pages.
    if url_str.starts_with("vex://") || url_str == "about:blank" {
        let html = match vex_browser::internal_pages::resolve_internal_url(url_str) {
            Some("settings") => vex_browser::internal_pages::settings_page(),
            Some("history") => vex_browser::internal_pages::history_page(&[]),
            Some("bookmarks") => vex_browser::internal_pages::bookmarks_page(&[]),
            Some("downloads") => vex_browser::internal_pages::downloads_page(&[]),
            _ => vex_browser::internal_pages::newtab_page(),
        };
        let tab = tab_mgr.active_tab_mut();
        tab.url = url.clone();
        tab.load_html(&html, viewport);
        tab.set_content_size(vp_w, 1400.0);
        return;
    }

    // Task 37: detect download URLs by file extension.
    if is_download_url(url_str) {
        let filename = vex_browser::downloads::suggest_filename(url_str);
        tracing::info!("Download detected: {url_str} → {filename}");
        // Actual download wiring will use DownloadManager from caller context.
        return;
    }

    // Real network navigation for HTTP(S).
    if url_str.starts_with("http://") || url_str.starts_with("https://") {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create tokio runtime for navigation: {e}");
                let tab = tab_mgr.active_tab_mut();
                tab.start_load(url.clone());
                tab.load_html(
                    &format!(
                        "<!doctype html><html><head><title>Navigation Error</title></head><body><h1>Navigation Error</h1><p>{e}</p></body></html>"
                    ),
                    viewport,
                );
                tab.set_content_size(vp_w, vp_h);
                return;
            }
        };

        let tab = tab_mgr.active_tab_mut();
        rt.block_on(tab.load_url(url.clone(), viewport));

        let content_height = tab
            .layout
            .as_ref()
            .map(estimated_layout_height)
            .unwrap_or(vp_h)
            .max(vp_h);
        tab.set_content_size(vp_w, content_height + 32.0);
        return;
    }

    // Unknown schemes fallback to an internal error page.
    let tab = tab_mgr.active_tab_mut();
    tab.start_load(url.clone());
    tab.load_html(
        &format!(
            "<!doctype html><html><head><title>Unsupported URL</title></head><body><h1>Unsupported URL scheme</h1><p>{}</p></body></html>",
            url_str
        ),
        viewport,
    );
    tab.set_content_size(vp_w, vp_h);
}

#[cfg(target_os = "windows")]
fn estimated_layout_height(root: &vex_layout::LayoutBox) -> f32 {
    fn walk(node: &vex_layout::LayoutBox, max_bottom: &mut f32) {
        let rect = node.margin_box();
        let bottom = rect.origin.y + rect.size.height;
        if bottom > *max_bottom {
            *max_bottom = bottom;
        }
        for child in &node.children {
            walk(child, max_bottom);
        }
    }

    let mut max_bottom = 0.0;
    walk(root, &mut max_bottom);
    max_bottom
}

/// Reload the active tab.
#[cfg(target_os = "windows")]
fn reload_active_tab(tab_mgr: &mut vex_browser::TabManager, vp_w: f32, vp_h: f32) {
    let url = tab_mgr.active_tab().url.clone();
    navigate_tab(tab_mgr, &url, vp_w, vp_h);
}

/// Check if a URL looks like a direct download.
#[cfg(target_os = "windows")]
fn is_download_url(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_lowercase();
    path.ends_with(".zip")
        || path.ends_with(".exe")
        || path.ends_with(".msi")
        || path.ends_with(".pdf")
        || path.ends_with(".tar.gz")
        || path.ends_with(".7z")
        || path.ends_with(".dmg")
}

// ─── Action handlers ──────────────────────────────────────────────────

/// Handle nav bar click + bookmark bar fallthrough (Tasks 32, 35).
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn handle_nav_action(
    action: vex_browser::ui::nav_bar::NavBarAction,
    tab_mgr: &mut vex_browser::TabManager,
    nav_histories: &mut std::collections::HashMap<
        vex_browser::tab::TabId,
        vex_browser::NavigationHistory,
    >,
    address_bar_focused: &mut bool,
    address_bar_text: &mut String,
    bookmarks: &vex_browser::BookmarkManager,
    show_bookmarks: bool,
    toolbar: &vex_browser::ToolbarLayout,
    fx: f32,
    fy: f32,
    vp_w: f32,
    vp_h: f32,
) {
    use vex_browser::ui::nav_bar::NavBarAction;

    match action {
        NavBarAction::Back => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.back() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
            *address_bar_focused = false;
        }
        NavBarAction::Forward => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.forward() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
            *address_bar_focused = false;
        }
        NavBarAction::ReloadOrStop => {
            let tab = tab_mgr.active_tab_mut();
            if tab.is_loading() {
                tab.stop();
            } else {
                reload_active_tab(tab_mgr, vp_w, vp_h);
            }
            *address_bar_focused = false;
        }
        NavBarAction::AddressBar => {
            *address_bar_focused = true;
            *address_bar_text = tab_mgr.active_tab().url.to_string();
        }
        NavBarAction::None => {
            // Task 35: Bookmark bar clicks.
            if show_bookmarks && toolbar.bookmark_bar.size.height > 0.0 {
                if let Some(url_str) =
                    hit_test_bookmark_bar(fx, fy, bookmarks, toolbar.bookmark_bar)
                {
                    if let Ok(url) = vex_core::VexUrl::parse(&url_str) {
                        navigate_tab(tab_mgr, &url, vp_w, vp_h);
                    }
                    *address_bar_focused = false;
                }
            }
        }
    }
}

/// Execute a BrowserAction from keyboard shortcuts (Task 33).
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn handle_browser_action(
    action: vex_browser::ui::shortcuts::BrowserAction,
    tab_mgr: &mut vex_browser::TabManager,
    nav_histories: &mut std::collections::HashMap<
        vex_browser::tab::TabId,
        vex_browser::NavigationHistory,
    >,
    zoom: &mut vex_browser::ZoomState,
    find: &mut vex_browser::FindState,
    find_bar_visible: &mut bool,
    find_bar_text: &mut String,
    address_bar_focused: &mut bool,
    address_bar_text: &mut String,
    downloads: &mut vex_browser::DownloadManager,
    vp_w: f32,
    vp_h: f32,
) {
    use vex_browser::ui::shortcuts::BrowserAction;
    use vex_core::geometry::Size;

    match action {
        BrowserAction::NewTab => load_welcome_tab(tab_mgr, vp_w, vp_h),
        BrowserAction::CloseTab => {
            let id = tab_mgr.active_tab_id();
            tab_mgr.close_tab(id);
            *address_bar_focused = false;
        }
        BrowserAction::ReopenTab => {
            if let Some(id) = tab_mgr.reopen_closed_tab() {
                let url = tab_mgr.tab(id).map(|t| t.url.clone());
                if let Some(url) = url {
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
        }
        BrowserAction::FocusAddressBar => {
            *address_bar_focused = true;
            *address_bar_text = tab_mgr.active_tab().url.to_string();
        }
        BrowserAction::Reload | BrowserAction::HardReload => {
            reload_active_tab(tab_mgr, vp_w, vp_h);
        }
        BrowserAction::Back => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.back() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
        }
        BrowserAction::Forward => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.forward() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
        }
        BrowserAction::NextTab => {
            tab_mgr.next_tab();
            *address_bar_focused = false;
        }
        BrowserAction::PrevTab => {
            tab_mgr.prev_tab();
            *address_bar_focused = false;
        }
        // Task 36: Find-in-page toggle.
        BrowserAction::FindInPage => {
            *find_bar_visible = !*find_bar_visible;
            if !*find_bar_visible {
                find.clear();
                find_bar_text.clear();
            }
        }
        // Task 38: Zoom with re-layout.
        BrowserAction::ZoomIn => {
            let pct = zoom.zoom_in();
            tracing::info!("Zoom: {pct}%");
            let s = zoom.scale();
            tab_mgr
                .active_tab_mut()
                .relayout(Size::new(vp_w / s, vp_h / s));
        }
        BrowserAction::ZoomOut => {
            let pct = zoom.zoom_out();
            tracing::info!("Zoom: {pct}%");
            let s = zoom.scale();
            tab_mgr
                .active_tab_mut()
                .relayout(Size::new(vp_w / s, vp_h / s));
        }
        BrowserAction::ZoomReset => {
            zoom.reset();
            tracing::info!("Zoom: 100%");
            tab_mgr.active_tab_mut().relayout(Size::new(vp_w, vp_h));
        }
        BrowserAction::Stop => tab_mgr.active_tab_mut().stop(),
        BrowserAction::Downloads => {
            tracing::info!("Downloads: {} total", downloads.count());
            if let Ok(url) = vex_core::VexUrl::parse("vex://downloads") {
                navigate_tab(tab_mgr, &url, vp_w, vp_h);
            }
        }
        BrowserAction::Bookmarks => {
            if let Ok(url) = vex_core::VexUrl::parse("vex://bookmarks") {
                navigate_tab(tab_mgr, &url, vp_w, vp_h);
            }
        }
        BrowserAction::History => {
            if let Ok(url) = vex_core::VexUrl::parse("vex://history") {
                navigate_tab(tab_mgr, &url, vp_w, vp_h);
            }
        }
        BrowserAction::Settings => {
            if let Ok(url) = vex_core::VexUrl::parse("vex://settings") {
                navigate_tab(tab_mgr, &url, vp_w, vp_h);
            }
        }
        BrowserAction::DevTools | BrowserAction::Fullscreen => {
            tracing::info!("{action:?} (UI not yet wired)");
        }
    }
}

/// Handle a context menu action (Task 34).
#[cfg(target_os = "windows")]
fn handle_menu_action(
    action: &vex_browser::ui::context_menu::MenuAction,
    tab_mgr: &mut vex_browser::TabManager,
    nav_histories: &mut std::collections::HashMap<
        vex_browser::tab::TabId,
        vex_browser::NavigationHistory,
    >,
    vp_w: f32,
    vp_h: f32,
) {
    use vex_browser::ui::context_menu::MenuAction;
    match action {
        MenuAction::Back => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.back() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
        }
        MenuAction::Forward => {
            let tab_id = tab_mgr.active_tab_id();
            if let Some(h) = nav_histories.get_mut(&tab_id) {
                if let Some(entry) = h.forward() {
                    let url = entry.url.clone();
                    navigate_tab(tab_mgr, &url, vp_w, vp_h);
                }
            }
        }
        MenuAction::Reload => reload_active_tab(tab_mgr, vp_w, vp_h),
        _ => tracing::debug!("Context menu: {action:?}"),
    }
}

/// Handle a click inside the page content area: DOM events, focus, and link actions.
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn handle_content_click(
    tab_mgr: &mut vex_browser::TabManager,
    nav_histories: &mut std::collections::HashMap<
        vex_browser::tab::TabId,
        vex_browser::NavigationHistory,
    >,
    focused_node: &mut Option<vex_core::VexId>,
    mouse_x: f32,
    mouse_y: f32,
    content_origin_x: f32,
    content_origin_y: f32,
    vp_w: f32,
    vp_h: f32,
) {
    let local_x = (mouse_x - content_origin_x).max(0.0);
    let local_y = (mouse_y - content_origin_y).max(0.0);

    // Determine hit target for focus updates.
    let hit_target = {
        let tab = tab_mgr.active_tab();
        tab.layout.as_ref().and_then(|layout| {
            vex_layout::hit_test(
                layout,
                local_x + tab.scroll.offset_x,
                local_y + tab.scroll.offset_y,
            )
        })
    };

    let click_result = {
        let tab = tab_mgr.active_tab_mut();
        match (&tab.layout, &tab.shared_doc, tab.runtime.as_mut()) {
            (Some(layout), Some(shared), Some(runtime)) => {
                let doc = shared.borrow();
                Some(vex_browser::event_handler::process_click(
                    local_x,
                    local_y,
                    layout,
                    &tab.scroll,
                    &doc,
                    shared,
                    &tab.url,
                    runtime,
                ))
            }
            _ => None,
        }
    };

    // Focus follows click target (if any).
    if let Some(target) = hit_target {
        let maybe_focus = {
            let tab = tab_mgr.active_tab_mut();
            match (&tab.shared_doc, tab.runtime.as_mut()) {
                (Some(shared), Some(runtime)) => vex_browser::event_handler::process_focus_change(
                    target,
                    *focused_node,
                    shared,
                    runtime,
                ),
                _ => None,
            }
        };
        if let Some(new_focus) = maybe_focus {
            *focused_node = Some(new_focus);
        }
    } else {
        *focused_node = None;
    }

    match click_result {
        Some(vex_browser::event_handler::ClickResult::Navigate(
            vex_browser::links::LinkAction::Navigate(url),
        )) => {
            let tab_id = tab_mgr.active_tab_id();
            let title = tab_mgr.active_tab().title.clone();
            let scroll = vex_core::geometry::Point::new(
                tab_mgr.active_tab().scroll.offset_x,
                tab_mgr.active_tab().scroll.offset_y,
            );
            nav_histories
                .entry(tab_id)
                .or_default()
                .push(url.clone(), title, scroll);
            navigate_tab(tab_mgr, &url, vp_w, vp_h);
            *focused_node = None;
        }
        Some(vex_browser::event_handler::ClickResult::Navigate(
            vex_browser::links::LinkAction::NewTab(url),
        )) => {
            tab_mgr.new_tab(url.clone());
            let tab_id = tab_mgr.active_tab_id();
            nav_histories.entry(tab_id).or_default().push(
                url.clone(),
                "New Tab".to_string(),
                vex_core::geometry::Point::new(0.0, 0.0),
            );
            navigate_tab(tab_mgr, &url, vp_w, vp_h);
            *focused_node = None;
        }
        Some(vex_browser::event_handler::ClickResult::Navigate(
            vex_browser::links::LinkAction::RunScript(script),
        )) => {
            let script_ran = {
                let tab = tab_mgr.active_tab_mut();
                if let Some(runtime) = tab.runtime.as_mut() {
                    match runtime.execute(&script) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!("javascript: URL execution failed: {e}");
                            false
                        }
                    }
                } else {
                    false
                }
            };

            if script_ran {
                let tab = tab_mgr.active_tab_mut();
                tab.relayout(vex_core::geometry::Size::new(vp_w, vp_h));
                let content_height = tab
                    .layout
                    .as_ref()
                    .map(estimated_layout_height)
                    .unwrap_or(vp_h)
                    .max(vp_h);
                tab.set_content_size(vp_w, content_height + 32.0);
            }
        }
        Some(vex_browser::event_handler::ClickResult::Handled)
        | Some(vex_browser::event_handler::ClickResult::Miss)
        | None => {}
        Some(vex_browser::event_handler::ClickResult::Navigate(
            vex_browser::links::LinkAction::None,
        )) => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Frame composition
// ═══════════════════════════════════════════════════════════════════════

/// Compose the full frame: tab bar + nav bar + bookmarks + content + overlays + DevTools.
#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn compose_frame(
    tab_mgr: &vex_browser::TabManager,
    toolbar: &vex_browser::ToolbarLayout,
    bookmarks: &vex_browser::BookmarkManager,
    find: &vex_browser::FindState,
    find_bar_visible: bool,
    find_bar_text: &str,
    context_menu: &Option<vex_browser::ui::context_menu::ContextMenu>,
    address_bar_focused: bool,
    address_bar_text: &str,
    nav_histories: &std::collections::HashMap<
        vex_browser::tab::TabId,
        vex_browser::NavigationHistory,
    >,
    settings: &vex_browser::BrowserSettings,
    zoom: &vex_browser::ZoomState,
    vp_w: f32,
    devtools: &vex_browser::DevToolsState,
    console_state: &vex_browser::devtools::console::ConsoleState,
    perf_state: &vex_browser::devtools::performance::PerformanceState,
    sources_state: &vex_browser::devtools::sources::SourcesState,
) -> vex_render::display_list::DisplayList {
    use vex_browser::ui::nav_bar::{address_cursor_x, render_nav_bar, NavBarState};
    use vex_browser::ui::tab_bar::render_tab_bar;
    use vex_core::color::Color;
    use vex_core::geometry::{Point, Rect};
    use vex_render::display_list::{DisplayCommand, DisplayList};

    let tab = tab_mgr.active_tab();
    let tab_id = tab_mgr.active_tab_id();
    let page_cap = tab
        .display_list
        .as_ref()
        .map_or(0, |dl| dl.commands().len());
    let mut dl = DisplayList::with_capacity(32 + page_cap);

    // ── Tab bar (Task 31) ───────────────────────────────────────
    render_tab_bar(
        &mut dl,
        tab_mgr.tabs(),
        tab_mgr.active_index(),
        toolbar.tab_bar,
    );

    // ── Nav bar (Task 32) ───────────────────────────────────────
    let history = nav_histories.get(&tab_id);
    let url_display = if address_bar_focused {
        address_bar_text
    } else {
        tab.url.as_ref()
    };
    let nav_state = NavBarState {
        url: url_display,
        can_go_back: history.is_some_and(|h| h.can_go_back()),
        can_go_forward: history.is_some_and(|h| h.can_go_forward()),
        is_loading: tab.is_loading(),
        is_https: tab.url.as_ref().starts_with("https://"),
        is_address_focused: address_bar_focused,
    };
    render_nav_bar(&mut dl, &nav_state, toolbar.nav_bar);

    // Cursor blink when address bar is focused.
    if address_bar_focused {
        let cursor_x = address_cursor_x(toolbar.nav_bar, address_bar_text);
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(
                cursor_x.min(vp_w - 40.0),
                toolbar.nav_bar.origin.y + 12.0,
                1.0,
                16.0,
            ),
            color: Color::rgb(200, 200, 220),
            border_radius: 0.0,
        });
    }

    // ── Bookmark bar (Task 35) ──────────────────────────────────
    if settings.show_bookmarks_bar && toolbar.bookmark_bar.size.height > 0.0 {
        render_bookmark_bar(&mut dl, bookmarks, toolbar.bookmark_bar);
    }

    // ── Accent line ─────────────────────────────────────────────
    dl.push(DisplayCommand::FillRect {
        rect: toolbar.accent_line,
        color: Color::rgb(124, 88, 255),
        border_radius: 0.0,
    });

    // ── Page content (clipped + scrolled) ───────────────────────
    let content_y = toolbar.content_area.origin.y;
    let scroll_y = tab.scroll.offset_y;

    dl.push(DisplayCommand::PushClip {
        rect: toolbar.content_area,
    });

    if let Some(ref page_dl) = tab.display_list {
        for cmd in page_dl.commands() {
            dl.push(offset_command(cmd, 0.0, content_y - scroll_y));
        }
    }

    dl.push(DisplayCommand::PopClip);

    // ── Find bar overlay (Task 36) ──────────────────────────────
    if find_bar_visible {
        if let Some(ref find_rect) = toolbar.find_bar {
            render_find_bar(&mut dl, find, find_bar_text, *find_rect);
        }
    }

    // ── Zoom indicator (Task 38) ────────────────────────────────
    if !zoom.is_default() {
        let text = format!("{}%", zoom.percent());
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(vp_w - 70.0, content_y + 8.0, 60.0, 22.0),
            color: Color::rgb(30, 30, 40),
            border_radius: 0.0,
        });
        dl.push(DisplayCommand::DrawText {
            position: Point::new(vp_w - 65.0, content_y + 12.0),
            text,
            color: Color::rgb(180, 180, 200),
            font_size: 12.0,
            line_height: 16.0,
        });
    }

    // ── DevTools panel (Tasks 51-56) ──────────────────────────
    if devtools.is_open() {
        let vp_h = toolbar.content_area.origin.y + toolbar.content_area.size.height;
        let available = Rect::new(0.0, toolbar.content_area.origin.y, vp_w, vp_h);
        let dt_layout = devtools.layout(available);

        // Background for DevTools area.
        dl.push(DisplayCommand::FillRect {
            rect: dt_layout.devtools,
            color: Color::rgb(30, 30, 38),
            border_radius: 0.0,
        });

        // Tab bar.
        dl.push(DisplayCommand::FillRect {
            rect: dt_layout.tab_bar,
            color: Color::rgb(38, 38, 48),
            border_radius: 0.0,
        });
        let tab_count = vex_browser::DevToolsPanel::ALL.len();
        let tab_w = dt_layout.tab_bar.size.width / tab_count as f32;
        for (i, &panel) in vex_browser::DevToolsPanel::ALL.iter().enumerate() {
            let tx = dt_layout.tab_bar.origin.x + i as f32 * tab_w;
            let is_active = devtools.active_panel() == panel;
            if is_active {
                dl.push(DisplayCommand::FillRect {
                    rect: Rect::new(
                        tx,
                        dt_layout.tab_bar.origin.y,
                        tab_w,
                        dt_layout.tab_bar.size.height,
                    ),
                    color: Color::rgb(50, 50, 65),
                    border_radius: 0.0,
                });
            }
            dl.push(DisplayCommand::DrawText {
                position: Point::new(tx + 8.0, dt_layout.tab_bar.origin.y + 8.0),
                text: panel.label().to_owned(),
                color: if is_active {
                    Color::rgb(220, 220, 240)
                } else {
                    Color::rgb(140, 140, 160)
                },
                font_size: 12.0,
                line_height: 16.0,
            });
        }

        // Panel body content.
        let body = dt_layout.panel_body;
        dl.push(DisplayCommand::PushClip { rect: body });

        match devtools.active_panel() {
            // Task 51: Elements panel — show DOM tree.
            vex_browser::DevToolsPanel::Elements => {
                if let Some(doc) = tab_mgr.active_tab().borrow_document() {
                    let tree_state = vex_browser::devtools::elements::DomTreeState::new();
                    let rows = tree_state.build_rows(&doc);
                    let mut row_y = body.origin.y + 4.0;
                    for row in rows.iter().take(30) {
                        let indent = row.depth as f32 * 16.0;
                        dl.push(DisplayCommand::DrawText {
                            position: Point::new(body.origin.x + 8.0 + indent, row_y),
                            text: row.label.clone(),
                            color: if row.is_closing_tag {
                                Color::rgb(120, 120, 150)
                            } else {
                                Color::rgb(200, 150, 100)
                            },
                            font_size: 12.0,
                            line_height: 16.0,
                        });
                        row_y += 18.0;
                        if row_y > body.origin.y + body.size.height {
                            break;
                        }
                    }
                }
            }
            // Task 53: Console panel — show log entries.
            vex_browser::DevToolsPanel::Console => {
                let entries = console_state.visible_entries();
                let mut entry_y = body.origin.y + 4.0;
                for entry in entries.iter().rev().take(30) {
                    let color = match entry.level {
                        vex_browser::devtools::console::LogLevel::Error => Color::rgb(220, 80, 80),
                        vex_browser::devtools::console::LogLevel::Warn => Color::rgb(220, 180, 60),
                        vex_browser::devtools::console::LogLevel::Info => Color::rgb(100, 180, 220),
                        vex_browser::devtools::console::LogLevel::Debug => {
                            Color::rgb(120, 120, 140)
                        }
                        _ => Color::rgb(200, 200, 210),
                    };
                    let summary = entry.summary();
                    let display = if summary.len() > 100 {
                        format!("{}…", &summary[..100])
                    } else {
                        summary
                    };
                    dl.push(DisplayCommand::DrawText {
                        position: Point::new(body.origin.x + 8.0, entry_y),
                        text: format!("[{}] {}", entry.level.label(), display),
                        color,
                        font_size: 12.0,
                        line_height: 16.0,
                    });
                    entry_y += 18.0;
                    if entry_y > body.origin.y + body.size.height {
                        break;
                    }
                }
                if entries.is_empty() {
                    dl.push(DisplayCommand::DrawText {
                        position: Point::new(body.origin.x + 8.0, body.origin.y + 4.0),
                        text: "No console output".to_owned(),
                        color: Color::rgb(100, 100, 120),
                        font_size: 12.0,
                        line_height: 16.0,
                    });
                }
            }
            // Task 54: Network panel — show requests.
            vex_browser::DevToolsPanel::Network => {
                dl.push(DisplayCommand::DrawText {
                    position: Point::new(body.origin.x + 8.0, body.origin.y + 4.0),
                    text: "Network requests will appear here during page loads.".to_owned(),
                    color: Color::rgb(140, 140, 160),
                    font_size: 12.0,
                    line_height: 16.0,
                });
            }
            // Task 56: Sources panel — show page source.
            vex_browser::DevToolsPanel::Sources => {
                if let Some(source) = sources_state.active_source() {
                    let lines = source.lines(1, 40);
                    let mut ly = body.origin.y + 4.0;
                    for (num, line) in &lines {
                        dl.push(DisplayCommand::DrawText {
                            position: Point::new(body.origin.x + 8.0, ly),
                            text: format!("{num:>4} │ {line}"),
                            color: Color::rgb(180, 180, 200),
                            font_size: 11.0,
                            line_height: 14.0,
                        });
                        ly += 16.0;
                        if ly > body.origin.y + body.size.height {
                            break;
                        }
                    }
                } else {
                    dl.push(DisplayCommand::DrawText {
                        position: Point::new(body.origin.x + 8.0, body.origin.y + 4.0),
                        text: "No sources loaded.".to_owned(),
                        color: Color::rgb(140, 140, 160),
                        font_size: 12.0,
                        line_height: 16.0,
                    });
                }
            }
            // Task 55: Performance panel — show frame timing.
            vex_browser::DevToolsPanel::Performance => {
                let stats = perf_state.stats();
                dl.push(DisplayCommand::DrawText {
                    position: Point::new(body.origin.x + 8.0, body.origin.y + 4.0),
                    text: stats.summary(),
                    color: Color::rgb(180, 200, 220),
                    font_size: 12.0,
                    line_height: 16.0,
                });
                // Show last 30 frame bars.
                let frames = perf_state.frames();
                let max_time = perf_state.max_frame_time();
                let bar_area_y = body.origin.y + 30.0;
                let bar_area_h = (body.size.height - 40.0).max(20.0);
                let bar_w = 6.0;
                let start = frames.len().saturating_sub(60);
                for (i, frame) in frames[start..].iter().enumerate() {
                    let total_ms = frame.total().as_secs_f32() * 1000.0;
                    let max_ms = max_time.as_secs_f32() * 1000.0;
                    let ratio = if max_ms > 0.0 {
                        (total_ms / max_ms).min(1.0)
                    } else {
                        0.0
                    };
                    let h = ratio * bar_area_h;
                    let bx = body.origin.x + 8.0 + i as f32 * (bar_w + 2.0);
                    let by = bar_area_y + bar_area_h - h;
                    let color = if frame.is_slow() {
                        Color::rgb(220, 80, 80)
                    } else {
                        Color::rgb(80, 180, 120)
                    };
                    dl.push(DisplayCommand::FillRect {
                        rect: Rect::new(bx, by, bar_w, h),
                        color,
                        border_radius: 0.0,
                    });
                    if bx + bar_w > body.origin.x + body.size.width - 10.0 {
                        break;
                    }
                }
            }
        }

        dl.push(DisplayCommand::PopClip);
    }

    // ── Context menu (Task 34, on top of everything) ────────────
    if let Some(ref menu) = context_menu {
        menu.render(&mut dl);
    }

    dl
}

// ─── toolbar sub-renderers ─────────────────────────────────────────────

/// Render the bookmark bar (Task 35).
#[cfg(target_os = "windows")]
fn render_bookmark_bar(
    dl: &mut vex_render::display_list::DisplayList,
    bookmarks: &vex_browser::BookmarkManager,
    rect: vex_core::geometry::Rect,
) {
    use vex_core::color::Color;
    use vex_core::geometry::{Point, Rect};
    use vex_render::display_list::DisplayCommand;

    dl.push(DisplayCommand::FillRect {
        rect,
        color: Color::rgb(18, 21, 30),
        border_radius: 0.0,
    });

    // Subtle top separator for depth.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(rect.origin.x, rect.origin.y, rect.size.width, 1.0),
        color: Color::rgb(44, 49, 66),
        border_radius: 0.0,
    });

    let items = bookmarks.bar_bookmarks();
    let mut x = rect.origin.x + 8.0;
    for bm in items.iter().take(12) {
        let title = clamp_title_for_chip(&bm.title);
        let w = bookmark_chip_width(&title);
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(x, rect.origin.y + 4.0, w, 22.0),
            color: Color::rgb(34, 39, 54),
            border_radius: 9.0,
        });

        // Small glyph marker.
        dl.push(DisplayCommand::DrawText {
            position: Point::new(x + 7.0, rect.origin.y + 8.0),
            text: "•".into(),
            color: Color::rgb(140, 146, 176),
            font_size: 11.0,
            line_height: 14.0,
        });

        dl.push(DisplayCommand::DrawText {
            position: Point::new(x + 14.0, rect.origin.y + 8.0),
            text: title,
            color: Color::rgb(216, 220, 235),
            font_size: 11.0,
            line_height: 14.0,
        });
        x += w + 4.0;
        if x > rect.origin.x + rect.size.width - 40.0 {
            break;
        }
    }
}

/// Hit-test bookmark bar. Returns the URL string if a bookmark was clicked.
#[cfg(target_os = "windows")]
fn hit_test_bookmark_bar(
    x: f32,
    y: f32,
    bookmarks: &vex_browser::BookmarkManager,
    rect: vex_core::geometry::Rect,
) -> Option<String> {
    if y < rect.origin.y || y > rect.origin.y + rect.size.height {
        return None;
    }
    let items = bookmarks.bar_bookmarks();
    let mut bx = rect.origin.x + 8.0;
    for bm in items.iter().take(12) {
        let title = clamp_title_for_chip(&bm.title);
        let w = bookmark_chip_width(&title);
        if x >= bx && x <= bx + w && y >= rect.origin.y + 4.0 && y <= rect.origin.y + 26.0 {
            return Some(bm.url.clone());
        }
        bx += w + 4.0;
        if bx > rect.origin.x + rect.size.width - 40.0 {
            break;
        }
    }
    None
}

/// Render the find-in-page bar (Task 36).
#[cfg(target_os = "windows")]
fn render_find_bar(
    dl: &mut vex_render::display_list::DisplayList,
    find: &vex_browser::FindState,
    query: &str,
    rect: vex_core::geometry::Rect,
) {
    use vex_core::color::Color;
    use vex_core::geometry::{Point, Rect};
    use vex_render::display_list::DisplayCommand;

    dl.push(DisplayCommand::FillRect {
        rect,
        color: Color::rgb(27, 30, 43),
        border_radius: 0.0,
    });

    // Input area
    let input_rect = Rect::new(rect.origin.x + 12.0, rect.origin.y + 6.0, 300.0, 24.0);
    dl.push(DisplayCommand::FillRect {
        rect: input_rect,
        color: Color::rgb(70, 82, 130),
        border_radius: 8.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            input_rect.origin.x + 1.0,
            input_rect.origin.y + 1.0,
            input_rect.size.width - 2.0,
            input_rect.size.height - 2.0,
        ),
        color: Color::rgb(20, 23, 33),
        border_radius: 7.0,
    });

    let text = if query.is_empty() {
        "Find in page…"
    } else {
        query
    };
    dl.push(DisplayCommand::DrawText {
        position: Point::new(rect.origin.x + 20.0, rect.origin.y + 11.0),
        text: text.to_string(),
        color: if query.is_empty() {
            Color::rgb(112, 118, 145)
        } else {
            Color::rgb(210, 210, 225)
        },
        font_size: 12.0,
        line_height: 16.0,
    });

    let status = find.status();
    if !status.is_empty() {
        dl.push(DisplayCommand::DrawText {
            position: Point::new(rect.origin.x + 326.0, rect.origin.y + 11.0),
            text: status,
            color: Color::rgb(160, 166, 194),
            font_size: 12.0,
            line_height: 16.0,
        });
    }

    // Keyboard hints.
    dl.push(DisplayCommand::DrawText {
        position: Point::new(
            rect.origin.x + rect.size.width - 180.0,
            rect.origin.y + 11.0,
        ),
        text: "Enter/Shift+Enter to navigate".into(),
        color: Color::rgb(130, 136, 164),
        font_size: 11.0,
        line_height: 14.0,
    });

    // Close button
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            rect.origin.x + rect.size.width - 34.0,
            rect.origin.y + 8.0,
            22.0,
            20.0,
        ),
        color: Color::rgb(40, 44, 60),
        border_radius: 6.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(rect.origin.x + rect.size.width - 27.0, rect.origin.y + 10.0),
        text: "✕".into(),
        color: Color::rgb(180, 186, 210),
        font_size: 13.0,
        line_height: 16.0,
    });
}

#[cfg(target_os = "windows")]
fn clamp_title_for_chip(title: &str) -> String {
    if title.len() > 16 {
        format!("{}…", &title[..15])
    } else {
        title.to_string()
    }
}

#[cfg(target_os = "windows")]
fn bookmark_chip_width(clamped_title: &str) -> f32 {
    (clamped_title.len() as f32 * 7.0 + 24.0).min(170.0)
}

// ═══════════════════════════════════════════════════════════════════════
//  Platform event mapping
// ═══════════════════════════════════════════════════════════════════════

/// Convert a Win32 VK code + modifier bits → `Shortcut`.
#[cfg(target_os = "windows")]
fn platform_to_shortcut(
    keycode: u32,
    modifiers: u32,
) -> Option<vex_browser::ui::shortcuts::Shortcut> {
    use vex_browser::ui::shortcuts::{Key, Shortcut};

    let key = match keycode {
        0x09 => Key::Tab,
        0x0D => Key::Enter,
        0x1B => Key::Escape,
        0x08 => Key::Backspace,
        0x25 => Key::Left,
        0x27 => Key::Right,
        0xBB => Key::Plus,
        0xBD => Key::Minus,
        0x30 => Key::Zero,
        0x41..=0x5A => Key::Char(keycode as u8 as char), // A-Z
        0x70..=0x7B => Key::F((keycode - 0x70 + 1) as u8),
        _ => return None,
    };

    Some(Shortcut {
        ctrl: modifiers & 0x01 != 0,
        shift: modifiers & 0x02 != 0,
        alt: modifiers & 0x04 != 0,
        key,
    })
}

/// Convert a Win32 VK code to a character for text input.
#[cfg(target_os = "windows")]
fn vk_to_char(keycode: u32, shift: bool) -> Option<char> {
    match keycode {
        0x41..=0x5A => {
            let ch = (keycode as u8 - 0x41 + b'a') as char;
            Some(if shift { ch.to_ascii_uppercase() } else { ch })
        }
        0x30..=0x39 if shift => Some(b")!@#$%^&*("[(keycode - 0x30) as usize] as char),
        0x30..=0x39 => Some(keycode as u8 as char),
        0xBE => Some('.'),
        0xBA => Some(if shift { ':' } else { ';' }),
        0xBF => Some(if shift { '?' } else { '/' }),
        0xBD => Some(if shift { '_' } else { '-' }),
        0xBB => Some(if shift { '+' } else { '=' }),
        0xBC => Some(if shift { '<' } else { ',' }),
        0x20 => Some(' '),
        _ => None,
    }
}

/// Shift a display command by `(dx, dy)`.
#[cfg(target_os = "windows")]
fn offset_command(
    cmd: &vex_render::display_list::DisplayCommand,
    dx: f32,
    dy: f32,
) -> vex_render::display_list::DisplayCommand {
    use vex_core::geometry::{Insets, Point, Rect};
    use vex_render::display_list::DisplayCommand;

    match cmd {
        DisplayCommand::FillRect {
            rect,
            color,
            border_radius,
        } => DisplayCommand::FillRect {
            rect: Rect::new(
                rect.origin.x + dx,
                rect.origin.y + dy,
                rect.size.width,
                rect.size.height,
            ),
            color: *color,
            border_radius: *border_radius,
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
            widths: Insets::new(widths.top, widths.right, widths.bottom, widths.left),
            colors: *colors,
            styles: *styles,
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
        DisplayCommand::PopClip => DisplayCommand::PopClip,
        DisplayCommand::PushOpacity { opacity } => {
            DisplayCommand::PushOpacity { opacity: *opacity }
        }
        DisplayCommand::PopOpacity => DisplayCommand::PopOpacity,
    }
}
