// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tab manager — owns all tabs and provides browser-level tab operations.

use vex_core::VexUrl;

use crate::tab::{Tab, TabId};

/// Manages the collection of open browser tabs.
pub struct TabManager {
    /// All open tabs in display order.
    tabs: Vec<Tab>,
    /// Index of the currently active (visible) tab.
    active_index: usize,
    /// Monotonically increasing counter for unique tab IDs.
    next_id: u32,
    /// Stack of recently closed tab URLs (for "reopen closed tab").
    closed_stack: Vec<VexUrl>,
}

impl TabManager {
    /// Create a new tab manager with one blank tab.
    pub fn new() -> Self {
        let mut mgr = Self {
            tabs: Vec::new(),
            active_index: 0,
            next_id: 1,
            closed_stack: Vec::new(),
        };
        mgr.new_tab_blank();
        mgr
    }

    /// Open a new tab at the given URL and switch to it.
    /// Returns the new tab's ID.
    pub fn new_tab(&mut self, url: VexUrl) -> TabId {
        let id = TabId::new(self.next_id);
        self.next_id += 1;
        let tab = Tab::new(id, url);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        id
    }

    /// Open a new blank tab and switch to it.
    pub fn new_tab_blank(&mut self) -> TabId {
        let id = TabId::new(self.next_id);
        self.next_id += 1;
        let tab = Tab::blank(id);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        id
    }

    /// Close the tab with the given ID.
    /// Returns `true` if the tab was found and closed.
    /// If the last tab is closed, a new blank tab is created automatically.
    pub fn close_tab(&mut self, id: TabId) -> bool {
        let Some(pos) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };

        // Save the URL for the closed-tabs stack.
        let closed_url = self.tabs[pos].url.clone();
        self.closed_stack.push(closed_url);

        self.tabs.remove(pos);

        // Ensure we always have at least one tab.
        if self.tabs.is_empty() {
            self.new_tab_blank();
            return true;
        }

        // Adjust the active index.
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        } else if pos < self.active_index {
            self.active_index -= 1;
        }

        true
    }

    /// Switch to the tab with the given ID.
    /// Returns `true` if the tab was found.
    pub fn switch_to(&mut self, id: TabId) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.active_index = pos;
            true
        } else {
            false
        }
    }

    /// Get a reference to the currently active tab.
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    /// Get a mutable reference to the currently active tab.
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    /// Get the ID of the currently active tab.
    pub fn active_tab_id(&self) -> TabId {
        self.tabs[self.active_index].id
    }

    /// Get a reference to a tab by ID.
    pub fn tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Get a mutable reference to a tab by ID.
    pub fn tab_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Move a tab from one position to another in the tab bar.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let was_active = self.active_index == from;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        if was_active {
            self.active_index = to;
        } else if from < self.active_index && to >= self.active_index {
            self.active_index -= 1;
        } else if from > self.active_index && to <= self.active_index {
            self.active_index += 1;
        }
    }

    /// Duplicate the tab with the given ID. Creates a new tab with the same URL.
    /// Returns the new tab's ID, or `None` if the source tab wasn't found.
    pub fn duplicate_tab(&mut self, id: TabId) -> Option<TabId> {
        let url = self.tab(id)?.url.clone();
        let title = self.tab(id)?.title.clone();
        let new_id = self.new_tab(url);
        if let Some(tab) = self.tab_mut(new_id) {
            tab.set_title(title);
        }
        Some(new_id)
    }

    /// Reopen the most recently closed tab, if any.
    /// Returns the new tab's ID, or `None` if no tabs have been closed.
    pub fn reopen_closed_tab(&mut self) -> Option<TabId> {
        let url = self.closed_stack.pop()?;
        Some(self.new_tab(url))
    }

    /// Number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Iterate over all tabs in display order.
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Switch to the next tab (wraps around).
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// Switch to the previous tab (wraps around).
    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// Index of the currently active tab.
    pub fn active_index(&self) -> usize {
        self.active_index
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_has_one_blank_tab() {
        let mgr = TabManager::new();
        assert_eq!(mgr.tab_count(), 1);
        assert_eq!(mgr.active_tab().title, "New Tab");
    }

    #[test]
    fn new_tab_and_switch() {
        let mut mgr = TabManager::new();
        let id1 = mgr.active_tab_id();

        let url = VexUrl::parse("https://example.com").unwrap();
        let id2 = mgr.new_tab(url);
        assert_eq!(mgr.tab_count(), 2);
        assert_eq!(mgr.active_tab_id(), id2);

        assert!(mgr.switch_to(id1));
        assert_eq!(mgr.active_tab_id(), id1);
    }

    #[test]
    fn close_tab_keeps_at_least_one() {
        let mut mgr = TabManager::new();
        let id = mgr.active_tab_id();
        assert!(mgr.close_tab(id));
        // A new blank tab should have been created.
        assert_eq!(mgr.tab_count(), 1);
    }

    #[test]
    fn close_nonexistent_tab_returns_false() {
        let mut mgr = TabManager::new();
        assert!(!mgr.close_tab(TabId::new(999)));
    }

    #[test]
    fn move_tab_reorders() {
        let mut mgr = TabManager::new();
        let url_a = VexUrl::parse("https://a.com").unwrap();
        let url_b = VexUrl::parse("https://b.com").unwrap();
        let _id1 = mgr.active_tab_id();
        let _id2 = mgr.new_tab(url_a);
        let id3 = mgr.new_tab(url_b);

        // Active is last (index 2). Move it to index 0.
        assert_eq!(mgr.active_tab_id(), id3);
        mgr.move_tab(2, 0);
        assert_eq!(mgr.active_tab_id(), id3);
        assert_eq!(mgr.active_index(), 0);
    }

    #[test]
    fn duplicate_tab_copies_url() {
        let mut mgr = TabManager::new();
        let url = VexUrl::parse("https://example.com").unwrap();
        let id1 = mgr.new_tab(url.clone());
        if let Some(tab) = mgr.tab_mut(id1) {
            tab.set_title("Example");
        }

        let id2 = mgr.duplicate_tab(id1).unwrap();
        let dup = mgr.tab(id2).unwrap();
        assert_eq!(dup.url, url);
        assert_eq!(dup.title, "Example");
    }

    #[test]
    fn reopen_closed_tab_restores_url() {
        let mut mgr = TabManager::new();
        let url = VexUrl::parse("https://closed.example.com").unwrap();
        let id = mgr.new_tab(url.clone());
        mgr.close_tab(id);

        let reopened_id = mgr.reopen_closed_tab().unwrap();
        let tab = mgr.tab(reopened_id).unwrap();
        assert_eq!(tab.url, url);
    }

    #[test]
    fn next_prev_tab_wraps() {
        let mut mgr = TabManager::new();
        let _id2 = mgr.new_tab(VexUrl::parse("https://two.com").unwrap());
        let _id3 = mgr.new_tab(VexUrl::parse("https://three.com").unwrap());
        // active_index == 2
        assert_eq!(mgr.active_index(), 2);
        mgr.next_tab();
        assert_eq!(mgr.active_index(), 0); // wraps around
        mgr.prev_tab();
        assert_eq!(mgr.active_index(), 2); // wraps back
    }
}
