// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Browsing history — in-memory history with JSON persistence.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vex_core::error::VexError;
use vex_core::VexResult;

/// A single history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    /// URL visited.
    pub url: String,
    /// Page title (may be empty).
    pub title: String,
    /// Unix timestamp of the visit (seconds).
    pub visited_at: u64,
    /// Number of times this URL was visited.
    pub visit_count: u32,
}

/// Browsing history manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowsingHistory {
    records: Vec<HistoryRecord>,
    /// Maximum number of records to keep.
    max_records: usize,
}

impl Default for BrowsingHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowsingHistory {
    /// Create an empty history with default max records.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            max_records: 10_000,
        }
    }

    /// Create an empty history with a custom max record count.
    pub fn with_max_records(max: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records: max,
        }
    }

    /// Record a visit. If the URL was recently visited, increment its count.
    pub fn record_visit(&mut self, url: &str, title: &str) {
        let now = current_timestamp();

        // Check if the most recent entry is the same URL (avoid duplicate consecutive entries).
        if let Some(last) = self.records.last_mut() {
            if last.url == url {
                last.visit_count += 1;
                last.visited_at = now;
                if !title.is_empty() {
                    last.title = title.to_string();
                }
                return;
            }
        }

        self.records.push(HistoryRecord {
            url: url.to_string(),
            title: title.to_string(),
            visited_at: now,
            visit_count: 1,
        });

        // Trim old entries if we exceed the limit.
        if self.records.len() > self.max_records {
            let excess = self.records.len() - self.max_records;
            self.records.drain(..excess);
        }
    }

    /// Get the full history, most recent first.
    pub fn all(&self) -> impl Iterator<Item = &HistoryRecord> {
        self.records.iter().rev()
    }

    /// Recent history (last N entries), most recent first.
    pub fn recent(&self, count: usize) -> Vec<&HistoryRecord> {
        self.records.iter().rev().take(count).collect()
    }

    /// Search history by title or URL substring.
    pub fn search(&self, query: &str) -> Vec<&HistoryRecord> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .rev()
            .filter(|r| r.title.to_lowercase().contains(&q) || r.url.to_lowercase().contains(&q))
            .collect()
    }

    /// Remove all entries matching a URL.
    pub fn remove_url(&mut self, url: &str) {
        self.records.retain(|r| r.url != url);
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Total number of entries.
    pub fn count(&self) -> usize {
        self.records.len()
    }

    /// Save history to a JSON file.
    pub fn save(&self, path: &Path) -> VexResult<()> {
        let json = serde_json::to_string(self)
            .map_err(|e| VexError::Internal(format!("Failed to serialize history: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| VexError::Internal(format!("Failed to write history: {e}")))?;
        Ok(())
    }

    /// Load history from a JSON file.
    pub fn load(path: &Path) -> VexResult<Self> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| VexError::Internal(format!("Failed to read history: {e}")))?;
        let history: Self = serde_json::from_str(&data)
            .map_err(|e| VexError::Internal(format!("Failed to parse history: {e}")))?;
        Ok(history)
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://example.com", "Example");
        history.record_visit("https://rust-lang.org", "Rust");

        assert_eq!(history.count(), 2);
        let recent = history.recent(10);
        assert_eq!(recent[0].url, "https://rust-lang.org");
        assert_eq!(recent[1].url, "https://example.com");
    }

    #[test]
    fn duplicate_consecutive_increments_count() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://example.com", "Example");
        history.record_visit("https://example.com", "Example - Updated");

        assert_eq!(history.count(), 1);
        let r = history.recent(1);
        assert_eq!(r[0].visit_count, 2);
        assert_eq!(r[0].title, "Example - Updated");
    }

    #[test]
    fn search_history() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://rust-lang.org", "Rust Programming");
        history.record_visit("https://zig.dev", "Zig Language");
        history.record_visit("https://example.com", "Example");

        let results = history.search("lang");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn remove_url() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://a.com", "A");
        history.record_visit("https://b.com", "B");
        history.remove_url("https://a.com");
        assert_eq!(history.count(), 1);
    }

    #[test]
    fn clear_history() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://a.com", "A");
        history.record_visit("https://b.com", "B");
        history.clear();
        assert_eq!(history.count(), 0);
    }

    #[test]
    fn max_records_trims_old() {
        let mut history = BrowsingHistory::with_max_records(3);
        for i in 0..5 {
            history.record_visit(&format!("https://{i}.com"), &format!("Page {i}"));
        }
        assert_eq!(history.count(), 3);
        // Should keep the most recent 3.
        let recent = history.recent(3);
        assert_eq!(recent[0].url, "https://4.com");
    }

    #[test]
    fn save_and_load() {
        let mut history = BrowsingHistory::new();
        history.record_visit("https://example.com", "Test");

        let dir = std::env::temp_dir().join("vex_history_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.json");
        history.save(&path).unwrap();

        let loaded = BrowsingHistory::load(&path).unwrap();
        assert_eq!(loaded.count(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
