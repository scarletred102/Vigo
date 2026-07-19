// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Fullscreen API.
//!
//! Manages entering and exiting fullscreen mode for elements,
//! with proper event dispatch and error handling.

use vex_core::VexId;

// ── Types ────────────────────────────────────────────────────────────────────

/// Error returned when fullscreen requests fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullscreenError {
    /// The element is not allowed to go fullscreen.
    NotAllowed,
    /// An element is already fullscreen.
    AlreadyFullscreen,
    /// The element was not found.
    ElementNotFound,
    /// Fullscreen is not supported.
    NotSupported,
}

impl std::fmt::Display for FullscreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed => write!(f, "Fullscreen request not allowed"),
            Self::AlreadyFullscreen => write!(f, "Already in fullscreen mode"),
            Self::ElementNotFound => write!(f, "Element not found"),
            Self::NotSupported => write!(f, "Fullscreen not supported"),
        }
    }
}

/// Fullscreen navigation UI preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FullscreenNavigationUi {
    /// Let the UA decide.
    #[default]
    Auto,
    /// Show navigation UI.
    Show,
    /// Hide navigation UI.
    Hide,
}

/// Options for requesting fullscreen.
#[derive(Debug, Clone, Default)]
pub struct FullscreenOptions {
    pub navigation_ui: FullscreenNavigationUi,
}

/// Callback invoked on fullscreen change/error.
pub type FullscreenCallback = Box<dyn Fn(FullscreenEvent) + Send>;

/// Fullscreen event types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullscreenEvent {
    Change { element: Option<VexId> },
    Error { message: String },
}

// ── FullscreenManager ────────────────────────────────────────────────────────

/// Manages fullscreen state for a document/window.
#[derive(Default)]
pub struct FullscreenManager {
    /// Stack of fullscreen elements (last = topmost).
    stack: Vec<VexId>,
    /// Whether fullscreen is supported.
    supported: bool,
    /// Allowed elements (if empty, all are allowed).
    allowlist: Vec<VexId>,
    /// Callbacks for fullscreen events.
    callbacks: Vec<FullscreenCallback>,
}

impl std::fmt::Debug for FullscreenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullscreenManager")
            .field("stack", &self.stack)
            .field("supported", &self.supported)
            .field("allowlist", &self.allowlist)
            .field("callbacks_count", &self.callbacks.len())
            .finish()
    }
}

impl FullscreenManager {
    pub fn new(supported: bool) -> Self {
        Self {
            supported,
            ..Default::default()
        }
    }

    /// Whether fullscreen is available.
    pub fn fullscreen_enabled(&self) -> bool {
        self.supported
    }

    /// The current fullscreen element, if any.
    pub fn fullscreen_element(&self) -> Option<VexId> {
        self.stack.last().copied()
    }

    /// Whether we are currently in fullscreen.
    pub fn is_fullscreen(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Add an element to the allowlist.
    pub fn allow_element(&mut self, id: VexId) {
        if !self.allowlist.contains(&id) {
            self.allowlist.push(id);
        }
    }

    /// Register a callback for fullscreen events.
    pub fn on_change(&mut self, callback: FullscreenCallback) {
        self.callbacks.push(callback);
    }

    /// Request fullscreen for an element.
    pub fn request_fullscreen(
        &mut self,
        element: VexId,
        _options: FullscreenOptions,
    ) -> Result<(), FullscreenError> {
        if !self.supported {
            self.fire_error("Fullscreen not supported");
            return Err(FullscreenError::NotSupported);
        }

        if !self.allowlist.is_empty() && !self.allowlist.contains(&element) {
            self.fire_error("Element not allowed fullscreen");
            return Err(FullscreenError::NotAllowed);
        }

        if self.stack.last() == Some(&element) {
            return Err(FullscreenError::AlreadyFullscreen);
        }

        self.stack.push(element);
        self.fire_change();
        Ok(())
    }

    /// Exit fullscreen — removes the top element from the stack.
    pub fn exit_fullscreen(&mut self) -> Result<(), FullscreenError> {
        if self.stack.is_empty() {
            return Ok(()); // Not in fullscreen, no-op per spec
        }

        self.stack.pop();
        self.fire_change();
        Ok(())
    }

    /// Fully exit fullscreen — clear the entire stack.
    pub fn fully_exit_fullscreen(&mut self) {
        if !self.stack.is_empty() {
            self.stack.clear();
            self.fire_change();
        }
    }

    fn fire_change(&self) {
        let event = FullscreenEvent::Change {
            element: self.fullscreen_element(),
        };
        for cb in &self.callbacks {
            cb(event.clone());
        }
    }

    fn fire_error(&self, message: &str) {
        let event = FullscreenEvent::Error {
            message: message.to_string(),
        };
        for cb in &self.callbacks {
            cb(event.clone());
        }
    }
}

// ── Screen Orientation ──────────────────────────────────────────────────────

/// Screen orientation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OrientationType {
    #[default]
    PortraitPrimary,
    PortraitSecondary,
    LandscapePrimary,
    LandscapeSecondary,
}

impl OrientationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PortraitPrimary => "portrait-primary",
            Self::PortraitSecondary => "portrait-secondary",
            Self::LandscapePrimary => "landscape-primary",
            Self::LandscapeSecondary => "landscape-secondary",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "portrait-primary" => Some(Self::PortraitPrimary),
            "portrait-secondary" => Some(Self::PortraitSecondary),
            "landscape-primary" => Some(Self::LandscapePrimary),
            "landscape-secondary" => Some(Self::LandscapeSecondary),
            _ => None,
        }
    }

    /// Whether this orientation is portrait.
    pub fn is_portrait(&self) -> bool {
        matches!(self, Self::PortraitPrimary | Self::PortraitSecondary)
    }

    /// Whether this orientation is landscape.
    pub fn is_landscape(&self) -> bool {
        !self.is_portrait()
    }

    /// The angle in degrees for this orientation.
    pub fn angle(&self) -> u16 {
        match self {
            Self::PortraitPrimary => 0,
            Self::LandscapePrimary => 90,
            Self::PortraitSecondary => 180,
            Self::LandscapeSecondary => 270,
        }
    }
}

/// Lock target for screen orientation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrientationLock {
    Any,
    Natural,
    Landscape,
    Portrait,
    PortraitPrimary,
    PortraitSecondary,
    LandscapePrimary,
    LandscapeSecondary,
}

/// Manages screen orientation state.
#[derive(Debug)]
pub struct ScreenOrientation {
    current: OrientationType,
    locked: Option<OrientationLock>,
}

impl Default for ScreenOrientation {
    fn default() -> Self {
        Self::new(OrientationType::LandscapePrimary)
    }
}

impl ScreenOrientation {
    pub fn new(initial: OrientationType) -> Self {
        Self {
            current: initial,
            locked: None,
        }
    }

    pub fn orientation_type(&self) -> OrientationType {
        self.current
    }

    pub fn angle(&self) -> u16 {
        self.current.angle()
    }

    pub fn is_locked(&self) -> bool {
        self.locked.is_some()
    }

    /// Lock the orientation.
    pub fn lock(&mut self, target: OrientationLock) -> Result<(), String> {
        // Check if current orientation is compatible
        let ok = match &target {
            OrientationLock::Any => true,
            OrientationLock::Natural => true,
            OrientationLock::Landscape => self.current.is_landscape(),
            OrientationLock::Portrait => self.current.is_portrait(),
            OrientationLock::LandscapePrimary => self.current == OrientationType::LandscapePrimary,
            OrientationLock::LandscapeSecondary => {
                self.current == OrientationType::LandscapeSecondary
            }
            OrientationLock::PortraitPrimary => self.current == OrientationType::PortraitPrimary,
            OrientationLock::PortraitSecondary => {
                self.current == OrientationType::PortraitSecondary
            }
        };

        if ok || matches!(target, OrientationLock::Any | OrientationLock::Natural) {
            self.locked = Some(target);
            Ok(())
        } else {
            Err("Current orientation incompatible with lock target".to_string())
        }
    }

    /// Unlock the orientation.
    pub fn unlock(&mut self) {
        self.locked = None;
    }

    /// Set the orientation (e.g., from a system event).
    /// Returns true if the orientation changed.
    pub fn set_orientation(&mut self, orientation: OrientationType) -> bool {
        if let Some(ref lock) = self.locked {
            let allowed = match lock {
                OrientationLock::Any | OrientationLock::Natural => true,
                OrientationLock::Landscape => orientation.is_landscape(),
                OrientationLock::Portrait => orientation.is_portrait(),
                OrientationLock::LandscapePrimary => {
                    orientation == OrientationType::LandscapePrimary
                }
                OrientationLock::LandscapeSecondary => {
                    orientation == OrientationType::LandscapeSecondary
                }
                OrientationLock::PortraitPrimary => orientation == OrientationType::PortraitPrimary,
                OrientationLock::PortraitSecondary => {
                    orientation == OrientationType::PortraitSecondary
                }
            };
            if !allowed {
                return false;
            }
        }

        if self.current != orientation {
            self.current = orientation;
            true
        } else {
            false
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fullscreen ──────────────────────────────────────────────

    #[test]
    fn initial_state() {
        let mgr = FullscreenManager::new(true);
        assert!(mgr.fullscreen_enabled());
        assert!(!mgr.is_fullscreen());
        assert_eq!(mgr.fullscreen_element(), None);
    }

    #[test]
    fn request_fullscreen() {
        let mut mgr = FullscreenManager::new(true);
        let elem = VexId::new(1);
        mgr.request_fullscreen(elem, FullscreenOptions::default())
            .unwrap();
        assert!(mgr.is_fullscreen());
        assert_eq!(mgr.fullscreen_element(), Some(elem));
    }

    #[test]
    fn exit_fullscreen() {
        let mut mgr = FullscreenManager::new(true);
        mgr.request_fullscreen(VexId::new(1), FullscreenOptions::default())
            .unwrap();
        mgr.exit_fullscreen().unwrap();
        assert!(!mgr.is_fullscreen());
    }

    #[test]
    fn fullscreen_stack() {
        let mut mgr = FullscreenManager::new(true);
        mgr.request_fullscreen(VexId::new(1), FullscreenOptions::default())
            .unwrap();
        mgr.request_fullscreen(VexId::new(2), FullscreenOptions::default())
            .unwrap();
        assert_eq!(mgr.fullscreen_element(), Some(VexId::new(2)));
        mgr.exit_fullscreen().unwrap();
        assert_eq!(mgr.fullscreen_element(), Some(VexId::new(1)));
    }

    #[test]
    fn fully_exit_fullscreen() {
        let mut mgr = FullscreenManager::new(true);
        mgr.request_fullscreen(VexId::new(1), FullscreenOptions::default())
            .unwrap();
        mgr.request_fullscreen(VexId::new(2), FullscreenOptions::default())
            .unwrap();
        mgr.fully_exit_fullscreen();
        assert!(!mgr.is_fullscreen());
    }

    #[test]
    fn not_supported() {
        let mut mgr = FullscreenManager::new(false);
        let err = mgr
            .request_fullscreen(VexId::new(1), FullscreenOptions::default())
            .unwrap_err();
        assert_eq!(err, FullscreenError::NotSupported);
    }

    #[test]
    fn already_fullscreen() {
        let mut mgr = FullscreenManager::new(true);
        let elem = VexId::new(1);
        mgr.request_fullscreen(elem, FullscreenOptions::default())
            .unwrap();
        let err = mgr
            .request_fullscreen(elem, FullscreenOptions::default())
            .unwrap_err();
        assert_eq!(err, FullscreenError::AlreadyFullscreen);
    }

    #[test]
    fn allowlist() {
        let mut mgr = FullscreenManager::new(true);
        mgr.allow_element(VexId::new(1));

        // Allowed element OK
        mgr.request_fullscreen(VexId::new(1), FullscreenOptions::default())
            .unwrap();
        mgr.exit_fullscreen().unwrap();

        // Not-allowed element fails
        let err = mgr
            .request_fullscreen(VexId::new(2), FullscreenOptions::default())
            .unwrap_err();
        assert_eq!(err, FullscreenError::NotAllowed);
    }

    #[test]
    fn exit_when_not_fullscreen() {
        let mut mgr = FullscreenManager::new(true);
        // No-op, should not error
        mgr.exit_fullscreen().unwrap();
    }

    // ── Screen Orientation ──────────────────────────────────────

    #[test]
    fn orientation_type_properties() {
        assert!(OrientationType::PortraitPrimary.is_portrait());
        assert!(OrientationType::LandscapePrimary.is_landscape());
        assert_eq!(OrientationType::PortraitPrimary.angle(), 0);
        assert_eq!(OrientationType::LandscapePrimary.angle(), 90);
    }

    #[test]
    fn orientation_from_label() {
        assert_eq!(
            OrientationType::from_label("landscape-primary"),
            Some(OrientationType::LandscapePrimary)
        );
        assert_eq!(OrientationType::from_label("invalid"), None);
    }

    #[test]
    fn screen_orientation_initial() {
        let so = ScreenOrientation::new(OrientationType::LandscapePrimary);
        assert_eq!(so.orientation_type(), OrientationType::LandscapePrimary);
        assert_eq!(so.angle(), 90);
        assert!(!so.is_locked());
    }

    #[test]
    fn screen_orientation_lock_unlock() {
        let mut so = ScreenOrientation::new(OrientationType::LandscapePrimary);
        so.lock(OrientationLock::Landscape).unwrap();
        assert!(so.is_locked());

        // Reject portrait while locked to landscape
        assert!(!so.set_orientation(OrientationType::PortraitPrimary));
        assert_eq!(so.orientation_type(), OrientationType::LandscapePrimary);

        // Allow landscape secondary
        assert!(so.set_orientation(OrientationType::LandscapeSecondary));
        assert_eq!(so.orientation_type(), OrientationType::LandscapeSecondary);

        so.unlock();
        assert!(!so.is_locked());
    }

    #[test]
    fn set_orientation_no_change() {
        let mut so = ScreenOrientation::new(OrientationType::PortraitPrimary);
        assert!(!so.set_orientation(OrientationType::PortraitPrimary)); // no change
    }

    #[test]
    fn lock_any_allows_all() {
        let mut so = ScreenOrientation::new(OrientationType::PortraitPrimary);
        so.lock(OrientationLock::Any).unwrap();
        assert!(so.set_orientation(OrientationType::LandscapePrimary));
    }
}
