// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Service Worker lifecycle management.
//!
//! Implements the core Service Worker registration/install/activate/fetch
//! lifecycle per the Service Worker specification (simplified).
//!
//! Each registration tracks:
//! - A scope URL (path prefix it controls)
//! - The script URL
//! - Lifecycle state: installing → installed → activating → activated → redundant
//! - An optional JS context for running the worker script

use std::collections::HashMap;
use std::fmt;

use thiserror::Error;

/// Service worker lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwState {
    /// Script is being parsed/installed.
    Installing,
    /// Install succeeded; waiting for activation.
    Installed,
    /// Activate event is being dispatched.
    Activating,
    /// Worker is active and handling fetch events.
    Activated,
    /// Worker has been superseded or installation failed.
    Redundant,
}

impl fmt::Display for SwState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Installing => write!(f, "installing"),
            Self::Installed => write!(f, "installed"),
            Self::Activating => write!(f, "activating"),
            Self::Activated => write!(f, "activated"),
            Self::Redundant => write!(f, "redundant"),
        }
    }
}

/// Errors from service worker operations.
#[derive(Debug, Error)]
pub enum SwError {
    #[error("registration not found for scope: {0}")]
    NotFound(String),
    #[error("invalid scope: {0}")]
    InvalidScope(String),
    #[error("invalid script URL: {0}")]
    InvalidScript(String),
    #[error("worker failed to install: {0}")]
    InstallFailed(String),
    #[error("security error: {0}")]
    SecurityError(String),
}

pub type SwResult<T> = Result<T, SwError>;

/// Unique ID for a service worker registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegistrationId(u64);

impl RegistrationId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RegistrationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SwReg({})", self.0)
    }
}

/// A single service worker instance.
#[derive(Debug, Clone)]
pub struct ServiceWorker {
    /// Script URL.
    pub script_url: String,
    /// Current state.
    pub state: SwState,
    /// Script content (fetched source).
    pub script_content: Option<String>,
}

/// A service worker registration (scope → worker mapping).
#[derive(Debug, Clone)]
pub struct SwRegistration {
    /// Unique registration ID.
    pub id: RegistrationId,
    /// Scope URL (path prefix).
    pub scope: String,
    /// The currently installing worker (if any).
    pub installing: Option<ServiceWorker>,
    /// The waiting worker (installed, not yet activated).
    pub waiting: Option<ServiceWorker>,
    /// The active worker.
    pub active: Option<ServiceWorker>,
}

impl SwRegistration {
    /// The script URL of the most relevant worker.
    pub fn script_url(&self) -> Option<&str> {
        self.active
            .as_ref()
            .or(self.waiting.as_ref())
            .or(self.installing.as_ref())
            .map(|w| w.script_url.as_str())
    }

    /// Whether there's an active worker handling fetch events.
    pub fn is_active(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|w| w.state == SwState::Activated)
    }
}

/// The navigator.serviceWorker.register() / update / unregister manager.
#[derive(Debug)]
pub struct SwManager {
    registrations: HashMap<RegistrationId, SwRegistration>,
    /// Map from scope → registration ID for quick lookup.
    scope_index: HashMap<String, RegistrationId>,
    next_id: u64,
}

impl Default for SwManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SwManager {
    pub fn new() -> Self {
        Self {
            registrations: HashMap::new(),
            scope_index: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a service worker for a scope.
    ///
    /// If a registration already exists for this scope, starts an update.
    /// Otherwise creates a new registration in "installing" state.
    pub fn register(&mut self, scope: &str, script_url: &str) -> SwResult<RegistrationId> {
        validate_scope(scope)?;
        validate_script_url(script_url)?;

        // If there's an existing registration for this scope, update it.
        if let Some(&existing_id) = self.scope_index.get(scope) {
            if let Some(reg) = self.registrations.get_mut(&existing_id) {
                // If the script URL hasn't changed, do nothing.
                if reg.script_url() == Some(script_url) {
                    return Ok(existing_id);
                }
                // Start installing a new worker.
                reg.installing = Some(ServiceWorker {
                    script_url: script_url.to_owned(),
                    state: SwState::Installing,
                    script_content: None,
                });
                return Ok(existing_id);
            }
        }

        let id = RegistrationId(self.next_id);
        self.next_id += 1;

        let reg = SwRegistration {
            id,
            scope: scope.to_owned(),
            installing: Some(ServiceWorker {
                script_url: script_url.to_owned(),
                state: SwState::Installing,
                script_content: None,
            }),
            waiting: None,
            active: None,
        };

        self.registrations.insert(id, reg);
        self.scope_index.insert(scope.to_owned(), id);

        Ok(id)
    }

    /// Signal that installation succeeded — move worker to "installed" (waiting).
    pub fn finish_install(&mut self, id: RegistrationId) -> SwResult<()> {
        let reg = self
            .registrations
            .get_mut(&id)
            .ok_or_else(|| SwError::NotFound(id.to_string()))?;

        if let Some(mut worker) = reg.installing.take() {
            worker.state = SwState::Installed;
            // Move the old waiting worker to redundant.
            reg.waiting = Some(worker);
        }

        Ok(())
    }

    /// Activate the waiting worker: move it from waiting → active.
    ///
    /// The old active worker (if any) becomes redundant.
    pub fn activate(&mut self, id: RegistrationId) -> SwResult<()> {
        let reg = self
            .registrations
            .get_mut(&id)
            .ok_or_else(|| SwError::NotFound(id.to_string()))?;

        if let Some(mut worker) = reg.waiting.take() {
            worker.state = SwState::Activated;
            // Old active worker becomes redundant (we just drop it).
            reg.active = Some(worker);
        }

        Ok(())
    }

    /// Skip the waiting phase and immediately activate.
    ///
    /// This is called when `skipWaiting()` is invoked inside the SW install handler.
    pub fn skip_waiting(&mut self, id: RegistrationId) -> SwResult<()> {
        self.finish_install(id)?;
        self.activate(id)
    }

    /// Unregister a service worker. Marks it redundant.
    pub fn unregister(&mut self, id: RegistrationId) -> SwResult<()> {
        if let Some(reg) = self.registrations.remove(&id) {
            self.scope_index.remove(&reg.scope);
            Ok(())
        } else {
            Err(SwError::NotFound(id.to_string()))
        }
    }

    /// Find the registration that matches a client URL.
    ///
    /// Uses longest-prefix matching on the scope.
    pub fn match_registration(&self, url: &str) -> Option<&SwRegistration> {
        let mut best: Option<&SwRegistration> = None;
        let mut best_len = 0;

        for reg in self.registrations.values() {
            if url.starts_with(&reg.scope) && reg.scope.len() > best_len {
                best = Some(reg);
                best_len = reg.scope.len();
            }
        }

        best
    }

    /// Get a registration by ID.
    pub fn get(&self, id: RegistrationId) -> Option<&SwRegistration> {
        self.registrations.get(&id)
    }

    /// Get a mutable registration by ID.
    pub fn get_mut(&mut self, id: RegistrationId) -> Option<&mut SwRegistration> {
        self.registrations.get_mut(&id)
    }

    /// All registrations.
    pub fn registrations(&self) -> impl Iterator<Item = &SwRegistration> {
        self.registrations.values()
    }

    /// Number of active registrations.
    pub fn count(&self) -> usize {
        self.registrations.len()
    }

    /// Set the script content for the installing worker.
    pub fn set_script_content(&mut self, id: RegistrationId, content: String) -> SwResult<()> {
        let reg = self
            .registrations
            .get_mut(&id)
            .ok_or_else(|| SwError::NotFound(id.to_string()))?;

        if let Some(ref mut worker) = reg.installing {
            worker.script_content = Some(content);
        } else if let Some(ref mut worker) = reg.waiting {
            worker.script_content = Some(content);
        }

        Ok(())
    }

    /// Mark the installing worker as failed (→ redundant).
    pub fn fail_install(&mut self, id: RegistrationId) -> SwResult<()> {
        let reg = self
            .registrations
            .get_mut(&id)
            .ok_or_else(|| SwError::NotFound(id.to_string()))?;

        if let Some(mut worker) = reg.installing.take() {
            worker.state = SwState::Redundant;
        }

        // If there's no active or waiting worker, remove the registration.
        if reg.active.is_none() && reg.waiting.is_none() {
            let scope = reg.scope.clone();
            self.registrations.remove(&id);
            self.scope_index.remove(&scope);
        }

        Ok(())
    }
}

fn validate_scope(scope: &str) -> SwResult<()> {
    if scope.is_empty() {
        return Err(SwError::InvalidScope("scope cannot be empty".to_string()));
    }
    if !scope.starts_with('/') && !scope.starts_with("http://") && !scope.starts_with("https://") {
        return Err(SwError::InvalidScope(format!(
            "scope must start with / or http(s)://, got: {scope}"
        )));
    }
    Ok(())
}

fn validate_script_url(url: &str) -> SwResult<()> {
    if url.is_empty() {
        return Err(SwError::InvalidScript(
            "script URL cannot be empty".to_string(),
        ));
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_new_worker() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/app/", "/sw.js").unwrap();
        assert_eq!(mgr.count(), 1);

        let reg = mgr.get(id).unwrap();
        assert_eq!(reg.scope, "/app/");
        assert!(reg.installing.is_some());
        assert!(reg.waiting.is_none());
        assert!(reg.active.is_none());
    }

    #[test]
    fn install_and_activate_lifecycle() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/sw.js").unwrap();

        // Installing → installed (waiting).
        mgr.finish_install(id).unwrap();
        let reg = mgr.get(id).unwrap();
        assert!(reg.installing.is_none());
        assert!(reg.waiting.is_some());
        assert_eq!(reg.waiting.as_ref().unwrap().state, SwState::Installed);

        // Waiting → activated.
        mgr.activate(id).unwrap();
        let reg = mgr.get(id).unwrap();
        assert!(reg.waiting.is_none());
        assert!(reg.active.is_some());
        assert_eq!(reg.active.as_ref().unwrap().state, SwState::Activated);
        assert!(reg.is_active());
    }

    #[test]
    fn skip_waiting() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/sw.js").unwrap();
        mgr.skip_waiting(id).unwrap();

        let reg = mgr.get(id).unwrap();
        assert!(reg.installing.is_none());
        assert!(reg.waiting.is_none());
        assert!(reg.is_active());
    }

    #[test]
    fn unregister() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/sw.js").unwrap();
        mgr.unregister(id).unwrap();
        assert_eq!(mgr.count(), 0);
        assert!(mgr.get(id).is_none());
    }

    #[test]
    fn match_registration_longest_prefix() {
        let mut mgr = SwManager::new();
        let id1 = mgr.register("/", "/root-sw.js").unwrap();
        mgr.skip_waiting(id1).unwrap();

        let id2 = mgr.register("/app/", "/app-sw.js").unwrap();
        mgr.skip_waiting(id2).unwrap();

        let id3 = mgr.register("/app/settings/", "/settings-sw.js").unwrap();
        mgr.skip_waiting(id3).unwrap();

        // Exact nested match.
        let reg = mgr.match_registration("/app/settings/theme").unwrap();
        assert_eq!(reg.scope, "/app/settings/");

        // /app/ prefix match.
        let reg = mgr.match_registration("/app/home").unwrap();
        assert_eq!(reg.scope, "/app/");

        // Root match.
        let reg = mgr.match_registration("/other/page").unwrap();
        assert_eq!(reg.scope, "/");
    }

    #[test]
    fn update_existing_registration() {
        let mut mgr = SwManager::new();
        let id1 = mgr.register("/", "/sw-v1.js").unwrap();
        mgr.skip_waiting(id1).unwrap();

        // Re-register with new script → starts installing.
        let id2 = mgr.register("/", "/sw-v2.js").unwrap();
        assert_eq!(id1, id2); // Same registration ID.

        let reg = mgr.get(id2).unwrap();
        // The new worker is installing while the old one stays active.
        assert!(reg.installing.is_some());
        assert_eq!(
            reg.installing.as_ref().unwrap().script_url,
            "/sw-v2.js"
        );
        assert!(reg.is_active()); // Old v1 still active.
    }

    #[test]
    fn fail_install_cleans_up() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/bad-sw.js").unwrap();
        mgr.fail_install(id).unwrap();

        // No active/waiting, so registration is removed.
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn fail_install_keeps_active() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/sw-v1.js").unwrap();
        mgr.skip_waiting(id).unwrap();

        // Start update with bad script.
        mgr.register("/", "/sw-v2.js").unwrap();
        mgr.fail_install(id).unwrap();

        // Old active worker is preserved.
        assert_eq!(mgr.count(), 1);
        assert!(mgr.get(id).unwrap().is_active());
    }

    #[test]
    fn set_script_content() {
        let mut mgr = SwManager::new();
        let id = mgr.register("/", "/sw.js").unwrap();
        mgr.set_script_content(id, "self.addEventListener('fetch', e => {})".to_string())
            .unwrap();

        let reg = mgr.get(id).unwrap();
        assert_eq!(
            reg.installing.as_ref().unwrap().script_content.as_deref(),
            Some("self.addEventListener('fetch', e => {})")
        );
    }

    #[test]
    fn invalid_scope_rejected() {
        let mut mgr = SwManager::new();
        assert!(mgr.register("", "/sw.js").is_err());
        assert!(mgr.register("no-slash", "/sw.js").is_err());
    }

    #[test]
    fn invalid_script_rejected() {
        let mut mgr = SwManager::new();
        assert!(mgr.register("/", "").is_err());
    }

    #[test]
    fn sw_state_display() {
        assert_eq!(SwState::Installing.to_string(), "installing");
        assert_eq!(SwState::Installed.to_string(), "installed");
        assert_eq!(SwState::Activating.to_string(), "activating");
        assert_eq!(SwState::Activated.to_string(), "activated");
        assert_eq!(SwState::Redundant.to_string(), "redundant");
    }

    #[test]
    fn registration_id_display() {
        let id = RegistrationId(42);
        assert_eq!(id.to_string(), "SwReg(42)");
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn registration_script_url_priority() {
        let reg = SwRegistration {
            id: RegistrationId(1),
            scope: "/".to_string(),
            installing: Some(ServiceWorker {
                script_url: "/installing.js".into(),
                state: SwState::Installing,
                script_content: None,
            }),
            waiting: Some(ServiceWorker {
                script_url: "/waiting.js".into(),
                state: SwState::Installed,
                script_content: None,
            }),
            active: Some(ServiceWorker {
                script_url: "/active.js".into(),
                state: SwState::Activated,
                script_content: None,
            }),
        };
        // Active takes priority.
        assert_eq!(reg.script_url(), Some("/active.js"));

        let reg_no_active = SwRegistration {
            id: RegistrationId(2),
            scope: "/".to_string(),
            installing: Some(ServiceWorker {
                script_url: "/installing.js".into(),
                state: SwState::Installing,
                script_content: None,
            }),
            waiting: Some(ServiceWorker {
                script_url: "/waiting.js".into(),
                state: SwState::Installed,
                script_content: None,
            }),
            active: None,
        };
        assert_eq!(reg_no_active.script_url(), Some("/waiting.js"));
    }
}
