// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Session persistence — save/restore open tabs on browser close/start.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vex_core::{VexResult, VexUrl};

/// Serializable session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// URLs of all open tabs (in order).
    pub tabs: Vec<TabSnapshot>,
    /// Index of the active tab.
    pub active_tab: usize,
}

/// Snapshot of a single tab for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSnapshot {
    /// URL of the page.
    pub url: String,
    /// Page title.
    pub title: String,
    /// Vertical scroll offset.
    pub scroll_y: f32,
}

impl SessionState {
    /// Create a new empty session.
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
        }
    }

    /// Save the session to a JSON file.
    pub fn save(&self, path: &Path) -> VexResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| vex_core::VexError::Internal(format!("session serialize: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| vex_core::VexError::Internal(format!("session write: {e}")))?;
        tracing::info!(
            "Session saved: {} tabs to {}",
            self.tabs.len(),
            path.display()
        );
        Ok(())
    }

    /// Load a session from a JSON file.
    /// Returns an empty session if the file doesn't exist.
    pub fn load(path: &Path) -> VexResult<Self> {
        if !path.exists() {
            return Ok(Self::empty());
        }
        let json = std::fs::read_to_string(path)
            .map_err(|e| vex_core::VexError::Internal(format!("session read: {e}")))?;
        let state: Self = serde_json::from_str(&json)
            .map_err(|e| vex_core::VexError::Internal(format!("session parse: {e}")))?;
        tracing::info!(
            "Session loaded: {} tabs from {}",
            state.tabs.len(),
            path.display()
        );
        Ok(state)
    }

    /// Build a session state from the current tab manager.
    pub fn from_tabs(tabs: &[crate::Tab], active_index: usize) -> Self {
        let snapshots = tabs
            .iter()
            .map(|t| TabSnapshot {
                url: t.url.to_string(),
                title: t.title.clone(),
                scroll_y: t.scroll.offset_y,
            })
            .collect();
        Self {
            tabs: snapshots,
            active_tab: active_index,
        }
    }

    /// Restore tabs into a tab manager (lazy — only URLs are loaded, pages are not fetched).
    pub fn restore_into(&self, tab_mgr: &mut crate::TabManager) {
        for snap in &self.tabs {
            if let Ok(url) = VexUrl::parse(&snap.url) {
                let id = tab_mgr.new_tab(url);
                if let Some(tab) = tab_mgr.tab_mut(id) {
                    tab.set_title(&snap.title);
                    tab.scroll.scroll_to(0.0, snap.scroll_y);
                }
            }
        }
        // Switch to the previously active tab (the first restored tab is at index 1
        // because TabManager initializes with a blank tab — we'll remove that blank
        // tab if we restored any).
        if !self.tabs.is_empty() {
            // Close the initial blank tab.
            let blank_id = tab_mgr.tabs()[0].id;
            tab_mgr.close_tab(blank_id);

            let target_index = self.active_tab.min(tab_mgr.tab_count().saturating_sub(1));
            let target_id = tab_mgr.tabs()[target_index].id;
            tab_mgr.switch_to(target_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_serialize() {
        let state = SessionState {
            tabs: vec![
                TabSnapshot {
                    url: "https://example.com".into(),
                    title: "Example".into(),
                    scroll_y: 150.0,
                },
                TabSnapshot {
                    url: "https://rust-lang.org".into(),
                    title: "Rust".into(),
                    scroll_y: 0.0,
                },
            ],
            active_tab: 1,
        };

        let json = serde_json::to_string(&state).unwrap();
        let restored: SessionState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tabs.len(), 2);
        assert_eq!(restored.active_tab, 1);
        assert_eq!(restored.tabs[0].url, "https://example.com");
        assert_eq!(restored.tabs[1].title, "Rust");
    }

    #[test]
    fn empty_session() {
        let state = SessionState::empty();
        assert!(state.tabs.is_empty());
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join("vex_test_session");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("session.json");

        let state = SessionState {
            tabs: vec![TabSnapshot {
                url: "https://example.com".into(),
                title: "Example".into(),
                scroll_y: 42.0,
            }],
            active_tab: 0,
        };

        state.save(&path).unwrap();
        let loaded = SessionState::load(&path).unwrap();

        assert_eq!(loaded.tabs.len(), 1);
        assert_eq!(loaded.tabs[0].scroll_y, 42.0);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let path = Path::new("nonexistent_session_file.json");
        let loaded = SessionState::load(path).unwrap();
        assert!(loaded.tabs.is_empty());
    }

    #[test]
    fn from_tabs_snapshots_correctly() {
        let url = VexUrl::parse("https://test.com").unwrap();
        let mut tab = crate::Tab::new(crate::TabId::new(1), url);
        tab.set_title("Test");
        // Must set content size > viewport so scroll_to doesn't clamp to 0.
        tab.scroll.set_content_size(1280.0, 2000.0);
        tab.scroll.scroll_to(0.0, 200.0);

        let state = SessionState::from_tabs(&[tab], 0);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].title, "Test");
        assert_eq!(state.tabs[0].scroll_y, 200.0);
    }
}
