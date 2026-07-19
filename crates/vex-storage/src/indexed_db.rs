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

    /// List all object store names in this database.
    pub fn object_store_names(&self) -> StorageResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' \
             AND name NOT LIKE 'idb_%' AND name != 'sqlite_sequence' \
             ORDER BY name",
        )?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// Create an index on an object store.
    ///
    /// The index is a SQL index on a JSON-extracted field stored in a
    /// companion column. For simplicity, we create a separate index table
    /// mapping the indexed field value to the primary key.
    pub fn create_index(
        &self,
        store: &str,
        index_name: &str,
        key_path: &str,
        unique: bool,
    ) -> StorageResult<()> {
        validate_name(store)?;
        validate_name(index_name)?;

        // Create index tracking table.
        let table = format!("idb_idx_{store}_{index_name}");
        let unique_kw = if unique { "UNIQUE" } else { "" };
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS [{table}] (\
                 idx_key TEXT NOT NULL,\
                 pk      TEXT NOT NULL {unique_kw},\
                 PRIMARY KEY (idx_key, pk)\
             )"
        );
        self.conn.execute_batch(&sql)?;

        // Populate index from existing data.
        let select_sql = format!("SELECT key, value FROM [{store}]");
        let mut stmt = self.conn.prepare(&select_sql)?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let insert_sql = format!("INSERT OR REPLACE INTO [{table}] (idx_key, pk) VALUES (?1, ?2)");
        for (pk, json_str) in &rows {
            if let Ok(val) = serde_json::from_str::<JsonValue>(json_str) {
                if let Some(idx_val) = extract_key_path(&val, key_path) {
                    self.conn.execute(&insert_sql, params![idx_val, pk])?;
                }
            }
        }

        Ok(())
    }

    /// Get records by index value.
    pub fn get_by_index(
        &self,
        store: &str,
        index_name: &str,
        value: &str,
    ) -> StorageResult<Vec<(String, JsonValue)>> {
        validate_name(store)?;
        validate_name(index_name)?;

        let table = format!("idb_idx_{store}_{index_name}");
        let sql = format!(
            "SELECT s.key, s.value FROM [{table}] i \
             JOIN [{store}] s ON i.pk = s.key \
             WHERE i.idx_key = ?1 ORDER BY s.key"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let results: Vec<(String, JsonValue)> = stmt
            .query_map(params![value], |row| {
                let key: String = row.get(0)?;
                let val_str: String = row.get(1)?;
                Ok((key, val_str))
            })?
            .filter_map(|r| {
                r.ok()
                    .and_then(|(k, v)| serde_json::from_str(&v).ok().map(|parsed| (k, parsed)))
            })
            .collect();
        Ok(results)
    }

    /// Get all records in an object store, optionally within a key range.
    pub fn get_all(
        &self,
        store: &str,
        range: Option<&KeyRange>,
    ) -> StorageResult<Vec<(String, JsonValue)>> {
        validate_name(store)?;
        let (where_clause, bind_params) = range.map(|r| r.to_sql_clause("key")).unwrap_or_default();

        let sql = if where_clause.is_empty() {
            format!("SELECT key, value FROM [{store}] ORDER BY key")
        } else {
            format!("SELECT key, value FROM [{store}] WHERE {where_clause} ORDER BY key")
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = bind_params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let results: Vec<(String, JsonValue)> = stmt
            .query_map(refs.as_slice(), |row| {
                let key: String = row.get(0)?;
                let val_str: String = row.get(1)?;
                Ok((key, val_str))
            })?
            .filter_map(|r| {
                r.ok()
                    .and_then(|(k, v)| serde_json::from_str(&v).ok().map(|parsed| (k, parsed)))
            })
            .collect();
        Ok(results)
    }

    /// Begin a transaction that groups multiple operations atomically.
    pub fn transaction(&mut self, mode: TransactionMode) -> StorageResult<IdbTransaction<'_>> {
        let sql = match mode {
            TransactionMode::ReadOnly => "BEGIN DEFERRED",
            TransactionMode::ReadWrite => "BEGIN IMMEDIATE",
        };
        self.conn.execute_batch(sql)?;
        Ok(IdbTransaction {
            conn: &self.conn,
            committed: false,
            mode,
        })
    }
}

/// A key range for querying subsets of records.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyRange {
    /// key == value
    Only(String),
    /// lower <= key <= upper (with optional exclusion of bounds)
    Bound {
        lower: Option<String>,
        upper: Option<String>,
        lower_open: bool,
        upper_open: bool,
    },
}

impl KeyRange {
    /// `IDBKeyRange.only(value)`.
    pub fn only(value: impl Into<String>) -> Self {
        Self::Only(value.into())
    }

    /// `IDBKeyRange.lowerBound(lower, open)`.
    pub fn lower_bound(lower: impl Into<String>, open: bool) -> Self {
        Self::Bound {
            lower: Some(lower.into()),
            upper: None,
            lower_open: open,
            upper_open: false,
        }
    }

    /// `IDBKeyRange.upperBound(upper, open)`.
    pub fn upper_bound(upper: impl Into<String>, open: bool) -> Self {
        Self::Bound {
            lower: None,
            upper: Some(upper.into()),
            lower_open: false,
            upper_open: open,
        }
    }

    /// `IDBKeyRange.bound(lower, upper, lower_open, upper_open)`.
    pub fn bound(
        lower: impl Into<String>,
        upper: impl Into<String>,
        lower_open: bool,
        upper_open: bool,
    ) -> Self {
        Self::Bound {
            lower: Some(lower.into()),
            upper: Some(upper.into()),
            lower_open,
            upper_open,
        }
    }

    /// Check if a key falls within this range.
    pub fn includes(&self, key: &str) -> bool {
        match self {
            Self::Only(v) => key == v,
            Self::Bound {
                lower,
                upper,
                lower_open,
                upper_open,
            } => {
                if let Some(lo) = lower {
                    if *lower_open && key <= lo.as_str() {
                        return false;
                    }
                    if !*lower_open && key < lo.as_str() {
                        return false;
                    }
                }
                if let Some(hi) = upper {
                    if *upper_open && key >= hi.as_str() {
                        return false;
                    }
                    if !*upper_open && key > hi.as_str() {
                        return false;
                    }
                }
                true
            }
        }
    }

    /// Convert to SQL WHERE clause fragment and bind parameters.
    fn to_sql_clause(&self, col: &str) -> (String, Vec<String>) {
        match self {
            Self::Only(v) => (format!("{col} = ?1"), vec![v.clone()]),
            Self::Bound {
                lower,
                upper,
                lower_open,
                upper_open,
            } => {
                let mut parts = Vec::new();
                let mut params = Vec::new();
                let mut idx = 1;
                if let Some(lo) = lower {
                    let op = if *lower_open { ">" } else { ">=" };
                    parts.push(format!("{col} {op} ?{idx}"));
                    params.push(lo.clone());
                    idx += 1;
                }
                if let Some(hi) = upper {
                    let op = if *upper_open { "<" } else { "<=" };
                    parts.push(format!("{col} {op} ?{idx}"));
                    params.push(hi.clone());
                }
                (parts.join(" AND "), params)
            }
        }
    }
}

/// Transaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    ReadOnly,
    ReadWrite,
}

/// An IndexedDB transaction — groups operations into an atomic unit.
///
/// Must be explicitly committed; dropping without commit rolls back.
pub struct IdbTransaction<'a> {
    conn: &'a Connection,
    committed: bool,
    mode: TransactionMode,
}

impl<'a> IdbTransaction<'a> {
    /// Transaction mode.
    pub fn mode(&self) -> TransactionMode {
        self.mode
    }

    /// Put a value (insert or update).
    pub fn put(&self, store: &str, key: &str, value: &JsonValue) -> StorageResult<()> {
        if self.mode == TransactionMode::ReadOnly {
            return Err(StorageError::InvalidArgument(
                "cannot write in readonly transaction".to_string(),
            ));
        }
        validate_name(store)?;
        let json = value.to_string();
        let sql = format!(
            "INSERT OR REPLACE INTO [{}] (key, value) VALUES (?1, ?2)",
            store
        );
        self.conn.execute(&sql, params![key, json])?;
        Ok(())
    }

    /// Get a value by key.
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
        if self.mode == TransactionMode::ReadOnly {
            return Err(StorageError::InvalidArgument(
                "cannot write in readonly transaction".to_string(),
            ));
        }
        validate_name(store)?;
        let sql = format!("DELETE FROM [{}] WHERE key = ?1", store);
        self.conn.execute(&sql, params![key])?;
        Ok(())
    }

    /// Open a cursor over all records in a store, optionally filtered by key range.
    pub fn open_cursor(
        &self,
        store: &str,
        range: Option<&KeyRange>,
        direction: CursorDirection,
    ) -> StorageResult<IdbCursor> {
        validate_name(store)?;
        let (where_clause, bind_params) = range.map(|r| r.to_sql_clause("key")).unwrap_or_default();

        let order = match direction {
            CursorDirection::Next | CursorDirection::NextUnique => "ASC",
            CursorDirection::Prev | CursorDirection::PrevUnique => "DESC",
        };

        let sql = if where_clause.is_empty() {
            format!("SELECT key, value FROM [{store}] ORDER BY key {order}")
        } else {
            format!("SELECT key, value FROM [{store}] WHERE {where_clause} ORDER BY key {order}")
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = bind_params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows: Vec<(String, JsonValue)> = stmt
            .query_map(refs.as_slice(), |row| {
                let key: String = row.get(0)?;
                let val_str: String = row.get(1)?;
                Ok((key, val_str))
            })?
            .filter_map(|r| {
                r.ok()
                    .and_then(|(k, v)| serde_json::from_str(&v).ok().map(|parsed| (k, parsed)))
            })
            .collect();

        Ok(IdbCursor {
            records: rows,
            position: 0,
            direction,
        })
    }

    /// Commit the transaction.
    pub fn commit(mut self) -> StorageResult<()> {
        self.conn.execute_batch("COMMIT")?;
        self.committed = true;
        Ok(())
    }

    /// Abort (rollback) the transaction.
    pub fn abort(mut self) -> StorageResult<()> {
        self.conn.execute_batch("ROLLBACK")?;
        self.committed = true; // prevent double-rollback in Drop
        Ok(())
    }
}

impl<'a> Drop for IdbTransaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.conn.execute_batch("ROLLBACK");
        }
    }
}

/// Cursor direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    Next,
    NextUnique,
    Prev,
    PrevUnique,
}

/// A cursor over records in an object store.
///
/// Records are eagerly loaded (since we can't hold a SQLite statement
/// across the cursor's lifetime easily). For large stores a streaming
/// approach would be better, but this is fine for browser use.
pub struct IdbCursor {
    records: Vec<(String, JsonValue)>,
    position: usize,
    direction: CursorDirection,
}

impl IdbCursor {
    /// Current key, or `None` if the cursor is exhausted.
    pub fn key(&self) -> Option<&str> {
        self.records.get(self.position).map(|(k, _)| k.as_str())
    }

    /// Current value, or `None` if the cursor is exhausted.
    pub fn value(&self) -> Option<&JsonValue> {
        self.records.get(self.position).map(|(_, v)| v)
    }

    /// Current (key, value) pair.
    pub fn current(&self) -> Option<(&str, &JsonValue)> {
        self.records
            .get(self.position)
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Advance the cursor by one position.
    pub fn advance(&mut self) -> bool {
        if self.position < self.records.len() {
            self.position += 1;
        }
        self.position < self.records.len()
    }

    /// Continue to the next record (alias for advance).
    pub fn continue_cursor(&mut self) -> bool {
        self.advance()
    }

    /// Continue to the record at or past the given key.
    pub fn continue_primary_key(&mut self, target: &str) -> bool {
        while self.position < self.records.len() {
            let key = &self.records[self.position].0;
            let found = match self.direction {
                CursorDirection::Next | CursorDirection::NextUnique => key.as_str() >= target,
                CursorDirection::Prev | CursorDirection::PrevUnique => key.as_str() <= target,
            };
            if found {
                return true;
            }
            self.position += 1;
        }
        false
    }

    /// Direction of this cursor.
    pub fn direction(&self) -> CursorDirection {
        self.direction
    }

    /// How many records the cursor covers in total.
    pub fn total_count(&self) -> usize {
        self.records.len()
    }
}

/// Extract a value from a JSON object by a simple dot-separated key path.
fn extract_key_path(val: &JsonValue, path: &str) -> Option<String> {
    let mut current = val;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    match current {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        _ => Some(current.to_string()),
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

    #[test]
    fn object_store_names() {
        let db = db();
        db.create_object_store("users").unwrap();
        let names = db.object_store_names().unwrap();
        assert!(names.contains(&"items".to_string()));
        assert!(names.contains(&"users".to_string()));
    }

    #[test]
    fn transaction_commit() {
        let mut db = IdbDatabase::open_in_memory("txdb", 1).unwrap();
        db.create_object_store("data").unwrap();

        let tx = db.transaction(TransactionMode::ReadWrite).unwrap();
        tx.put("data", "a", &json!(1)).unwrap();
        tx.put("data", "b", &json!(2)).unwrap();
        tx.commit().unwrap();

        // Data persists after commit.
        assert_eq!(db.get("data", "a").unwrap(), Some(json!(1)));
        assert_eq!(db.get("data", "b").unwrap(), Some(json!(2)));
    }

    #[test]
    fn transaction_abort_rolls_back() {
        let mut db = IdbDatabase::open_in_memory("txdb", 1).unwrap();
        db.create_object_store("data").unwrap();
        db.put("data", "existing", &json!("keep")).unwrap();

        let tx = db.transaction(TransactionMode::ReadWrite).unwrap();
        tx.put("data", "new_key", &json!("discard")).unwrap();
        tx.delete("data", "existing").unwrap();
        tx.abort().unwrap();

        // Abort should restore original state.
        assert_eq!(db.get("data", "existing").unwrap(), Some(json!("keep")));
        assert!(db.get("data", "new_key").unwrap().is_none());
    }

    #[test]
    fn transaction_drop_rolls_back() {
        let mut db = IdbDatabase::open_in_memory("txdb", 1).unwrap();
        db.create_object_store("data").unwrap();

        {
            let tx = db.transaction(TransactionMode::ReadWrite).unwrap();
            tx.put("data", "dropped", &json!(99)).unwrap();
            // tx dropped without commit
        }

        assert!(db.get("data", "dropped").unwrap().is_none());
    }

    #[test]
    fn transaction_readonly_rejects_writes() {
        let mut db = IdbDatabase::open_in_memory("txdb", 1).unwrap();
        db.create_object_store("data").unwrap();

        let tx = db.transaction(TransactionMode::ReadOnly).unwrap();
        assert!(tx.put("data", "x", &json!(1)).is_err());
        assert!(tx.delete("data", "x").is_err());
        // Read should work.
        assert!(tx.get("data", "x").unwrap().is_none());
    }

    #[test]
    fn cursor_forward_iteration() {
        let mut db = IdbDatabase::open_in_memory("curdb", 1).unwrap();
        db.create_object_store("items").unwrap();
        db.put("items", "c", &json!(3)).unwrap();
        db.put("items", "a", &json!(1)).unwrap();
        db.put("items", "b", &json!(2)).unwrap();

        let tx = db.transaction(TransactionMode::ReadOnly).unwrap();
        let mut cursor = tx
            .open_cursor("items", None, CursorDirection::Next)
            .unwrap();

        assert_eq!(cursor.key(), Some("a"));
        assert_eq!(cursor.value(), Some(&json!(1)));
        assert!(cursor.advance());
        assert_eq!(cursor.key(), Some("b"));
        assert!(cursor.advance());
        assert_eq!(cursor.key(), Some("c"));
        assert!(!cursor.advance());
        assert_eq!(cursor.key(), None);
    }

    #[test]
    fn cursor_reverse_iteration() {
        let mut db = IdbDatabase::open_in_memory("curdb", 1).unwrap();
        db.create_object_store("items").unwrap();
        db.put("items", "a", &json!(1)).unwrap();
        db.put("items", "b", &json!(2)).unwrap();
        db.put("items", "c", &json!(3)).unwrap();

        let tx = db.transaction(TransactionMode::ReadOnly).unwrap();
        let mut cursor = tx
            .open_cursor("items", None, CursorDirection::Prev)
            .unwrap();

        assert_eq!(cursor.key(), Some("c"));
        assert!(cursor.advance());
        assert_eq!(cursor.key(), Some("b"));
        assert!(cursor.advance());
        assert_eq!(cursor.key(), Some("a"));
        assert!(!cursor.advance());
    }

    #[test]
    fn cursor_with_key_range() {
        let mut db = IdbDatabase::open_in_memory("curdb", 1).unwrap();
        db.create_object_store("items").unwrap();
        for c in ['a', 'b', 'c', 'd', 'e'] {
            db.put("items", &c.to_string(), &json!(c.to_string()))
                .unwrap();
        }

        let range = KeyRange::bound("b", "d", false, false);
        let tx = db.transaction(TransactionMode::ReadOnly).unwrap();
        let cursor = tx
            .open_cursor("items", Some(&range), CursorDirection::Next)
            .unwrap();

        assert_eq!(cursor.total_count(), 3);
        assert_eq!(cursor.key(), Some("b"));
    }

    #[test]
    fn key_range_only() {
        let range = KeyRange::only("hello");
        assert!(range.includes("hello"));
        assert!(!range.includes("world"));
    }

    #[test]
    fn key_range_lower_bound() {
        let range = KeyRange::lower_bound("c", false);
        assert!(!range.includes("b"));
        assert!(range.includes("c"));
        assert!(range.includes("d"));

        let open = KeyRange::lower_bound("c", true);
        assert!(!open.includes("c"));
        assert!(open.includes("d"));
    }

    #[test]
    fn key_range_upper_bound() {
        let range = KeyRange::upper_bound("c", false);
        assert!(range.includes("a"));
        assert!(range.includes("c"));
        assert!(!range.includes("d"));

        let open = KeyRange::upper_bound("c", true);
        assert!(!open.includes("c"));
        assert!(open.includes("b"));
    }

    #[test]
    fn key_range_bound_open() {
        let range = KeyRange::bound("b", "d", true, true);
        assert!(!range.includes("b"));
        assert!(range.includes("c"));
        assert!(!range.includes("d"));
    }

    #[test]
    fn get_all_with_range() {
        let db = db();
        for c in ['a', 'b', 'c', 'd', 'e'] {
            db.put("items", &c.to_string(), &json!(c.to_string()))
                .unwrap();
        }

        let range = KeyRange::bound("b", "d", false, true);
        let results = db.get_all("items", Some(&range)).unwrap();
        let keys: Vec<&str> = results.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["b", "c"]);
    }

    #[test]
    fn get_all_without_range() {
        let db = db();
        db.put("items", "x", &json!(1)).unwrap();
        db.put("items", "y", &json!(2)).unwrap();
        let results = db.get_all("items", None).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn create_and_query_index() {
        let db = db();
        db.put("items", "1", &json!({"name": "Alice", "age": 30}))
            .unwrap();
        db.put("items", "2", &json!({"name": "Bob", "age": 25}))
            .unwrap();
        db.put("items", "3", &json!({"name": "Alice", "age": 35}))
            .unwrap();

        db.create_index("items", "by_name", "name", false).unwrap();
        let results = db.get_by_index("items", "by_name", "Alice").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "1");
        assert_eq!(results[1].0, "3");
    }

    #[test]
    fn cursor_continue_primary_key() {
        let mut db = IdbDatabase::open_in_memory("curdb", 1).unwrap();
        db.create_object_store("items").unwrap();
        for c in ['a', 'b', 'c', 'd', 'e'] {
            db.put("items", &c.to_string(), &json!(c.to_string()))
                .unwrap();
        }

        let tx = db.transaction(TransactionMode::ReadOnly).unwrap();
        let mut cursor = tx
            .open_cursor("items", None, CursorDirection::Next)
            .unwrap();

        assert_eq!(cursor.key(), Some("a"));
        assert!(cursor.continue_primary_key("c"));
        assert_eq!(cursor.key(), Some("c"));
    }

    #[test]
    fn extract_nested_key_path() {
        let val = json!({"user": {"name": "test"}});
        assert_eq!(
            extract_key_path(&val, "user.name"),
            Some("test".to_string())
        );
        assert_eq!(extract_key_path(&val, "missing"), None);
    }
}
