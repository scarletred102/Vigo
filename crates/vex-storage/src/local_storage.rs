// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Per-origin `localStorage` backed by SQLite.
//!
//! Each origin gets a key-value namespace with a 5 MB quota (measured by
//! the sum of UTF-8 key+value byte lengths).

use rusqlite::{params, Connection};

use crate::error::{StorageError, StorageResult};

/// 5 MB quota per origin (in bytes).
const QUOTA_BYTES: usize = 5 * 1024 * 1024;

const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS local_storage (
    origin TEXT NOT NULL,
    key    TEXT NOT NULL,
    value  TEXT NOT NULL,
    PRIMARY KEY (origin, key)
)";

/// SQLite-backed `localStorage`.
pub struct LocalStorage {
    conn: Connection,
}

impl LocalStorage {
    /// Open (or create) a localStorage database at the given path.
    pub fn open(path: &str) -> StorageResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(Self { conn })
    }

    /// Get a value by origin and key.
    pub fn get_item(&self, origin: &str, key: &str) -> StorageResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM local_storage WHERE origin = ?1 AND key = ?2")?;
        let mut rows = stmt.query_map(params![origin, key], |row| row.get(0))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Set a key-value pair for an origin, enforcing the 5 MB quota.
    pub fn set_item(&self, origin: &str, key: &str, value: &str) -> StorageResult<()> {
        // Calculate the size this entry would occupy.
        let entry_size = key.len() + value.len();

        // Get current usage excluding the key being set (it will be replaced).
        let current_usage = self.usage_excluding(origin, key)?;

        if current_usage + entry_size > QUOTA_BYTES {
            return Err(StorageError::QuotaExceeded {
                origin: origin.into(),
                detail: format!(
                    "would use {} bytes (limit {})",
                    current_usage + entry_size,
                    QUOTA_BYTES,
                ),
            });
        }

        self.conn.execute(
            "INSERT OR REPLACE INTO local_storage (origin, key, value) VALUES (?1, ?2, ?3)",
            params![origin, key, value],
        )?;
        Ok(())
    }

    /// Remove a key for an origin.
    pub fn remove_item(&self, origin: &str, key: &str) -> StorageResult<()> {
        self.conn.execute(
            "DELETE FROM local_storage WHERE origin = ?1 AND key = ?2",
            params![origin, key],
        )?;
        Ok(())
    }

    /// Remove all keys for an origin.
    pub fn clear(&self, origin: &str) -> StorageResult<()> {
        self.conn.execute(
            "DELETE FROM local_storage WHERE origin = ?1",
            params![origin],
        )?;
        Ok(())
    }

    /// Return the number of keys stored for an origin.
    pub fn length(&self, origin: &str) -> StorageResult<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM local_storage WHERE origin = ?1",
            params![origin],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// Return total byte usage for an origin.
    pub fn usage(&self, origin: &str) -> StorageResult<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) \
             FROM local_storage WHERE origin = ?1",
            params![origin],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// Usage for an origin, excluding a specific key (for quota check on update).
    fn usage_excluding(&self, origin: &str, exclude_key: &str) -> StorageResult<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) \
             FROM local_storage WHERE origin = ?1 AND key != ?2",
            params![origin, exclude_key],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";

    #[test]
    fn get_missing_key_returns_none() {
        let ls = LocalStorage::open_in_memory().unwrap();
        assert_eq!(ls.get_item(ORIGIN, "missing").unwrap(), None);
    }

    #[test]
    fn set_and_get_item() {
        let ls = LocalStorage::open_in_memory().unwrap();
        ls.set_item(ORIGIN, "theme", "dark").unwrap();
        assert_eq!(
            ls.get_item(ORIGIN, "theme").unwrap(),
            Some("dark".to_owned())
        );
    }

    #[test]
    fn overwrite_existing_key() {
        let ls = LocalStorage::open_in_memory().unwrap();
        ls.set_item(ORIGIN, "k", "v1").unwrap();
        ls.set_item(ORIGIN, "k", "v2").unwrap();
        assert_eq!(ls.get_item(ORIGIN, "k").unwrap(), Some("v2".to_owned()));
        assert_eq!(ls.length(ORIGIN).unwrap(), 1);
    }

    #[test]
    fn remove_item() {
        let ls = LocalStorage::open_in_memory().unwrap();
        ls.set_item(ORIGIN, "k", "v").unwrap();
        ls.remove_item(ORIGIN, "k").unwrap();
        assert_eq!(ls.get_item(ORIGIN, "k").unwrap(), None);
        assert_eq!(ls.length(ORIGIN).unwrap(), 0);
    }

    #[test]
    fn clear_origin() {
        let ls = LocalStorage::open_in_memory().unwrap();
        ls.set_item(ORIGIN, "a", "1").unwrap();
        ls.set_item(ORIGIN, "b", "2").unwrap();

        let other = "https://other.com";
        ls.set_item(other, "c", "3").unwrap();

        ls.clear(ORIGIN).unwrap();
        assert_eq!(ls.length(ORIGIN).unwrap(), 0);
        // Other origin unaffected.
        assert_eq!(ls.length(other).unwrap(), 1);
    }

    #[test]
    fn quota_enforcement() {
        let ls = LocalStorage::open_in_memory().unwrap();
        // Create a value that's ~5 MB.
        let big = "x".repeat(QUOTA_BYTES);
        let err = ls.set_item(ORIGIN, "big", &big).unwrap_err();
        assert!(matches!(err, StorageError::QuotaExceeded { .. }));
    }

    #[test]
    fn separate_origins_isolated() {
        let ls = LocalStorage::open_in_memory().unwrap();
        ls.set_item("https://a.com", "key", "a").unwrap();
        ls.set_item("https://b.com", "key", "b").unwrap();
        assert_eq!(
            ls.get_item("https://a.com", "key").unwrap(),
            Some("a".into())
        );
        assert_eq!(
            ls.get_item("https://b.com", "key").unwrap(),
            Some("b".into())
        );
    }
}
