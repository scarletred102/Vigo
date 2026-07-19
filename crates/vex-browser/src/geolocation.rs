// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Geolocation API.
//!
//! Provides position data (latitude, longitude, altitude, accuracy)
//! with permission gating and error handling.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Position ─────────────────────────────────────────────────────────────────

/// Geographic coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Coordinates {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Altitude in meters (optional).
    pub altitude: Option<f64>,
    /// Accuracy of latitude/longitude in meters.
    pub accuracy: f64,
    /// Accuracy of altitude in meters (optional).
    pub altitude_accuracy: Option<f64>,
    /// Heading in degrees clockwise from true north (optional).
    pub heading: Option<f64>,
    /// Speed in meters per second (optional).
    pub speed: Option<f64>,
}

impl Default for Coordinates {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            altitude: None,
            accuracy: 0.0,
            altitude_accuracy: None,
            heading: None,
            speed: None,
        }
    }
}

/// A geolocation position with coordinates and timestamp.
#[derive(Debug, Clone)]
pub struct GeoPosition {
    pub coords: Coordinates,
    /// Timestamp in milliseconds since epoch.
    pub timestamp: f64,
}

impl GeoPosition {
    pub fn new(coords: Coordinates, timestamp: f64) -> Self {
        Self { coords, timestamp }
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Geolocation error codes per the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeoErrorCode {
    /// The user denied the permission request.
    PermissionDenied = 1,
    /// The position could not be determined.
    PositionUnavailable = 2,
    /// The request timed out.
    Timeout = 3,
}

/// A geolocation error.
#[derive(Debug, Clone)]
pub struct GeoError {
    pub code: GeoErrorCode,
    pub message: String,
}

impl GeoError {
    pub fn permission_denied() -> Self {
        Self {
            code: GeoErrorCode::PermissionDenied,
            message: "User denied Geolocation permission".to_string(),
        }
    }

    pub fn position_unavailable() -> Self {
        Self {
            code: GeoErrorCode::PositionUnavailable,
            message: "Position unavailable".to_string(),
        }
    }

    pub fn timeout() -> Self {
        Self {
            code: GeoErrorCode::Timeout,
            message: "Geolocation request timed out".to_string(),
        }
    }
}

impl std::fmt::Display for GeoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeoError({}): {}", self.code as u8, self.message)
    }
}

// ── Options ──────────────────────────────────────────────────────────────────

/// Options for getCurrentPosition / watchPosition.
#[derive(Debug, Clone)]
pub struct PositionOptions {
    /// Whether to use high-accuracy mode.
    pub enable_high_accuracy: bool,
    /// Maximum age of a cached position in ms (0 = never cache).
    pub maximum_age: u64,
    /// Timeout in ms.
    pub timeout: u64,
}

impl Default for PositionOptions {
    fn default() -> Self {
        Self {
            enable_high_accuracy: false,
            maximum_age: 0,
            timeout: u64::MAX,
        }
    }
}

// ── WatchId ──────────────────────────────────────────────────────────────────

/// Unique ID for a position watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchId(u64);

static NEXT_WATCH_ID: AtomicU64 = AtomicU64::new(1);

impl WatchId {
    pub fn new() -> Self {
        Self(NEXT_WATCH_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for WatchId {
    fn default() -> Self {
        Self::new()
    }
}

// ── GeolocationService ──────────────────────────────────────────────────────

/// Permission state for geolocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeoPermission {
    Granted,
    Denied,
    #[default]
    Prompt,
}

/// Active watcher entry.
#[derive(Debug)]
struct Watcher {
    #[allow(dead_code)]
    options: PositionOptions,
}

/// The Geolocation service (one per browsing context).
#[derive(Debug)]
pub struct GeolocationService {
    /// Per-origin permission.
    permissions: HashMap<String, GeoPermission>,
    /// Active watchers.
    watchers: HashMap<WatchId, Watcher>,
    /// Last known position (simulated or from OS).
    pub last_position: Option<GeoPosition>,
    /// Simulated position for testing.
    pub simulated_position: Option<GeoPosition>,
}

impl Default for GeolocationService {
    fn default() -> Self {
        Self::new()
    }
}

impl GeolocationService {
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            watchers: HashMap::new(),
            last_position: None,
            simulated_position: None,
        }
    }

    /// Set the permission state for an origin.
    pub fn set_permission(&mut self, origin: &str, permission: GeoPermission) {
        self.permissions.insert(origin.to_string(), permission);
    }

    /// Get the permission state for an origin.
    pub fn get_permission(&self, origin: &str) -> GeoPermission {
        self.permissions.get(origin).copied().unwrap_or_default()
    }

    /// Get the current position (respecting permission).
    pub fn get_current_position(
        &mut self,
        origin: &str,
        _options: &PositionOptions,
    ) -> Result<GeoPosition, GeoError> {
        match self.get_permission(origin) {
            GeoPermission::Denied => return Err(GeoError::permission_denied()),
            GeoPermission::Prompt => return Err(GeoError::permission_denied()),
            GeoPermission::Granted => {}
        }

        if let Some(pos) = &self.simulated_position {
            let pos = pos.clone();
            self.last_position = Some(pos.clone());
            return Ok(pos);
        }

        Err(GeoError::position_unavailable())
    }

    /// Start watching position changes.
    pub fn watch_position(
        &mut self,
        origin: &str,
        options: PositionOptions,
    ) -> Result<WatchId, GeoError> {
        match self.get_permission(origin) {
            GeoPermission::Denied | GeoPermission::Prompt => {
                return Err(GeoError::permission_denied());
            }
            GeoPermission::Granted => {}
        }

        let id = WatchId::new();
        self.watchers.insert(id, Watcher { options });
        Ok(id)
    }

    /// Stop watching a position.
    pub fn clear_watch(&mut self, id: WatchId) -> bool {
        self.watchers.remove(&id).is_some()
    }

    /// Number of active watchers.
    pub fn watcher_count(&self) -> usize {
        self.watchers.len()
    }

    /// Set a simulated position (for testing).
    pub fn set_simulated_position(&mut self, coords: Coordinates, timestamp: f64) {
        self.simulated_position = Some(GeoPosition::new(coords, timestamp));
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.com";

    fn make_coords() -> Coordinates {
        Coordinates {
            latitude: 37.7749,
            longitude: -122.4194,
            accuracy: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn permission_denied_by_default() {
        let mut geo = GeolocationService::new();
        let result = geo.get_current_position(ORIGIN, &PositionOptions::default());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, GeoErrorCode::PermissionDenied);
    }

    #[test]
    fn get_position_granted() {
        let mut geo = GeolocationService::new();
        geo.set_permission(ORIGIN, GeoPermission::Granted);
        geo.set_simulated_position(make_coords(), 1000.0);

        let pos = geo
            .get_current_position(ORIGIN, &PositionOptions::default())
            .unwrap();
        assert!((pos.coords.latitude - 37.7749).abs() < 0.001);
        assert!((pos.coords.longitude - (-122.4194)).abs() < 0.001);
    }

    #[test]
    fn position_unavailable_no_sim() {
        let mut geo = GeolocationService::new();
        geo.set_permission(ORIGIN, GeoPermission::Granted);
        let result = geo.get_current_position(ORIGIN, &PositionOptions::default());
        assert_eq!(result.unwrap_err().code, GeoErrorCode::PositionUnavailable);
    }

    #[test]
    fn watch_position() {
        let mut geo = GeolocationService::new();
        geo.set_permission(ORIGIN, GeoPermission::Granted);
        let id = geo
            .watch_position(ORIGIN, PositionOptions::default())
            .unwrap();
        assert_eq!(geo.watcher_count(), 1);
        assert!(geo.clear_watch(id));
        assert_eq!(geo.watcher_count(), 0);
    }

    #[test]
    fn watch_denied() {
        let mut geo = GeolocationService::new();
        let result = geo.watch_position(ORIGIN, PositionOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn clear_nonexistent_watch() {
        let mut geo = GeolocationService::new();
        assert!(!geo.clear_watch(WatchId::new()));
    }

    #[test]
    fn per_origin_permissions() {
        let mut geo = GeolocationService::new();
        geo.set_permission("https://a.com", GeoPermission::Granted);
        geo.set_permission("https://b.com", GeoPermission::Denied);
        assert_eq!(geo.get_permission("https://a.com"), GeoPermission::Granted);
        assert_eq!(geo.get_permission("https://b.com"), GeoPermission::Denied);
        assert_eq!(geo.get_permission("https://c.com"), GeoPermission::Prompt);
    }

    #[test]
    fn last_position_saved() {
        let mut geo = GeolocationService::new();
        geo.set_permission(ORIGIN, GeoPermission::Granted);
        geo.set_simulated_position(make_coords(), 500.0);
        let _ = geo.get_current_position(ORIGIN, &PositionOptions::default());
        assert!(geo.last_position.is_some());
    }

    #[test]
    fn geo_error_display() {
        let e = GeoError::permission_denied();
        let s = format!("{e}");
        assert!(s.contains("denied"));
    }

    #[test]
    fn coordinates_default() {
        let c = Coordinates::default();
        assert_eq!(c.latitude, 0.0);
        assert_eq!(c.longitude, 0.0);
        assert!(c.altitude.is_none());
    }
}
