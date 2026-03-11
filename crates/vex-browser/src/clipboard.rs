// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Clipboard API — async clipboard read/write and DataTransfer.
//!
//! Implements the core types for the [Clipboard API](https://w3c.github.io/clipboard-apis/):
//!
//! - `ClipboardItem`: a typed blob of data with a MIME type.
//! - `Clipboard`: async read/write access to the system clipboard.
//! - `DataTransfer`: the drag-and-drop / copy-paste data carrier.
//! - `DataTransferItem`: a single item in a DataTransfer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── ClipboardItem ────────────────────────────────────────────────────────────

/// A single clipboard item with a MIME type and data.
#[derive(Debug, Clone)]
pub struct ClipboardItem {
    /// MIME type → data mapping.
    types: HashMap<String, Vec<u8>>,
}

impl ClipboardItem {
    /// Create a new ClipboardItem from a single MIME type and data.
    pub fn new(mime_type: &str, data: Vec<u8>) -> Self {
        let mut types = HashMap::new();
        types.insert(mime_type.to_string(), data);
        Self { types }
    }

    /// Create a ClipboardItem from multiple MIME type → data pairs.
    pub fn from_map(types: HashMap<String, Vec<u8>>) -> Self {
        Self { types }
    }

    /// Get the list of MIME types available.
    pub fn types(&self) -> Vec<&str> {
        self.types.keys().map(|s| s.as_str()).collect()
    }

    /// Get data for a specific MIME type.
    pub fn get_type(&self, mime_type: &str) -> Option<&[u8]> {
        self.types.get(mime_type).map(|v| v.as_slice())
    }

    /// Whether this item contains the given MIME type.
    pub fn has_type(&self, mime_type: &str) -> bool {
        self.types.contains_key(mime_type)
    }
}

// ── Clipboard ────────────────────────────────────────────────────────────────

/// Permission state for clipboard access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardPermission {
    Granted,
    Denied,
    Prompt,
}

/// Simulated system clipboard (in-process for now).
///
/// In a real browser this would interface with the OS clipboard via platform
/// APIs. This implementation provides the API surface and an in-memory store.
#[derive(Debug, Clone)]
pub struct Clipboard {
    items: Arc<Mutex<Vec<ClipboardItem>>>,
    read_permission: ClipboardPermission,
    write_permission: ClipboardPermission,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            read_permission: ClipboardPermission::Granted,
            write_permission: ClipboardPermission::Granted,
        }
    }

    /// Set read permission.
    pub fn set_read_permission(&mut self, perm: ClipboardPermission) {
        self.read_permission = perm;
    }

    /// Set write permission.
    pub fn set_write_permission(&mut self, perm: ClipboardPermission) {
        self.write_permission = perm;
    }

    /// Read all clipboard items (async in spec, sync here).
    pub fn read(&self) -> Result<Vec<ClipboardItem>, ClipboardError> {
        if self.read_permission == ClipboardPermission::Denied {
            return Err(ClipboardError::NotAllowed);
        }
        let items = self.items.lock().map_err(|_| ClipboardError::Internal)?;
        Ok(items.clone())
    }

    /// Read clipboard as plain text.
    pub fn read_text(&self) -> Result<String, ClipboardError> {
        if self.read_permission == ClipboardPermission::Denied {
            return Err(ClipboardError::NotAllowed);
        }
        let items = self.items.lock().map_err(|_| ClipboardError::Internal)?;
        for item in items.iter() {
            if let Some(data) = item.get_type("text/plain") {
                return String::from_utf8(data.to_vec())
                    .map_err(|_| ClipboardError::InvalidData);
            }
        }
        Ok(String::new())
    }

    /// Write items to the clipboard.
    pub fn write(&self, new_items: Vec<ClipboardItem>) -> Result<(), ClipboardError> {
        if self.write_permission == ClipboardPermission::Denied {
            return Err(ClipboardError::NotAllowed);
        }
        let mut items = self.items.lock().map_err(|_| ClipboardError::Internal)?;
        *items = new_items;
        Ok(())
    }

    /// Write plain text to the clipboard.
    pub fn write_text(&self, text: &str) -> Result<(), ClipboardError> {
        if self.write_permission == ClipboardPermission::Denied {
            return Err(ClipboardError::NotAllowed);
        }
        let item = ClipboardItem::new("text/plain", text.as_bytes().to_vec());
        let mut items = self.items.lock().map_err(|_| ClipboardError::Internal)?;
        *items = vec![item];
        Ok(())
    }

    /// Clear the clipboard.
    pub fn clear(&self) -> Result<(), ClipboardError> {
        let mut items = self.items.lock().map_err(|_| ClipboardError::Internal)?;
        items.clear();
        Ok(())
    }
}

// ── DataTransfer ─────────────────────────────────────────────────────────────

/// The `effectAllowed` property of a DataTransfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectAllowed {
    #[default]
    Uninitialized,
    None,
    Copy,
    CopyLink,
    CopyMove,
    Link,
    LinkMove,
    Move,
    All,
}

/// The `dropEffect` property of a DataTransfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropEffect {
    #[default]
    None,
    Copy,
    Link,
    Move,
}

/// The kind of a DataTransferItem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTransferItemKind {
    String,
    File,
}

/// A single item in a DataTransfer.
#[derive(Debug, Clone)]
pub struct DataTransferItem {
    pub kind: DataTransferItemKind,
    pub mime_type: String,
    pub data: Vec<u8>,
}

impl DataTransferItem {
    /// Create a string item.
    pub fn string(mime_type: &str, data: &str) -> Self {
        Self {
            kind: DataTransferItemKind::String,
            mime_type: mime_type.to_string(),
            data: data.as_bytes().to_vec(),
        }
    }

    /// Create a file item.
    pub fn file(mime_type: &str, data: Vec<u8>) -> Self {
        Self {
            kind: DataTransferItemKind::File,
            mime_type: mime_type.to_string(),
            data,
        }
    }

    /// Get the string data (if this is a string item).
    pub fn as_string(&self) -> Option<String> {
        if self.kind == DataTransferItemKind::String {
            String::from_utf8(self.data.clone()).ok()
        } else {
            None
        }
    }
}

/// DataTransfer — the data carrier for clipboard and drag-and-drop events.
#[derive(Debug, Clone, Default)]
pub struct DataTransfer {
    pub drop_effect: DropEffect,
    pub effect_allowed: EffectAllowed,
    items: Vec<DataTransferItem>,
}

impl DataTransfer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the list of items.
    pub fn items(&self) -> &[DataTransferItem] {
        &self.items
    }

    /// Number of items.
    pub fn length(&self) -> usize {
        self.items.len()
    }

    /// Add a string item.
    pub fn set_data(&mut self, mime_type: &str, data: &str) {
        // Remove existing item of same type.
        self.items.retain(|item| item.mime_type != mime_type);
        self.items.push(DataTransferItem::string(mime_type, data));
    }

    /// Get string data by MIME type.
    pub fn get_data(&self, mime_type: &str) -> String {
        self.items
            .iter()
            .find(|item| item.mime_type == mime_type && item.kind == DataTransferItemKind::String)
            .and_then(|item| item.as_string())
            .unwrap_or_default()
    }

    /// Remove an item by MIME type.
    pub fn clear_data(&mut self, mime_type: Option<&str>) {
        match mime_type {
            Some(mt) => self.items.retain(|item| item.mime_type != mt),
            None => self.items.clear(),
        }
    }

    /// Add a file item.
    pub fn add_file(&mut self, mime_type: &str, data: Vec<u8>) {
        self.items.push(DataTransferItem::file(mime_type, data));
    }

    /// Get all file items.
    pub fn files(&self) -> Vec<&DataTransferItem> {
        self.items
            .iter()
            .filter(|item| item.kind == DataTransferItemKind::File)
            .collect()
    }

    /// Get the list of MIME types present.
    pub fn types(&self) -> Vec<&str> {
        self.items.iter().map(|item| item.mime_type.as_str()).collect()
    }
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Clipboard operation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardError {
    /// The operation was not allowed (permission denied).
    NotAllowed,
    /// The clipboard data was invalid.
    InvalidData,
    /// Internal error (mutex poisoned, etc.).
    Internal,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed => write!(f, "clipboard: operation not allowed"),
            Self::InvalidData => write!(f, "clipboard: invalid data"),
            Self::Internal => write!(f, "clipboard: internal error"),
        }
    }
}

impl std::error::Error for ClipboardError {}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ClipboardItem tests ──────────────────────────────────────────

    #[test]
    fn clipboard_item_single_type() {
        let item = ClipboardItem::new("text/plain", b"hello".to_vec());
        assert!(item.has_type("text/plain"));
        assert!(!item.has_type("text/html"));
        assert_eq!(item.get_type("text/plain"), Some(b"hello".as_slice()));
    }

    #[test]
    fn clipboard_item_multiple_types() {
        let mut map = HashMap::new();
        map.insert("text/plain".to_string(), b"hello".to_vec());
        map.insert("text/html".to_string(), b"<b>hello</b>".to_vec());
        let item = ClipboardItem::from_map(map);
        assert_eq!(item.types().len(), 2);
        assert!(item.has_type("text/plain"));
        assert!(item.has_type("text/html"));
    }

    // ── Clipboard tests ─────────────────────────────────────────────

    #[test]
    fn clipboard_write_and_read_text() {
        let clipboard = Clipboard::new();
        clipboard.write_text("hello world").unwrap();
        assert_eq!(clipboard.read_text().unwrap(), "hello world");
    }

    #[test]
    fn clipboard_write_and_read_items() {
        let clipboard = Clipboard::new();
        let item = ClipboardItem::new("text/plain", b"data".to_vec());
        clipboard.write(vec![item]).unwrap();
        let items = clipboard.read().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get_type("text/plain"), Some(b"data".as_slice()));
    }

    #[test]
    fn clipboard_clear() {
        let clipboard = Clipboard::new();
        clipboard.write_text("test").unwrap();
        clipboard.clear().unwrap();
        let items = clipboard.read().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn clipboard_read_empty() {
        let clipboard = Clipboard::new();
        assert_eq!(clipboard.read_text().unwrap(), "");
    }

    #[test]
    fn clipboard_permission_denied_read() {
        let mut clipboard = Clipboard::new();
        clipboard.set_read_permission(ClipboardPermission::Denied);
        assert_eq!(clipboard.read_text(), Err(ClipboardError::NotAllowed));
    }

    #[test]
    fn clipboard_permission_denied_write() {
        let mut clipboard = Clipboard::new();
        clipboard.set_write_permission(ClipboardPermission::Denied);
        assert_eq!(
            clipboard.write_text("denied"),
            Err(ClipboardError::NotAllowed)
        );
    }

    #[test]
    fn clipboard_overwrite() {
        let clipboard = Clipboard::new();
        clipboard.write_text("first").unwrap();
        clipboard.write_text("second").unwrap();
        assert_eq!(clipboard.read_text().unwrap(), "second");
    }

    // ── DataTransfer tests ──────────────────────────────────────────

    #[test]
    fn data_transfer_set_and_get() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "hello");
        assert_eq!(dt.get_data("text/plain"), "hello");
        assert_eq!(dt.length(), 1);
    }

    #[test]
    fn data_transfer_multiple_types() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "hello");
        dt.set_data("text/html", "<b>hello</b>");
        assert_eq!(dt.length(), 2);
        assert_eq!(dt.get_data("text/plain"), "hello");
        assert_eq!(dt.get_data("text/html"), "<b>hello</b>");
    }

    #[test]
    fn data_transfer_overwrite_same_type() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "first");
        dt.set_data("text/plain", "second");
        assert_eq!(dt.length(), 1);
        assert_eq!(dt.get_data("text/plain"), "second");
    }

    #[test]
    fn data_transfer_clear_specific() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "a");
        dt.set_data("text/html", "b");
        dt.clear_data(Some("text/plain"));
        assert_eq!(dt.length(), 1);
        assert_eq!(dt.get_data("text/plain"), "");
        assert_eq!(dt.get_data("text/html"), "b");
    }

    #[test]
    fn data_transfer_clear_all() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "a");
        dt.set_data("text/html", "b");
        dt.clear_data(None);
        assert_eq!(dt.length(), 0);
    }

    #[test]
    fn data_transfer_files() {
        let mut dt = DataTransfer::new();
        dt.add_file("image/png", vec![0x89, 0x50, 0x4E, 0x47]);
        dt.set_data("text/plain", "description");
        assert_eq!(dt.files().len(), 1);
        assert_eq!(dt.files()[0].mime_type, "image/png");
    }

    #[test]
    fn data_transfer_types() {
        let mut dt = DataTransfer::new();
        dt.set_data("text/plain", "a");
        dt.set_data("text/html", "b");
        let types = dt.types();
        assert!(types.contains(&"text/plain"));
        assert!(types.contains(&"text/html"));
    }

    #[test]
    fn data_transfer_get_missing() {
        let dt = DataTransfer::new();
        assert_eq!(dt.get_data("text/plain"), "");
    }

    #[test]
    fn data_transfer_effect_defaults() {
        let dt = DataTransfer::new();
        assert_eq!(dt.drop_effect, DropEffect::None);
        assert_eq!(dt.effect_allowed, EffectAllowed::Uninitialized);
    }

    // ── DataTransferItem tests ──────────────────────────────────────

    #[test]
    fn data_transfer_item_string() {
        let item = DataTransferItem::string("text/plain", "test");
        assert_eq!(item.kind, DataTransferItemKind::String);
        assert_eq!(item.as_string(), Some("test".to_string()));
    }

    #[test]
    fn data_transfer_item_file() {
        let item = DataTransferItem::file("application/pdf", vec![0x25, 0x50, 0x44, 0x46]);
        assert_eq!(item.kind, DataTransferItemKind::File);
        assert_eq!(item.as_string(), None);
    }
}
