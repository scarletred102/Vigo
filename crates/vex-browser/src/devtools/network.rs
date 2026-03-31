// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Network panel — request log, detail view, filtering.
//!
//! Captures every HTTP request made by the page. Each request is stored
//! with its URL, method, status, content-type, size, and timing breakdown.
//! The panel supports sorting, filtering by resource type / status range /
//! search text, and a detail view with headers and body preview.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use vex_core::geometry::Rect;

/// Unique identifier for a network entry within the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(pub u64);

/// Type of resource being fetched — used for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// HTML document (main frame or iframe).
    Document,
    /// JavaScript file.
    Script,
    /// CSS stylesheet.
    Stylesheet,
    /// Image (PNG, JPEG, WebP, etc.).
    Image,
    /// Font file (WOFF2, TTF, etc.).
    Font,
    /// Audio or video media.
    Media,
    /// XMLHttpRequest.
    Xhr,
    /// Fetch API call.
    Fetch,
    /// WebSocket upgrade.
    WebSocket,
    /// Anything that doesn't fit the above.
    Other,
}

impl ResourceType {
    /// Display label for filter buttons.
    pub fn label(self) -> &'static str {
        match self {
            Self::Document => "Doc",
            Self::Script => "JS",
            Self::Stylesheet => "CSS",
            Self::Image => "Img",
            Self::Font => "Font",
            Self::Media => "Media",
            Self::Xhr => "XHR",
            Self::Fetch => "Fetch",
            Self::WebSocket => "WS",
            Self::Other => "Other",
        }
    }

    /// All resource types for filter UI.
    pub const ALL: &'static [ResourceType] = &[
        Self::Document,
        Self::Script,
        Self::Stylesheet,
        Self::Image,
        Self::Font,
        Self::Media,
        Self::Xhr,
        Self::Fetch,
        Self::WebSocket,
        Self::Other,
    ];

    /// Infer resource type from Content-Type header value.
    pub fn from_content_type(ct: &str) -> Self {
        let ct = ct.to_ascii_lowercase();
        if ct.starts_with("text/html") {
            Self::Document
        } else if ct.contains("javascript") || ct.contains("ecmascript") {
            Self::Script
        } else if ct.starts_with("text/css") {
            Self::Stylesheet
        } else if ct.starts_with("image/") {
            Self::Image
        } else if ct.contains("font") || ct.starts_with("application/x-font") {
            Self::Font
        } else if ct.starts_with("audio/") || ct.starts_with("video/") {
            Self::Media
        } else {
            Self::Other
        }
    }
}

/// Lifecycle phase of a network request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPhase {
    /// Request is in-flight.
    Pending,
    /// Response received successfully.
    Complete,
    /// Request failed (DNS error, timeout, etc.).
    Failed,
    /// Request was cancelled.
    Cancelled,
}

/// Timing breakdown for a network request.
///
/// Each field is the duration of that phase. Zero if the phase
/// was skipped (e.g. no DNS for cached requests).
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestTiming {
    /// DNS lookup time.
    pub dns: Duration,
    /// TCP connection time (after DNS).
    pub connect: Duration,
    /// TLS handshake time (after connection).
    pub tls: Duration,
    /// Time to first byte (after TLS / sending request).
    pub first_byte: Duration,
    /// Download time (first byte → last byte).
    pub download: Duration,
}

impl RequestTiming {
    /// Total request time (sum of all phases).
    pub fn total(&self) -> Duration {
        self.dns + self.connect + self.tls + self.first_byte + self.download
    }

    /// Format total duration for display (e.g. "123 ms", "1.2 s").
    pub fn total_display(&self) -> String {
        format_duration(self.total())
    }
}

/// Format a duration as a human-readable string.
fn format_duration(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 {
        format!("{:.0} µs", d.as_micros())
    } else if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{:.2} s", d.as_secs_f64())
    }
}

/// A single network request entry.
#[derive(Debug, Clone)]
pub struct NetworkEntry {
    /// Unique ID within the panel.
    pub id: RequestId,
    /// When the request was initiated.
    pub started_at: SystemTime,
    /// Request URL.
    pub url: String,
    /// HTTP method.
    pub method: String,
    /// HTTP status code (0 if still pending).
    pub status: u16,
    /// Response Content-Type header (if available).
    pub content_type: Option<String>,
    /// Response body size in bytes.
    pub size: u64,
    /// Inferred resource type.
    pub resource_type: ResourceType,
    /// Timing breakdown.
    pub timing: RequestTiming,
    /// Request headers.
    pub request_headers: HashMap<String, String>,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Body preview (first N bytes or formatted excerpt).
    pub body_preview: Option<String>,
    /// Whether the response was served from cache.
    pub was_cached: bool,
    /// Current lifecycle phase.
    pub phase: RequestPhase,
    /// Optional error message (for failed requests).
    pub error: Option<String>,
}

impl NetworkEntry {
    /// Short display name: last path segment or host.
    pub fn display_name(&self) -> &str {
        self.url
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or(&self.url)
    }

    /// Format the body size for display.
    pub fn size_display(&self) -> String {
        format_size(self.size)
    }

    /// Status code category (2 = success, 3 = redirect, etc.).
    pub fn status_category(&self) -> u8 {
        (self.status / 100) as u8
    }
}

/// Format a byte size for display.
fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_owned();
    }
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

/// Column that the request log can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    /// Sort by display name / URL.
    Name,
    /// Sort by HTTP method.
    Method,
    /// Sort by HTTP status code.
    Status,
    /// Sort by resource type.
    Type,
    /// Sort by response body size.
    Size,
    /// Sort by total time.
    Time,
}

impl SortColumn {
    /// Header label for the column.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Method => "Method",
            Self::Status => "Status",
            Self::Type => "Type",
            Self::Size => "Size",
            Self::Time => "Time",
        }
    }

    /// All columns in display order.
    pub const ALL: &'static [SortColumn] = &[
        Self::Name,
        Self::Method,
        Self::Status,
        Self::Type,
        Self::Size,
        Self::Time,
    ];
}

/// Filter by HTTP status code range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    /// Show all statuses.
    All,
    /// 2xx responses.
    Success,
    /// 3xx responses.
    Redirect,
    /// 4xx responses.
    ClientError,
    /// 5xx responses.
    ServerError,
}

impl StatusFilter {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Success => "2xx",
            Self::Redirect => "3xx",
            Self::ClientError => "4xx",
            Self::ServerError => "5xx",
        }
    }

    /// Whether a status code passes this filter.
    pub fn matches(self, status: u16) -> bool {
        match self {
            Self::All => true,
            Self::Success => (200..300).contains(&status),
            Self::Redirect => (300..400).contains(&status),
            Self::ClientError => (400..500).contains(&status),
            Self::ServerError => (500..600).contains(&status),
        }
    }
}

/// A row in the waterfall timing visualization.
#[derive(Debug, Clone)]
pub struct WaterfallBar {
    /// Label for the phase.
    pub label: &'static str,
    /// Normalized start position (0.0–1.0 of total).
    pub start: f32,
    /// Normalized width (0.0–1.0 of total).
    pub width: f32,
    /// Duration of this phase.
    pub duration: Duration,
}

/// Build waterfall bars for the timing visualization.
pub fn build_waterfall(timing: &RequestTiming) -> Vec<WaterfallBar> {
    let total = timing.total().as_secs_f64();
    if total <= 0.0 {
        return Vec::new();
    }

    let phases: &[(&str, Duration)] = &[
        ("DNS", timing.dns),
        ("Connect", timing.connect),
        ("TLS", timing.tls),
        ("Waiting", timing.first_byte),
        ("Download", timing.download),
    ];

    let mut offset = 0.0_f64;
    let mut bars = Vec::new();
    for &(label, dur) in phases {
        let secs = dur.as_secs_f64();
        if secs > 0.0 {
            bars.push(WaterfallBar {
                label,
                start: (offset / total) as f32,
                width: (secs / total) as f32,
                duration: dur,
            });
        }
        offset += secs;
    }
    bars
}

/// Detail view tabs for a selected request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    /// Request and response headers.
    Headers,
    /// Response body preview.
    Preview,
    /// Timing waterfall.
    Timing,
}

impl DetailTab {
    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Headers => "Headers",
            Self::Preview => "Preview",
            Self::Timing => "Timing",
        }
    }

    /// All tabs in display order.
    pub const ALL: &'static [DetailTab] = &[Self::Headers, Self::Preview, Self::Timing];
}

/// A formatted header pair for display.
#[derive(Debug, Clone)]
pub struct HeaderRow {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

/// Build header rows from a headers map, sorted by name.
pub fn build_header_rows(headers: &HashMap<String, String>) -> Vec<HeaderRow> {
    let mut rows: Vec<HeaderRow> = headers
        .iter()
        .map(|(k, v)| HeaderRow {
            name: k.clone(),
            value: v.clone(),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    rows
}

/// Format a response body for preview.
///
/// If the body looks like JSON, pretty-print it. Otherwise return
/// the raw text (truncated to `max_len` characters).
pub fn format_body_preview(body: &str, content_type: Option<&str>, max_len: usize) -> String {
    let is_json = content_type.map(|ct| ct.contains("json")).unwrap_or(false)
        || body.trim_start().starts_with('{')
        || body.trim_start().starts_with('[');

    if is_json {
        // Attempt JSON pretty-print.
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                return truncate_string(&pretty, max_len);
            }
        }
    }

    truncate_string(body, max_len)
}

/// Truncate a string to `max_len` characters, appending "…" if truncated.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_owned()
    } else {
        let mut result = String::with_capacity(max_len + 3);
        result.push_str(&s[..max_len]);
        result.push('…');
        result
    }
}

/// Maximum number of entries kept in the network log.
const MAX_NETWORK_ENTRIES: usize = 5000;

/// Maximum body preview length in characters.
const MAX_PREVIEW_LEN: usize = 10_000;

/// Data for completing a network request.
#[derive(Debug, Clone)]
pub struct ResponseData {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub response_headers: HashMap<String, String>,
    /// Response body size in bytes.
    pub size: u64,
    /// Timing breakdown.
    pub timing: RequestTiming,
    /// Body preview text (raw, will be formatted).
    pub body_preview: Option<String>,
    /// Whether the response was served from cache.
    pub was_cached: bool,
}

/// State for the Network DevTools panel.
#[derive(Debug, Clone)]
pub struct NetworkState {
    /// All captured network entries in chronological order.
    entries: Vec<NetworkEntry>,
    /// Auto-incrementing entry ID.
    next_id: u64,
    /// Currently selected entry for detail view (by ID).
    selected: Option<RequestId>,
    /// Active detail tab.
    detail_tab: DetailTab,
    /// Sort column.
    sort_column: SortColumn,
    /// Sort direction (true = ascending).
    sort_ascending: bool,
    /// Filter by resource type (None = show all).
    type_filter: Option<ResourceType>,
    /// Filter by status range.
    status_filter: StatusFilter,
    /// Search text filter (matches against URL).
    search_text: String,
    /// Scroll offset (in rows from the top).
    scroll_offset: usize,
    /// Whether to preserve log across navigations.
    preserve_log: bool,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            selected: None,
            detail_tab: DetailTab::Headers,
            sort_column: SortColumn::Time,
            sort_ascending: true,
            type_filter: None,
            status_filter: StatusFilter::All,
            search_text: String::new(),
            scroll_offset: 0,
            preserve_log: false,
        }
    }
}

impl NetworkState {
    /// Create a new empty network state.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Recording ──

    /// Start recording a new request. Returns the assigned [`RequestId`].
    pub fn start_request(
        &mut self,
        url: String,
        method: String,
        request_headers: HashMap<String, String>,
        resource_type: ResourceType,
    ) -> RequestId {
        let id = RequestId(self.next_id);
        self.next_id += 1;

        let entry = NetworkEntry {
            id,
            started_at: SystemTime::now(),
            url,
            method,
            status: 0,
            content_type: None,
            size: 0,
            resource_type,
            timing: RequestTiming::default(),
            request_headers,
            response_headers: HashMap::new(),
            body_preview: None,
            was_cached: false,
            phase: RequestPhase::Pending,
            error: None,
        };

        self.entries.push(entry);

        // Trim old entries.
        while self.entries.len() > MAX_NETWORK_ENTRIES {
            self.entries.remove(0);
        }

        id
    }

    /// Record that a response has been received for a pending request.
    pub fn complete_request(&mut self, id: RequestId, data: ResponseData) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.status = data.status;
            entry.size = data.size;
            entry.timing = data.timing;
            entry.response_headers = data.response_headers.clone();
            entry.was_cached = data.was_cached;
            entry.phase = RequestPhase::Complete;

            // Infer content type from response headers.
            entry.content_type = data
                .response_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.clone());

            // Update resource type from Content-Type if it was generic.
            if entry.resource_type == ResourceType::Other {
                if let Some(ref ct) = entry.content_type {
                    entry.resource_type = ResourceType::from_content_type(ct);
                }
            }

            // Store formatted body preview.
            if let Some(raw) = data.body_preview {
                entry.body_preview = Some(format_body_preview(
                    &raw,
                    entry.content_type.as_deref(),
                    MAX_PREVIEW_LEN,
                ));
            }
        }
    }

    /// Record that a request has failed.
    pub fn fail_request(&mut self, id: RequestId, error: String) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.phase = RequestPhase::Failed;
            entry.error = Some(error);
        }
    }

    /// Cancel a pending request.
    pub fn cancel_request(&mut self, id: RequestId) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.phase = RequestPhase::Cancelled;
        }
    }

    // ── Selection & detail ──

    /// Select a request for detail view.
    pub fn select(&mut self, id: RequestId) {
        self.selected = Some(id);
    }

    /// Deselect any selected request.
    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// The currently selected request ID.
    pub fn selected_id(&self) -> Option<RequestId> {
        self.selected
    }

    /// Get the selected entry.
    pub fn selected_entry(&self) -> Option<&NetworkEntry> {
        self.selected
            .and_then(|id| self.entries.iter().find(|e| e.id == id))
    }

    /// Set the active detail tab.
    pub fn set_detail_tab(&mut self, tab: DetailTab) {
        self.detail_tab = tab;
    }

    /// Current detail tab.
    pub fn detail_tab(&self) -> DetailTab {
        self.detail_tab
    }

    // ── Sorting ──

    /// Set sort column. If already sorted by this column, toggle direction.
    pub fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
    }

    /// Current sort column.
    pub fn sort_column(&self) -> SortColumn {
        self.sort_column
    }

    /// Whether sort is ascending.
    pub fn sort_ascending(&self) -> bool {
        self.sort_ascending
    }

    // ── Filtering ──

    /// Set the resource type filter.
    pub fn set_type_filter(&mut self, filter: Option<ResourceType>) {
        self.type_filter = filter;
        self.scroll_offset = 0;
    }

    /// Current type filter.
    pub fn type_filter(&self) -> Option<ResourceType> {
        self.type_filter
    }

    /// Set the status range filter.
    pub fn set_status_filter(&mut self, filter: StatusFilter) {
        self.status_filter = filter;
        self.scroll_offset = 0;
    }

    /// Current status filter.
    pub fn status_filter(&self) -> StatusFilter {
        self.status_filter
    }

    /// Set search text filter.
    pub fn set_search(&mut self, text: String) {
        self.search_text = text;
        self.scroll_offset = 0;
    }

    /// Current search text.
    pub fn search_text(&self) -> &str {
        &self.search_text
    }

    /// Clear all filters and search text.
    pub fn clear_filters(&mut self) {
        self.type_filter = None;
        self.status_filter = StatusFilter::All;
        self.search_text.clear();
        self.scroll_offset = 0;
    }

    // ── Queries ──

    /// All entries that pass the current filters, sorted.
    pub fn visible_entries(&self) -> Vec<&NetworkEntry> {
        let mut entries: Vec<&NetworkEntry> = self
            .entries
            .iter()
            .filter(|e| self.passes_filters(e))
            .collect();

        let asc = self.sort_ascending;
        entries.sort_by(|a, b| {
            let ord = match self.sort_column {
                SortColumn::Name => a.display_name().cmp(b.display_name()),
                SortColumn::Method => a.method.cmp(&b.method),
                SortColumn::Status => a.status.cmp(&b.status),
                SortColumn::Type => a.resource_type.label().cmp(b.resource_type.label()),
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::Time => a.timing.total().cmp(&b.timing.total()),
            };
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });

        entries
    }

    /// Check if an entry passes all active filters.
    fn passes_filters(&self, entry: &NetworkEntry) -> bool {
        // Type filter.
        if let Some(rt) = self.type_filter {
            if entry.resource_type != rt {
                return false;
            }
        }
        // Status filter (only applies to complete requests).
        if self.status_filter != StatusFilter::All
            && entry.phase == RequestPhase::Complete
            && !self.status_filter.matches(entry.status)
        {
            return false;
        }
        // Search text filter (case-insensitive match on URL).
        if !self.search_text.is_empty() {
            let url_lower = entry.url.to_ascii_lowercase();
            let search_lower = self.search_text.to_ascii_lowercase();
            if !url_lower.contains(&search_lower) {
                return false;
            }
        }
        true
    }

    /// Total entry count (unfiltered).
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all entries (e.g. on page navigation if preserve_log is off).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.next_id = 1;
    }

    /// Whether to preserve log across navigations.
    pub fn preserve_log(&self) -> bool {
        self.preserve_log
    }

    /// Toggle preserve log setting.
    pub fn set_preserve_log(&mut self, preserve: bool) {
        self.preserve_log = preserve;
    }

    /// Scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Summary statistics for the current entries.
    pub fn summary(&self) -> NetworkSummary {
        let total_count = self.entries.len();
        let total_size: u64 = self.entries.iter().map(|e| e.size).sum();
        let pending_count = self
            .entries
            .iter()
            .filter(|e| e.phase == RequestPhase::Pending)
            .count();
        let failed_count = self
            .entries
            .iter()
            .filter(|e| e.phase == RequestPhase::Failed)
            .count();
        NetworkSummary {
            total_count,
            total_size,
            pending_count,
            failed_count,
        }
    }
}

/// Summary statistics displayed in the network panel footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSummary {
    /// Total number of requests.
    pub total_count: usize,
    /// Total transferred size in bytes.
    pub total_size: u64,
    /// Number of pending requests.
    pub pending_count: usize,
    /// Number of failed requests.
    pub failed_count: usize,
}

impl NetworkSummary {
    /// Format as a status-bar string.
    pub fn display(&self) -> String {
        format!(
            "{} requests  |  {} transferred{}{}",
            self.total_count,
            format_size(self.total_size),
            if self.pending_count > 0 {
                format!("  |  {} pending", self.pending_count)
            } else {
                String::new()
            },
            if self.failed_count > 0 {
                format!("  |  {} failed", self.failed_count)
            } else {
                String::new()
            },
        )
    }
}

/// Layout regions for the network panel.
#[derive(Debug, Clone)]
pub struct NetworkLayout {
    /// Toolbar row (filters + search).
    pub toolbar: Rect,
    /// Column headers row.
    pub headers: Rect,
    /// Main request list area.
    pub request_list: Rect,
    /// Detail panel (when a request is selected).
    pub detail: Option<Rect>,
    /// Footer / summary bar.
    pub footer: Rect,
}

/// Row height in the request list.
const ROW_HEIGHT: f32 = 24.0;
/// Toolbar height.
const TOOLBAR_HEIGHT: f32 = 32.0;
/// Header row height.
const HEADER_HEIGHT: f32 = 24.0;
/// Footer height.
const FOOTER_HEIGHT: f32 = 24.0;
/// Detail panel height ratio (fraction of panel body).
const DETAIL_RATIO: f32 = 0.4;

/// Compute the network panel layout within the given panel body rect.
pub fn compute_network_layout(body: Rect, has_selection: bool) -> NetworkLayout {
    let toolbar = Rect::new(
        body.origin.x,
        body.origin.y,
        body.size.width,
        TOOLBAR_HEIGHT,
    );
    let headers = Rect::new(
        body.origin.x,
        toolbar.origin.y + TOOLBAR_HEIGHT,
        body.size.width,
        HEADER_HEIGHT,
    );
    let footer = Rect::new(
        body.origin.x,
        body.origin.y + body.size.height - FOOTER_HEIGHT,
        body.size.width,
        FOOTER_HEIGHT,
    );

    let list_top = headers.origin.y + HEADER_HEIGHT;
    let available_h = (footer.origin.y - list_top).max(0.0);

    let (request_list, detail) = if has_selection {
        let detail_h = (available_h * DETAIL_RATIO).max(80.0).min(available_h);
        let list_h = (available_h - detail_h).max(0.0);
        let request_list = Rect::new(body.origin.x, list_top, body.size.width, list_h);
        let detail_rect = Rect::new(body.origin.x, list_top + list_h, body.size.width, detail_h);
        (request_list, Some(detail_rect))
    } else {
        let request_list = Rect::new(body.origin.x, list_top, body.size.width, available_h);
        (request_list, None)
    };

    NetworkLayout {
        toolbar,
        headers,
        request_list,
        detail,
        footer,
    }
}

/// Number of visible rows in the request list.
pub fn visible_row_count(list_rect: &Rect) -> usize {
    (list_rect.size.height / ROW_HEIGHT).floor() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ResourceType ──

    #[test]
    fn resource_type_from_content_type() {
        assert_eq!(
            ResourceType::from_content_type("text/html; charset=utf-8"),
            ResourceType::Document
        );
        assert_eq!(
            ResourceType::from_content_type("application/javascript"),
            ResourceType::Script
        );
        assert_eq!(
            ResourceType::from_content_type("text/css"),
            ResourceType::Stylesheet
        );
        assert_eq!(
            ResourceType::from_content_type("image/png"),
            ResourceType::Image
        );
        assert_eq!(
            ResourceType::from_content_type("font/woff2"),
            ResourceType::Font
        );
        assert_eq!(
            ResourceType::from_content_type("video/mp4"),
            ResourceType::Media
        );
        assert_eq!(
            ResourceType::from_content_type("application/octet-stream"),
            ResourceType::Other
        );
    }

    #[test]
    fn resource_type_labels() {
        assert_eq!(ResourceType::Document.label(), "Doc");
        assert_eq!(ResourceType::Script.label(), "JS");
        assert_eq!(ResourceType::Image.label(), "Img");
    }

    // ── RequestTiming ──

    #[test]
    fn timing_total() {
        let timing = RequestTiming {
            dns: Duration::from_millis(10),
            connect: Duration::from_millis(20),
            tls: Duration::from_millis(30),
            first_byte: Duration::from_millis(40),
            download: Duration::from_millis(50),
        };
        assert_eq!(timing.total(), Duration::from_millis(150));
    }

    #[test]
    fn timing_display() {
        let timing = RequestTiming {
            dns: Duration::from_millis(5),
            connect: Duration::from_millis(10),
            tls: Duration::from_millis(15),
            first_byte: Duration::from_millis(20),
            download: Duration::from_millis(50),
        };
        assert_eq!(timing.total_display(), "100 ms");
    }

    // ── NetworkEntry ──

    #[test]
    fn display_name_extracts_last_segment() {
        let entry = make_entry("https://example.com/js/app.js", 200);
        assert_eq!(entry.display_name(), "app.js");
    }

    #[test]
    fn display_name_fallback_for_root() {
        let entry = make_entry("https://example.com/", 200);
        // Last non-empty segment is "example.com" (after stripping trailing /).
        assert_eq!(entry.display_name(), "example.com");
    }

    #[test]
    fn size_display() {
        let mut entry = make_entry("https://x.com/a", 200);
        entry.size = 0;
        assert_eq!(entry.size_display(), "0 B");
        entry.size = 512;
        assert_eq!(entry.size_display(), "512 B");
        entry.size = 2048;
        assert_eq!(entry.size_display(), "2.0 KB");
        entry.size = 1_500_000;
        assert_eq!(entry.size_display(), "1.4 MB");
    }

    #[test]
    fn status_category() {
        let mut entry = make_entry("https://x.com/a", 200);
        assert_eq!(entry.status_category(), 2);
        entry.status = 301;
        assert_eq!(entry.status_category(), 3);
        entry.status = 404;
        assert_eq!(entry.status_category(), 4);
        entry.status = 500;
        assert_eq!(entry.status_category(), 5);
    }

    // ── StatusFilter ──

    #[test]
    fn status_filter_matches() {
        assert!(StatusFilter::All.matches(200));
        assert!(StatusFilter::All.matches(500));
        assert!(StatusFilter::Success.matches(200));
        assert!(StatusFilter::Success.matches(204));
        assert!(!StatusFilter::Success.matches(301));
        assert!(StatusFilter::Redirect.matches(301));
        assert!(!StatusFilter::Redirect.matches(200));
        assert!(StatusFilter::ClientError.matches(404));
        assert!(StatusFilter::ServerError.matches(503));
    }

    // ── NetworkState: recording ──

    #[test]
    fn start_and_complete_request() {
        let mut state = NetworkState::new();
        let id = state.start_request(
            "https://example.com/api".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Fetch,
        );
        assert_eq!(state.entry_count(), 1);

        let mut resp_headers = HashMap::new();
        resp_headers.insert("content-type".to_owned(), "application/json".to_owned());

        state.complete_request(
            id,
            ResponseData {
                status: 200,
                response_headers: resp_headers,
                size: 1024,
                timing: RequestTiming {
                    dns: Duration::from_millis(5),
                    connect: Duration::from_millis(10),
                    tls: Duration::from_millis(15),
                    first_byte: Duration::from_millis(20),
                    download: Duration::from_millis(50),
                },
                body_preview: Some(r#"{"ok": true}"#.to_owned()),
                was_cached: false,
            },
        );

        let entry = &state.entries[0];
        assert_eq!(entry.status, 200);
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.phase, RequestPhase::Complete);
        assert!(entry.content_type.as_deref() == Some("application/json"));
        assert!(entry.body_preview.is_some());
    }

    #[test]
    fn fail_request() {
        let mut state = NetworkState::new();
        let id = state.start_request(
            "https://bad.test/".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Document,
        );
        state.fail_request(id, "DNS resolution failed".to_owned());

        let entry = &state.entries[0];
        assert_eq!(entry.phase, RequestPhase::Failed);
        assert!(entry.error.as_deref() == Some("DNS resolution failed"));
    }

    #[test]
    fn cancel_request() {
        let mut state = NetworkState::new();
        let id = state.start_request(
            "https://slow.test/".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Document,
        );
        state.cancel_request(id);
        assert_eq!(state.entries[0].phase, RequestPhase::Cancelled);
    }

    // ── Filtering ──

    #[test]
    fn type_filter() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/app.js",
            "GET",
            200,
            ResourceType::Script,
            5000,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/style.css",
            "GET",
            200,
            ResourceType::Stylesheet,
            2000,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/logo.png",
            "GET",
            200,
            ResourceType::Image,
            10000,
        );

        state.set_type_filter(Some(ResourceType::Script));
        let visible = state.visible_entries();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].url.contains("app.js"));
    }

    #[test]
    fn status_filter_on_entries() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/ok",
            "GET",
            200,
            ResourceType::Document,
            1000,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/moved",
            "GET",
            301,
            ResourceType::Document,
            0,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/gone",
            "GET",
            404,
            ResourceType::Document,
            0,
        );

        state.set_status_filter(StatusFilter::Success);
        assert_eq!(state.visible_entries().len(), 1);

        state.set_status_filter(StatusFilter::ClientError);
        assert_eq!(state.visible_entries().len(), 1);
    }

    #[test]
    fn search_filter() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://example.com/api/users",
            "GET",
            200,
            ResourceType::Fetch,
            500,
        );
        add_complete_entry(
            &mut state,
            "https://cdn.test/image.png",
            "GET",
            200,
            ResourceType::Image,
            10000,
        );

        state.set_search("api".to_owned());
        let visible = state.visible_entries();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].url.contains("api"));
    }

    #[test]
    fn search_case_insensitive() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://example.com/API/Users",
            "GET",
            200,
            ResourceType::Fetch,
            500,
        );

        state.set_search("api".to_owned());
        assert_eq!(state.visible_entries().len(), 1);
    }

    #[test]
    fn clear_filters() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/a",
            "GET",
            200,
            ResourceType::Script,
            100,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/b",
            "GET",
            200,
            ResourceType::Image,
            200,
        );

        state.set_type_filter(Some(ResourceType::Script));
        state.set_status_filter(StatusFilter::Success);
        state.set_search("nothing".to_owned());
        assert_eq!(state.visible_entries().len(), 0);

        state.clear_filters();
        assert_eq!(state.visible_entries().len(), 2);
    }

    // ── Sorting ──

    #[test]
    fn sort_by_size() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/small",
            "GET",
            200,
            ResourceType::Script,
            100,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/big",
            "GET",
            200,
            ResourceType::Image,
            50000,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/med",
            "GET",
            200,
            ResourceType::Stylesheet,
            5000,
        );

        state.toggle_sort(SortColumn::Size);
        let entries = state.visible_entries();
        assert!(entries[0].size <= entries[1].size);
        assert!(entries[1].size <= entries[2].size);
    }

    #[test]
    fn sort_toggle_direction() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/a",
            "GET",
            200,
            ResourceType::Script,
            100,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/b",
            "GET",
            200,
            ResourceType::Script,
            200,
        );

        state.toggle_sort(SortColumn::Size);
        assert!(state.sort_ascending());

        state.toggle_sort(SortColumn::Size);
        assert!(!state.sort_ascending());

        let entries = state.visible_entries();
        assert!(entries[0].size >= entries[1].size);
    }

    // ── Selection ──

    #[test]
    fn select_and_deselect() {
        let mut state = NetworkState::new();
        let id = state.start_request(
            "https://example.com/".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Document,
        );
        state.select(id);
        assert_eq!(state.selected_id(), Some(id));
        assert!(state.selected_entry().is_some());

        state.deselect();
        assert_eq!(state.selected_id(), None);
    }

    #[test]
    fn detail_tab_switching() {
        let mut state = NetworkState::new();
        assert_eq!(state.detail_tab(), DetailTab::Headers);
        state.set_detail_tab(DetailTab::Timing);
        assert_eq!(state.detail_tab(), DetailTab::Timing);
    }

    // ── Clear ──

    #[test]
    fn clear_removes_all() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/a",
            "GET",
            200,
            ResourceType::Script,
            100,
        );
        state.clear();
        assert_eq!(state.entry_count(), 0);
        assert_eq!(state.selected_id(), None);
    }

    // ── Summary ──

    #[test]
    fn summary_stats() {
        let mut state = NetworkState::new();
        add_complete_entry(
            &mut state,
            "https://x.com/a",
            "GET",
            200,
            ResourceType::Script,
            1000,
        );
        add_complete_entry(
            &mut state,
            "https://x.com/b",
            "GET",
            200,
            ResourceType::Image,
            2000,
        );
        let id = state.start_request(
            "https://x.com/c".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Fetch,
        );
        state.fail_request(id, "timeout".to_owned());

        let summary = state.summary();
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.total_size, 3000);
        assert_eq!(summary.pending_count, 0);
        assert_eq!(summary.failed_count, 1);
    }

    #[test]
    fn summary_display() {
        let summary = NetworkSummary {
            total_count: 5,
            total_size: 10240,
            pending_count: 1,
            failed_count: 0,
        };
        let display = summary.display();
        assert!(display.contains("5 requests"));
        assert!(display.contains("10.0 KB"));
        assert!(display.contains("1 pending"));
    }

    // ── Waterfall ──

    #[test]
    fn waterfall_builds_bars() {
        let timing = RequestTiming {
            dns: Duration::from_millis(10),
            connect: Duration::from_millis(20),
            tls: Duration::from_millis(0), // skipped
            first_byte: Duration::from_millis(30),
            download: Duration::from_millis(40),
        };
        let bars = build_waterfall(&timing);
        assert_eq!(bars.len(), 4); // TLS skipped
        assert_eq!(bars[0].label, "DNS");
        assert_eq!(bars[1].label, "Connect");
        assert_eq!(bars[2].label, "Waiting");
        assert_eq!(bars[3].label, "Download");

        // Bars should cover the full width.
        let total_width: f32 = bars.iter().map(|b| b.width).sum();
        assert!((total_width - 1.0).abs() < 0.01);
    }

    #[test]
    fn waterfall_empty_for_zero_timing() {
        let bars = build_waterfall(&RequestTiming::default());
        assert!(bars.is_empty());
    }

    // ── Body preview ──

    #[test]
    fn json_body_pretty_printed() {
        let body = r#"{"name":"Alice","age":30}"#;
        let preview = format_body_preview(body, Some("application/json"), 10000);
        assert!(preview.contains("\"name\": \"Alice\""));
        assert!(preview.contains('\n')); // pretty-printed has newlines
    }

    #[test]
    fn plain_text_truncated() {
        let body = "a".repeat(200);
        let preview = format_body_preview(&body, Some("text/plain"), 100);
        // 100 ASCII chars + "…" (3 bytes in UTF-8) = 103 bytes.
        assert_eq!(preview.len(), 103);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn header_rows_sorted() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_owned(), "text/html".to_owned());
        headers.insert("Accept".to_owned(), "*/*".to_owned());
        headers.insert("X-Custom".to_owned(), "value".to_owned());

        let rows = build_header_rows(&headers);
        assert_eq!(rows[0].name, "Accept");
        assert_eq!(rows[1].name, "Content-Type");
        assert_eq!(rows[2].name, "X-Custom");
    }

    // ── Layout ──

    #[test]
    fn network_layout_without_selection() {
        let body = Rect::new(0.0, 0.0, 800.0, 400.0);
        let layout = compute_network_layout(body, false);
        assert!(layout.detail.is_none());
        assert!(layout.request_list.size.height > 0.0);
    }

    #[test]
    fn network_layout_with_selection() {
        let body = Rect::new(0.0, 0.0, 800.0, 400.0);
        let layout = compute_network_layout(body, true);
        assert!(layout.detail.is_some());
        // Detail + list should fit in available space.
        let total_h = layout.request_list.size.height + layout.detail.unwrap().size.height;
        let available_h = body.size.height - TOOLBAR_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT;
        assert!((total_h - available_h).abs() < 1.0);
    }

    #[test]
    fn visible_row_count_calc() {
        let rect = Rect::new(0.0, 0.0, 800.0, 240.0);
        assert_eq!(visible_row_count(&rect), 10); // 240 / 24 = 10
    }

    // ── Max entries trim ──

    #[test]
    fn max_entries_trimmed() {
        let mut state = NetworkState::new();
        for i in 0..5100 {
            state.start_request(
                format!("https://x.com/{i}"),
                "GET".to_owned(),
                HashMap::new(),
                ResourceType::Other,
            );
        }
        assert!(state.entry_count() <= MAX_NETWORK_ENTRIES);
    }

    // ── Preserve log ──

    #[test]
    fn preserve_log_setting() {
        let mut state = NetworkState::new();
        assert!(!state.preserve_log());
        state.set_preserve_log(true);
        assert!(state.preserve_log());
    }

    // ── Content type auto-detection ──

    #[test]
    fn content_type_refines_resource_type() {
        let mut state = NetworkState::new();
        let id = state.start_request(
            "https://cdn.test/bundle".to_owned(),
            "GET".to_owned(),
            HashMap::new(),
            ResourceType::Other, // unknown at request time
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_owned(),
            "application/javascript".to_owned(),
        );
        state.complete_request(
            id,
            ResponseData {
                status: 200,
                response_headers: headers,
                size: 5000,
                timing: RequestTiming::default(),
                body_preview: None,
                was_cached: false,
            },
        );

        assert_eq!(state.entries[0].resource_type, ResourceType::Script);
    }

    // ── Helpers ──

    /// Create a minimal entry for testing.
    fn make_entry(url: &str, status: u16) -> NetworkEntry {
        NetworkEntry {
            id: RequestId(1),
            started_at: SystemTime::now(),
            url: url.to_owned(),
            method: "GET".to_owned(),
            status,
            content_type: None,
            size: 0,
            resource_type: ResourceType::Document,
            timing: RequestTiming::default(),
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
            body_preview: None,
            was_cached: false,
            phase: RequestPhase::Complete,
            error: None,
        }
    }

    /// Add a complete entry to the state for testing.
    fn add_complete_entry(
        state: &mut NetworkState,
        url: &str,
        method: &str,
        status: u16,
        resource_type: ResourceType,
        size: u64,
    ) {
        let id = state.start_request(
            url.to_owned(),
            method.to_owned(),
            HashMap::new(),
            resource_type,
        );
        state.complete_request(
            id,
            ResponseData {
                status,
                response_headers: HashMap::new(),
                size,
                timing: RequestTiming::default(),
                body_preview: None,
                was_cached: false,
            },
        );
    }
}
