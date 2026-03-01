// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Zoom — per-tab zoom level management.

/// Supported zoom levels (percentage).
const ZOOM_LEVELS: &[u32] = &[
    25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300,
];

/// Default zoom index (100%).
const DEFAULT_ZOOM_INDEX: usize = 7;

/// Per-tab zoom state.
#[derive(Debug, Clone)]
pub struct ZoomState {
    /// Current index into `ZOOM_LEVELS`.
    level_index: usize,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoomState {
    /// Create a zoom state at 100%.
    pub fn new() -> Self {
        Self {
            level_index: DEFAULT_ZOOM_INDEX,
        }
    }

    /// Current zoom level as a percentage (e.g., 100).
    pub fn percent(&self) -> u32 {
        ZOOM_LEVELS[self.level_index]
    }

    /// Current zoom level as a scale factor (e.g., 1.0).
    pub fn scale(&self) -> f32 {
        self.percent() as f32 / 100.0
    }

    /// Zoom in one step. Returns the new percentage.
    pub fn zoom_in(&mut self) -> u32 {
        if self.level_index < ZOOM_LEVELS.len() - 1 {
            self.level_index += 1;
        }
        self.percent()
    }

    /// Zoom out one step. Returns the new percentage.
    pub fn zoom_out(&mut self) -> u32 {
        if self.level_index > 0 {
            self.level_index -= 1;
        }
        self.percent()
    }

    /// Reset to 100%. Returns the new percentage.
    pub fn reset(&mut self) -> u32 {
        self.level_index = DEFAULT_ZOOM_INDEX;
        self.percent()
    }

    /// Set zoom to a specific percentage (snaps to nearest level).
    pub fn set_percent(&mut self, target: u32) {
        self.level_index = ZOOM_LEVELS
            .iter()
            .enumerate()
            .min_by_key(|(_, &level)| (level as i32 - target as i32).unsigned_abs())
            .map(|(i, _)| i)
            .unwrap_or(DEFAULT_ZOOM_INDEX);
    }

    /// Whether we can zoom in further.
    pub fn can_zoom_in(&self) -> bool {
        self.level_index < ZOOM_LEVELS.len() - 1
    }

    /// Whether we can zoom out further.
    pub fn can_zoom_out(&self) -> bool {
        self.level_index > 0
    }

    /// Whether at default (100%) zoom.
    pub fn is_default(&self) -> bool {
        self.level_index == DEFAULT_ZOOM_INDEX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_100_percent() {
        let z = ZoomState::new();
        assert_eq!(z.percent(), 100);
        assert!((z.scale() - 1.0).abs() < f32::EPSILON);
        assert!(z.is_default());
    }

    #[test]
    fn zoom_in_increases() {
        let mut z = ZoomState::new();
        let p = z.zoom_in();
        assert!(p > 100);
        assert!(!z.is_default());
    }

    #[test]
    fn zoom_out_decreases() {
        let mut z = ZoomState::new();
        let p = z.zoom_out();
        assert!(p < 100);
    }

    #[test]
    fn reset_returns_to_100() {
        let mut z = ZoomState::new();
        z.zoom_in();
        z.zoom_in();
        z.reset();
        assert_eq!(z.percent(), 100);
        assert!(z.is_default());
    }

    #[test]
    fn zoom_in_max_clamps() {
        let mut z = ZoomState::new();
        for _ in 0..50 {
            z.zoom_in();
        }
        assert_eq!(z.percent(), 300);
        assert!(!z.can_zoom_in());
    }

    #[test]
    fn zoom_out_min_clamps() {
        let mut z = ZoomState::new();
        for _ in 0..50 {
            z.zoom_out();
        }
        assert_eq!(z.percent(), 25);
        assert!(!z.can_zoom_out());
    }

    #[test]
    fn set_percent_snaps_to_nearest() {
        let mut z = ZoomState::new();
        z.set_percent(120);
        assert_eq!(z.percent(), 125);
    }

    #[test]
    fn scale_matches_percent() {
        let mut z = ZoomState::new();
        z.zoom_in(); // 110%
        assert!((z.scale() - 1.1).abs() < 0.01);
    }
}
