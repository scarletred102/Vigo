// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Navigation API implementation.
//!
//! Provides the modern Navigation API (`window.navigation`) for managing
//! browser navigation state, intercepts, and transitions.
//! <https://html.spec.whatwg.org/multipage/nav-history-apis.html#navigation-api>

use std::collections::VecDeque;
use std::time::Instant;

// ── NavigationType ───────────────────────────────────────────────────────────

/// The type of navigation that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationType {
    /// A push navigation (new entry).
    Push,
    /// A replace navigation (replaces current entry).
    Replace,
    /// A reload.
    Reload,
    /// A traverse (back/forward).
    Traverse,
}

impl NavigationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Push => "push",
            Self::Replace => "replace",
            Self::Reload => "reload",
            Self::Traverse => "traverse",
        }
    }
}

// ── NavigationHistoryEntry ───────────────────────────────────────────────────

/// A single entry in the navigation history.
#[derive(Debug, Clone)]
pub struct NavigationHistoryEntry {
    /// Unique key for this entry (survives reloads).
    pub key: String,
    /// Unique ID for this entry (doesn't survive reloads).
    pub id: String,
    /// The URL of this entry.
    pub url: String,
    /// The index of this entry in the list.
    pub index: i32,
    /// Whether this entry can be reached via same-document navigation.
    pub same_document: bool,
    /// Serialized state associated with this entry.
    pub state: Option<String>,
    /// When this entry was created.
    pub timestamp: Instant,
}

impl NavigationHistoryEntry {
    fn new(key: &str, id: &str, url: &str, index: i32) -> Self {
        Self {
            key: key.to_string(),
            id: id.to_string(),
            url: url.to_string(),
            index,
            same_document: true,
            state: None,
            timestamp: Instant::now(),
        }
    }

    /// Get the state, if any.
    pub fn get_state(&self) -> Option<&str> {
        self.state.as_deref()
    }
}

// ── NavigateEvent ────────────────────────────────────────────────────────────

/// A NavigateEvent dispatched when navigation is about to occur.
#[derive(Debug, Clone)]
pub struct NavigateEvent {
    /// The navigation type.
    pub navigation_type: NavigationType,
    /// The destination URL.
    pub destination_url: String,
    /// Whether this can be intercepted.
    pub can_intercept: bool,
    /// Whether a user gesture initiated this.
    pub user_initiated: bool,
    /// Whether this replaces the current entry.
    pub hash_change: bool,
    /// Download request filename, if any.
    pub download_request: Option<String>,
    /// Form data, if any.
    pub form_data: Option<String>,
    /// Whether the event has been intercepted.
    pub intercepted: bool,
    /// Whether the event has been cancelled.
    pub cancelled: bool,
}

impl NavigateEvent {
    pub fn new(nav_type: NavigationType, url: &str) -> Self {
        Self {
            navigation_type: nav_type,
            destination_url: url.to_string(),
            can_intercept: true,
            user_initiated: false,
            hash_change: false,
            download_request: None,
            form_data: None,
            intercepted: false,
            cancelled: false,
        }
    }

    pub fn intercept(&mut self) {
        if self.can_intercept {
            self.intercepted = true;
        }
    }

    pub fn prevent_default(&mut self) {
        self.cancelled = true;
    }
}

// ── NavigationTransition ─────────────────────────────────────────────────────

/// Represents an ongoing navigation transition.
#[derive(Debug, Clone)]
pub struct NavigationTransition {
    /// The navigation type.
    pub navigation_type: NavigationType,
    /// The entry being navigated from.
    pub from_url: String,
    /// Whether the transition is complete.
    pub finished: bool,
    /// Start time.
    pub started_at: Instant,
}

impl NavigationTransition {
    fn new(nav_type: NavigationType, from_url: &str) -> Self {
        Self {
            navigation_type: nav_type,
            from_url: from_url.to_string(),
            finished: false,
            started_at: Instant::now(),
        }
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }
}

// ── Navigation ───────────────────────────────────────────────────────────────

/// The main Navigation API object (corresponds to `window.navigation`).
pub struct Navigation {
    /// All history entries.
    entries: VecDeque<NavigationHistoryEntry>,
    /// Index of the current entry.
    current_index: i32,
    /// Counter for generating unique keys.
    next_key: u64,
    /// Counter for generating unique IDs.
    next_id: u64,
    /// Current transition, if any.
    pub transition: Option<NavigationTransition>,
    /// Whether back navigation is possible.
    pub can_go_back: bool,
    /// Whether forward navigation is possible.
    pub can_go_forward: bool,
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}

impl Navigation {
    pub fn new() -> Self {
        let mut nav = Self {
            entries: VecDeque::new(),
            current_index: -1,
            next_key: 1,
            next_id: 1,
            transition: None,
            can_go_back: false,
            can_go_forward: false,
        };
        // Push an initial blank entry
        nav.push_entry("about:blank");
        nav
    }

    /// Get the current entry.
    pub fn current_entry(&self) -> Option<&NavigationHistoryEntry> {
        if self.current_index >= 0 {
            self.entries.get(self.current_index as usize)
        } else {
            None
        }
    }

    /// Get all entries.
    pub fn entries(&self) -> Vec<&NavigationHistoryEntry> {
        self.entries.iter().collect()
    }

    /// Navigate to a new URL (push).
    pub fn navigate(&mut self, url: &str) -> NavigateEvent {
        let event = NavigateEvent::new(NavigationType::Push, url);
        if !event.cancelled {
            let from_url = self
                .current_entry()
                .map(|e| e.url.clone())
                .unwrap_or_default();
            self.transition = Some(NavigationTransition::new(NavigationType::Push, &from_url));
            // Remove forward entries
            while self.entries.len() as i32 > self.current_index + 1 {
                self.entries.pop_back();
            }
            self.push_entry(url);
            self.finish_transition();
        }
        self.update_can_go();
        event
    }

    /// Navigate with replace semantics.
    pub fn navigate_replace(&mut self, url: &str) -> NavigateEvent {
        let event = NavigateEvent::new(NavigationType::Replace, url);
        if !event.cancelled {
            let new_id = self.generate_id();
            if let Some(entry) = self.entries.get_mut(self.current_index as usize) {
                entry.url = url.to_string();
                entry.id = new_id;
                entry.timestamp = Instant::now();
            }
        }
        event
    }

    /// Go back in history.
    pub fn back(&mut self) -> Option<NavigateEvent> {
        if self.current_index <= 0 {
            return None;
        }
        let from_url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        self.current_index -= 1;
        self.transition = Some(NavigationTransition::new(
            NavigationType::Traverse,
            &from_url,
        ));
        self.finish_transition();
        self.update_can_go();
        let url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        Some(NavigateEvent::new(NavigationType::Traverse, &url))
    }

    /// Go forward in history.
    pub fn forward(&mut self) -> Option<NavigateEvent> {
        if self.current_index as usize >= self.entries.len() - 1 {
            return None;
        }
        let from_url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        self.current_index += 1;
        self.transition = Some(NavigationTransition::new(
            NavigationType::Traverse,
            &from_url,
        ));
        self.finish_transition();
        self.update_can_go();
        let url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        Some(NavigateEvent::new(NavigationType::Traverse, &url))
    }

    /// Traverse to a specific entry by key.
    pub fn traverse_to(&mut self, key: &str) -> Option<NavigateEvent> {
        let target_idx = self.entries.iter().position(|e| e.key == key)?;
        let from_url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        self.current_index = target_idx as i32;
        self.transition = Some(NavigationTransition::new(
            NavigationType::Traverse,
            &from_url,
        ));
        self.finish_transition();
        self.update_can_go();
        let url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        Some(NavigateEvent::new(NavigationType::Traverse, &url))
    }

    /// Reload the current entry.
    pub fn reload(&mut self) -> NavigateEvent {
        let url = self
            .current_entry()
            .map(|e| e.url.clone())
            .unwrap_or_default();
        NavigateEvent::new(NavigationType::Reload, &url)
    }

    /// Update the state of the current entry.
    pub fn update_current_entry_state(&mut self, state: &str) {
        if let Some(entry) = self.entries.get_mut(self.current_index as usize) {
            entry.state = Some(state.to_string());
        }
    }

    // Internal helpers

    fn push_entry(&mut self, url: &str) {
        let key = self.generate_key();
        let id = self.generate_id();
        self.current_index += 1;
        let entry = NavigationHistoryEntry::new(&key, &id, url, self.current_index);
        self.entries.push_back(entry);
    }

    fn generate_key(&mut self) -> String {
        let key = format!("nav-key-{}", self.next_key);
        self.next_key += 1;
        key
    }

    fn generate_id(&mut self) -> String {
        let id = format!("nav-id-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn finish_transition(&mut self) {
        if let Some(ref mut t) = self.transition {
            t.finish();
        }
    }

    fn update_can_go(&mut self) {
        self.can_go_back = self.current_index > 0;
        self.can_go_forward = (self.current_index as usize) < self.entries.len().saturating_sub(1);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_initial_state() {
        let nav = Navigation::new();
        let entry = nav.current_entry().unwrap();
        assert_eq!(entry.url, "about:blank");
        assert!(!nav.can_go_back);
        assert!(!nav.can_go_forward);
    }

    #[test]
    fn navigation_push() {
        let mut nav = Navigation::new();
        nav.navigate("https://example.com");
        let entry = nav.current_entry().unwrap();
        assert_eq!(entry.url, "https://example.com");
        assert!(nav.can_go_back);
        assert!(!nav.can_go_forward);
    }

    #[test]
    fn navigation_back_forward() {
        let mut nav = Navigation::new();
        nav.navigate("https://a.com");
        nav.navigate("https://b.com");
        assert_eq!(nav.current_entry().unwrap().url, "https://b.com");

        nav.back();
        assert_eq!(nav.current_entry().unwrap().url, "https://a.com");
        assert!(nav.can_go_forward);

        nav.forward();
        assert_eq!(nav.current_entry().unwrap().url, "https://b.com");
    }

    #[test]
    fn navigation_replace() {
        let mut nav = Navigation::new();
        nav.navigate("https://example.com");
        nav.navigate_replace("https://replaced.com");
        assert_eq!(nav.current_entry().unwrap().url, "https://replaced.com");
        assert_eq!(nav.entries().len(), 2); // Still 2 entries (initial + replaced)
    }

    #[test]
    fn navigation_traverse_to() {
        let mut nav = Navigation::new();
        nav.navigate("https://a.com");
        let key = nav.current_entry().unwrap().key.clone();
        nav.navigate("https://b.com");
        nav.navigate("https://c.com");

        nav.traverse_to(&key);
        assert_eq!(nav.current_entry().unwrap().url, "https://a.com");
    }

    #[test]
    fn navigation_prunes_forward_on_push() {
        let mut nav = Navigation::new();
        nav.navigate("https://a.com");
        nav.navigate("https://b.com");
        nav.back(); // at a.com
        nav.navigate("https://c.com"); // prunes b.com
        assert_eq!(nav.entries().len(), 3); // blank, a, c
        assert!(!nav.can_go_forward);
    }

    #[test]
    fn navigation_state() {
        let mut nav = Navigation::new();
        nav.navigate("https://example.com");
        nav.update_current_entry_state(r#"{"scroll":100}"#);
        let state = nav.current_entry().unwrap().get_state().unwrap();
        assert!(state.contains("scroll"));
    }

    #[test]
    fn navigation_reload() {
        let mut nav = Navigation::new();
        nav.navigate("https://example.com");
        let event = nav.reload();
        assert_eq!(event.navigation_type, NavigationType::Reload);
        assert_eq!(event.destination_url, "https://example.com");
    }

    #[test]
    fn navigate_event_intercept() {
        let mut event = NavigateEvent::new(NavigationType::Push, "https://example.com");
        assert!(!event.intercepted);
        event.intercept();
        assert!(event.intercepted);
    }

    #[test]
    fn navigate_event_cancel() {
        let mut event = NavigateEvent::new(NavigationType::Push, "https://example.com");
        event.prevent_default();
        assert!(event.cancelled);
    }

    #[test]
    fn navigation_type_as_str() {
        assert_eq!(NavigationType::Push.as_str(), "push");
        assert_eq!(NavigationType::Replace.as_str(), "replace");
        assert_eq!(NavigationType::Reload.as_str(), "reload");
        assert_eq!(NavigationType::Traverse.as_str(), "traverse");
    }

    #[test]
    fn transition_lifecycle() {
        let mut t = NavigationTransition::new(NavigationType::Push, "https://a.com");
        assert!(!t.finished);
        t.finish();
        assert!(t.finished);
    }
}
