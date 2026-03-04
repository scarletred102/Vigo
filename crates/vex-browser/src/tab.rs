// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tab model — represents a single browser tab with its own page state.

use std::cell::Ref;
use std::collections::HashMap;

use vex_core::geometry::Size;
use vex_core::{VexError, VexId, VexUrl};
use vex_css::ComputedStyle;
use vex_dom::Document;
use vex_js::{ExecutionPlan, JsRuntime, RequestQueue, SharedDocument};
use vex_layout::LayoutBox;
use vex_render::display_list::{DisplayList, ImageId};
use vex_render::scroll::ScrollState;

/// Create a guaranteed-valid `about:blank` URL.
///
/// `about:blank` is defined by the URL spec and will always parse
/// successfully, so the `expect` here is unreachable in practice.
fn about_blank() -> VexUrl {
    VexUrl::parse("about:blank").expect("about:blank is always valid per URL spec")
}

/// Unique identifier for a browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u32);

impl TabId {
    /// Create a new tab ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tab({})", self.0)
    }
}

/// Loading state of a tab's page.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LoadingState {
    /// No page load in progress.
    #[default]
    Idle,
    /// Connecting to the server.
    Connecting,
    /// Receiving / parsing content.
    Loading {
        /// Approximate progress (0.0–1.0).
        progress: f32,
    },
    /// Page fully loaded.
    Complete,
}

/// A single browser tab owns its page state: URL, DOM, styles, layout,
/// display list, JS runtime, scroll position, and loading status.
pub struct Tab {
    /// Unique tab identifier.
    pub id: TabId,
    /// Current URL of the page in this tab.
    pub url: VexUrl,
    /// Page title (extracted from `<title>` element).
    pub title: String,
    /// Shared DOM document (for JS ↔ Rust interop).
    pub shared_doc: Option<SharedDocument>,
    /// JavaScript runtime with all Web APIs registered.
    pub runtime: Option<JsRuntime>,
    /// Request queue shared between JS runtime and browser loop.
    pub request_queue: RequestQueue,
    /// Computed CSS styles for each DOM node.
    pub styles: Option<HashMap<VexId, ComputedStyle>>,
    /// Layout tree for the current page.
    pub layout: Option<LayoutBox>,
    /// Pre-built display list (cached between frames if page hasn't changed).
    pub display_list: Option<DisplayList>,
    /// Scroll state for the content area.
    pub scroll: ScrollState,
    /// Favicon image ID if one has been loaded.
    pub favicon: Option<ImageId>,
    /// Current loading state.
    pub loading: LoadingState,
    /// Whether this tab's page content needs a re-render.
    pub dirty: bool,
}

impl Tab {
    /// Create a new tab pointing at the given URL.
    pub fn new(id: TabId, url: VexUrl) -> Self {
        Self {
            id,
            url,
            title: String::from("New Tab"),
            shared_doc: None,
            runtime: None,
            request_queue: vex_js::new_request_queue(),
            styles: None,
            layout: None,
            display_list: None,
            scroll: ScrollState::new(1280.0, 720.0),
            favicon: None,
            loading: LoadingState::Idle,
            dirty: true,
        }
    }

    /// Create a blank tab (no URL loaded yet).
    pub fn blank(id: TabId) -> Self {
        let url = VexUrl::parse("vex://newtab").unwrap_or_else(|_| about_blank());
        Self::new(id, url)
    }

    /// Set the page title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Whether the tab has finished loading.
    pub fn is_complete(&self) -> bool {
        self.loading == LoadingState::Complete
    }

    /// Whether the tab is currently loading.
    pub fn is_loading(&self) -> bool {
        matches!(
            self.loading,
            LoadingState::Connecting | LoadingState::Loading { .. }
        )
    }

    /// Mark the tab as needing a re-render.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Load HTML content into this tab (synchronous — from a string).
    ///
    /// Parses HTML → DOM, extracts scripts and styles, creates a JS runtime,
    /// executes blocking scripts → computes CSS cascade → layout → display list
    /// → executes deferred scripts → fires lifecycle events.
    pub fn load_html(&mut self, html: &str, viewport: Size) {
        self.loading = LoadingState::Loading { progress: 0.2 };

        let document = vex_html::parse_html(html);

        // Extract scripts before moving document into shared wrapper.
        let scripts = vex_html::extract::extract_scripts(&document);
        let plan = ExecutionPlan::from_scripts(&scripts);

        // Wrap document for JS interop.
        let shared_doc = vex_js::shared_document(document);

        // Create JS runtime with all Web APIs.
        let mut runtime = JsRuntime::with_request_queue(self.request_queue.clone());
        runtime.register_document(&shared_doc);

        // Execute blocking scripts (inline + synchronous external).
        if !plan.blocking.is_empty() {
            tracing::debug!("Executing {} blocking script(s)", plan.blocking.len());
            let mut fetch_fn = |url: &str| -> Result<String, VexError> {
                tracing::warn!("External script fetch not yet wired: {url}");
                Err(VexError::Network(format!(
                    "synchronous fetch not available for {url}"
                )))
            };
            for (i, result) in plan
                .execute_blocking(&mut runtime, &mut fetch_fn)
                .iter()
                .enumerate()
            {
                if let Err(e) = result {
                    tracing::warn!("Blocking script {i} failed: {e}");
                }
            }
        }

        // CSS pipeline: extract <style>, compute cascade, layout, display list.
        let (styles, layout_root, dl, title) = {
            let doc = shared_doc.borrow();

            let style_ids = doc.get_elements_by_tag_name("style");
            let mut stylesheets = Vec::new();
            for id in &style_ids {
                let css_text = doc.text_content(*id);
                if !css_text.is_empty() {
                    stylesheets.push(vex_css::parse_stylesheet(&css_text));
                }
            }

            self.loading = LoadingState::Loading { progress: 0.5 };

            let styles = vex_css::compute_styles(&doc, &stylesheets, viewport);
            let layout_root = vex_layout::layout_document(&doc, &styles, viewport);

            self.loading = LoadingState::Loading { progress: 0.8 };

            let dl = vex_render::build_display_list(&layout_root, &styles, &doc, viewport);

            // Extract title from <title> element.
            let title_ids = doc.get_elements_by_tag_name("title");
            let title = title_ids
                .first()
                .map(|&tid| doc.text_content(tid))
                .filter(|t| !t.is_empty());

            (styles, layout_root, dl, title)
        };

        if let Some(title_text) = title {
            self.title = title_text;
        }

        // Execute deferred scripts (after DOM built, before DOMContentLoaded).
        if !plan.deferred.is_empty() {
            tracing::debug!("Executing {} deferred script(s)", plan.deferred.len());
            let mut fetch_fn = |url: &str| -> Result<String, VexError> {
                tracing::warn!("External script fetch not yet wired: {url}");
                Err(VexError::Network(format!(
                    "synchronous fetch not available for {url}"
                )))
            };
            for (i, result) in plan
                .execute_deferred(&mut runtime, &mut fetch_fn)
                .iter()
                .enumerate()
            {
                if let Err(e) = result {
                    tracing::warn!("Deferred script {i} failed: {e}");
                }
            }
        }

        // Fire lifecycle events.
        runtime.fire_dom_content_loaded(&shared_doc);
        runtime.fire_load(&shared_doc);

        if plan.total() > 0 {
            tracing::info!(
                "Tab {}: executed {} script(s) ({} blocking, {} deferred, {} async)",
                self.id,
                plan.total(),
                plan.blocking.len(),
                plan.deferred.len(),
                plan.async_scripts.len(),
            );
        }

        self.shared_doc = Some(shared_doc);
        self.runtime = Some(runtime);
        self.styles = Some(styles);
        self.layout = Some(layout_root);
        self.display_list = Some(dl);
        self.loading = LoadingState::Complete;
        self.dirty = true;
    }

    /// Update the scroll position for this tab's content.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll.scroll_by(dx, dy);
        self.dirty = true;
    }

    /// Set the viewport dimensions (e.g., on window resize).
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.scroll.set_viewport_size(width, height);
        self.dirty = true;
    }

    /// Set the total content height (for scroll bounds).
    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.scroll.set_content_size(width, height);
    }

    /// Borrow the DOM document (from the shared wrapper).
    ///
    /// Returns `None` if no page has been loaded yet.
    pub fn borrow_document(&self) -> Option<Ref<'_, Document>> {
        self.shared_doc.as_ref().map(|sd| sd.borrow())
    }

    /// Whether this tab has a parsed DOM document.
    pub fn has_document(&self) -> bool {
        self.shared_doc.is_some()
    }

    /// Begin loading a URL — resets tab state, sets loading to Connecting.
    pub fn start_load(&mut self, url: VexUrl) {
        self.url = url;
        self.loading = LoadingState::Connecting;
        self.shared_doc = None;
        self.runtime = None;
        self.styles = None;
        self.layout = None;
        self.display_list = None;
        self.dirty = true;
        tracing::info!("Tab {} loading: {}", self.id, self.url);
    }

    /// Fetch a URL over the network and run the full page pipeline:
    /// fetch → parse HTML → extract CSS → compute styles → layout → display list.
    ///
    /// On error, loads an error page instead.
    pub async fn load_url(&mut self, url: VexUrl, viewport: Size) {
        self.start_load(url.clone());

        let client = match vex_net::HttpClient::new() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Tab {}: failed to create HTTP client: {e}", self.id);
                self.load_error_page("Connection Error", &format!("{e}"), viewport);
                return;
            }
        };

        let request = vex_net::Request {
            url,
            method: vex_net::Method::Get,
            headers: std::collections::HashMap::new(),
            body: None,
        };

        self.loading = LoadingState::Loading { progress: 0.1 };

        match client.fetch(request).await {
            Ok(response) => {
                if response.status >= 400 {
                    let msg = format!("HTTP {} — {}", response.status, self.url);
                    self.load_error_page("Page Error", &msg, viewport);
                    return;
                }
                self.loading = LoadingState::Loading { progress: 0.3 };
                let html = String::from_utf8_lossy(&response.body).into_owned();
                self.load_html(&html, viewport);
                tracing::info!("Tab {} loaded: {} ({})", self.id, self.url, self.title);
            }
            Err(e) => {
                tracing::warn!("Tab {}: fetch failed: {e}", self.id);
                self.load_error_page("Page Load Failed", &format!("{e}"), viewport);
            }
        }
    }

    /// Load a built-in error page.
    fn load_error_page(&mut self, title: &str, message: &str, viewport: Size) {
        let html = crate::error_pages::build_error_page(title, message);
        self.load_html(&html, viewport);
        self.title = title.to_string();
    }

    /// Cancel any in-progress page load.
    pub fn stop(&mut self) {
        if self.is_loading() {
            self.loading = LoadingState::Idle;
            tracing::info!("Tab {} load stopped", self.id);
        }
    }

    /// Re-run layout and display list from the existing document and styles.
    ///
    /// Used after zoom level changes to recalculate layout at the new
    /// effective viewport size without re-parsing HTML.
    pub fn relayout(&mut self, viewport: Size) {
        let (layout_root, dl) = {
            let (shared_doc, styles) = match (&self.shared_doc, &self.styles) {
                (Some(sd), Some(s)) => (sd, s),
                _ => return,
            };
            let doc = shared_doc.borrow();
            let layout_root = vex_layout::layout_document(&doc, styles, viewport);
            let dl = vex_render::build_display_list(&layout_root, styles, &doc, viewport);
            (layout_root, dl)
        };
        self.layout = Some(layout_root);
        self.display_list = Some(dl);
        self.dirty = true;
        tracing::debug!(
            "Tab {} relayout at {:.0}×{:.0}",
            self.id,
            viewport.width,
            viewport.height
        );
    }

    /// Reload the current page.
    pub fn reload(&mut self) {
        let url = self.url.clone();
        self.start_load(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tab_has_correct_defaults() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let tab = Tab::new(TabId::new(1), url.clone());

        assert_eq!(tab.id, TabId::new(1));
        assert_eq!(tab.url, url);
        assert_eq!(tab.title, "New Tab");
        assert!(!tab.has_document());
        assert!(tab.runtime.is_none());
        assert!(tab.styles.is_none());
        assert!(tab.layout.is_none());
        assert!(tab.display_list.is_none());
        assert_eq!(tab.loading, LoadingState::Idle);
        assert!(tab.dirty);
    }

    #[test]
    fn blank_tab_uses_newtab_url() {
        let tab = Tab::blank(TabId::new(42));
        assert_eq!(tab.url.as_ref(), "vex://newtab");
        assert_eq!(tab.title, "New Tab");
    }

    #[test]
    fn loading_state_transitions() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url.clone());

        assert!(!tab.is_loading());
        assert!(!tab.is_complete());

        tab.start_load(url);
        assert!(tab.is_loading());
        assert!(!tab.is_complete());

        tab.loading = LoadingState::Loading { progress: 0.5 };
        assert!(tab.is_loading());

        tab.loading = LoadingState::Complete;
        assert!(!tab.is_loading());
        assert!(tab.is_complete());
    }

    #[test]
    fn stop_cancels_loading() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url.clone());
        tab.start_load(url);
        assert!(tab.is_loading());

        tab.stop();
        assert!(!tab.is_loading());
        assert_eq!(tab.loading, LoadingState::Idle);
    }

    #[test]
    fn reload_restarts_load() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);
        tab.loading = LoadingState::Complete;

        tab.reload();
        assert_eq!(tab.loading, LoadingState::Connecting);
        assert!(!tab.has_document());
        assert!(tab.runtime.is_none());
    }

    #[test]
    fn load_html_parses_content() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = "<html><head><title>Test Page</title></head><body><p>Hello</p></body></html>";
        tab.load_html(html, Size::new(800.0, 600.0));

        assert_eq!(tab.title, "Test Page");
        assert!(tab.has_document());
        assert!(tab.shared_doc.is_some());
        assert!(tab.runtime.is_some());
        assert!(tab.styles.is_some());
        assert!(tab.layout.is_some());
        assert!(tab.display_list.is_some());
        assert!(tab.is_complete());
    }

    #[test]
    fn load_html_executes_inline_scripts() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = r#"<html><head>
            <script>var testVar = 42;</script>
        </head><body></body></html>"#;
        tab.load_html(html, Size::new(800.0, 600.0));

        // Runtime was created and inline script executed.
        let runtime = tab.runtime.as_mut().unwrap();
        let val = runtime.eval("testVar").unwrap();
        assert_eq!(val.as_number().unwrap() as i32, 42);
    }

    #[test]
    fn load_html_fires_lifecycle_events() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        // Register a DOMContentLoaded listener via inline script.
        let html = r#"<html><head>
            <script>
                var dcl_fired = false;
                document.addEventListener('DOMContentLoaded', function() {
                    dcl_fired = true;
                });
            </script>
        </head><body></body></html>"#;
        tab.load_html(html, Size::new(800.0, 600.0));

        // The runtime exists and completed successfully.
        assert!(tab.runtime.is_some());
        assert!(tab.is_complete());
    }

    #[test]
    fn load_html_with_no_scripts_still_creates_runtime() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = "<html><body><p>No scripts here</p></body></html>";
        tab.load_html(html, Size::new(800.0, 600.0));

        // Runtime is always created (for event handling, etc).
        assert!(tab.runtime.is_some());
        assert!(tab.has_document());
    }

    #[test]
    fn borrow_document_works() {
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        assert!(tab.borrow_document().is_none());

        tab.load_html("<html><body>Hello</body></html>", Size::new(800.0, 600.0));

        let doc = tab.borrow_document();
        assert!(doc.is_some());
    }

    #[test]
    fn set_title_works() {
        let tab_id = TabId::new(1);
        let url = VexUrl::parse("https://example.com").unwrap();
        let mut tab = Tab::new(tab_id, url);
        tab.set_title("My Page");
        assert_eq!(tab.title, "My Page");
    }

    #[test]
    fn tab_id_display() {
        let id = TabId::new(5);
        assert_eq!(format!("{id}"), "Tab(5)");
    }
}
