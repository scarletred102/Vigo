// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Simplified IndexedDB-style key-value object store backed by SQLite.
//!
//! This is *not* a full IndexedDB implementation — it provides the minimal
//! surface needed to pass basic Web Platform Tests and support simple JS
//! usage: open a database, create/delete object stores, and CRUD by key.
//!
//! Each origin gets its own SQLite file in practice (caller manages paths).

use rusqlite::{params, Connection};
use serde_json::Value as JsonValue;

use crate::error::{StorageError, StorageResult};

/// An IndexedDB database with one or more object stores.
pub struct IdbDatabase {
    conn: Connection,
    name: String,
    version: u32,
}

impl IdbDatabase {
    /// Open (or create) a database at the given SQLite path.
    pub fn open(path: &str, name: &str, version: u32) -> StorageResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS idb_meta (
                 name    TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             )",
        )?;

        // Upsert version.
        conn.execute(
            "INSERT OR REPLACE INTO idb_meta (name, version) VALUES (?1, ?2)",
            params![name, version],
        )?;

        Ok(Self {
            conn,
            name: name.to_owned(),
            version,
        })
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory(name: &str, version: u32) -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS idb_meta (
                 name    TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             )",
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO idb_meta (name, version) VALUES (?1, ?2)",
            params![name, version],
        )?;
        Ok(Self {
            conn,
            name: name.to_owned(),
            version,
        })
    }

    /// Get the database name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the database version.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Create an object store (SQL table) if it doesn't exist.
    pub fn create_object_store(&self, store_name: &str) -> StorageResult<()> {
        validate_name(store_name)?;
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS [{}] (\
                 key   TEXT PRIMARY KEY,\
                 value TEXT NOT NULL\
             )",
            store_name
        );
        self.conn.execute_batch(&sql)?;
        Ok(())
    }

    /// Delete an object store.
    pub fn delete_object_store(&self, store_name: &str) -> StorageResult<()> {
        validate_name(store_name)?;
        let sql = format!("DROP TABLE IF EXISTS [{}]", store_name);
        self.conn.execute_batch(&sql)?;
        Ok(())
    }

    /// Put a JSON value into an object store (insert or update).
    pub fn put(&self, store: &str, key: &str, value: &JsonValue) -> StorageResult<()> {
        validate_name(store)?;
        let json = value.to_string();
        let sql = format!(
            "INSERT OR REPLACE INTO [{}] (key, value) VALUES (?1, ?2)",
            store
        );
        self.conn.execute(&sql, params![key, json])?;
        Ok(())
    }

    /// Get a JSON value by key. Returns `None` if not found.
    pub fn get(&self, store: &str, key: &str) -> StorageResult<Option<JsonValue>> {
        validate_name(store)?;
        let sql = format!("SELECT value FROM [{}] WHERE key = ?1", store);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![key], |row| {
            let s: String = row.get(0)?;
            Ok(s)
        })?;
        match rows.next() {
            Some(row) => {
                let s = row?;
                let v: JsonValue = serde_json::from_str(&s).map_err(|e| {
                    StorageError::InvalidArgument(format!("bad JSON in store: {e}"))
                })?;
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    /// Delete a record by key.
    pub fn delete(&self, store: &str, key: &str) -> StorageResult<()> {
        validate_name(store)?;
        let sql = format!("DELETE FROM [{}] WHERE key = ?1", store);
        self.conn.execute(&sql, params![key])?;
        Ok(())
    }

    /// Return all keys in an object store.
    pub fn get_all_keys(&self, store: &str) -> StorageResult<Vec<String>> {
        validate_name(store)?;
        let sql = format!("SELECT key FROM [{}] ORDER BY key", store);
        let mut stmt = self.conn.prepare(&sql)?;
        let keys: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    /// Count records in an object store.
    pub fn count(&self, store: &str) -> StorageResult<usize> {
        validate_name(store)?;
        let sql = format!("SELECT COUNT(*) FROM [{}]", store);
        let n: i64 = self.conn.query_row(&sql, [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// Clear all records in an object store (but keep the table).
    pub fn clear(&self, store: &str) -> StorageResult<()> {
        validate_name(store)?;
        let sql = format!("DELETE FROM [{}]", store);
        self.conn.execute_batch(&sql)?;
        Ok(())
    }
}

/// Reject names that could cause SQL injection.
fn validate_name(name: &str) -> StorageResult<()> {
    if name.is_empty()
        || name.contains(']')
        || name.contains(';')
        || name.contains('\'')
        || name.contains('"')
        || name.contains('\0')
        || name.starts_with("idb_")
    {
        return Err(StorageError::InvalidArgument(format!(
            "invalid store name: {name:?}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn db() -> IdbDatabase {
        let db = IdbDatabase::open_in_memory("testdb", 1).unwrap();
        db.create_object_store("items").unwrap();
        db
    }

    #[test]
    fn open_and_version() {
        let db = IdbDatabase::open_in_memory("mydb", 3).unwrap();
        assert_eq!(db.name(), "mydb");
        assert_eq!(db.version(), 3);
    }

    #[test]
    fn put_and_get() {
        let db = db();
        db.put("items", "k1", &json!({"hello": "world"})).unwrap();
        let val = db.get("items", "k1").unwrap().unwrap();
        assert_eq!(val["hello"], "world");
    }

    #[test]
    fn get_missing_returns_none() {
        let db = db();
        assert!(db.get("items", "nope").unwrap().is_none());
    }

    #[test]
    fn delete_record() {
        let db = db();
        db.put("items", "k1", &json!(1)).unwrap();
        db.delete("items", "k1").unwrap();
        assert!(db.get("items", "k1").unwrap().is_none());
    }

    #[test]
    fn get_all_keys() {
        let db = db();
        db.put("items", "b", &json!(2)).unwrap();
        db.put("items", "a", &json!(1)).unwrap();
        let keys = db.get_all_keys("items").unwrap();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn count_and_clear() {
        let db = db();
        db.put("items", "a", &json!(1)).unwrap();
        db.put("items", "b", &json!(2)).unwrap();
        assert_eq!(db.count("items").unwrap(), 2);
        db.clear("items").unwrap();
        assert_eq!(db.count("items").unwrap(), 0);
    }

    #[test]
    fn invalid_store_name_rejected() {
        let db = IdbDatabase::open_in_memory("testdb", 1).unwrap();
        assert!(db.create_object_store("bad;name").is_err());
        assert!(db.create_object_store("idb_meta").is_err());
        assert!(db.create_object_store("").is_err());
    }
}
