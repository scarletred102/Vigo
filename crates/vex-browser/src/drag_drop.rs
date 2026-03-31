// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTML Drag and Drop API.
//!
//! Implements the drag-and-drop event system with `DataTransfer`,
//! `DragEvent` types, and a `DragController` to manage the drag session.

use std::collections::HashMap;

use vex_core::VexId;

// ── DropEffect / EffectAllowed ───────────────────────────────────────────────

/// The visual feedback for a drag operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropEffect {
    #[default]
    None,
    Copy,
    Move,
    Link,
}

impl DropEffect {
    pub fn parse(s: &str) -> Self {
        match s {
            "copy" => Self::Copy,
            "move" => Self::Move,
            "link" => Self::Link,
            _ => Self::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Copy => "copy",
            Self::Move => "move",
            Self::Link => "link",
        }
    }
}

/// Allowed operations for a drag source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectAllowed {
    None,
    Copy,
    CopyLink,
    CopyMove,
    Link,
    LinkMove,
    Move,
    #[default]
    All,
    Uninitialized,
}

impl EffectAllowed {
    pub fn parse(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "copy" => Self::Copy,
            "copyLink" => Self::CopyLink,
            "copyMove" => Self::CopyMove,
            "link" => Self::Link,
            "linkMove" => Self::LinkMove,
            "move" => Self::Move,
            "all" => Self::All,
            "uninitialized" => Self::Uninitialized,
            _ => Self::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Copy => "copy",
            Self::CopyLink => "copyLink",
            Self::CopyMove => "copyMove",
            Self::Link => "link",
            Self::LinkMove => "linkMove",
            Self::Move => "move",
            Self::All => "all",
            Self::Uninitialized => "uninitialized",
        }
    }

    /// Whether this effect allows the given drop effect.
    pub fn allows(&self, effect: DropEffect) -> bool {
        match self {
            Self::None => effect == DropEffect::None,
            Self::Copy => matches!(effect, DropEffect::Copy | DropEffect::None),
            Self::Move => matches!(effect, DropEffect::Move | DropEffect::None),
            Self::Link => matches!(effect, DropEffect::Link | DropEffect::None),
            Self::CopyLink => matches!(effect, DropEffect::Copy | DropEffect::Link | DropEffect::None),
            Self::CopyMove => matches!(effect, DropEffect::Copy | DropEffect::Move | DropEffect::None),
            Self::LinkMove => matches!(effect, DropEffect::Link | DropEffect::Move | DropEffect::None),
            Self::All | Self::Uninitialized => true,
        }
    }
}

// ── DataTransferItem ─────────────────────────────────────────────────────────

/// The kind of data in a DataTransferItem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTransferItemKind {
    String,
    File,
}

/// A single item in a drag data store.
#[derive(Debug, Clone)]
pub struct DataTransferItem {
    pub kind: DataTransferItemKind,
    pub mime_type: String,
    pub data: String,
}

impl DataTransferItem {
    pub fn new_string(mime_type: &str, data: &str) -> Self {
        Self {
            kind: DataTransferItemKind::String,
            mime_type: mime_type.to_lowercase(),
            data: data.to_string(),
        }
    }

    pub fn new_file(name: &str) -> Self {
        Self {
            kind: DataTransferItemKind::File,
            mime_type: "application/octet-stream".to_string(),
            data: name.to_string(),
        }
    }
}

// ── DragDataStore ────────────────────────────────────────────────────────────

/// The drag data store holds items during a drag operation.
#[derive(Debug, Clone, Default)]
pub struct DragDataStore {
    items: Vec<DataTransferItem>,
    mode: DragDataStoreMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragDataStoreMode {
    /// ReadWrite: source can modify data (during dragstart).
    #[default]
    ReadWrite,
    /// ReadOnly: data can be read but not modified (during drop).
    ReadOnly,
    /// Protected: data types visible but values hidden (during drag events).
    Protected,
}

impl DragDataStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a data entry. If an item with the same type exists, replace it.
    pub fn set_data(&mut self, mime_type: &str, data: &str) -> bool {
        if self.mode != DragDataStoreMode::ReadWrite {
            return false;
        }
        let lower = mime_type.to_lowercase();
        if let Some(item) = self.items.iter_mut().find(|i| i.mime_type == lower) {
            item.data = data.to_string();
        } else {
            self.items.push(DataTransferItem::new_string(&lower, data));
        }
        true
    }

    /// Get data for a given MIME type.
    pub fn get_data(&self, mime_type: &str) -> Option<&str> {
        if self.mode == DragDataStoreMode::Protected {
            return None;
        }
        let lower = mime_type.to_lowercase();
        self.items
            .iter()
            .find(|i| i.mime_type == lower)
            .map(|i| i.data.as_str())
    }

    /// Remove data for a given MIME type.
    pub fn clear_data(&mut self, mime_type: &str) -> bool {
        if self.mode != DragDataStoreMode::ReadWrite {
            return false;
        }
        let lower = mime_type.to_lowercase();
        let len = self.items.len();
        self.items.retain(|i| i.mime_type != lower);
        self.items.len() < len
    }

    /// Remove all items.
    pub fn clear_all(&mut self) -> bool {
        if self.mode != DragDataStoreMode::ReadWrite {
            return false;
        }
        self.items.clear();
        true
    }

    /// Add a file item.
    pub fn add_file(&mut self, name: &str) -> bool {
        if self.mode != DragDataStoreMode::ReadWrite {
            return false;
        }
        self.items.push(DataTransferItem::new_file(name));
        true
    }

    /// Get all item types.
    pub fn types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.items.iter().map(|i| i.mime_type.clone()).collect();
        types.dedup();
        types
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get all items (respects mode).
    pub fn items(&self) -> &[DataTransferItem] {
        &self.items
    }

    /// Get all file items.
    pub fn files(&self) -> Vec<&DataTransferItem> {
        self.items
            .iter()
            .filter(|i| i.kind == DataTransferItemKind::File)
            .collect()
    }

    /// Set the store mode.
    pub fn set_mode(&mut self, mode: DragDataStoreMode) {
        self.mode = mode;
    }
}

// ── DragEvent ────────────────────────────────────────────────────────────────

/// Type of drag event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragEventType {
    DragStart,
    Drag,
    DragEnter,
    DragOver,
    DragLeave,
    Drop,
    DragEnd,
}

impl DragEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DragStart => "dragstart",
            Self::Drag => "drag",
            Self::DragEnter => "dragenter",
            Self::DragOver => "dragover",
            Self::DragLeave => "dragleave",
            Self::Drop => "drop",
            Self::DragEnd => "dragend",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "dragstart" => Some(Self::DragStart),
            "drag" => Some(Self::Drag),
            "dragenter" => Some(Self::DragEnter),
            "dragover" => Some(Self::DragOver),
            "dragleave" => Some(Self::DragLeave),
            "drop" => Some(Self::Drop),
            "dragend" => Some(Self::DragEnd),
            _ => None,
        }
    }

    /// Whether this event is cancelable.
    pub fn cancelable(&self) -> bool {
        matches!(
            self,
            Self::DragStart | Self::DragEnter | Self::DragOver | Self::Drop
        )
    }
}

/// A drag-and-drop event.
#[derive(Debug, Clone)]
pub struct DragEvent {
    pub event_type: DragEventType,
    pub target: VexId,
    pub client_x: f32,
    pub client_y: f32,
    pub drop_effect: DropEffect,
    pub effect_allowed: EffectAllowed,
    pub default_prevented: bool,
}

impl DragEvent {
    pub fn new(event_type: DragEventType, target: VexId, x: f32, y: f32) -> Self {
        Self {
            event_type,
            target,
            client_x: x,
            client_y: y,
            drop_effect: DropEffect::None,
            effect_allowed: EffectAllowed::All,
            default_prevented: false,
        }
    }

    pub fn prevent_default(&mut self) {
        if self.event_type.cancelable() {
            self.default_prevented = true;
        }
    }
}

// ── DragController ───────────────────────────────────────────────────────────

/// Active drag session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    /// No active drag.
    Idle,
    /// Drag in progress.
    Dragging,
}

/// Manages the drag-and-drop session lifecycle.
#[derive(Debug)]
pub struct DragController {
    /// Current phase.
    pub phase: DragPhase,
    /// Source element being dragged.
    pub source: Option<VexId>,
    /// Current element under the cursor.
    pub current_target: Option<VexId>,
    /// The drag data store.
    pub data_store: DragDataStore,
    /// Drop effect.
    pub drop_effect: DropEffect,
    /// Allowed effects.
    pub effect_allowed: EffectAllowed,
    /// Registered drop zones and their accepted types.
    drop_zones: HashMap<VexId, Vec<String>>,
    /// Event log (for testing/debugging).
    event_log: Vec<(DragEventType, VexId)>,
}

impl Default for DragController {
    fn default() -> Self {
        Self::new()
    }
}

impl DragController {
    pub fn new() -> Self {
        Self {
            phase: DragPhase::Idle,
            source: None,
            current_target: None,
            data_store: DragDataStore::new(),
            drop_effect: DropEffect::None,
            effect_allowed: EffectAllowed::All,
            drop_zones: HashMap::new(),
            event_log: Vec::new(),
        }
    }

    /// Register a drop zone that accepts certain MIME types.
    pub fn register_drop_zone(&mut self, element_id: VexId, accepted_types: Vec<String>) {
        self.drop_zones.insert(element_id, accepted_types);
    }

    /// Unregister a drop zone.
    pub fn unregister_drop_zone(&mut self, element_id: VexId) {
        self.drop_zones.remove(&element_id);
    }

    /// Whether an element is a registered drop zone.
    pub fn is_drop_zone(&self, element_id: VexId) -> bool {
        self.drop_zones.contains_key(&element_id)
    }

    /// Begin a drag operation from a source element.
    /// Returns the dragstart event.
    pub fn drag_start(&mut self, source: VexId, x: f32, y: f32) -> DragEvent {
        self.phase = DragPhase::Dragging;
        self.source = Some(source);
        self.data_store = DragDataStore::new();
        self.data_store.set_mode(DragDataStoreMode::ReadWrite);
        self.drop_effect = DropEffect::None;
        self.effect_allowed = EffectAllowed::All;

        let event = DragEvent::new(DragEventType::DragStart, source, x, y);
        self.event_log.push((DragEventType::DragStart, source));
        event
    }

    /// During drag, mouse moves. Fires drag on source and dragenter/dragover/dragleave
    /// on targets.
    pub fn drag_move(&mut self, target: VexId, x: f32, y: f32) -> Vec<DragEvent> {
        if self.phase != DragPhase::Dragging {
            return Vec::new();
        }

        self.data_store.set_mode(DragDataStoreMode::Protected);
        let mut events = Vec::new();

        // Fire drag on source
        if let Some(source) = self.source {
            let drag_event = DragEvent::new(DragEventType::Drag, source, x, y);
            events.push(drag_event);
            self.event_log.push((DragEventType::Drag, source));
        }

        // Handle target changes
        let old_target = self.current_target;
        if old_target != Some(target) {
            // dragleave from old target
            if let Some(old) = old_target {
                let leave = DragEvent::new(DragEventType::DragLeave, old, x, y);
                events.push(leave);
                self.event_log.push((DragEventType::DragLeave, old));
            }
            // dragenter to new target
            let enter = DragEvent::new(DragEventType::DragEnter, target, x, y);
            events.push(enter);
            self.event_log.push((DragEventType::DragEnter, target));
            self.current_target = Some(target);
        }

        // dragover on current target
        let over = DragEvent::new(DragEventType::DragOver, target, x, y);
        events.push(over);
        self.event_log.push((DragEventType::DragOver, target));

        events
    }

    /// User releases mouse — fires drop (if valid drop zone) and dragend.
    pub fn drag_end(&mut self, target: VexId, x: f32, y: f32) -> Vec<DragEvent> {
        if self.phase != DragPhase::Dragging {
            return Vec::new();
        }

        let mut events = Vec::new();

        // Check if this is a valid drop zone
        let is_valid_drop = self.is_valid_drop(target);

        if is_valid_drop {
            self.data_store.set_mode(DragDataStoreMode::ReadOnly);
            let mut drop_event = DragEvent::new(DragEventType::Drop, target, x, y);
            drop_event.drop_effect = self.drop_effect;
            drop_event.effect_allowed = self.effect_allowed;
            events.push(drop_event);
            self.event_log.push((DragEventType::Drop, target));
        }

        // dragend on source
        if let Some(source) = self.source {
            let mut end_event = DragEvent::new(DragEventType::DragEnd, source, x, y);
            end_event.drop_effect = if is_valid_drop {
                self.drop_effect
            } else {
                DropEffect::None
            };
            events.push(end_event);
            self.event_log.push((DragEventType::DragEnd, source));
        }

        self.reset();
        events
    }

    /// Cancel the current drag.
    pub fn cancel(&mut self) -> Option<DragEvent> {
        if self.phase != DragPhase::Dragging {
            return None;
        }
        let event = self.source.map(|source| {
            self.event_log.push((DragEventType::DragEnd, source));
            let mut e = DragEvent::new(DragEventType::DragEnd, source, 0.0, 0.0);
            e.drop_effect = DropEffect::None;
            e
        });
        self.reset();
        event
    }

    /// Whether a drop on this target would be valid.
    fn is_valid_drop(&self, target: VexId) -> bool {
        if !self.is_drop_zone(target) {
            return false;
        }

        // Check if the drop zone accepts any of the dragged types
        if let Some(accepted) = self.drop_zones.get(&target) {
            if accepted.is_empty() {
                return true; // Accepts all
            }
            let dragged_types = self.data_store.types();
            accepted.iter().any(|a| dragged_types.contains(a))
        } else {
            false
        }
    }

    /// Reset the controller to idle.
    fn reset(&mut self) {
        self.phase = DragPhase::Idle;
        self.source = None;
        self.current_target = None;
        self.data_store = DragDataStore::new();
        self.drop_effect = DropEffect::None;
        self.effect_allowed = EffectAllowed::All;
    }

    /// Get the event log (for testing).
    pub fn event_log(&self) -> &[(DragEventType, VexId)] {
        &self.event_log
    }

    /// Clear the event log.
    pub fn clear_log(&mut self) {
        self.event_log.clear();
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> VexId {
        VexId::new(n)
    }

    // ── DropEffect / EffectAllowed ──────────────────────────────

    #[test]
    fn drop_effect_parse() {
        assert_eq!(DropEffect::parse("copy"), DropEffect::Copy);
        assert_eq!(DropEffect::parse("move"), DropEffect::Move);
        assert_eq!(DropEffect::parse("link"), DropEffect::Link);
        assert_eq!(DropEffect::parse("none"), DropEffect::None);
        assert_eq!(DropEffect::parse("invalid"), DropEffect::None);
    }

    #[test]
    fn effect_allowed_roundtrip() {
        for name in &["none", "copy", "copyLink", "copyMove", "link", "linkMove", "move", "all", "uninitialized"] {
            let e = EffectAllowed::parse(name);
            assert_eq!(e.as_str(), *name);
        }
    }

    #[test]
    fn effect_allowed_allows() {
        assert!(EffectAllowed::All.allows(DropEffect::Copy));
        assert!(EffectAllowed::Copy.allows(DropEffect::Copy));
        assert!(!EffectAllowed::Copy.allows(DropEffect::Move));
        assert!(EffectAllowed::CopyMove.allows(DropEffect::Copy));
        assert!(EffectAllowed::CopyMove.allows(DropEffect::Move));
        assert!(!EffectAllowed::CopyMove.allows(DropEffect::Link));
        assert!(!EffectAllowed::None.allows(DropEffect::Copy));
    }

    // ── DragDataStore ───────────────────────────────────────────

    #[test]
    fn data_store_set_get() {
        let mut store = DragDataStore::new();
        assert!(store.set_data("text/plain", "hello"));
        assert_eq!(store.get_data("text/plain"), Some("hello"));
        assert_eq!(store.get_data("TEXT/PLAIN"), Some("hello"));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn data_store_replace() {
        let mut store = DragDataStore::new();
        store.set_data("text/plain", "first");
        store.set_data("text/plain", "second");
        assert_eq!(store.get_data("text/plain"), Some("second"));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn data_store_clear() {
        let mut store = DragDataStore::new();
        store.set_data("text/plain", "hello");
        store.set_data("text/html", "<b>hi</b>");
        assert!(store.clear_data("text/plain"));
        assert_eq!(store.get_data("text/plain"), None);
        assert_eq!(store.get_data("text/html"), Some("<b>hi</b>"));
        store.clear_all();
        assert!(store.is_empty());
    }

    #[test]
    fn data_store_protected_mode() {
        let mut store = DragDataStore::new();
        store.set_data("text/plain", "secret");
        store.set_mode(DragDataStoreMode::Protected);
        assert_eq!(store.get_data("text/plain"), None);
        assert!(!store.types().is_empty()); // Types still visible
    }

    #[test]
    fn data_store_readonly_mode() {
        let mut store = DragDataStore::new();
        store.set_data("text/plain", "hello");
        store.set_mode(DragDataStoreMode::ReadOnly);
        assert!(!store.set_data("text/html", "nope")); // Can't write
        assert_eq!(store.get_data("text/plain"), Some("hello")); // Can read
    }

    #[test]
    fn data_store_files() {
        let mut store = DragDataStore::new();
        store.add_file("doc.pdf");
        store.add_file("img.png");
        assert_eq!(store.files().len(), 2);
        assert_eq!(store.files()[0].data, "doc.pdf");
    }

    // ── DragEventType ───────────────────────────────────────────

    #[test]
    fn event_type_parse_roundtrip() {
        for name in &["dragstart", "drag", "dragenter", "dragover", "dragleave", "drop", "dragend"] {
            let t = DragEventType::parse(name).unwrap();
            assert_eq!(t.as_str(), *name);
        }
        assert_eq!(DragEventType::parse("invalid"), None);
    }

    #[test]
    fn event_type_cancelable() {
        assert!(DragEventType::DragStart.cancelable());
        assert!(DragEventType::DragOver.cancelable());
        assert!(DragEventType::Drop.cancelable());
        assert!(!DragEventType::Drag.cancelable());
        assert!(!DragEventType::DragEnd.cancelable());
    }

    // ── DragEvent ───────────────────────────────────────────────

    #[test]
    fn drag_event_prevent_default() {
        let mut event = DragEvent::new(DragEventType::DragOver, id(1), 10.0, 20.0);
        assert!(!event.default_prevented);
        event.prevent_default();
        assert!(event.default_prevented);

        // Non-cancelable event
        let mut event = DragEvent::new(DragEventType::Drag, id(1), 10.0, 20.0);
        event.prevent_default();
        assert!(!event.default_prevented);
    }

    // ── DragController ──────────────────────────────────────────

    #[test]
    fn controller_lifecycle() {
        let mut ctrl = DragController::new();
        assert_eq!(ctrl.phase, DragPhase::Idle);

        // Start drag
        let _start = ctrl.drag_start(id(1), 10.0, 10.0);
        assert_eq!(ctrl.phase, DragPhase::Dragging);
        assert_eq!(ctrl.source, Some(id(1)));

        // Set data
        ctrl.data_store.set_data("text/plain", "hello");

        // Move to target
        ctrl.register_drop_zone(id(2), vec!["text/plain".to_string()]);
        let move_events = ctrl.drag_move(id(2), 50.0, 50.0);
        assert!(move_events.len() >= 3); // drag, dragenter, dragover

        // Drop
        let end_events = ctrl.drag_end(id(2), 50.0, 50.0);
        assert!(end_events.iter().any(|e| e.event_type == DragEventType::Drop));
        assert!(end_events.iter().any(|e| e.event_type == DragEventType::DragEnd));
        assert_eq!(ctrl.phase, DragPhase::Idle);
    }

    #[test]
    fn controller_drop_zone() {
        let mut ctrl = DragController::new();
        ctrl.register_drop_zone(id(10), vec!["text/plain".to_string()]);
        assert!(ctrl.is_drop_zone(id(10)));
        assert!(!ctrl.is_drop_zone(id(11)));
        ctrl.unregister_drop_zone(id(10));
        assert!(!ctrl.is_drop_zone(id(10)));
    }

    #[test]
    fn controller_invalid_drop() {
        let mut ctrl = DragController::new();
        ctrl.drag_start(id(1), 0.0, 0.0);
        ctrl.data_store.set_data("text/plain", "data");

        // Drop on non-drop-zone
        let events = ctrl.drag_end(id(99), 0.0, 0.0);
        // Should have dragend but NO drop
        assert!(!events.iter().any(|e| e.event_type == DragEventType::Drop));
        assert!(events.iter().any(|e| e.event_type == DragEventType::DragEnd));
    }

    #[test]
    fn controller_type_mismatch() {
        let mut ctrl = DragController::new();
        ctrl.register_drop_zone(id(5), vec!["image/png".to_string()]);
        ctrl.drag_start(id(1), 0.0, 0.0);
        ctrl.data_store.set_data("text/plain", "text data");

        let events = ctrl.drag_end(id(5), 0.0, 0.0);
        // Drop zone accepts image/png but data is text/plain
        assert!(!events.iter().any(|e| e.event_type == DragEventType::Drop));
    }

    #[test]
    fn controller_cancel() {
        let mut ctrl = DragController::new();
        ctrl.drag_start(id(1), 0.0, 0.0);
        let event = ctrl.cancel();
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, DragEventType::DragEnd);
        assert_eq!(ctrl.phase, DragPhase::Idle);
    }

    #[test]
    fn controller_target_transitions() {
        let mut ctrl = DragController::new();
        ctrl.drag_start(id(1), 0.0, 0.0);

        // Move to first target
        let events = ctrl.drag_move(id(2), 10.0, 10.0);
        assert!(events.iter().any(|e| e.event_type == DragEventType::DragEnter && e.target == id(2)));

        // Move to second target — should get dragleave from first
        let events = ctrl.drag_move(id(3), 20.0, 20.0);
        assert!(events.iter().any(|e| e.event_type == DragEventType::DragLeave && e.target == id(2)));
        assert!(events.iter().any(|e| e.event_type == DragEventType::DragEnter && e.target == id(3)));
    }

    #[test]
    fn controller_event_log() {
        let mut ctrl = DragController::new();
        ctrl.drag_start(id(1), 0.0, 0.0);
        ctrl.drag_move(id(2), 10.0, 10.0);
        assert!(!ctrl.event_log().is_empty());

        let log = ctrl.event_log().to_vec();
        assert_eq!(log[0], (DragEventType::DragStart, id(1)));
        ctrl.clear_log();
        assert!(ctrl.event_log().is_empty());
    }

    #[test]
    fn controller_empty_accepted_types() {
        let mut ctrl = DragController::new();
        ctrl.register_drop_zone(id(5), vec![]); // Accepts all types
        ctrl.drag_start(id(1), 0.0, 0.0);
        ctrl.data_store.set_data("text/plain", "anything");

        let events = ctrl.drag_end(id(5), 0.0, 0.0);
        assert!(events.iter().any(|e| e.event_type == DragEventType::Drop));
    }

    #[test]
    fn controller_idle_operations() {
        let mut ctrl = DragController::new();
        // Operations when idle should be no-ops
        let events = ctrl.drag_move(id(1), 0.0, 0.0);
        assert!(events.is_empty());
        let events = ctrl.drag_end(id(1), 0.0, 0.0);
        assert!(events.is_empty());
        assert!(ctrl.cancel().is_none());
    }
}
