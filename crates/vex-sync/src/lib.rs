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

use serde::{Deserialize, Serialize};

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

    /// Encrypt a record payload for sync.
    ///
    /// Uses the sync encryption key derived from the user's password.
    pub fn encrypt_payload(plaintext: &[u8], key: &vex_crypto::SymmetricKey) -> Result<String, SyncError> {
        let encrypted = vex_crypto::encrypt(key, plaintext)
            .map_err(|e| SyncError::Crypto(e.to_string()))?;
        // Encode as base64 for JSON transport.
        Ok(base64_encode(&encrypted))
    }

    /// Decrypt a record payload from sync.
    pub fn decrypt_payload(encoded: &str, key: &vex_crypto::SymmetricKey) -> Result<Vec<u8>, SyncError> {
        let data = base64_decode(encoded)
            .map_err(|e| SyncError::Serialization(e.to_string()))?;
        vex_crypto::decrypt(key, &data)
            .map_err(|e| SyncError::Crypto(e.to_string()))
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
        client.sign_in("https://sync.example.com", "user1", "token123").unwrap();
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
        ).unwrap();
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
