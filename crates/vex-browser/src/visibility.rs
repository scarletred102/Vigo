// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Page Visibility API.
//!
//! Tracks the visibility state of a document (visible, hidden)
//! and fires visibility change events.

// ── VisibilityState ──────────────────────────────────────────────────────────

/// The visibility state of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VisibilityState {
    /// The page is visible (tab is focused or foreground).
    #[default]
    Visible,
    /// The page is hidden (tab is minimized or background).
    Hidden,
}

impl VisibilityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Visible => "visible",
            Self::Hidden => "hidden",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }
}

// ── FullscreenState ──────────────────────────────────────────────────────────

/// Fullscreen API state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullscreenError {
    /// The element is not allowed to enter fullscreen.
    NotAllowed,
    /// The element is not connected to a document.
    NotConnected,
    /// Already in fullscreen.
    AlreadyFullscreen,
    /// Not in fullscreen (can't exit).
    NotInFullscreen,
}

impl std::fmt::Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed => write!(f, "Fullscreen not allowed"),
            Self::NotConnected => write!(f, "Element not connected to document"),
            Self::AlreadyFullscreen => write!(f, "Already in fullscreen"),
            Self::NotInFullscreen => write!(f, "Not in fullscreen"),
        }
    }
}

// ── VisibilityManager ────────────────────────────────────────────────────────

/// Manages page visibility state and fullscreen mode.
#[derive(Debug)]
pub struct VisibilityManager {
    pub state: VisibilityState,
    pub fullscreen: bool,
    /// Element ID currently in fullscreen, if any.
    pub fullscreen_element: Option<u64>,
    /// Whether fullscreen is allowed for the current context.
    pub fullscreen_allowed: bool,
    /// History of visibility changes: (state, timestamp_ms).
    changes: Vec<(VisibilityState, f64)>,
    /// Total time spent hidden (ms).
    pub total_hidden_time: f64,
    /// Timestamp when last became hidden.
    last_hidden_at: Option<f64>,
}

impl Default for VisibilityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VisibilityManager {
    pub fn new() -> Self {
        Self {
            state: VisibilityState::Visible,
            fullscreen: false,
            fullscreen_element: None,
            fullscreen_allowed: true,
            changes: Vec::new(),
            total_hidden_time: 0.0,
            last_hidden_at: None,
        }
    }

    /// Set the page visibility state (e.g., when tab is switched).
    /// Returns `true` if the state actually changed.
    pub fn set_visibility(&mut self, state: VisibilityState, timestamp: f64) -> bool {
        if self.state == state {
            return false;
        }

        // Track hidden time
        match state {
            VisibilityState::Hidden => {
                self.last_hidden_at = Some(timestamp);
            }
            VisibilityState::Visible => {
                if let Some(hidden_at) = self.last_hidden_at.take() {
                    self.total_hidden_time += timestamp - hidden_at;
                }
            }
        }

        self.state = state;
        self.changes.push((state, timestamp));
        true
    }

    /// Whether the document is currently hidden.
    pub fn is_hidden(&self) -> bool {
        self.state == VisibilityState::Hidden
    }

    /// Request fullscreen for an element.
    pub fn request_fullscreen(&mut self, element_id: u64) -> Result<(), FullscreenError> {
        if !self.fullscreen_allowed {
            return Err(FullscreenError::NotAllowed);
        }
        if self.fullscreen {
            return Err(FullscreenError::AlreadyFullscreen);
        }
        self.fullscreen = true;
        self.fullscreen_element = Some(element_id);
        Ok(())
    }

    /// Exit fullscreen mode.
    pub fn exit_fullscreen(&mut self) -> Result<(), FullscreenError> {
        if !self.fullscreen {
            return Err(FullscreenError::NotInFullscreen);
        }
        self.fullscreen = false;
        self.fullscreen_element = None;
        Ok(())
    }

    /// Number of visibility changes recorded.
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// Get the visibility change history.
    pub fn changes(&self) -> &[(VisibilityState, f64)] {
        &self.changes
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_visible() {
        let vm = VisibilityManager::new();
        assert_eq!(vm.state, VisibilityState::Visible);
        assert!(!vm.is_hidden());
    }

    #[test]
    fn visibility_change_to_hidden() {
        let mut vm = VisibilityManager::new();
        let changed = vm.set_visibility(VisibilityState::Hidden, 100.0);
        assert!(changed);
        assert!(vm.is_hidden());
        assert_eq!(vm.state.as_str(), "hidden");
    }

    #[test]
    fn no_change_same_state() {
        let mut vm = VisibilityManager::new();
        let changed = vm.set_visibility(VisibilityState::Visible, 100.0);
        assert!(!changed);
    }

    #[test]
    fn hidden_time_tracking() {
        let mut vm = VisibilityManager::new();
        vm.set_visibility(VisibilityState::Hidden, 100.0);
        vm.set_visibility(VisibilityState::Visible, 300.0);
        assert!((vm.total_hidden_time - 200.0).abs() < 0.001);
    }

    #[test]
    fn multiple_visibility_changes() {
        let mut vm = VisibilityManager::new();
        vm.set_visibility(VisibilityState::Hidden, 100.0);
        vm.set_visibility(VisibilityState::Visible, 200.0);
        vm.set_visibility(VisibilityState::Hidden, 500.0);
        vm.set_visibility(VisibilityState::Visible, 700.0);
        assert_eq!(vm.change_count(), 4);
        assert!((vm.total_hidden_time - 300.0).abs() < 0.001); // 100 + 200
    }

    #[test]
    fn visibility_state_parse() {
        assert_eq!(
            VisibilityState::from_name("visible"),
            Some(VisibilityState::Visible)
        );
        assert_eq!(
            VisibilityState::from_name("hidden"),
            Some(VisibilityState::Hidden)
        );
        assert_eq!(VisibilityState::from_name("invalid"), None);
    }

    #[test]
    fn request_fullscreen() {
        let mut vm = VisibilityManager::new();
        assert!(vm.request_fullscreen(42).is_ok());
        assert!(vm.fullscreen);
        assert_eq!(vm.fullscreen_element, Some(42));
    }

    #[test]
    fn exit_fullscreen() {
        let mut vm = VisibilityManager::new();
        vm.request_fullscreen(42).unwrap();
        assert!(vm.exit_fullscreen().is_ok());
        assert!(!vm.fullscreen);
        assert_eq!(vm.fullscreen_element, None);
    }

    #[test]
    fn double_fullscreen_error() {
        let mut vm = VisibilityManager::new();
        vm.request_fullscreen(1).unwrap();
        assert_eq!(
            vm.request_fullscreen(2),
            Err(FullscreenError::AlreadyFullscreen)
        );
    }

    #[test]
    fn exit_when_not_fullscreen() {
        let mut vm = VisibilityManager::new();
        assert_eq!(vm.exit_fullscreen(), Err(FullscreenError::NotInFullscreen));
    }

    #[test]
    fn fullscreen_not_allowed() {
        let mut vm = VisibilityManager::new();
        vm.fullscreen_allowed = false;
        assert_eq!(vm.request_fullscreen(42), Err(FullscreenError::NotAllowed));
    }

    #[test]
    fn change_history() {
        let mut vm = VisibilityManager::new();
        vm.set_visibility(VisibilityState::Hidden, 50.0);
        vm.set_visibility(VisibilityState::Visible, 100.0);
        let history = vm.changes();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], (VisibilityState::Hidden, 50.0));
        assert_eq!(history[1], (VisibilityState::Visible, 100.0));
    }
}
