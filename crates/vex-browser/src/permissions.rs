// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web Permissions API.
//!
//! Manages permission requests and states for browser features like
//! geolocation, notifications, camera, microphone, clipboard, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Permission Names ─────────────────────────────────────────────────────────

/// Standard permission names from the Permissions API spec.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionName {
    Geolocation,
    Notifications,
    Camera,
    Microphone,
    ClipboardRead,
    ClipboardWrite,
    Push,
    PersistentStorage,
    BackgroundSync,
    Midi,
    ScreenWakeLock,
    /// Non-standard but commonly used permission.
    Custom(String),
}

impl PermissionName {
    pub fn parse(s: &str) -> Self {
        match s {
            "geolocation" => Self::Geolocation,
            "notifications" => Self::Notifications,
            "camera" => Self::Camera,
            "microphone" => Self::Microphone,
            "clipboard-read" => Self::ClipboardRead,
            "clipboard-write" => Self::ClipboardWrite,
            "push" => Self::Push,
            "persistent-storage" => Self::PersistentStorage,
            "background-sync" => Self::BackgroundSync,
            "midi" => Self::Midi,
            "screen-wake-lock" => Self::ScreenWakeLock,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Geolocation => "geolocation",
            Self::Notifications => "notifications",
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::ClipboardRead => "clipboard-read",
            Self::ClipboardWrite => "clipboard-write",
            Self::Push => "push",
            Self::PersistentStorage => "persistent-storage",
            Self::BackgroundSync => "background-sync",
            Self::Midi => "midi",
            Self::ScreenWakeLock => "screen-wake-lock",
            Self::Custom(s) => s.as_str(),
        }
    }
}

// ── Permission State ─────────────────────────────────────────────────────────

/// Permission state (W3C spec states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PermissionState {
    /// The user has granted permission.
    Granted,
    /// The user has denied permission.
    Denied,
    /// No decision yet — should prompt the user.
    #[default]
    Prompt,
}

impl PermissionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Prompt => "prompt",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "granted" => Self::Granted,
            "denied" => Self::Denied,
            _ => Self::Prompt,
        }
    }
}

// ── PermissionPolicy ─────────────────────────────────────────────────────────

/// Controls what the default policy is for unrecognized permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DefaultPolicy {
    /// Deny unknown permissions automatically.
    #[default]
    Deny,
    /// Prompt for unknown permissions.
    Prompt,
}

/// Per-origin permission decisions.
#[derive(Debug, Clone)]
pub struct PermissionDescriptor {
    pub name: PermissionName,
    pub state: PermissionState,
}

// ── PermissionManager ────────────────────────────────────────────────────────

/// Manages permissions per origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionManager {
    /// Map from origin → (permission_name → state).
    permissions: HashMap<String, HashMap<PermissionName, PermissionState>>,
    /// Default policy for unrecognized permissions.
    pub default_policy: DefaultPolicy,
    /// Global overrides that apply to all origins.
    global_overrides: HashMap<PermissionName, PermissionState>,
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            default_policy: DefaultPolicy::Deny,
            global_overrides: HashMap::new(),
        }
    }

    /// Query the permission state for an origin.
    pub fn query(&self, origin: &str, permission: &PermissionName) -> PermissionState {
        // Check global overrides first
        if let Some(state) = self.global_overrides.get(permission) {
            return *state;
        }

        // Check per-origin
        if let Some(origin_map) = self.permissions.get(origin) {
            if let Some(state) = origin_map.get(permission) {
                return *state;
            }
        }

        // Default
        match self.default_policy {
            DefaultPolicy::Deny => PermissionState::Prompt,
            DefaultPolicy::Prompt => PermissionState::Prompt,
        }
    }

    /// Set permission state for an origin.
    pub fn set(&mut self, origin: &str, permission: PermissionName, state: PermissionState) {
        self.permissions
            .entry(origin.to_string())
            .or_default()
            .insert(permission, state);
    }

    /// Set a global override (applies to all origins).
    pub fn set_global(&mut self, permission: PermissionName, state: PermissionState) {
        self.global_overrides.insert(permission, state);
    }

    /// Remove a global override.
    pub fn remove_global(&mut self, permission: &PermissionName) {
        self.global_overrides.remove(permission);
    }

    /// Request a permission. Returns the current or newly set state.
    /// In a real browser, this would show a prompt. Here we simulate it.
    pub fn request(
        &mut self,
        origin: &str,
        permission: PermissionName,
        user_response: Option<PermissionState>,
    ) -> PermissionState {
        let current = self.query(origin, &permission);

        // If already granted or denied, return current state
        if current != PermissionState::Prompt {
            return current;
        }

        // A prompt would be shown; apply the user's response if given
        if let Some(response) = user_response {
            self.set(origin, permission, response);
            response
        } else {
            PermissionState::Prompt
        }
    }

    /// Revoke a permission for an origin (reset to prompt).
    pub fn revoke(&mut self, origin: &str, permission: &PermissionName) {
        if let Some(origin_map) = self.permissions.get_mut(origin) {
            origin_map.remove(permission);
        }
    }

    /// Revoke all permissions for an origin.
    pub fn revoke_all(&mut self, origin: &str) {
        self.permissions.remove(origin);
    }

    /// Get all permissions for an origin.
    pub fn get_origin_permissions(&self, origin: &str) -> Vec<PermissionDescriptor> {
        self.permissions
            .get(origin)
            .map(|map| {
                map.iter()
                    .map(|(name, state)| PermissionDescriptor {
                        name: name.clone(),
                        state: *state,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all origins that have any permission set.
    pub fn origins_with_permissions(&self) -> Vec<&str> {
        self.permissions.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a feature is allowed for an origin.
    /// This is a convenience helper that returns true only for Granted.
    pub fn is_allowed(&self, origin: &str, permission: &PermissionName) -> bool {
        self.query(origin, permission) == PermissionState::Granted
    }

    /// Bulk set permissions for an origin.
    pub fn set_bulk(&mut self, origin: &str, permissions: Vec<(PermissionName, PermissionState)>) {
        let map = self.permissions.entry(origin.to_string()).or_default();
        for (name, state) in permissions {
            map.insert(name, state);
        }
    }

    /// Number of origins with stored permissions.
    pub fn origin_count(&self) -> usize {
        self.permissions.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";
    const OTHER: &str = "https://other.com";

    #[test]
    fn permission_name_roundtrip() {
        let names = vec![
            "geolocation",
            "notifications",
            "camera",
            "microphone",
            "clipboard-read",
            "clipboard-write",
            "push",
            "persistent-storage",
            "background-sync",
            "midi",
            "screen-wake-lock",
        ];
        for name in names {
            let parsed = PermissionName::parse(name);
            assert_eq!(parsed.as_str(), name);
        }
    }

    #[test]
    fn permission_name_custom() {
        let p = PermissionName::parse("my-custom-permission");
        assert_eq!(p.as_str(), "my-custom-permission");
        assert!(matches!(p, PermissionName::Custom(_)));
    }

    #[test]
    fn state_parse_roundtrip() {
        assert_eq!(PermissionState::parse("granted"), PermissionState::Granted);
        assert_eq!(PermissionState::parse("denied"), PermissionState::Denied);
        assert_eq!(PermissionState::parse("prompt"), PermissionState::Prompt);
        assert_eq!(PermissionState::parse("unknown"), PermissionState::Prompt);
    }

    #[test]
    fn query_default() {
        let manager = PermissionManager::new();
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Geolocation),
            PermissionState::Prompt
        );
    }

    #[test]
    fn set_and_query() {
        let mut manager = PermissionManager::new();
        manager.set(
            ORIGIN,
            PermissionName::Notifications,
            PermissionState::Granted,
        );
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Notifications),
            PermissionState::Granted
        );
        assert_eq!(
            manager.query(OTHER, &PermissionName::Notifications),
            PermissionState::Prompt
        );
    }

    #[test]
    fn global_override() {
        let mut manager = PermissionManager::new();
        manager.set(ORIGIN, PermissionName::Camera, PermissionState::Granted);
        manager.set_global(PermissionName::Camera, PermissionState::Denied);

        // Global overrides per-origin
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Camera),
            PermissionState::Denied
        );

        // Remove global → per-origin visible again
        manager.remove_global(&PermissionName::Camera);
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Camera),
            PermissionState::Granted
        );
    }

    #[test]
    fn request_permission() {
        let mut manager = PermissionManager::new();
        // Request with user response
        let state = manager.request(
            ORIGIN,
            PermissionName::Geolocation,
            Some(PermissionState::Granted),
        );
        assert_eq!(state, PermissionState::Granted);

        // Second request — already granted, user response ignored
        let state = manager.request(
            ORIGIN,
            PermissionName::Geolocation,
            Some(PermissionState::Denied),
        );
        assert_eq!(state, PermissionState::Granted);
    }

    #[test]
    fn request_no_response() {
        let mut manager = PermissionManager::new();
        let state = manager.request(ORIGIN, PermissionName::Microphone, None);
        assert_eq!(state, PermissionState::Prompt);
    }

    #[test]
    fn revoke() {
        let mut manager = PermissionManager::new();
        manager.set(
            ORIGIN,
            PermissionName::Notifications,
            PermissionState::Granted,
        );
        manager.revoke(ORIGIN, &PermissionName::Notifications);
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Notifications),
            PermissionState::Prompt
        );
    }

    #[test]
    fn revoke_all() {
        let mut manager = PermissionManager::new();
        manager.set(
            ORIGIN,
            PermissionName::Geolocation,
            PermissionState::Granted,
        );
        manager.set(ORIGIN, PermissionName::Camera, PermissionState::Denied);
        manager.revoke_all(ORIGIN);
        assert_eq!(
            manager.query(ORIGIN, &PermissionName::Geolocation),
            PermissionState::Prompt
        );
    }

    #[test]
    fn is_allowed() {
        let mut manager = PermissionManager::new();
        manager.set(
            ORIGIN,
            PermissionName::Notifications,
            PermissionState::Granted,
        );
        assert!(manager.is_allowed(ORIGIN, &PermissionName::Notifications));
        assert!(!manager.is_allowed(ORIGIN, &PermissionName::Camera));
    }

    #[test]
    fn get_origin_permissions() {
        let mut manager = PermissionManager::new();
        manager.set(
            ORIGIN,
            PermissionName::Geolocation,
            PermissionState::Granted,
        );
        manager.set(ORIGIN, PermissionName::Camera, PermissionState::Denied);

        let perms = manager.get_origin_permissions(ORIGIN);
        assert_eq!(perms.len(), 2);

        let empty = manager.get_origin_permissions(OTHER);
        assert!(empty.is_empty());
    }

    #[test]
    fn origins_with_permissions() {
        let mut manager = PermissionManager::new();
        manager.set(ORIGIN, PermissionName::Push, PermissionState::Granted);
        manager.set(OTHER, PermissionName::Midi, PermissionState::Denied);

        let origins = manager.origins_with_permissions();
        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn bulk_set() {
        let mut manager = PermissionManager::new();
        manager.set_bulk(
            ORIGIN,
            vec![
                (PermissionName::Geolocation, PermissionState::Granted),
                (PermissionName::Camera, PermissionState::Denied),
                (PermissionName::Microphone, PermissionState::Granted),
            ],
        );
        assert!(manager.is_allowed(ORIGIN, &PermissionName::Geolocation));
        assert!(!manager.is_allowed(ORIGIN, &PermissionName::Camera));
        assert!(manager.is_allowed(ORIGIN, &PermissionName::Microphone));
    }

    #[test]
    fn origin_count() {
        let mut manager = PermissionManager::new();
        assert_eq!(manager.origin_count(), 0);
        manager.set(ORIGIN, PermissionName::Push, PermissionState::Granted);
        assert_eq!(manager.origin_count(), 1);
        manager.set(OTHER, PermissionName::Push, PermissionState::Denied);
        assert_eq!(manager.origin_count(), 2);
    }
}
