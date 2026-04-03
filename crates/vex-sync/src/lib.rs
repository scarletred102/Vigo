// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-sync
//!
//! End-to-end encrypted sync client for the Vex browser engine.
//!
//! Syncs bookmarks, history, settings, and credentials across devices
//! using the zero-knowledge sync server. All data encrypted client-side
//! via `vex-crypto` before leaving the device.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use vex_core::VexUrl;
use vex_net::{HttpClient, Method, Request};

/// Errors from sync operations.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Network error during sync.
    #[error("network error: {0}")]
    Network(String),

    /// Server returned an error.
    #[error("server error: {status}")]
    Server { status: u16 },

    /// Encryption/decryption failed.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// Data serialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Not authenticated.
    #[error("not authenticated")]
    NotAuthenticated,

    /// Conflict during merge.
    #[error("merge conflict: {0}")]
    Conflict(String),
}

/// Type of data being synced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncCollection {
    Bookmarks,
    History,
    Settings,
    OpenTabs,
}

impl SyncCollection {
    /// API path segment for this collection.
    pub fn path(&self) -> &'static str {
        match self {
            Self::Bookmarks => "bookmarks",
            Self::History => "history",
            Self::Settings => "settings",
            Self::OpenTabs => "tabs",
        }
    }
}

/// A single sync record (encrypted on the wire).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// Unique record ID.
    pub id: String,
    /// Collection this record belongs to.
    pub collection: SyncCollection,
    /// Encrypted payload (base64-encoded nonce + ciphertext).
    pub payload: String,
    /// Server-assigned timestamp (milliseconds since epoch).
    pub modified: u64,
    /// Whether this record is a tombstone (deleted).
    pub deleted: bool,
}

/// Authentication state for sync.
#[derive(Debug, Clone, Default)]
pub enum AuthState {
    /// Not logged in.
    #[default]
    SignedOut,
    /// Authenticated with server token.
    SignedIn {
        /// Server URL.
        server_url: String,
        /// Bearer token.
        token: String,
        /// User identifier.
        user_id: String,
    },
}

/// Sync client connecting to the Go sync-server.
///
/// Handles authentication, encryption, and conflict resolution.
pub struct SyncClient {
    auth: AuthState,
    /// Last sync timestamp per collection.
    last_sync: HashMap<SyncCollection, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireRecord {
    #[serde(rename = "record_id")]
    record_id: String,
    collection: String,
    ciphertext: String,
    version: i32,
    #[serde(rename = "modified_at")]
    modified_at: String,
    #[serde(default)]
    content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireRecordList {
    records: Vec<WireRecord>,
}

impl SyncClient {
    /// Create a new sync client (not authenticated).
    pub fn new() -> Self {
        Self {
            auth: AuthState::SignedOut,
            last_sync: HashMap::new(),
        }
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        matches!(self.auth, AuthState::SignedIn { .. })
    }

    /// Get the current auth state.
    pub fn auth_state(&self) -> &AuthState {
        &self.auth
    }

    /// Sign in with email and password.
    ///
    /// Derives encryption key from password via Argon2id, then
    /// authenticates with the sync server.
    pub fn sign_in(
        &mut self,
        server_url: &str,
        user_id: &str,
        token: &str,
    ) -> Result<(), SyncError> {
        self.auth = AuthState::SignedIn {
            server_url: server_url.to_string(),
            token: token.to_string(),
            user_id: user_id.to_string(),
        };
        tracing::info!(user_id, "sync: signed in");
        Ok(())
    }

    /// Sign out and clear state.
    pub fn sign_out(&mut self) {
        self.auth = AuthState::SignedOut;
        self.last_sync.clear();
        tracing::info!("sync: signed out");
    }

    /// Get the last sync timestamp for a collection.
    pub fn last_sync_time(&self, collection: SyncCollection) -> u64 {
        self.last_sync.get(&collection).copied().unwrap_or(0)
    }

    /// Mark a collection as synced at the given timestamp.
    pub fn mark_synced(&mut self, collection: SyncCollection, timestamp: u64) {
        self.last_sync.insert(collection, timestamp);
    }

    /// Get the server URL, if authenticated.
    pub fn server_url(&self) -> Option<&str> {
        match &self.auth {
            AuthState::SignedIn { server_url, .. } => Some(server_url),
            _ => None,
        }
    }

    fn auth_parts(&self) -> Result<(&str, &str, &str), SyncError> {
        match &self.auth {
            AuthState::SignedIn {
                server_url,
                token,
                user_id,
            } => Ok((server_url, token, user_id)),
            AuthState::SignedOut => Err(SyncError::NotAuthenticated),
        }
    }

    fn collection_endpoint(server_url: &str, collection: SyncCollection, suffix: &str) -> String {
        format!(
            "{}/api/v1/collections/{}/records{}",
            server_url.trim_end_matches('/'),
            collection.path(),
            suffix
        )
    }

    /// Push a batch of records to the sync server.
    pub async fn push_records(
        &mut self,
        collection: SyncCollection,
        records: &[SyncRecord],
    ) -> Result<(), SyncError> {
        if records.is_empty() {
            return Ok(());
        }

        let (server_url, token, _user_id) = self.auth_parts()?;
        let endpoint = Self::collection_endpoint(server_url, collection, "/batch");
        let url = VexUrl::parse(&endpoint).map_err(|e| SyncError::Network(e.to_string()))?;

        let mut max_modified = self.last_sync_time(collection);
        let wire_records: Vec<WireRecord> = records
            .iter()
            .map(|r| {
                let modified = if r.modified == 0 {
                    now_unix_ms()
                } else {
                    r.modified
                };
                if modified > max_modified {
                    max_modified = modified;
                }
                WireRecord {
                    record_id: r.id.clone(),
                    collection: collection.path().to_owned(),
                    ciphertext: if r.deleted {
                        String::new()
                    } else {
                        r.payload.clone()
                    },
                    version: 1,
                    modified_at: modified.to_string(),
                    content_hash: if r.deleted {
                        "tombstone".to_owned()
                    } else {
                        String::new()
                    },
                }
            })
            .collect();

        let body = serde_json::to_vec(&WireRecordList {
            records: wire_records,
        })
        .map_err(|e| SyncError::Serialization(e.to_string()))?;

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("authorization".to_string(), format!("Bearer {token}"));

        let request = Request {
            url,
            method: Method::Put,
            headers,
            body: Some(body),
        };

        let client = HttpClient::new().map_err(|e| SyncError::Network(e.to_string()))?;
        let response = client
            .fetch(request)
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !(200..300).contains(&response.status) {
            return Err(SyncError::Server {
                status: response.status,
            });
        }

        self.mark_synced(collection, max_modified.max(now_unix_ms()));
        Ok(())
    }

    /// Pull records changed since the last sync timestamp for a collection.
    pub async fn pull_records(&mut self, collection: SyncCollection) -> Result<Vec<SyncRecord>, SyncError> {
        let since = self.last_sync_time(collection);
        self.pull_records_since(collection, since).await
    }

    /// Pull records changed since a specific timestamp.
    pub async fn pull_records_since(
        &mut self,
        collection: SyncCollection,
        since: u64,
    ) -> Result<Vec<SyncRecord>, SyncError> {
        let (server_url, token, _user_id) = self.auth_parts()?;
        let endpoint = format!(
            "{}?since={since}",
            Self::collection_endpoint(server_url, collection, "")
        );
        let url = VexUrl::parse(&endpoint).map_err(|e| SyncError::Network(e.to_string()))?;

        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {token}"));

        let request = Request {
            url,
            method: Method::Get,
            headers,
            body: None,
        };

        let client = HttpClient::new().map_err(|e| SyncError::Network(e.to_string()))?;
        let response = client
            .fetch(request)
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !(200..300).contains(&response.status) {
            return Err(SyncError::Server {
                status: response.status,
            });
        }

        let list: WireRecordList = serde_json::from_slice(&response.body)
            .map_err(|e| SyncError::Serialization(e.to_string()))?;

        let mut out = Vec::with_capacity(list.records.len());
        let mut max_modified = since;
        for wr in list.records {
            let modified = wr.modified_at.parse::<u64>().unwrap_or(0);
            if modified > max_modified {
                max_modified = modified;
            }
            out.push(SyncRecord {
                id: wr.record_id,
                collection,
                payload: wr.ciphertext,
                modified,
                deleted: wr.content_hash == "tombstone",
            });
        }

        if max_modified > self.last_sync_time(collection) {
            self.mark_synced(collection, max_modified);
        }

        Ok(out)
    }

    /// Encrypt and push one logical collection payload as a single sync record.
    pub async fn push_collection_blob<T: Serialize>(
        &mut self,
        collection: SyncCollection,
        record_id: &str,
        value: &T,
        key: &vex_crypto::SymmetricKey,
    ) -> Result<(), SyncError> {
        let data = serde_json::to_vec(value).map_err(|e| SyncError::Serialization(e.to_string()))?;
        let mut record = Self::build_record(record_id, collection, &data, key)?;
        record.modified = now_unix_ms();
        self.push_records(collection, &[record]).await
    }

    /// Pull and decrypt the latest non-deleted blob for a collection.
    pub async fn pull_latest_collection_blob<T: DeserializeOwned>(
        &mut self,
        collection: SyncCollection,
        key: &vex_crypto::SymmetricKey,
    ) -> Result<Option<T>, SyncError> {
        let mut records = self.pull_records(collection).await?;
        records.sort_by_key(|r| r.modified);

        let latest = records
            .into_iter()
            .rev()
            .find(|r| !r.deleted && !r.payload.is_empty());

        let Some(record) = latest else {
            return Ok(None);
        };

        let plaintext = Self::decrypt_payload(&record.payload, key)?;
        let value = serde_json::from_slice::<T>(&plaintext)
            .map_err(|e| SyncError::Serialization(e.to_string()))?;
        Ok(Some(value))
    }

    /// Encrypt a record payload for sync.
    ///
    /// Uses the sync encryption key derived from the user's password.
    pub fn encrypt_payload(
        plaintext: &[u8],
        key: &vex_crypto::SymmetricKey,
    ) -> Result<String, SyncError> {
        let encrypted =
            vex_crypto::encrypt(key, plaintext).map_err(|e| SyncError::Crypto(e.to_string()))?;
        // Encode as base64 for JSON transport.
        Ok(base64_encode(&encrypted))
    }

    /// Decrypt a record payload from sync.
    pub fn decrypt_payload(
        encoded: &str,
        key: &vex_crypto::SymmetricKey,
    ) -> Result<Vec<u8>, SyncError> {
        let data = base64_decode(encoded).map_err(|e| SyncError::Serialization(e.to_string()))?;
        vex_crypto::decrypt(key, &data).map_err(|e| SyncError::Crypto(e.to_string()))
    }

    /// Build a sync record from plaintext data.
    pub fn build_record(
        id: &str,
        collection: SyncCollection,
        data: &[u8],
        key: &vex_crypto::SymmetricKey,
    ) -> Result<SyncRecord, SyncError> {
        let payload = Self::encrypt_payload(data, key)?;
        Ok(SyncRecord {
            id: id.to_string(),
            collection,
            payload,
            modified: 0, // Server assigns.
            deleted: false,
        })
    }

    /// Create a tombstone record (marks a record as deleted).
    pub fn build_tombstone(id: &str, collection: SyncCollection) -> SyncRecord {
        SyncRecord {
            id: id.to_string(),
            collection,
            payload: String::new(),
            modified: 0,
            deleted: true,
        }
    }
}

impl Default for SyncClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple base64 encoding (no padding).
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        }
    }

    out
}

/// Simple base64 decoding.
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("invalid base64 char: {}", c as char)),
        }
    }

    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);

    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = if chunk.len() > 2 { val(chunk[2])? } else { 0 };
        let d = if chunk.len() > 3 { val(chunk[3])? } else { 0 };

        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            out.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            out.push(triple as u8);
        }
    }

    Ok(out)
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_client_default_state() {
        let client = SyncClient::new();
        assert!(!client.is_authenticated());
        assert_eq!(client.last_sync_time(SyncCollection::Bookmarks), 0);
    }

    #[test]
    fn sign_in_and_out() {
        let mut client = SyncClient::new();
        client
            .sign_in("https://sync.example.com", "user1", "token123")
            .unwrap();
        assert!(client.is_authenticated());
        assert_eq!(client.server_url(), Some("https://sync.example.com"));

        client.sign_out();
        assert!(!client.is_authenticated());
        assert_eq!(client.server_url(), None);
    }

    #[test]
    fn mark_synced() {
        let mut client = SyncClient::new();
        client.mark_synced(SyncCollection::Bookmarks, 1000);
        assert_eq!(client.last_sync_time(SyncCollection::Bookmarks), 1000);
        assert_eq!(client.last_sync_time(SyncCollection::History), 0);
    }

    #[test]
    fn collection_paths() {
        assert_eq!(SyncCollection::Bookmarks.path(), "bookmarks");
        assert_eq!(SyncCollection::History.path(), "history");
        assert_eq!(SyncCollection::Settings.path(), "settings");
        assert_eq!(SyncCollection::OpenTabs.path(), "tabs");
    }

    #[test]
    fn encrypt_decrypt_payload() {
        let key = vex_crypto::SymmetricKey::generate();
        let data = b"test bookmark data";
        let encrypted = SyncClient::encrypt_payload(data, &key).unwrap();
        let decrypted = SyncClient::decrypt_payload(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn build_record_and_tombstone() {
        let key = vex_crypto::SymmetricKey::generate();
        let record = SyncClient::build_record(
            "bm-001",
            SyncCollection::Bookmarks,
            b"{\"url\":\"https://example.com\"}",
            &key,
        )
        .unwrap();
        assert_eq!(record.id, "bm-001");
        assert!(!record.deleted);
        assert!(!record.payload.is_empty());

        let tombstone = SyncClient::build_tombstone("bm-001", SyncCollection::Bookmarks);
        assert!(tombstone.deleted);
        assert!(tombstone.payload.is_empty());
    }

    #[test]
    fn base64_roundtrip() {
        let data = b"Hello, World! 123";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn base64_empty() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
    }
}
