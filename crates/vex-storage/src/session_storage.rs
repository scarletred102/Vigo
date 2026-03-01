// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Per-origin, per-tab `sessionStorage`.
//!
//! Unlike [`LocalStorage`](super::LocalStorage), session storage is
//! purely in-memory and scoped to a (origin, tab) pair. Data is lost
//! when the tab closes.

use std::collections::HashMap;

use crate::error::{StorageError, StorageResult};

/// 5 MB quota per (origin, tab) pair.
const QUOTA_BYTES: usize = 5 * 1024 * 1024;

/// Key for isolating storage: (origin, tab_id).
type SessionKey = (String, u64);

/// In-memory `sessionStorage`.
///
/// Each (origin, tab) pair gets an independent key-value map.
pub struct SessionStorage {
    stores: HashMap<SessionKey, HashMap<String, String>>,
}

impl Default for SessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStorage {
    /// Create a new empty session storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    /// Get a value.
    #[must_use]
    pub fn get_item(&self, origin: &str, tab_id: u64, key: &str) -> Option<&str> {
        self.stores
            .get(&(origin.to_owned(), tab_id))
            .and_then(|m| m.get(key).map(String::as_str))
    }

    /// Set a key-value pair, enforcing the 5 MB quota.
    pub fn set_item(
        &mut self,
        origin: &str,
        tab_id: u64,
        key: &str,
        value: &str,
    ) -> StorageResult<()> {
        let sk = (origin.to_owned(), tab_id);
        let map = self.stores.entry(sk.clone()).or_default();

        // Current usage excluding the key being set.
        let usage: usize = map
            .iter()
            .filter(|(k, _)| k.as_str() != key)
            .map(|(k, v)| k.len() + v.len())
            .sum();

        let new_entry = key.len() + value.len();
        if usage + new_entry > QUOTA_BYTES {
            return Err(StorageError::QuotaExceeded {
                origin: origin.into(),
                detail: format!(
                    "would use {} bytes (limit {})",
                    usage + new_entry,
                    QUOTA_BYTES,
                ),
            });
        }

        map.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    /// Remove a key.
    pub fn remove_item(&mut self, origin: &str, tab_id: u64, key: &str) {
        if let Some(map) = self.stores.get_mut(&(origin.to_owned(), tab_id)) {
            map.remove(key);
        }
    }

    /// Remove all keys for an (origin, tab) pair.
    pub fn clear(&mut self, origin: &str, tab_id: u64) {
        self.stores.remove(&(origin.to_owned(), tab_id));
    }

    /// Number of keys for an (origin, tab) pair.
    #[must_use]
    pub fn length(&self, origin: &str, tab_id: u64) -> usize {
        self.stores
            .get(&(origin.to_owned(), tab_id))
            .map_or(0, HashMap::len)
    }

    /// Drop all data for a specific tab (all origins). Call on tab close.
    pub fn drop_tab(&mut self, tab_id: u64) {
        self.stores.retain(|(_, tid), _| *tid != tab_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";
    const TAB: u64 = 1;

    #[test]
    fn get_missing_returns_none() {
        let ss = SessionStorage::new();
        assert_eq!(ss.get_item(ORIGIN, TAB, "x"), None);
    }

    #[test]
    fn set_and_get() {
        let mut ss = SessionStorage::new();
        ss.set_item(ORIGIN, TAB, "k", "v").unwrap();
        assert_eq!(ss.get_item(ORIGIN, TAB, "k"), Some("v"));
    }

    #[test]
    fn tabs_are_isolated() {
        let mut ss = SessionStorage::new();
        ss.set_item(ORIGIN, 1, "k", "a").unwrap();
        ss.set_item(ORIGIN, 2, "k", "b").unwrap();
        assert_eq!(ss.get_item(ORIGIN, 1, "k"), Some("a"));
        assert_eq!(ss.get_item(ORIGIN, 2, "k"), Some("b"));
    }

    #[test]
    fn clear_only_affects_target() {
        let mut ss = SessionStorage::new();
        ss.set_item(ORIGIN, 1, "k", "v").unwrap();
        ss.set_item("https://other.com", 1, "k", "v").unwrap();

        ss.clear(ORIGIN, 1);
        assert_eq!(ss.length(ORIGIN, 1), 0);
        assert_eq!(ss.length("https://other.com", 1), 1);
    }

    #[test]
    fn drop_tab_removes_all_origins() {
        let mut ss = SessionStorage::new();
        ss.set_item("https://a.com", 1, "k", "v").unwrap();
        ss.set_item("https://b.com", 1, "k", "v").unwrap();
        ss.set_item("https://a.com", 2, "k", "v").unwrap();

        ss.drop_tab(1);
        assert_eq!(ss.length("https://a.com", 1), 0);
        assert_eq!(ss.length("https://b.com", 1), 0);
        // Tab 2 unaffected.
        assert_eq!(ss.length("https://a.com", 2), 1);
    }

    #[test]
    fn quota_enforcement() {
        let mut ss = SessionStorage::new();
        let big = "x".repeat(QUOTA_BYTES);
        let err = ss.set_item(ORIGIN, TAB, "big", &big).unwrap_err();
        assert!(matches!(err, StorageError::QuotaExceeded { .. }));
    }
}
