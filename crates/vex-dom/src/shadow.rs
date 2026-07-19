// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Shadow DOM implementation.
//!
//! Supports attaching shadow roots (open or closed) to host elements,
//! with slot-based content distribution and encapsulation.

use std::collections::HashMap;
use vex_core::VexId;

// ── Types ────────────────────────────────────────────────────────────────────

/// Shadow root mode (open or closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShadowRootMode {
    /// Shadow root is accessible via `element.shadowRoot`.
    #[default]
    Open,
    /// Shadow root is not accessible from outside.
    Closed,
}

/// Slot assignment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SlotAssignment {
    /// Named slot assignment (default — matches `slot` attribute on children).
    #[default]
    Named,
    /// Manual slot assignment (requires explicit `slot.assign()` calls).
    Manual,
}

/// Options for `attachShadow`.
#[derive(Debug, Clone, Default)]
pub struct ShadowRootInit {
    pub mode: ShadowRootMode,
    pub delegates_focus: bool,
    pub slot_assignment: SlotAssignment,
}

/// A shadow root attached to a host element.
#[derive(Debug, Clone)]
pub struct ShadowRoot {
    /// The host element this shadow root is attached to.
    pub host: VexId,
    /// The root node ID of the shadow tree.
    pub root: VexId,
    /// Open or closed mode.
    pub mode: ShadowRootMode,
    /// Whether focus delegation is enabled.
    pub delegates_focus: bool,
    /// Slot assignment mode.
    pub slot_assignment: SlotAssignment,
    /// Named slots: slot_name → slot_element_id.
    pub slots: HashMap<String, VexId>,
    /// Active stylesheets within this shadow root.
    pub adopted_stylesheets: Vec<String>,
}

impl ShadowRoot {
    pub fn new(host: VexId, root: VexId, init: ShadowRootInit) -> Self {
        Self {
            host,
            root,
            mode: init.mode,
            delegates_focus: init.delegates_focus,
            slot_assignment: init.slot_assignment,
            slots: HashMap::new(),
            adopted_stylesheets: Vec::new(),
        }
    }

    /// Get the innerHTML of this shadow root (would serialize the shadow tree).
    pub fn inner_html(&self) -> String {
        // Placeholder — real implementation would serialize the shadow tree.
        String::new()
    }

    /// Register a slot element.
    pub fn register_slot(&mut self, name: &str, slot_id: VexId) {
        self.slots.insert(name.to_string(), slot_id);
    }

    /// Remove a slot element.
    pub fn unregister_slot(&mut self, name: &str) {
        self.slots.remove(name);
    }

    /// Get the slot element for a given name.
    pub fn get_slot(&self, name: &str) -> Option<VexId> {
        self.slots.get(name).copied()
    }

    /// Get the default slot (empty name).
    pub fn default_slot(&self) -> Option<VexId> {
        self.slots.get("").copied()
    }

    /// Add an adopted stylesheet.
    pub fn adopt_stylesheet(&mut self, css: String) {
        self.adopted_stylesheets.push(css);
    }
}

/// Error when shadow root attachment fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowError {
    /// Element already has a shadow root.
    AlreadyAttached,
    /// Element type does not support shadow DOM.
    InvalidHost(String),
    /// Host element not found.
    HostNotFound,
}

impl std::fmt::Display for ShadowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttached => write!(f, "Element already has a shadow root"),
            Self::InvalidHost(tag) => write!(f, "Element <{tag}> cannot host shadow DOM"),
            Self::HostNotFound => write!(f, "Host element not found"),
        }
    }
}

// ── ShadowDomManager ─────────────────────────────────────────────────────────

/// Elements that can host a shadow root per the spec.
const VALID_SHADOW_HOSTS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "body",
    "div",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "main",
    "nav",
    "p",
    "section",
    "span",
];

/// Manages shadow DOM state across the document.
#[derive(Debug, Default)]
pub struct ShadowDomManager {
    /// Map of host element ID → shadow root.
    shadows: HashMap<VexId, ShadowRoot>,
    /// Reverse map: shadow root ID → host element ID.
    root_to_host: HashMap<VexId, VexId>,
}

impl ShadowDomManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a shadow root to a host element.
    ///
    /// `tag_name` is the lowercase tag name of the host element.
    /// `shadow_root_id` is the VexId that will serve as the root node of the shadow tree.
    pub fn attach_shadow(
        &mut self,
        host: VexId,
        tag_name: &str,
        shadow_root_id: VexId,
        init: ShadowRootInit,
    ) -> Result<&ShadowRoot, ShadowError> {
        // Check if already attached
        if self.shadows.contains_key(&host) {
            return Err(ShadowError::AlreadyAttached);
        }

        // Validate host element type
        let is_custom_element = tag_name.contains('-');
        if !is_custom_element && !VALID_SHADOW_HOSTS.contains(&tag_name) {
            return Err(ShadowError::InvalidHost(tag_name.to_string()));
        }

        let shadow = ShadowRoot::new(host, shadow_root_id, init);
        self.root_to_host.insert(shadow_root_id, host);
        self.shadows.insert(host, shadow);
        Ok(self.shadows.get(&host).unwrap())
    }

    /// Detach a shadow root from a host element.
    pub fn detach_shadow(&mut self, host: VexId) -> Option<ShadowRoot> {
        if let Some(shadow) = self.shadows.remove(&host) {
            self.root_to_host.remove(&shadow.root);
            Some(shadow)
        } else {
            None
        }
    }

    /// Get the shadow root for a host element (open only).
    pub fn get_shadow_root(&self, host: VexId) -> Option<&ShadowRoot> {
        self.shadows
            .get(&host)
            .filter(|s| s.mode == ShadowRootMode::Open)
    }

    /// Get the shadow root for a host element (any mode — internal use).
    pub fn get_shadow_root_internal(&self, host: VexId) -> Option<&ShadowRoot> {
        self.shadows.get(&host)
    }

    /// Get a mutable reference to the shadow root.
    pub fn get_shadow_root_mut(&mut self, host: VexId) -> Option<&mut ShadowRoot> {
        self.shadows.get_mut(&host)
    }

    /// Check whether a host element has a shadow root.
    pub fn has_shadow(&self, host: VexId) -> bool {
        self.shadows.contains_key(&host)
    }

    /// Find the host element for a shadow root node.
    pub fn host_of(&self, shadow_root_id: VexId) -> Option<VexId> {
        self.root_to_host.get(&shadow_root_id).copied()
    }

    /// Get all shadow hosts.
    pub fn hosts(&self) -> Vec<VexId> {
        self.shadows.keys().copied().collect()
    }

    /// Number of shadow roots.
    pub fn count(&self) -> usize {
        self.shadows.len()
    }

    /// Find the assigned slot for a child element.
    ///
    /// `slot_name` is the child's `slot` attribute value (empty string for default slot).
    /// `host` is the parent (host) element.
    pub fn find_assigned_slot(&self, host: VexId, slot_name: &str) -> Option<VexId> {
        self.shadows.get(&host)?.get_slot(slot_name)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_shadow_open() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        let root = VexId::new(100);

        let shadow = mgr
            .attach_shadow(host, "div", root, ShadowRootInit::default())
            .unwrap();
        assert_eq!(shadow.host, host);
        assert_eq!(shadow.root, root);
        assert_eq!(shadow.mode, ShadowRootMode::Open);
    }

    #[test]
    fn attach_shadow_closed() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        let root = VexId::new(100);

        let init = ShadowRootInit {
            mode: ShadowRootMode::Closed,
            ..Default::default()
        };
        mgr.attach_shadow(host, "div", root, init).unwrap();

        // Closed shadow root not accessible via get_shadow_root
        assert!(mgr.get_shadow_root(host).is_none());
        // But accessible internally
        assert!(mgr.get_shadow_root_internal(host).is_some());
    }

    #[test]
    fn already_attached() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();

        let err = mgr
            .attach_shadow(host, "div", VexId::new(200), ShadowRootInit::default())
            .unwrap_err();
        assert_eq!(err, ShadowError::AlreadyAttached);
    }

    #[test]
    fn invalid_host_element() {
        let mut mgr = ShadowDomManager::new();
        let err = mgr
            .attach_shadow(
                VexId::new(1),
                "input",
                VexId::new(100),
                ShadowRootInit::default(),
            )
            .unwrap_err();
        assert!(matches!(err, ShadowError::InvalidHost(_)));
    }

    #[test]
    fn custom_element_host() {
        let mut mgr = ShadowDomManager::new();
        // Custom elements (contain '-') are always valid hosts
        mgr.attach_shadow(
            VexId::new(1),
            "my-component",
            VexId::new(100),
            ShadowRootInit::default(),
        )
        .unwrap();
        assert!(mgr.has_shadow(VexId::new(1)));
    }

    #[test]
    fn detach_shadow() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();
        assert_eq!(mgr.count(), 1);

        let shadow = mgr.detach_shadow(host).unwrap();
        assert_eq!(shadow.host, host);
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn host_of() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        let root = VexId::new(100);
        mgr.attach_shadow(host, "div", root, ShadowRootInit::default())
            .unwrap();

        assert_eq!(mgr.host_of(root), Some(host));
        assert_eq!(mgr.host_of(VexId::new(999)), None);
    }

    #[test]
    fn slots() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();

        let shadow = mgr.get_shadow_root_mut(host).unwrap();

        // Register default slot
        shadow.register_slot("", VexId::new(10));
        // Register named slot
        shadow.register_slot("header", VexId::new(11));

        assert_eq!(shadow.default_slot(), Some(VexId::new(10)));
        assert_eq!(shadow.get_slot("header"), Some(VexId::new(11)));
        assert_eq!(shadow.get_slot("footer"), None);
    }

    #[test]
    fn slot_unregister() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();

        let shadow = mgr.get_shadow_root_mut(host).unwrap();
        shadow.register_slot("content", VexId::new(10));
        assert!(shadow.get_slot("content").is_some());

        shadow.unregister_slot("content");
        assert!(shadow.get_slot("content").is_none());
    }

    #[test]
    fn find_assigned_slot() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();

        let shadow = mgr.get_shadow_root_mut(host).unwrap();
        shadow.register_slot("", VexId::new(10));
        shadow.register_slot("title", VexId::new(11));

        assert_eq!(mgr.find_assigned_slot(host, ""), Some(VexId::new(10)));
        assert_eq!(mgr.find_assigned_slot(host, "title"), Some(VexId::new(11)));
        assert_eq!(mgr.find_assigned_slot(host, "missing"), None);
    }

    #[test]
    fn adopted_stylesheets() {
        let mut mgr = ShadowDomManager::new();
        let host = VexId::new(1);
        mgr.attach_shadow(host, "div", VexId::new(100), ShadowRootInit::default())
            .unwrap();

        let shadow = mgr.get_shadow_root_mut(host).unwrap();
        shadow.adopt_stylesheet(":host { color: red; }".to_string());
        assert_eq!(shadow.adopted_stylesheets.len(), 1);
    }

    #[test]
    fn delegates_focus() {
        let mut mgr = ShadowDomManager::new();
        let init = ShadowRootInit {
            delegates_focus: true,
            ..Default::default()
        };
        mgr.attach_shadow(VexId::new(1), "div", VexId::new(100), init)
            .unwrap();
        let shadow = mgr.get_shadow_root(VexId::new(1)).unwrap();
        assert!(shadow.delegates_focus);
    }

    #[test]
    fn hosts_list() {
        let mut mgr = ShadowDomManager::new();
        mgr.attach_shadow(
            VexId::new(1),
            "div",
            VexId::new(100),
            ShadowRootInit::default(),
        )
        .unwrap();
        mgr.attach_shadow(
            VexId::new(2),
            "span",
            VexId::new(200),
            ShadowRootInit::default(),
        )
        .unwrap();

        let hosts = mgr.hosts();
        assert_eq!(hosts.len(), 2);
        assert!(hosts.contains(&VexId::new(1)));
        assert!(hosts.contains(&VexId::new(2)));
    }
}
