// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Performance API (Navigation Timing, Resource Timing, User Timing).
//!
//! Provides high-resolution timing measurements for page loads,
//! resource fetches, and user-defined marks and measures.

use std::collections::VecDeque;

// ── PerformanceEntry ─────────────────────────────────────────────────────────

/// The type of a performance entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryType {
    /// Navigation timing entry.
    Navigation,
    /// Resource timing entry.
    Resource,
    /// User-defined mark.
    Mark,
    /// User-defined measure.
    Measure,
    /// Paint timing (first-paint, first-contentful-paint).
    Paint,
    /// Long task.
    LongTask,
}

impl EntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
            Self::Resource => "resource",
            Self::Mark => "mark",
            Self::Measure => "measure",
            Self::Paint => "paint",
            Self::LongTask => "longtask",
        }
    }
}

/// A performance timeline entry.
#[derive(Debug, Clone)]
pub struct PerformanceEntry {
    pub name: String,
    pub entry_type: EntryType,
    /// Start time in milliseconds since time origin.
    pub start_time: f64,
    /// Duration in milliseconds.
    pub duration: f64,
    /// Additional detail (JSON-encoded or simple string).
    pub detail: Option<String>,
}

impl PerformanceEntry {
    pub fn new(name: &str, entry_type: EntryType, start_time: f64, duration: f64) -> Self {
        Self {
            name: name.to_string(),
            entry_type,
            start_time,
            duration,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }
}

// ── NavigationTiming ─────────────────────────────────────────────────────────

/// Navigation timing milestones (simplified PerformanceNavigationTiming).
#[derive(Debug, Clone, Default)]
pub struct NavigationTiming {
    /// Time origin (page load start).
    pub time_origin: f64,
    /// DNS lookup start.
    pub dns_start: f64,
    /// DNS lookup end.
    pub dns_end: f64,
    /// TCP connect start.
    pub connect_start: f64,
    /// TCP connect end.
    pub connect_end: f64,
    /// TLS handshake start.
    pub secure_connection_start: f64,
    /// HTTP request start.
    pub request_start: f64,
    /// First byte received.
    pub response_start: f64,
    /// Response fully received.
    pub response_end: f64,
    /// DOM parsing start.
    pub dom_interactive: f64,
    /// DOM content loaded.
    pub dom_content_loaded: f64,
    /// DOM complete.
    pub dom_complete: f64,
    /// Load event start.
    pub load_event_start: f64,
    /// Load event end.
    pub load_event_end: f64,
}

impl NavigationTiming {
    /// Time To First Byte.
    pub fn ttfb(&self) -> f64 {
        self.response_start - self.request_start
    }

    /// DOM Content Loaded time.
    pub fn dcl(&self) -> f64 {
        self.dom_content_loaded - self.time_origin
    }

    /// Full load time.
    pub fn load_time(&self) -> f64 {
        self.load_event_end - self.time_origin
    }

    /// DNS lookup duration.
    pub fn dns_time(&self) -> f64 {
        self.dns_end - self.dns_start
    }

    /// TCP connect time.
    pub fn connect_time(&self) -> f64 {
        self.connect_end - self.connect_start
    }
}

// ── ResourceTiming ───────────────────────────────────────────────────────────

/// Timing for a single resource fetch.
#[derive(Debug, Clone)]
pub struct ResourceTiming {
    pub name: String,
    pub initiator_type: String,
    pub start_time: f64,
    pub duration: f64,
    pub transfer_size: u64,
    pub encoded_body_size: u64,
    pub decoded_body_size: u64,
    pub response_end: f64,
}

impl ResourceTiming {
    pub fn new(name: &str, initiator_type: &str, start: f64, end: f64) -> Self {
        Self {
            name: name.to_string(),
            initiator_type: initiator_type.to_string(),
            start_time: start,
            duration: end - start,
            transfer_size: 0,
            encoded_body_size: 0,
            decoded_body_size: 0,
            response_end: end,
        }
    }
}

// ── Performance ──────────────────────────────────────────────────────────────

/// The main Performance API interface.
#[derive(Debug)]
pub struct Performance {
    /// Time origin in absolute ms.
    pub time_origin: f64,
    /// All performance entries.
    entries: Vec<PerformanceEntry>,
    /// Navigation timing for the current page.
    pub navigation_timing: NavigationTiming,
    /// Resource timing buffer.
    resource_buffer: VecDeque<ResourceTiming>,
    /// Maximum resource buffer size.
    pub resource_buffer_limit: usize,
}

impl Default for Performance {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Performance {
    pub fn new(time_origin: f64) -> Self {
        Self {
            time_origin,
            entries: Vec::new(),
            navigation_timing: NavigationTiming {
                time_origin,
                ..Default::default()
            },
            resource_buffer: VecDeque::new(),
            resource_buffer_limit: 250,
        }
    }

    /// Get the current time relative to time origin (simulated).
    pub fn now(&self, absolute_time: f64) -> f64 {
        absolute_time - self.time_origin
    }

    /// Create a user mark.
    pub fn mark(&mut self, name: &str, time: f64) {
        self.entries.push(PerformanceEntry::new(
            name,
            EntryType::Mark,
            time - self.time_origin,
            0.0,
        ));
    }

    /// Create a user measure between two marks.
    pub fn measure(&mut self, name: &str, start_mark: &str, end_mark: &str) -> Option<f64> {
        let start = self
            .entries
            .iter()
            .rev()
            .find(|e| e.entry_type == EntryType::Mark && e.name == start_mark)?
            .start_time;
        let end = self
            .entries
            .iter()
            .rev()
            .find(|e| e.entry_type == EntryType::Mark && e.name == end_mark)?
            .start_time;
        let duration = end - start;
        self.entries.push(PerformanceEntry::new(
            name,
            EntryType::Measure,
            start,
            duration,
        ));
        Some(duration)
    }

    /// Record a paint timing entry.
    pub fn record_paint(&mut self, name: &str, time: f64) {
        self.entries.push(PerformanceEntry::new(
            name,
            EntryType::Paint,
            time - self.time_origin,
            0.0,
        ));
    }

    /// Add a resource timing entry.
    pub fn add_resource(&mut self, resource: ResourceTiming) {
        if self.resource_buffer.len() >= self.resource_buffer_limit {
            self.resource_buffer.pop_front();
        }
        let entry = PerformanceEntry::new(
            &resource.name,
            EntryType::Resource,
            resource.start_time - self.time_origin,
            resource.duration,
        );
        self.entries.push(entry);
        self.resource_buffer.push_back(resource);
    }

    /// Get all entries.
    pub fn get_entries(&self) -> &[PerformanceEntry] {
        &self.entries
    }

    /// Get entries by type.
    pub fn get_entries_by_type(&self, entry_type: &EntryType) -> Vec<&PerformanceEntry> {
        self.entries
            .iter()
            .filter(|e| &e.entry_type == entry_type)
            .collect()
    }

    /// Get entries by name.
    pub fn get_entries_by_name(&self, name: &str) -> Vec<&PerformanceEntry> {
        self.entries.iter().filter(|e| e.name == name).collect()
    }

    /// Clear marks.
    pub fn clear_marks(&mut self, name: Option<&str>) {
        self.entries
            .retain(|e| !(e.entry_type == EntryType::Mark && name.map_or(true, |n| e.name == n)));
    }

    /// Clear measures.
    pub fn clear_measures(&mut self, name: Option<&str>) {
        self.entries.retain(|e| {
            !(e.entry_type == EntryType::Measure && name.map_or(true, |n| e.name == n))
        });
    }

    /// Clear resource timing buffer.
    pub fn clear_resource_timings(&mut self) {
        self.resource_buffer.clear();
        self.entries.retain(|e| e.entry_type != EntryType::Resource);
    }

    /// Get the resource timing buffer.
    pub fn resource_timings(&self) -> &VecDeque<ResourceTiming> {
        &self.resource_buffer
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_relative() {
        let perf = Performance::new(1000.0);
        assert!((perf.now(1500.0) - 500.0).abs() < 0.001);
    }

    #[test]
    fn mark_and_measure() {
        let mut perf = Performance::new(0.0);
        perf.mark("start", 100.0);
        perf.mark("end", 300.0);
        let duration = perf.measure("my-measure", "start", "end").unwrap();
        assert!((duration - 200.0).abs() < 0.001);
    }

    #[test]
    fn measure_missing_mark() {
        let mut perf = Performance::new(0.0);
        perf.mark("start", 100.0);
        assert!(perf.measure("m", "start", "nope").is_none());
    }

    #[test]
    fn entries_by_type() {
        let mut perf = Performance::new(0.0);
        perf.mark("a", 10.0);
        perf.mark("b", 20.0);
        perf.record_paint("first-paint", 50.0);

        let marks = perf.get_entries_by_type(&EntryType::Mark);
        assert_eq!(marks.len(), 2);

        let paints = perf.get_entries_by_type(&EntryType::Paint);
        assert_eq!(paints.len(), 1);
    }

    #[test]
    fn entries_by_name() {
        let mut perf = Performance::new(0.0);
        perf.mark("checkpoint", 100.0);
        perf.mark("checkpoint", 200.0);
        let entries = perf.get_entries_by_name("checkpoint");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn clear_marks() {
        let mut perf = Performance::new(0.0);
        perf.mark("a", 10.0);
        perf.mark("b", 20.0);
        perf.clear_marks(Some("a"));
        assert_eq!(perf.get_entries_by_type(&EntryType::Mark).len(), 1);

        perf.mark("c", 30.0);
        perf.clear_marks(None);
        assert_eq!(perf.get_entries_by_type(&EntryType::Mark).len(), 0);
    }

    #[test]
    fn resource_timing() {
        let mut perf = Performance::new(0.0);
        let res = ResourceTiming::new("style.css", "link", 100.0, 250.0);
        perf.add_resource(res);
        assert_eq!(perf.resource_timings().len(), 1);
        assert_eq!(perf.resource_timings()[0].name, "style.css");
    }

    #[test]
    fn resource_buffer_limit() {
        let mut perf = Performance::new(0.0);
        perf.resource_buffer_limit = 3;
        for i in 0..5 {
            perf.add_resource(ResourceTiming::new(&format!("r{i}"), "fetch", 0.0, 1.0));
        }
        assert_eq!(perf.resource_timings().len(), 3);
    }

    #[test]
    fn navigation_timing_metrics() {
        let timing = NavigationTiming {
            time_origin: 0.0,
            request_start: 100.0,
            response_start: 150.0,
            dom_content_loaded: 500.0,
            load_event_end: 800.0,
            dns_start: 10.0,
            dns_end: 30.0,
            connect_start: 30.0,
            connect_end: 60.0,
            ..Default::default()
        };
        assert!((timing.ttfb() - 50.0).abs() < 0.001);
        assert!((timing.dcl() - 500.0).abs() < 0.001);
        assert!((timing.load_time() - 800.0).abs() < 0.001);
        assert!((timing.dns_time() - 20.0).abs() < 0.001);
        assert!((timing.connect_time() - 30.0).abs() < 0.001);
    }

    #[test]
    fn clear_resource_timings() {
        let mut perf = Performance::new(0.0);
        perf.add_resource(ResourceTiming::new("a.js", "script", 0.0, 1.0));
        perf.clear_resource_timings();
        assert!(perf.resource_timings().is_empty());
        assert!(perf.get_entries_by_type(&EntryType::Resource).is_empty());
    }

    #[test]
    fn paint_timing() {
        let mut perf = Performance::new(0.0);
        perf.record_paint("first-paint", 50.0);
        perf.record_paint("first-contentful-paint", 80.0);
        let paints = perf.get_entries_by_type(&EntryType::Paint);
        assert_eq!(paints.len(), 2);
        assert_eq!(paints[0].name, "first-paint");
    }

    #[test]
    fn entry_with_detail() {
        let entry =
            PerformanceEntry::new("test", EntryType::Mark, 0.0, 0.0).with_detail("extra info");
        assert_eq!(entry.detail.as_deref(), Some("extra info"));
    }
}
