// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Navigation history — per-tab back/forward stack.

use vex_core::geometry::Point;
use vex_core::VexUrl;

/// A single navigation history entry.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// URL of the page.
    pub url: VexUrl,
    /// Page title at time of visit.
    pub title: String,
    /// Scroll position when the user left this page.
    pub scroll_position: Point,
}

/// Per-tab navigation history with back/forward support.
#[derive(Debug)]
pub struct NavigationHistory {
    /// All history entries.
    entries: Vec<HistoryEntry>,
    /// Index of the currently displayed entry.
    current_index: usize,
}

impl NavigationHistory {
    /// Create an empty navigation history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_index: 0,
        }
    }

    /// Push a new URL onto the history, discarding any forward entries.
    pub fn push(&mut self, url: VexUrl, title: String, scroll_position: Point) {
        // Drop everything after current_index.
        if !self.entries.is_empty() {
            self.entries.truncate(self.current_index + 1);
        }
        self.entries.push(HistoryEntry {
            url,
            title,
            scroll_position,
        });
        self.current_index = self.entries.len() - 1;
    }

    /// Navigate back. Returns the previous entry, or `None` if at the start.
    pub fn back(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_back() {
            self.current_index -= 1;
            Some(&self.entries[self.current_index])
        } else {
            None
        }
    }

    /// Navigate forward. Returns the next entry, or `None` if at the end.
    pub fn forward(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_forward() {
            self.current_index += 1;
            Some(&self.entries[self.current_index])
        } else {
            None
        }
    }

    /// Whether there is a previous entry to go back to.
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    /// Whether there is a next entry to go forward to.
    pub fn can_go_forward(&self) -> bool {
        self.current_index + 1 < self.entries.len()
    }

    /// Get the current history entry.
    pub fn current(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.current_index)
    }

    /// Update the scroll position of the current entry.
    pub fn update_scroll(&mut self, scroll: Point) {
        if let Some(entry) = self.entries.get_mut(self.current_index) {
            entry.scroll_position = scroll;
        }
    }

    /// Total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current index in the history stack.
    pub fn current_index(&self) -> usize {
        self.current_index
    }
}

impl Default for NavigationHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> VexUrl {
        VexUrl::parse(s).unwrap()
    }

    #[test]
    fn push_and_navigate() {
        let mut history = NavigationHistory::new();
        history.push(url("https://a.com"), "A".into(), Point::new(0.0, 0.0));
        history.push(url("https://b.com"), "B".into(), Point::new(0.0, 100.0));
        history.push(url("https://c.com"), "C".into(), Point::new(0.0, 0.0));

        assert_eq!(history.len(), 3);
        assert!(!history.can_go_forward());
        assert!(history.can_go_back());

        let entry = history.back().unwrap();
        assert_eq!(entry.title, "B");

        let entry = history.forward().unwrap();
        assert_eq!(entry.title, "C");
    }

    #[test]
    fn push_after_back_truncates_forward() {
        let mut history = NavigationHistory::new();
        history.push(url("https://a.com"), "A".into(), Point::new(0.0, 0.0));
        history.push(url("https://b.com"), "B".into(), Point::new(0.0, 0.0));
        history.push(url("https://c.com"), "C".into(), Point::new(0.0, 0.0));

        history.back(); // now at B
        history.push(url("https://d.com"), "D".into(), Point::new(0.0, 0.0));

        assert_eq!(history.len(), 3); // A, B, D — C was dropped
        assert!(!history.can_go_forward());
        assert_eq!(history.current().unwrap().title, "D");
    }

    #[test]
    fn back_at_start_returns_none() {
        let mut history = NavigationHistory::new();
        history.push(url("https://a.com"), "A".into(), Point::new(0.0, 0.0));
        assert!(history.back().is_none());
    }

    #[test]
    fn forward_at_end_returns_none() {
        let mut history = NavigationHistory::new();
        history.push(url("https://a.com"), "A".into(), Point::new(0.0, 0.0));
        assert!(history.forward().is_none());
    }

    #[test]
    fn update_scroll_modifies_current() {
        let mut history = NavigationHistory::new();
        history.push(url("https://a.com"), "A".into(), Point::new(0.0, 0.0));
        history.update_scroll(Point::new(0.0, 500.0));
        assert_eq!(history.current().unwrap().scroll_position.y, 500.0);
    }
}
