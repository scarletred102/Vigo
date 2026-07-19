// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Bookmarks — in-memory bookmark storage with JSON persistence.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use vex_core::error::VexError;
use vex_core::VexResult;

/// Unique identifier for a bookmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkId(pub u64);

/// A single bookmark entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: BookmarkId,
    /// Display title.
    pub title: String,
    /// URL string.
    pub url: String,
    /// Folder path (e.g., "Bookmarks Bar", "Other Bookmarks/Tech").
    pub folder: String,
    /// Unix timestamp (seconds) when created.
    pub created_at: u64,
}

/// Bookmark manager with folder support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkManager {
    bookmarks: Vec<Bookmark>,
    next_id: u64,
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BookmarkManager {
    /// Create an empty bookmark manager.
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a bookmark to the given folder.
    pub fn add(&mut self, title: &str, url: &str, folder: &str) -> BookmarkId {
        let id = BookmarkId(self.next_id);
        self.next_id += 1;

        self.bookmarks.push(Bookmark {
            id,
            title: title.to_string(),
            url: url.to_string(),
            folder: folder.to_string(),
            created_at: current_timestamp(),
        });
        id
    }

    /// Remove a bookmark by ID. Returns whether it was found.
    pub fn remove(&mut self, id: BookmarkId) -> bool {
        let len_before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.id != id);
        self.bookmarks.len() < len_before
    }

    /// Update a bookmark's title and URL.
    pub fn update(&mut self, id: BookmarkId, title: &str, url: &str) -> bool {
        if let Some(b) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            b.title = title.to_string();
            b.url = url.to_string();
            true
        } else {
            false
        }
    }

    /// Get a bookmark by ID.
    pub fn get(&self, id: BookmarkId) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.id == id)
    }

    /// Get all bookmarks in a given folder.
    pub fn in_folder(&self, folder: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.folder == folder)
            .collect()
    }

    /// Get all bookmarks in the bookmarks bar (top-level).
    pub fn bar_bookmarks(&self) -> Vec<&Bookmark> {
        self.in_folder("Bookmarks Bar")
    }

    /// Search bookmarks by title or URL substring.
    pub fn search(&self, query: &str) -> Vec<&Bookmark> {
        let q = query.to_lowercase();
        self.bookmarks
            .iter()
            .filter(|b| b.title.to_lowercase().contains(&q) || b.url.to_lowercase().contains(&q))
            .collect()
    }

    /// Check if a URL is bookmarked.
    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.iter().any(|b| b.url == url)
    }

    /// Total bookmark count.
    pub fn count(&self) -> usize {
        self.bookmarks.len()
    }

    /// Get all bookmarks in the order they were added.
    pub fn all(&self) -> impl Iterator<Item = &Bookmark> {
        self.bookmarks.iter()
    }

    /// All unique folder names.
    pub fn folders(&self) -> Vec<String> {
        let mut seen = HashMap::new();
        for b in &self.bookmarks {
            seen.entry(b.folder.clone()).or_insert(());
        }
        seen.into_keys().collect()
    }

    /// Save bookmarks to a JSON file.
    pub fn save(&self, path: &Path) -> VexResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| VexError::Internal(format!("Failed to serialize bookmarks: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| VexError::Internal(format!("Failed to write bookmarks: {e}")))?;
        Ok(())
    }

    /// Load bookmarks from a JSON file.
    pub fn load(path: &Path) -> VexResult<Self> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| VexError::Internal(format!("Failed to read bookmarks: {e}")))?;
        let mgr: Self = serde_json::from_str(&data)
            .map_err(|e| VexError::Internal(format!("Failed to parse bookmarks: {e}")))?;
        Ok(mgr)
    }
}

/// Get current Unix timestamp in seconds.
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
    fn add_and_get_bookmark() {
        let mut mgr = BookmarkManager::new();
        let id = mgr.add("Rust", "https://rust-lang.org", "Bookmarks Bar");
        let b = mgr.get(id).unwrap();
        assert_eq!(b.title, "Rust");
        assert_eq!(b.url, "https://rust-lang.org");
    }

    #[test]
    fn remove_bookmark() {
        let mut mgr = BookmarkManager::new();
        let id = mgr.add("Test", "https://test.com", "Other");
        assert_eq!(mgr.count(), 1);
        assert!(mgr.remove(id));
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn update_bookmark() {
        let mut mgr = BookmarkManager::new();
        let id = mgr.add("Old", "https://old.com", "Bar");
        mgr.update(id, "New", "https://new.com");
        let b = mgr.get(id).unwrap();
        assert_eq!(b.title, "New");
        assert_eq!(b.url, "https://new.com");
    }

    #[test]
    fn search_bookmarks() {
        let mut mgr = BookmarkManager::new();
        mgr.add("Rust Lang", "https://rust-lang.org", "Dev");
        mgr.add("Zig Lang", "https://ziglang.org", "Dev");
        mgr.add("News", "https://news.com", "Other");

        let results = mgr.search("lang");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn is_bookmarked() {
        let mut mgr = BookmarkManager::new();
        mgr.add("Test", "https://example.com", "Bar");
        assert!(mgr.is_bookmarked("https://example.com"));
        assert!(!mgr.is_bookmarked("https://other.com"));
    }

    #[test]
    fn bar_bookmarks_filter() {
        let mut mgr = BookmarkManager::new();
        mgr.add("A", "https://a.com", "Bookmarks Bar");
        mgr.add("B", "https://b.com", "Other");
        let bar = mgr.bar_bookmarks();
        assert_eq!(bar.len(), 1);
        assert_eq!(bar[0].title, "A");
    }

    #[test]
    fn folders_lists_unique() {
        let mut mgr = BookmarkManager::new();
        mgr.add("A", "https://a.com", "Dev");
        mgr.add("B", "https://b.com", "Dev");
        mgr.add("C", "https://c.com", "Other");
        let folders = mgr.folders();
        assert_eq!(folders.len(), 2);
    }

    #[test]
    fn save_and_load_bookmarks() {
        let mut mgr = BookmarkManager::new();
        mgr.add("Test", "https://test.com", "Bar");

        let dir = std::env::temp_dir().join("vex_bookmark_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bookmarks.json");
        mgr.save(&path).unwrap();

        let loaded = BookmarkManager::load(&path).unwrap();
        assert_eq!(loaded.count(), 1);
        assert!(loaded.is_bookmarked("https://test.com"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
