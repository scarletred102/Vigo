// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Persistent cookie storage backed by SQLite.
//!
//! Stores cookies in a `cookies` table with columns for domain, path, name,
//! value, flags, and expiry. Integrates with [`vex_net::CookieJar`] by
//! providing load/save round-trips.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::error::StorageResult;

// ── Schema ─────────────────────────────────────────────────────────────

const CREATE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS cookies (
    domain    TEXT NOT NULL,
    path      TEXT NOT NULL DEFAULT '/',
    name      TEXT NOT NULL,
    value     TEXT NOT NULL,
    secure    INTEGER NOT NULL DEFAULT 0,
    http_only INTEGER NOT NULL DEFAULT 0,
    same_site TEXT NOT NULL DEFAULT 'Lax',
    expires   INTEGER,
    PRIMARY KEY (domain, path, name)
)";

// ── Cookie struct ──────────────────────────────────────────────────────

/// A persistent cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentCookie {
    pub domain: String,
    pub path: String,
    pub name: String,
    pub value: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    /// Expiry as seconds since UNIX epoch. `None` = session cookie (not persisted on reload).
    pub expires: Option<i64>,
}

// ── Cookie store ───────────────────────────────────────────────────────

/// SQLite-backed cookie store.
pub struct CookieStore {
    conn: Connection,
}

impl CookieStore {
    /// Open (or create) a cookie database at the given path.
    pub fn open(path: &str) -> StorageResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(Self { conn })
    }

    /// Open an in-memory cookie database (useful for tests).
    pub fn open_in_memory() -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(Self { conn })
    }

    /// Save (upsert) a cookie.
    pub fn save(&self, cookie: &PersistentCookie) -> StorageResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO cookies \
             (domain, path, name, value, secure, http_only, same_site, expires) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                cookie.domain,
                cookie.path,
                cookie.name,
                cookie.value,
                cookie.secure as i32,
                cookie.http_only as i32,
                cookie.same_site,
                cookie.expires,
            ],
        )?;
        Ok(())
    }

    /// Load all non-expired cookies matching a domain and path.
    pub fn load(&self, domain: &str, path: &str) -> StorageResult<Vec<PersistentCookie>> {
        let now = system_time_to_epoch(SystemTime::now());
        let mut stmt = self.conn.prepare(
            "SELECT domain, path, name, value, secure, http_only, same_site, expires \
             FROM cookies \
             WHERE domain = ?1 AND ?2 LIKE (path || '%') \
               AND (expires IS NULL OR expires > ?3)",
        )?;

        let rows = stmt.query_map(params![domain, path, now], |row| {
            Ok(PersistentCookie {
                domain: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                value: row.get(3)?,
                secure: row.get::<_, i32>(4)? != 0,
                http_only: row.get::<_, i32>(5)? != 0,
                same_site: row.get(6)?,
                expires: row.get(7)?,
            })
        })?;

        let mut cookies = Vec::new();
        for row in rows {
            cookies.push(row?);
        }
        Ok(cookies)
    }

    /// Delete all expired cookies.
    pub fn delete_expired(&self) -> StorageResult<usize> {
        let now = system_time_to_epoch(SystemTime::now());
        let count = self.conn.execute(
            "DELETE FROM cookies WHERE expires IS NOT NULL AND expires <= ?1",
            params![now],
        )?;
        Ok(count)
    }

    /// Delete all cookies.
    pub fn clear_all(&self) -> StorageResult<()> {
        self.conn.execute("DELETE FROM cookies", [])?;
        Ok(())
    }

    /// Count total stored cookies.
    pub fn count(&self) -> StorageResult<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM cookies", [], |r| r.get(0))?;
        Ok(n as usize)
    }
}

fn system_time_to_epoch(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cookie(name: &str, domain: &str) -> PersistentCookie {
        PersistentCookie {
            domain: domain.into(),
            path: "/".into(),
            name: name.into(),
            value: "val".into(),
            secure: false,
            http_only: false,
            same_site: "Lax".into(),
            expires: None,
        }
    }

    #[test]
    fn save_and_load_cookie() {
        let store = CookieStore::open_in_memory().unwrap();
        let c = sample_cookie("sid", "example.com");
        store.save(&c).unwrap();

        let loaded = store.load("example.com", "/").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "sid");
        assert_eq!(loaded[0].value, "val");
    }

    #[test]
    fn upsert_overwrites() {
        let store = CookieStore::open_in_memory().unwrap();
        let mut c = sample_cookie("sid", "example.com");
        store.save(&c).unwrap();

        c.value = "new_val".into();
        store.save(&c).unwrap();

        let loaded = store.load("example.com", "/").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].value, "new_val");
    }

    #[test]
    fn expired_cookies_excluded() {
        let store = CookieStore::open_in_memory().unwrap();
        let mut c = sample_cookie("old", "example.com");
        c.expires = Some(1); // epoch + 1 second → way in the past
        store.save(&c).unwrap();

        let loaded = store.load("example.com", "/").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_expired() {
        let store = CookieStore::open_in_memory().unwrap();
        let mut c1 = sample_cookie("old", "example.com");
        c1.expires = Some(1);
        store.save(&c1).unwrap();

        let c2 = sample_cookie("fresh", "example.com");
        store.save(&c2).unwrap();

        let deleted = store.delete_expired().unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn clear_all() {
        let store = CookieStore::open_in_memory().unwrap();
        store.save(&sample_cookie("a", "a.com")).unwrap();
        store.save(&sample_cookie("b", "b.com")).unwrap();
        assert_eq!(store.count().unwrap(), 2);

        store.clear_all().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }
}
