// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTML5 Session History API — pushState / replaceState / back / forward / go.
//!
//! This implements the per-tab session history stack that JavaScript interacts
//! with via `window.history`. It is distinct from the global browsing history
//! (`BrowsingHistory`) which records all visits for the omnibar.
//!
//! Each tab has a `SessionHistory` that tracks navigation entries. The API:
//! - `push_state(state, title, url)` — adds a new entry
//! - `replace_state(state, title, url)` — replaces the current entry
//! - `go(delta)` — move forward/backward by delta entries
//! - `back()` / `forward()` — convenience for go(-1) and go(1)
//! - `state()` — current entry's state
//! - `length` — total number of entries

use serde::{Deserialize, Serialize};

/// A single entry in the session history stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Serialized state object (JSON string).
    pub state: Option<String>,
    /// Document title (advisory, per spec).
    pub title: String,
    /// URL for the entry. Must be same-origin as current page.
    pub url: String,
    /// Scroll position to restore.
    pub scroll_x: f64,
    pub scroll_y: f64,
}

impl HistoryEntry {
    pub fn new(url: &str, title: &str, state: Option<String>) -> Self {
        Self {
            state,
            title: title.to_owned(),
            url: url.to_owned(),
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }
}

/// Navigation direction for popstate events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Forward,
    Back,
}

/// Result of a navigation attempt.
#[derive(Debug, Clone)]
pub enum NavigationResult {
    /// Navigation succeeded — the entry to navigate to.
    Navigated {
        entry: HistoryEntry,
        direction: NavigationDirection,
    },
    /// No navigation occurred (at boundary).
    AtBoundary,
}

/// Per-tab session history (HTML5 History API).
#[derive(Debug, Clone)]
pub struct SessionHistory {
    entries: Vec<HistoryEntry>,
    /// Index of the current entry.
    current: usize,
    /// Maximum entries to keep.
    max_entries: usize,
}

impl Default for SessionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionHistory {
    /// Create a new empty session history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current: 0,
            max_entries: 50,
        }
    }

    /// Create with a custom max entries limit.
    pub fn with_max_entries(max: usize) -> Self {
        Self {
            entries: Vec::new(),
            current: 0,
            max_entries: max,
        }
    }

    /// Initialize with the first navigation entry.
    pub fn navigate(&mut self, url: &str, title: &str) {
        // Truncate forward history.
        if !self.entries.is_empty() {
            self.entries.truncate(self.current + 1);
        }

        self.entries.push(HistoryEntry::new(url, title, None));
        self.current = self.entries.len() - 1;

        // Trim old entries if we exceed the limit.
        if self.entries.len() > self.max_entries {
            let excess = self.entries.len() - self.max_entries;
            self.entries.drain(..excess);
            self.current = self.entries.len() - 1;
        }
    }

    /// `history.pushState(state, title, url)` — add a new entry without navigation.
    ///
    /// Truncates any forward history, then pushes the new entry.
    pub fn push_state(&mut self, state: Option<String>, title: &str, url: &str) {
        // Truncate forward history beyond current.
        if !self.entries.is_empty() {
            self.entries.truncate(self.current + 1);
        }

        self.entries.push(HistoryEntry::new(url, title, state));
        self.current = self.entries.len() - 1;

        if self.entries.len() > self.max_entries {
            let excess = self.entries.len() - self.max_entries;
            self.entries.drain(..excess);
            self.current = self.entries.len() - 1;
        }
    }

    /// `history.replaceState(state, title, url)` — replace the current entry.
    pub fn replace_state(&mut self, state: Option<String>, title: &str, url: &str) {
        if let Some(entry) = self.entries.get_mut(self.current) {
            entry.state = state;
            entry.title = title.to_owned();
            entry.url = url.to_owned();
        } else {
            // No current entry — push as first.
            self.entries.push(HistoryEntry::new(url, title, state));
            self.current = 0;
        }
    }

    /// `history.go(delta)` — navigate by delta entries.
    ///
    /// Returns the entry navigated to, or `AtBoundary` if the delta exceeds bounds.
    pub fn go(&mut self, delta: i32) -> NavigationResult {
        let new_index = self.current as i64 + delta as i64;
        if new_index < 0 || new_index >= self.entries.len() as i64 {
            return NavigationResult::AtBoundary;
        }
        let new_index = new_index as usize;
        let direction = if delta < 0 {
            NavigationDirection::Back
        } else {
            NavigationDirection::Forward
        };
        self.current = new_index;
        NavigationResult::Navigated {
            entry: self.entries[self.current].clone(),
            direction,
        }
    }

    /// `history.back()` — go back one entry.
    pub fn back(&mut self) -> NavigationResult {
        self.go(-1)
    }

    /// `history.forward()` — go forward one entry.
    pub fn forward(&mut self) -> NavigationResult {
        self.go(1)
    }

    /// `history.length` — total number of entries.
    pub fn length(&self) -> usize {
        self.entries.len()
    }

    /// `history.state` — the state of the current entry.
    pub fn state(&self) -> Option<&str> {
        self.entries.get(self.current)?.state.as_deref()
    }

    /// The URL of the current entry.
    pub fn current_url(&self) -> Option<&str> {
        self.entries.get(self.current).map(|e| e.url.as_str())
    }

    /// The title of the current entry.
    pub fn current_title(&self) -> Option<&str> {
        self.entries.get(self.current).map(|e| e.title.as_str())
    }

    /// Get the current entry.
    pub fn current_entry(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.current)
    }

    /// Current index in the history stack.
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Can we go back?
    pub fn can_go_back(&self) -> bool {
        self.current > 0
    }

    /// Can we go forward?
    pub fn can_go_forward(&self) -> bool {
        self.current + 1 < self.entries.len()
    }

    /// Save scroll position for the current entry.
    pub fn save_scroll(&mut self, x: f64, y: f64) {
        if let Some(entry) = self.entries.get_mut(self.current) {
            entry.scroll_x = x;
            entry.scroll_y = y;
        }
    }

    /// Get saved scroll position for the current entry.
    pub fn scroll_position(&self) -> (f64, f64) {
        self.entries
            .get(self.current)
            .map(|e| (e.scroll_x, e.scroll_y))
            .unwrap_or((0.0, 0.0))
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_navigate() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "Page A");
        assert_eq!(h.length(), 1);
        assert_eq!(h.current_url(), Some("https://a.com"));
    }

    #[test]
    fn push_state_adds_entry() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.push_state(
            Some(r#"{"page": 2}"#.into()),
            "A Page 2",
            "https://a.com/page/2",
        );

        assert_eq!(h.length(), 2);
        assert_eq!(h.current_url(), Some("https://a.com/page/2"));
        assert_eq!(h.state(), Some(r#"{"page": 2}"#));
    }

    #[test]
    fn push_state_truncates_forward() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.navigate("https://b.com", "B");
        h.navigate("https://c.com", "C");

        // Go back to B.
        h.back();
        assert_eq!(h.current_url(), Some("https://b.com"));

        // Push new entry → should truncate C.
        h.push_state(None, "D", "https://d.com");
        assert_eq!(h.length(), 3); // A, B, D (C removed)
        assert!(!h.can_go_forward());
    }

    #[test]
    fn replace_state() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.replace_state(Some("replaced".into()), "A v2", "https://a.com/v2");

        assert_eq!(h.length(), 1); // No new entry.
        assert_eq!(h.current_url(), Some("https://a.com/v2"));
        assert_eq!(h.state(), Some("replaced"));
    }

    #[test]
    fn back_and_forward() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.navigate("https://b.com", "B");
        h.navigate("https://c.com", "C");

        assert!(h.can_go_back());
        assert!(!h.can_go_forward());

        let result = h.back();
        assert!(matches!(
            result,
            NavigationResult::Navigated {
                direction: NavigationDirection::Back,
                ..
            }
        ));
        assert_eq!(h.current_url(), Some("https://b.com"));
        assert!(h.can_go_forward());

        h.forward();
        assert_eq!(h.current_url(), Some("https://c.com"));
    }

    #[test]
    fn go_with_delta() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.navigate("https://b.com", "B");
        h.navigate("https://c.com", "C");
        h.navigate("https://d.com", "D");

        h.go(-2);
        assert_eq!(h.current_url(), Some("https://b.com"));

        h.go(2);
        assert_eq!(h.current_url(), Some("https://d.com"));
    }

    #[test]
    fn go_at_boundary() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");

        let result = h.back();
        assert!(matches!(result, NavigationResult::AtBoundary));

        let result = h.forward();
        assert!(matches!(result, NavigationResult::AtBoundary));
    }

    #[test]
    fn go_beyond_boundary() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.navigate("https://b.com", "B");

        let result = h.go(-10);
        assert!(matches!(result, NavigationResult::AtBoundary));
        // Current should not have changed.
        assert_eq!(h.current_url(), Some("https://b.com"));
    }

    #[test]
    fn max_entries_trims() {
        let mut h = SessionHistory::with_max_entries(3);
        for i in 0..5 {
            h.navigate(&format!("https://{i}.com"), &format!("Page {i}"));
        }
        assert_eq!(h.length(), 3);
        assert_eq!(h.current_url(), Some("https://4.com"));
    }

    #[test]
    fn scroll_position_save_restore() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.save_scroll(100.0, 500.0);
        h.navigate("https://b.com", "B");

        // Going back should restore scroll position of A.
        h.back();
        let (x, y) = h.scroll_position();
        assert_eq!(x, 100.0);
        assert_eq!(y, 500.0);
    }

    #[test]
    fn replace_state_on_empty() {
        let mut h = SessionHistory::new();
        h.replace_state(Some("initial".into()), "First", "https://first.com");
        assert_eq!(h.length(), 1);
        assert_eq!(h.state(), Some("initial"));
    }

    #[test]
    fn current_index() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        h.navigate("https://b.com", "B");
        h.navigate("https://c.com", "C");
        assert_eq!(h.current_index(), 2);
        h.back();
        assert_eq!(h.current_index(), 1);
    }

    #[test]
    fn state_none_by_default() {
        let mut h = SessionHistory::new();
        h.navigate("https://a.com", "A");
        assert_eq!(h.state(), None);
    }

    #[test]
    fn entry_clone_and_fields() {
        let entry = HistoryEntry::new("https://test.com", "Test", Some("data".into()));
        assert_eq!(entry.url, "https://test.com");
        assert_eq!(entry.title, "Test");
        assert_eq!(entry.state.as_deref(), Some("data"));
        assert_eq!(entry.scroll_x, 0.0);
        assert_eq!(entry.scroll_y, 0.0);

        let cloned = entry.clone();
        assert_eq!(cloned.url, entry.url);
    }
}
