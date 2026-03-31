// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension permissions — declaration, granting, enforcement.
//!
//! Each extension declares the permissions it needs in its manifest.
//! On first install, the user must approve the permission set.
//! At runtime, API calls are checked against the granted permissions.

use std::collections::HashSet;

/// A single permission that an extension can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Query and create tabs.
    Tabs,
    /// Read/write extension-local storage.
    Storage,
    /// Show desktop notifications.
    Notifications,
    /// Access the currently active tab's DOM.
    ActiveTab,
    /// Observe network requests.
    WebRequest,
    /// Read/write cookies.
    Cookies,
    /// Access browser history.
    History,
    /// Access bookmarks.
    Bookmarks,
}

impl Permission {
    /// Parse a permission from its manifest key string.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "tabs" => Some(Self::Tabs),
            "storage" => Some(Self::Storage),
            "notifications" => Some(Self::Notifications),
            "activeTab" => Some(Self::ActiveTab),
            "webRequest" => Some(Self::WebRequest),
            "cookies" => Some(Self::Cookies),
            "history" => Some(Self::History),
            "bookmarks" => Some(Self::Bookmarks),
            _ => None,
        }
    }

    /// The manifest key string for this permission.
    pub fn key(self) -> &'static str {
        match self {
            Self::Tabs => "tabs",
            Self::Storage => "storage",
            Self::Notifications => "notifications",
            Self::ActiveTab => "activeTab",
            Self::WebRequest => "webRequest",
            Self::Cookies => "cookies",
            Self::History => "history",
            Self::Bookmarks => "bookmarks",
        }
    }

    /// Human-readable description for the permission dialog.
    pub fn description(self) -> &'static str {
        match self {
            Self::Tabs => "Query and create browser tabs",
            Self::Storage => "Store data locally",
            Self::Notifications => "Show desktop notifications",
            Self::ActiveTab => "Access the current tab's content",
            Self::WebRequest => "Observe and modify network requests",
            Self::Cookies => "Read and write cookies",
            Self::History => "Access browsing history",
            Self::Bookmarks => "Access bookmarks",
        }
    }

    /// All known permissions.
    pub const ALL: &'static [Permission] = &[
        Self::Tabs,
        Self::Storage,
        Self::Notifications,
        Self::ActiveTab,
        Self::WebRequest,
        Self::Cookies,
        Self::History,
        Self::Bookmarks,
    ];
}

/// A set of permissions granted to an extension.
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    granted: HashSet<Permission>,
}

impl PermissionSet {
    /// Create an empty permission set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a permission set with the given permissions.
    pub fn from_permissions(perms: &[Permission]) -> Self {
        Self {
            granted: perms.iter().copied().collect(),
        }
    }

    /// Grant a permission.
    pub fn grant(&mut self, perm: Permission) {
        self.granted.insert(perm);
    }

    /// Revoke a permission.
    pub fn revoke(&mut self, perm: Permission) {
        self.granted.remove(&perm);
    }

    /// Check if a permission is granted.
    pub fn has(&self, perm: Permission) -> bool {
        self.granted.contains(&perm)
    }

    /// Check if all requested permissions are granted.
    pub fn has_all(&self, perms: &[Permission]) -> bool {
        perms.iter().all(|p| self.granted.contains(p))
    }

    /// All granted permissions.
    pub fn granted(&self) -> Vec<Permission> {
        self.granted.iter().copied().collect()
    }

    /// Number of granted permissions.
    pub fn count(&self) -> usize {
        self.granted.len()
    }

    /// Grant all requested permissions (auto-approve).
    pub fn grant_all(&mut self, perms: &[Permission]) {
        for &perm in perms {
            self.granted.insert(perm);
        }
    }

    /// Revoke all permissions.
    pub fn revoke_all(&mut self) {
        self.granted.clear();
    }
}

/// Result of a permission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionCheck {
    /// Permission is granted — API call allowed.
    Allowed,
    /// Permission is denied — API call blocked.
    Denied(Permission),
}

/// Check that an extension has permission for a specific API call.
pub fn check_permission(perms: &PermissionSet, required: Permission) -> PermissionCheck {
    if perms.has(required) {
        PermissionCheck::Allowed
    } else {
        PermissionCheck::Denied(required)
    }
}

/// Build a human-readable permission dialog message.
pub fn build_permission_dialog(extension_name: &str, requested: &[Permission]) -> String {
    let mut msg = format!("\"{extension_name}\" requests the following permissions:\n\n");
    for perm in requested {
        msg.push_str(&format!("  • {} — {}\n", perm.key(), perm.description()));
    }
    msg.push_str("\nAllow this extension?");
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_from_key() {
        assert_eq!(Permission::from_key("tabs"), Some(Permission::Tabs));
        assert_eq!(Permission::from_key("storage"), Some(Permission::Storage));
        assert_eq!(Permission::from_key("unknown"), None);
    }

    #[test]
    fn permission_roundtrip() {
        for &perm in Permission::ALL {
            assert_eq!(Permission::from_key(perm.key()), Some(perm));
        }
    }

    #[test]
    fn permission_set_grant_revoke() {
        let mut set = PermissionSet::new();
        assert!(!set.has(Permission::Tabs));

        set.grant(Permission::Tabs);
        assert!(set.has(Permission::Tabs));
        assert_eq!(set.count(), 1);

        set.revoke(Permission::Tabs);
        assert!(!set.has(Permission::Tabs));
    }

    #[test]
    fn permission_set_has_all() {
        let set = PermissionSet::from_permissions(&[Permission::Tabs, Permission::Storage]);
        assert!(set.has_all(&[Permission::Tabs, Permission::Storage]));
        assert!(!set.has_all(&[Permission::Tabs, Permission::Cookies]));
    }

    #[test]
    fn permission_set_grant_all() {
        let mut set = PermissionSet::new();
        set.grant_all(&[Permission::Tabs, Permission::Storage, Permission::History]);
        assert_eq!(set.count(), 3);
        assert!(set.has(Permission::History));
    }

    #[test]
    fn permission_check_allowed() {
        let set = PermissionSet::from_permissions(&[Permission::Tabs]);
        assert_eq!(
            check_permission(&set, Permission::Tabs),
            PermissionCheck::Allowed
        );
    }

    #[test]
    fn permission_check_denied() {
        let set = PermissionSet::new();
        assert_eq!(
            check_permission(&set, Permission::Cookies),
            PermissionCheck::Denied(Permission::Cookies)
        );
    }

    #[test]
    fn permission_dialog_message() {
        let msg = build_permission_dialog("Test Ext", &[Permission::Tabs, Permission::Storage]);
        assert!(msg.contains("Test Ext"));
        assert!(msg.contains("tabs"));
        assert!(msg.contains("storage"));
        assert!(msg.contains("Allow"));
    }

    #[test]
    fn permission_descriptions() {
        for &perm in Permission::ALL {
            assert!(!perm.description().is_empty());
        }
    }
}
