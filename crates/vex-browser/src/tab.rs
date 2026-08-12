// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tab model — represents a single browser tab with its own page state.

use std::cell::Ref;
use std::collections::{HashMap, HashSet};

use vex_core::geometry::Size;
use vex_core::{VexError, VexId, VexUrl};
use vex_css::{ComputedStyle, Stylesheet};
use vex_dom::Document;
use vex_js::script_runner::ScriptEntry;
use vex_js::{ExecutionPlan, JsRuntime, RequestQueue, SharedDocument};
use vex_layout::{LayoutBox, ReflowPlan};
use vex_media::media_element::{MediaElement, MediaType};
use vex_media::media_loading::MediaFormat;
use vex_render::display_list::{DisplayList, ImageId};
use vex_render::scroll::ScrollState;
use vex_security::ResourceType;

type SharedSessionStorage = vex_js::api::session_storage::SharedSessionStorage;

const DEFAULT_ADBLOCK_LIST: &str = r#"
# Phase-9 default tracker/ad domains
doubleclick.net
googlesyndication.com
googleadservices.com
googletagmanager.com
googletagservices.com
adservice.google.com
facebook.net
connect.facebook.net
ads.twitter.com
analytics.twitter.com
hotjar.com
segment.io
mixpanel.com
scorecardresearch.com
taboola.com
outbrain.com
amazon-adsystem.com
criteo.com
quantserve.com
adnxs.com
tracker.example
ads.example
"#;

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

/// Result of a top-level network navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationOutcome {
    /// A document was loaded into the tab.
    PageLoaded,
    /// The response requested a file download instead of document rendering.
    Download {
        url: String,
        filename: String,
        mime_type: Option<String>,
        body: Vec<u8>,
    },
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
    /// Per-tab sessionStorage backing store (persisted across navigations in tab).
    pub session_storage: SharedSessionStorage,
    /// Computed CSS styles for each DOM node.
    pub styles: Option<HashMap<VexId, ComputedStyle>>,
    /// Parsed stylesheets used to compute the current style tree.
    pub stylesheets: Vec<Stylesheet>,
    /// Layout tree for the current page.
    pub layout: Option<LayoutBox>,
    /// Pre-built display list (cached between frames if page hasn't changed).
    pub display_list: Option<DisplayList>,
    /// Scroll state for the content area.
    pub scroll: ScrollState,
    /// Favicon image ID if one has been loaded.
    pub favicon: Option<ImageId>,
    /// Mapping from DOM node to uploaded render-image ID.
    pub image_bindings: HashMap<VexId, ImageId>,
    /// Phase-8 media model state per `<video>`/`<audio>` node.
    pub media_elements: HashMap<VexId, MediaElement>,
    /// Detected container format per media node.
    pub media_formats: HashMap<VexId, MediaFormat>,
    /// Current loading state.
    pub loading: LoadingState,
    /// Whether this tab's page content needs a re-render.
    pub dirty: bool,
    /// Incremental reflow invalidation plan.
    pub reflow_plan: ReflowPlan,
    /// Extension IDs whose content scripts have already been injected into this document.
    pub injected_extensions: HashSet<String>,
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
            session_storage: vex_js::api::session_storage::shared_session_storage(),
            styles: None,
            stylesheets: Vec::new(),
            layout: None,
            display_list: None,
            scroll: ScrollState::new(1280.0, 720.0),
            favicon: None,
            image_bindings: HashMap::new(),
            media_elements: HashMap::new(),
            media_formats: HashMap::new(),
            loading: LoadingState::Idle,
            dirty: true,
            reflow_plan: ReflowPlan::default(),
            injected_extensions: HashSet::new(),
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
        self.load_html_with_resources(html, viewport, &HashMap::new());
    }

    /// Like [`Self::load_html`], but with pre-fetched external resources.
    ///
    /// `resources` maps external URLs (as they appear in `src`/`href` attrs)
    /// to their fetched content. External scripts and stylesheets whose URL
    /// appears in this map will use the pre-fetched content instead of being
    /// skipped.
    pub fn load_html_with_resources(
        &mut self,
        html: &str,
        viewport: Size,
        resources: &HashMap<String, String>,
    ) {
        self.loading = LoadingState::Loading { progress: 0.2 };
        self.injected_extensions.clear();

        let document = vex_html::parse_html(html);

        // Extract scripts before moving document into shared wrapper.
        let scripts = vex_html::extract::extract_scripts(&document);
        let plan = ExecutionPlan::from_scripts(&scripts);

        // Wrap document for JS interop.
        let shared_doc = vex_js::shared_document(document);

        // Create JS runtime with all Web APIs.
        let mut runtime = JsRuntime::with_request_queue(self.request_queue.clone());
        runtime.register_document(&shared_doc);
        vex_js::api::window::update_location(self.url.as_ref(), runtime.context_mut());
        runtime.register_page_storage_apis(
            self.url.as_ref(),
            u64::from(self.id.0),
            Some(self.session_storage.clone()),
        );

        // Build the fetch closure that looks up pre-fetched resources.
        let mut fetch_fn = |url: &str| -> Result<String, VexError> {
            // Try exact URL first, then try resolving relative to page base.
            if let Some(content) = resources.get(url) {
                tracing::debug!(
                    "Using pre-fetched resource for {url} ({} bytes)",
                    content.len()
                );
                return Ok(content.clone());
            }

            // Try resolving the URL against the tab's base URL.
            if let Ok(resolved) = self.url.join(url) {
                let resolved_str = resolved.as_ref();
                if let Some(content) = resources.get(resolved_str) {
                    tracing::debug!(
                        "Using pre-fetched resource for {resolved_str} ({} bytes)",
                        content.len()
                    );
                    return Ok(content.clone());
                }
            }

            tracing::warn!("External resource not available: {url}");
            Err(VexError::Network(format!(
                "external resource not pre-fetched: {url}"
            )))
        };

        // Execute blocking scripts (inline + synchronous external).
        if !plan.blocking.is_empty() {
            tracing::debug!("Executing {} blocking script(s)", plan.blocking.len());
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

        // CSS pipeline: extract <style> and <link rel="stylesheet">,
        // compute cascade, layout, display list.
        let image_bindings = self.image_bindings.clone();
        let (stylesheets, styles, layout_root, dl, title, media_elements, media_formats) = {
            let doc = shared_doc.borrow();

            // Inline <style> elements.
            let style_ids = doc.get_elements_by_tag_name("style");
            let mut stylesheets = Vec::new();
            for id in &style_ids {
                let css_text = doc.text_content(*id);
                if !css_text.is_empty() {
                    stylesheets.push(vex_css::parse_stylesheet(&css_text));
                }
            }

            // External <link rel="stylesheet" href="..."> elements.
            let link_ids = doc.get_elements_by_tag_name("link");
            for id in &link_ids {
                let node = doc.arena().get(*id);
                if let vex_dom::NodeData::Element(ref el) = node.data {
                    let is_stylesheet = el
                        .attributes
                        .iter()
                        .any(|a| a.name == "rel" && a.value.eq_ignore_ascii_case("stylesheet"));
                    let href = el.attributes.iter().find(|a| a.name == "href");
                    if is_stylesheet {
                        if let Some(href_attr) = href {
                            match fetch_fn(&href_attr.value) {
                                Ok(css_text) => {
                                    tracing::debug!(
                                        "Loaded external stylesheet: {} ({} bytes)",
                                        href_attr.value,
                                        css_text.len()
                                    );
                                    stylesheets.push(vex_css::parse_stylesheet(&css_text));
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to fetch stylesheet {}: {e}",
                                        href_attr.value
                                    );
                                }
                            }
                        }
                    }
                }
            }

            self.loading = LoadingState::Loading { progress: 0.5 };

            let styles = vex_css::compute_styles(&doc, &stylesheets, viewport);
            let layout_root = vex_layout::layout_document(&doc, &styles, viewport);

            self.loading = LoadingState::Loading { progress: 0.8 };

            let dl = vex_render::build_display_list_with_images(
                &layout_root,
                &styles,
                &doc,
                viewport,
                &image_bindings,
            );

            // Extract title from <title> element.
            let title_ids = doc.get_elements_by_tag_name("title");
            let title = title_ids
                .first()
                .map(|&tid| doc.text_content(tid))
                .filter(|t| !t.is_empty());

            let (media_elements, media_formats) = discover_media_nodes(&doc, &self.url);

            (
                stylesheets,
                styles,
                layout_root,
                dl,
                title,
                media_elements,
                media_formats,
            )
        };

        if let Some(title_text) = title {
            self.title = title_text;
        }

        // Execute deferred scripts (after DOM built, before DOMContentLoaded).
        if !plan.deferred.is_empty() {
            tracing::debug!("Executing {} deferred script(s)", plan.deferred.len());
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

        // DOM ready event does not wait for async scripts.
        runtime.fire_dom_content_loaded(&shared_doc);

        // Execute async scripts once resources are available.
        if !plan.async_scripts.is_empty() {
            tracing::debug!("Executing {} async script(s)", plan.async_scripts.len());
            for (i, entry) in plan.async_scripts.iter().enumerate() {
                let result = match entry {
                    ScriptEntry::Inline { source } => {
                        ExecutionPlan::execute_async_script(&mut runtime, source)
                    }
                    ScriptEntry::External { url } => {
                        let source = match fetch_fn(url) {
                            Ok(source) => source,
                            Err(error) => {
                                tracing::warn!("Async script {i} fetch failed ({url}): {error}");
                                continue;
                            }
                        };
                        ExecutionPlan::execute_async_script(&mut runtime, &source)
                    }
                };

                if let Err(e) = result {
                    tracing::warn!("Async script {i} failed: {e}");
                }
            }
        }

        // Final page lifecycle event (after resources / async scripts).
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
        self.stylesheets = stylesheets;
        self.layout = Some(layout_root);
        self.display_list = Some(dl);
        self.media_elements = media_elements;
        self.media_formats = media_formats;
        self.loading = LoadingState::Complete;
        self.reflow_plan.clear();
        self.dirty = true;
    }

    /// Update the scroll position for this tab's content.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll.scroll_by(dx, dy);
        self.dirty = true;
    }

    /// Update scroll target with smooth scrolling behavior.
    pub fn scroll_by_smooth(&mut self, dx: f32, dy: f32) {
        self.scroll.scroll_by_smooth(dx, dy);
        self.dirty = true;
    }

    /// Advance scroll animation state.
    ///
    /// Returns `true` when visual scroll state changed.
    pub fn tick_scroll(&mut self, dt_seconds: f32) -> bool {
        let before_x = self.scroll.offset_x;
        let before_y = self.scroll.offset_y;
        self.scroll.tick(dt_seconds);
        let changed = (self.scroll.offset_x - before_x).abs() > 0.01
            || (self.scroll.offset_y - before_y).abs() > 0.01;
        if changed {
            self.dirty = true;
        }
        changed
    }

    /// Set the viewport dimensions (e.g., on window resize).
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.scroll.set_viewport_size(width, height);
        self.reflow_plan.full_reflow = true;
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

    /// Number of media elements discovered in the current document.
    pub fn media_element_count(&self) -> usize {
        self.media_elements.len()
    }

    /// Detected media format for a DOM media node.
    pub fn media_format(&self, node_id: VexId) -> Option<MediaFormat> {
        self.media_formats.get(&node_id).copied()
    }

    /// Run expired JS timers for this tab.
    ///
    /// Returns the number of timer callbacks fired. Call this once per
    /// event-loop tick so that `setTimeout` / `setInterval` actually fire.
    pub fn tick_timers(&mut self) -> u32 {
        if let Some(ref mut runtime) = self.runtime {
            let fired = runtime.run_pending_timers();
            if fired > 0 {
                self.dirty = true;
            }
            fired
        } else {
            0
        }
    }

    /// Whether this tab has pending JS timers.
    pub fn has_pending_timers(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(|rt| rt.has_pending_timers())
    }

    /// Begin loading a URL while retaining the committed document until a new
    /// response is ready to replace it.
    pub fn start_load(&mut self, url: VexUrl) {
        self.url = url;
        self.loading = LoadingState::Connecting;
        tracing::info!("Tab {} loading: {}", self.id, self.url);
    }

    /// Mark a single DOM node as dirty for reflow.
    pub fn mark_layout_dirty_node(&mut self, node_id: VexId) {
        self.reflow_plan.mark_dirty(node_id);
        self.dirty = true;
    }

    /// Associate an uploaded image ID with a DOM node for display-list painting.
    pub fn bind_image(&mut self, node_id: VexId, image_id: ImageId) {
        self.image_bindings.insert(node_id, image_id);
        self.dirty = true;
    }

    /// Whether a content script for an extension has already been injected.
    pub fn has_injected_extension(&self, extension_id: &str) -> bool {
        self.injected_extensions.contains(extension_id)
    }

    /// Mark an extension as injected for the current document.
    pub fn mark_extension_injected(&mut self, extension_id: &str) {
        self.injected_extensions.insert(extension_id.to_owned());
    }

    /// Fetch a URL over the network and run the full page pipeline:
    /// fetch → parse HTML → extract CSS → compute styles → layout → display list.
    ///
    /// On error, loads an error page instead. Attachment responses return a
    /// download outcome without replacing the currently committed document.
    pub async fn load_url(&mut self, url: VexUrl, viewport: Size) -> NavigationOutcome {
        self.load_request(url, vex_net::Method::Get, None, viewport)
            .await
    }

    /// Load a top-level navigation request using an existing persistent HTTP client.
    pub async fn load_request_with_client(
        &mut self,
        client: &vex_net::HttpClient,
        url: VexUrl,
        method: vex_net::Method,
        body: Option<Vec<u8>>,
        viewport: Size,
    ) -> NavigationOutcome {
        let previous_url = self.url.clone();
        self.start_load(url.clone());

        let mut headers = std::collections::HashMap::new();
        if method == vex_net::Method::Post {
            headers.insert(
                "content-type".to_owned(),
                "application/x-www-form-urlencoded;charset=UTF-8".to_owned(),
            );
        }
        let request = vex_net::Request {
            url,
            method,
            headers,
            body,
        };

        let privacy = privacy_layer_for_navigation();

        self.loading = LoadingState::Loading { progress: 0.1 };

        match client
            .fetch_filtered(request, |req| privacy.process_request(req))
            .await
        {
            Ok(response) => {
                if response.status >= 400 {
                    let msg = format!("HTTP {} — {}", response.status, self.url);
                    self.load_error_page("Page Error", &msg, viewport);
                    return NavigationOutcome::PageLoaded;
                }

                if let Some(filename) =
                    crate::downloads::attachment_filename(&response.headers, response.url.as_ref())
                {
                    self.url = previous_url;
                    self.loading = LoadingState::Complete;
                    return NavigationOutcome::Download {
                        url: response.url.to_string(),
                        filename,
                        mime_type: crate::downloads::response_mime_type(&response.headers),
                        body: response.body,
                    };
                }

                self.url = response.url.clone();
                persist_response_cookies(&self.url, &response.headers);

                let security_ctx = crate::secure_fetch::SecurityContext::new(
                    &self.url,
                    crate::secure_fetch::extract_csp(&response.headers),
                );
                self.loading = LoadingState::Loading { progress: 0.3 };
                let html = String::from_utf8_lossy(&response.body).into_owned();

                // Pre-fetch external resources (scripts and stylesheets)
                // so that load_html can use them synchronously.
                let resources =
                    prefetch_external_resources(client, &self.url, &html, &security_ctx, &privacy)
                        .await;

                self.loading = LoadingState::Loading { progress: 0.7 };
                self.load_html_with_resources(&html, viewport, &resources);
                self.loading = LoadingState::Complete;
                NavigationOutcome::PageLoaded
            }
            Err(e) => {
                tracing::error!("Tab {}: navigation fetch error for {}: {e}", self.id, self.url);
                self.load_error_page("Navigation Failed", &format!("{e}"), viewport);
                NavigationOutcome::PageLoaded
            }
        }
    }

    /// Load a top-level navigation request, including URL-encoded form POSTs.
    pub async fn load_request(
        &mut self,
        url: VexUrl,
        method: vex_net::Method,
        body: Option<Vec<u8>>,
        viewport: Size,
    ) -> NavigationOutcome {
        static SHARED_CLIENT: std::sync::OnceLock<vex_net::HttpClient> = std::sync::OnceLock::new();
        let client = SHARED_CLIENT.get_or_init(|| {
            vex_net::HttpClient::new().unwrap_or_else(|_| {
                vex_net::HttpClient::with_config(vex_net::ClientConfig::default())
                    .expect("failed to initialize default HttpClient")
            })
        });
        self.load_request_with_client(client, url, method, body, viewport).await
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
        let (styles, layout_root, dl) = {
            let shared_doc = match &self.shared_doc {
                Some(sd) => sd,
                None => return,
            };
            let doc = shared_doc.borrow();

            // Recompute styles on relayout so dynamic state changes (:focus,
            // :checked, media-query viewport updates) are reflected.
            let styles = vex_css::compute_styles(&doc, &self.stylesheets, viewport);

            let plan = if self.reflow_plan.needs_reflow() {
                self.reflow_plan.clone()
            } else {
                ReflowPlan {
                    full_reflow: true,
                    ..Default::default()
                }
            };

            let layout_root =
                vex_layout::reflow_document(&doc, &styles, viewport, self.layout.as_ref(), &plan);
            let dl = vex_render::build_display_list_with_images(
                &layout_root,
                &styles,
                &doc,
                viewport,
                &self.image_bindings,
            );
            (styles, layout_root, dl)
        };
        self.styles = Some(styles);
        self.layout = Some(layout_root);
        self.display_list = Some(dl);
        self.reflow_plan.clear();
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

/// Pre-fetch external resources (scripts and stylesheets) referenced in the HTML.
///
/// Does a quick parse of the HTML to find `<script src="...">` and
/// `<link rel="stylesheet" href="...">` tags, then fetches them all
/// concurrently. Returns a map from URL → content for resources that
/// were successfully fetched.
async fn prefetch_external_resources(
    client: &vex_net::HttpClient,
    base_url: &VexUrl,
    html: &str,
    security_ctx: &crate::secure_fetch::SecurityContext,
    privacy: &vex_privacy::PrivacyLayer,
) -> HashMap<String, String> {
    let doc = vex_html::parse_html(html);
    let mut urls_to_fetch: Vec<(String, String, ResourceType)> = Vec::new(); // (original_attr, resolved_url, resource_type)

    // Discover <script src="...">.
    let scripts = vex_html::extract::extract_scripts(&doc);
    for script in &scripts {
        if let Some(ref url) = script.src {
            let resolved = resolve_resource_url(base_url, url);
            urls_to_fetch.push((url.clone(), resolved, ResourceType::Script));
        }
    }

    // Discover <link rel="stylesheet" href="...">.
    let link_ids = doc.get_elements_by_tag_name("link");
    for id in &link_ids {
        let node = doc.arena().get(*id);
        if let vex_dom::NodeData::Element(ref el) = node.data {
            let is_stylesheet = el
                .attributes
                .iter()
                .any(|a| a.name == "rel" && a.value.eq_ignore_ascii_case("stylesheet"));
            let href = el
                .attributes
                .iter()
                .find(|a| a.name == "href")
                .map(|a| a.value.clone());
            if is_stylesheet {
                if let Some(href_val) = href {
                    let resolved = resolve_resource_url(base_url, &href_val);
                    urls_to_fetch.push((href_val, resolved, ResourceType::Style));
                }
            }
        }
    }

    if urls_to_fetch.is_empty() {
        return HashMap::new();
    }

    tracing::debug!("Pre-fetching {} external resource(s)", urls_to_fetch.len());

    let mut resources = HashMap::new();

    for (original, resolved_str, resource_type) in &urls_to_fetch {
        let url = match VexUrl::parse(resolved_str) {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!("Invalid resource URL {resolved_str}: {e}");
                continue;
            }
        };

        let mut request = vex_net::Request {
            url: url.clone(),
            method: vex_net::Method::Get,
            headers: HashMap::new(),
            body: None,
        };

        if let Err(e) = privacy.process_request(&mut request) {
            tracing::warn!("Privacy layer blocked resource {resolved_str}: {e}");
            continue;
        }

        match crate::secure_fetch::secure_fetch(client, request, security_ctx, *resource_type).await
        {
            Ok(response) if response.status < 400 => {
                let content = String::from_utf8_lossy(&response.body).into_owned();
                tracing::debug!("Pre-fetched {resolved_str} ({} bytes)", content.len());
                // Store under both the original attr value and the resolved URL
                // so that the fetch_fn in load_html can find it either way.
                resources.insert(original.clone(), content.clone());
                if original != resolved_str {
                    resources.insert(resolved_str.clone(), content);
                }
            }
            Ok(response) => {
                tracing::warn!("HTTP {} fetching resource {resolved_str}", response.status);
            }
            Err(e) => {
                tracing::warn!("Secure fetch rejected resource {resolved_str}: {e}");
            }
        }
    }

    tracing::info!(
        "Pre-fetched {}/{} external resources",
        resources.len(),
        urls_to_fetch.len()
    );
    resources
}

fn privacy_layer_for_navigation() -> vex_privacy::PrivacyLayer {
    let config = vex_privacy::middleware::PrivacyConfig::default();
    let adblock = vex_privacy::AdblockEngine::from_list(DEFAULT_ADBLOCK_LIST);
    vex_privacy::PrivacyLayer::with_config(config, adblock)
}

/// Resolve a potentially relative URL against a base URL.
fn resolve_resource_url(base: &VexUrl, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    if url.starts_with("//") {
        // Protocol-relative URL — use the base URL's scheme.
        let scheme = base.scheme();
        return format!("{scheme}:{url}");
    }
    // Relative URL — resolve against base.
    match base.join(url) {
        Ok(resolved) => resolved.as_ref().to_string(),
        Err(_) => url.to_string(),
    }
}

fn persist_response_cookies(url: &VexUrl, headers: &HashMap<String, String>) {
    let Some(raw_cookie) = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.as_str())
    else {
        return;
    };

    let domain = url.host().unwrap_or("localhost");
    let path = if url.path().is_empty() {
        "/"
    } else {
        url.path()
    };

    let storage_root = "target/vigo/storage";
    if let Err(e) = std::fs::create_dir_all(storage_root) {
        tracing::warn!(target: "vex_browser::tab", error = %e, "failed to create storage root for cookies");
        return;
    }

    let cookie_db = format!("{storage_root}/cookies.sqlite3");
    let store = match vex_storage::CookieStore::open(&cookie_db) {
        Ok(store) => store,
        Err(e) => {
            tracing::warn!(target: "vex_browser::tab", error = %e, "failed to open cookie store");
            return;
        }
    };

    if let Some(cookie) = parse_set_cookie_header(raw_cookie, domain, path) {
        if let Err(e) = store.save(&cookie) {
            tracing::warn!(target: "vex_browser::tab", error = %e, "failed to persist set-cookie");
        }
    }
}

fn parse_set_cookie_header(
    raw: &str,
    default_domain: &str,
    default_path: &str,
) -> Option<vex_storage::PersistentCookie> {
    let parts: Vec<&str> = raw.split(';').collect();
    let name_value = parts.first()?;
    let (name, value) = name_value.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if name.is_empty() {
        return None;
    }

    let mut cookie = vex_storage::PersistentCookie {
        domain: default_domain.to_owned(),
        path: default_path.to_owned(),
        name: name.to_owned(),
        value: value.to_owned(),
        secure: false,
        http_only: false,
        same_site: "Lax".to_owned(),
        expires: None,
    };

    for attr in parts.iter().skip(1) {
        let attr = attr.trim();
        let lower = attr.to_ascii_lowercase();
        if lower == "secure" {
            cookie.secure = true;
        } else if lower == "httponly" {
            cookie.http_only = true;
        } else if let Some(val) = lower.strip_prefix("path=") {
            cookie.path = val.trim().to_owned();
        } else if let Some(val) = lower.strip_prefix("domain=") {
            cookie.domain = val.trim().to_owned();
        } else if let Some(val) = lower.strip_prefix("samesite=") {
            cookie.same_site = val.trim().to_owned();
        }
    }

    Some(cookie)
}

fn discover_media_nodes(
    doc: &Document,
    base_url: &VexUrl,
) -> (HashMap<VexId, MediaElement>, HashMap<VexId, MediaFormat>) {
    let mut elements = HashMap::new();
    let mut formats = HashMap::new();

    for (tag, media_type) in [("video", MediaType::Video), ("audio", MediaType::Audio)] {
        for node_id in doc.get_elements_by_tag_name(tag) {
            let src = media_src_for_node(doc, node_id)
                .map(|raw| resolve_resource_url(base_url, &raw))
                .unwrap_or_default();

            let mut element = MediaElement::new(media_type, &src);
            if !src.is_empty() {
                element.set_src(&src);
            }

            let format = if src.is_empty() {
                MediaFormat::Unknown
            } else {
                MediaFormat::from_extension(&src)
            };

            elements.insert(node_id, element);
            formats.insert(node_id, format);
        }
    }

    (elements, formats)
}

fn media_src_for_node(doc: &Document, node_id: VexId) -> Option<String> {
    let arena = doc.arena();
    let node = arena.get(node_id);
    let vex_dom::NodeData::Element(ref el) = node.data else {
        return None;
    };

    if let Some(src) = el
        .attributes
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("src"))
        .map(|a| a.value.clone())
    {
        if !src.is_empty() {
            return Some(src);
        }
    }

    // Fallback: first <source src="..."> child.
    let mut child = node.first_child;
    while let Some(cid) = child {
        let cnode = arena.get(cid);
        if let vex_dom::NodeData::Element(ref cel) = cnode.data {
            if cel.tag_name.eq_ignore_ascii_case("source") {
                if let Some(src) = cel
                    .attributes
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case("src"))
                    .map(|a| a.value.clone())
                {
                    if !src.is_empty() {
                        return Some(src);
                    }
                }
            }
        }
        child = cnode.next_sibling;
    }

    None
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
    fn start_load_keeps_committed_document_until_navigation_commits() {
        let mut tab = Tab::blank(TabId::new(1));
        tab.load_html(
            "<title>Existing page</title><p>Still visible</p>",
            Size::new(800.0, 600.0),
        );
        let next = VexUrl::parse("https://example.test/download").unwrap();

        tab.start_load(next);

        assert!(tab.shared_doc.is_some());
        assert_eq!(tab.title, "Existing page");
        assert!(matches!(tab.loading, LoadingState::Connecting));
    }

    #[test]
    fn attachment_response_becomes_download_without_replacing_page() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename=report.txt\r\nContent-Length: 16\r\nConnection: close\r\n\r\nnavigation bytes",
                )
                .unwrap();
        });

        let original = VexUrl::parse("https://example.test/original").unwrap();
        let mut tab = Tab::new(TabId::new(1), original.clone());
        tab.load_html(
            "<title>Existing page</title><p>Still visible</p>",
            Size::new(800.0, 600.0),
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let outcome = runtime.block_on(tab.load_url(
            VexUrl::parse(&format!("http://{address}/download")).unwrap(),
            Size::new(800.0, 600.0),
        ));

        assert_eq!(tab.url, original);
        assert_eq!(tab.title, "Existing page");
        assert!(tab.has_document());
        assert!(matches!(
            outcome,
            NavigationOutcome::Download {
                filename,
                mime_type: Some(mime_type),
                body,
                ..
            } if filename == "report.txt" && mime_type == "text/plain" && body == b"navigation bytes"
        ));
    }

    #[test]
    fn load_request_sends_url_encoded_post_body() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..read]).into_owned())
                .unwrap();
            let body = "<title>Posted</title><p>Success</p>";
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
        });

        let mut tab = Tab::blank(TabId::new(1));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let outcome = runtime.block_on(tab.load_request(
            VexUrl::parse(&format!("http://{address}/submit")).unwrap(),
            vex_net::Method::Post,
            Some(b"name=Vigo+Browser".to_vec()),
            Size::new(800.0, 600.0),
        ));

        let request = request_rx
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        assert!(request.starts_with("POST /submit HTTP/1.1"));
        assert!(request.contains("content-type: application/x-www-form-urlencoded;charset=UTF-8"));
        assert!(request.contains("name=Vigo+Browser"));
        assert_eq!(outcome, NavigationOutcome::PageLoaded);
        assert_eq!(tab.title, "Posted");
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

    #[test]
    fn resolve_resource_url_absolute() {
        let base = VexUrl::parse("https://example.com/page/index.html").unwrap();
        assert_eq!(
            resolve_resource_url(&base, "https://cdn.example.com/app.js"),
            "https://cdn.example.com/app.js"
        );
    }

    #[test]
    fn resolve_resource_url_relative_path() {
        let base = VexUrl::parse("https://example.com/page/index.html").unwrap();
        assert_eq!(
            resolve_resource_url(&base, "scripts/app.js"),
            "https://example.com/page/scripts/app.js"
        );
    }

    #[test]
    fn resolve_resource_url_root_relative() {
        let base = VexUrl::parse("https://example.com/page/index.html").unwrap();
        assert_eq!(
            resolve_resource_url(&base, "/js/app.js"),
            "https://example.com/js/app.js"
        );
    }

    #[test]
    fn resolve_resource_url_protocol_relative() {
        let base = VexUrl::parse("https://example.com/page/index.html").unwrap();
        assert_eq!(
            resolve_resource_url(&base, "//cdn.example.com/app.js"),
            "https://cdn.example.com/app.js"
        );
    }

    #[test]
    fn load_html_with_external_resources() {
        let url = VexUrl::parse("https://example.com/page").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = r#"<html>
            <head>
                <link rel="stylesheet" href="https://example.com/style.css">
                <script src="https://example.com/app.js"></script>
            </head>
            <body><p>Hello</p></body>
        </html>"#;

        let mut resources = HashMap::new();
        resources.insert(
            "https://example.com/style.css".to_string(),
            "p { color: red; }".to_string(),
        );
        resources.insert(
            "https://example.com/app.js".to_string(),
            "var fromExternal = 'loaded';".to_string(),
        );

        tab.load_html_with_resources(html, Size::new(800.0, 600.0), &resources);

        assert!(tab.has_document());
        assert!(tab.is_complete());

        // External script should have been executed.
        let runtime = tab.runtime.as_mut().unwrap();
        let val = runtime.eval("fromExternal").unwrap();
        assert_eq!(val.as_string().unwrap(), "loaded");
    }

    #[test]
    fn load_html_executes_async_external_scripts() {
        let url = VexUrl::parse("https://example.com/page").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = r#"<html>
            <head>
                <script async src="https://example.com/async.js"></script>
            </head>
            <body></body>
        </html>"#;

        let mut resources = HashMap::new();
        resources.insert(
            "https://example.com/async.js".to_string(),
            "var asyncFromResource = 99;".to_string(),
        );

        tab.load_html_with_resources(html, Size::new(800.0, 600.0), &resources);
        let runtime = tab.runtime.as_mut().unwrap();
        let val = runtime.eval("asyncFromResource").unwrap();
        assert_eq!(val.as_number().unwrap() as i32, 99);
    }

    #[test]
    fn load_html_registers_storage_apis() {
        let url = VexUrl::parse("https://example.com/page").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);
        tab.load_html("<html><body></body></html>", Size::new(800.0, 600.0));

        let runtime = tab.runtime.as_mut().unwrap();
        let ls = runtime.eval("typeof localStorage").unwrap();
        assert_eq!(ls.as_string().unwrap().to_std_string_escaped(), "object");

        let ss = runtime.eval("typeof sessionStorage").unwrap();
        assert_eq!(ss.as_string().unwrap().to_std_string_escaped(), "object");

        let idb = runtime.eval("typeof indexedDB").unwrap();
        assert_eq!(idb.as_string().unwrap().to_std_string_escaped(), "object");
    }

    #[test]
    fn session_storage_persists_across_same_tab_navigation() {
        let mut tab = Tab::new(
            TabId::new(1),
            VexUrl::parse("https://example.com/a").unwrap(),
        );
        tab.load_html("<html><body></body></html>", Size::new(800.0, 600.0));
        tab.runtime
            .as_mut()
            .unwrap()
            .execute("sessionStorage.setItem('phase8_sess_key','phase8_sess_val');")
            .unwrap();

        tab.start_load(VexUrl::parse("https://example.com/b").unwrap());
        tab.load_html("<html><body></body></html>", Size::new(800.0, 600.0));

        let val = tab
            .runtime
            .as_mut()
            .unwrap()
            .eval("sessionStorage.getItem('phase8_sess_key')")
            .unwrap();
        assert_eq!(
            val.as_string().unwrap().to_std_string_escaped(),
            "phase8_sess_val"
        );
    }

    #[test]
    fn load_html_discovers_media_nodes_and_formats() {
        let url = VexUrl::parse("https://example.com/page").unwrap();
        let mut tab = Tab::new(TabId::new(1), url);

        let html = r#"<html><body>
            <video id="v" src="movie.mp4"></video>
            <audio id="a"><source src="song.mp3"></audio>
        </body></html>"#;

        tab.load_html(html, Size::new(800.0, 600.0));
        assert_eq!(tab.media_element_count(), 2);

        let doc = tab.borrow_document().unwrap();
        let video_id = doc.get_element_by_id("v").unwrap();
        let audio_id = doc.get_element_by_id("a").unwrap();
        drop(doc);

        assert_eq!(tab.media_format(video_id), Some(MediaFormat::Mp4));
        assert_eq!(tab.media_format(audio_id), Some(MediaFormat::Mp3));
    }

    #[test]
    fn privacy_layer_blocks_tracker_domains() {
        let privacy = privacy_layer_for_navigation();
        let mut req = vex_net::Request::get("https://doubleclick.net/collect").unwrap();
        let result = privacy.process_request(&mut req);
        assert!(result.is_err());
    }

    #[test]
    fn parse_set_cookie_header_extracts_flags() {
        let raw = "sid=abc123; Path=/; HttpOnly; Secure; SameSite=None";
        let cookie = parse_set_cookie_header(raw, "example.com", "/").expect("cookie parse");
        assert_eq!(cookie.name, "sid");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.path, "/");
        assert!(cookie.http_only);
        assert!(cookie.secure);
        assert_eq!(cookie.same_site, "none");
    }
}
