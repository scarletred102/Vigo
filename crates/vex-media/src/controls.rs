// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Media player controls UI state.
//!
//! Custom-drawn overlay controls for `<video>` elements: play/pause,
//! seek bar, time display, volume slider, fullscreen button.
//! Controls show on hover and auto-hide after 3 seconds.

use std::time::{Duration, Instant};

use vex_core::geometry::Rect;

/// How long controls stay visible after interaction.
const AUTO_HIDE_DELAY: Duration = Duration::from_secs(3);

/// Height of the controls bar in pixels.
const CONTROLS_HEIGHT: f32 = 40.0;

/// Which control button was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    PlayPause,
    SeekTo(u32),      // position in permille (0–1000)
    VolumeChange(u8), // 0–100
    Mute,
    Fullscreen,
}

/// Layout regions for each control element.
#[derive(Debug, Clone)]
pub struct ControlsLayout {
    /// Full controls bar area.
    pub bar: Rect,
    /// Play/pause button.
    pub play_btn: Rect,
    /// Seek bar track.
    pub seek_bar: Rect,
    /// Current time label.
    pub time_label: Rect,
    /// Volume slider.
    pub volume_slider: Rect,
    /// Mute button.
    pub mute_btn: Rect,
    /// Fullscreen button.
    pub fullscreen_btn: Rect,
}

/// Media controls visible state and interaction.
#[derive(Debug)]
pub struct MediaControls {
    /// Whether controls are currently visible.
    visible: bool,
    /// When the controls were last interacted with.
    last_interaction: Instant,
    /// Whether the user is dragging the seek bar.
    seeking: bool,
    /// Seek position while dragging (0.0–1.0).
    seek_drag_pos: f32,
    /// Whether the user is hovering over the controls.
    hovering: bool,
    /// Current controls layout (computed from video rect).
    layout: Option<ControlsLayout>,
}

impl MediaControls {
    /// Create new hidden controls.
    #[must_use]
    pub fn new() -> Self {
        Self {
            visible: false,
            last_interaction: Instant::now(),
            seeking: false,
            seek_drag_pos: 0.0,
            hovering: false,
            layout: None,
        }
    }

    /// Whether controls should be drawn.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the controls and reset the auto-hide timer.
    pub fn show(&mut self) {
        self.visible = true;
        self.last_interaction = Instant::now();
    }

    /// Hide the controls immediately.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Called each frame — hides controls after the auto-hide delay.
    pub fn tick(&mut self) {
        if self.visible
            && !self.hovering
            && !self.seeking
            && self.last_interaction.elapsed() > AUTO_HIDE_DELAY
        {
            self.visible = false;
        }
    }

    /// Notify that the mouse entered the video area.
    pub fn on_mouse_enter(&mut self) {
        self.hovering = true;
        self.show();
    }

    /// Notify that the mouse left the video area.
    pub fn on_mouse_leave(&mut self) {
        self.hovering = false;
    }

    /// Notify of any user interaction (click, mouse move, etc.).
    pub fn on_interaction(&mut self) {
        self.last_interaction = Instant::now();
        if !self.visible {
            self.show();
        }
    }

    /// Compute the controls layout for a given video rect.
    pub fn compute_layout(&mut self, video_rect: Rect) {
        let bar_y = video_rect.origin.y + video_rect.size.height - CONTROLS_HEIGHT;
        let bar = Rect::new(
            video_rect.origin.x,
            bar_y,
            video_rect.size.width,
            CONTROLS_HEIGHT,
        );

        let btn_size = CONTROLS_HEIGHT - 8.0;
        let padding = 4.0;
        let mut x = bar.origin.x + padding;

        let play_btn = Rect::new(x, bar_y + 4.0, btn_size, btn_size);
        x += btn_size + padding;

        // Time label: ~60px
        let time_label = Rect::new(x, bar_y + 4.0, 60.0, btn_size);
        x += 60.0 + padding;

        // Seek bar: flexible width
        let right_controls_width = 120.0; // volume + mute + fullscreen
        let seek_width =
            (bar.size.width - (x - bar.origin.x) - right_controls_width - padding).max(50.0);
        let seek_bar = Rect::new(x, bar_y + CONTROLS_HEIGHT / 2.0 - 3.0, seek_width, 6.0);
        x += seek_width + padding;

        // Volume slider: ~50px
        let volume_slider = Rect::new(x, bar_y + CONTROLS_HEIGHT / 2.0 - 3.0, 50.0, 6.0);
        x += 50.0 + padding;

        // Mute button
        let mute_btn = Rect::new(x, bar_y + 4.0, btn_size, btn_size);
        x += btn_size + padding;

        // Fullscreen button (at right edge)
        let fullscreen_btn = Rect::new(x, bar_y + 4.0, btn_size, btn_size);

        self.layout = Some(ControlsLayout {
            bar,
            play_btn,
            seek_bar,
            time_label,
            volume_slider,
            mute_btn,
            fullscreen_btn,
        });
    }

    /// Hit-test a click position against the controls.
    /// Returns `None` if the click is outside controls.
    #[must_use]
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ControlAction> {
        let layout = self.layout.as_ref()?;

        if !self.visible {
            return None;
        }

        if contains(layout.play_btn, x, y) {
            return Some(ControlAction::PlayPause);
        }
        if contains(layout.seek_bar, x, y) {
            let pos = ((x - layout.seek_bar.origin.x) / layout.seek_bar.size.width).clamp(0.0, 1.0);
            return Some(ControlAction::SeekTo((pos * 1000.0) as u32));
        }
        if contains(layout.volume_slider, x, y) {
            let vol = ((x - layout.volume_slider.origin.x) / layout.volume_slider.size.width)
                .clamp(0.0, 1.0);
            return Some(ControlAction::VolumeChange((vol * 100.0) as u8));
        }
        if contains(layout.mute_btn, x, y) {
            return Some(ControlAction::Mute);
        }
        if contains(layout.fullscreen_btn, x, y) {
            return Some(ControlAction::Fullscreen);
        }

        None
    }

    /// Get the computed layout, if available.
    #[must_use]
    pub fn layout(&self) -> Option<&ControlsLayout> {
        self.layout.as_ref()
    }

    /// Whether the user is currently dragging the seek bar.
    #[must_use]
    pub fn is_seeking(&self) -> bool {
        self.seeking
    }

    /// Begin seek drag.
    pub fn start_seek_drag(&mut self, pos: f32) {
        self.seeking = true;
        self.seek_drag_pos = pos.clamp(0.0, 1.0);
    }

    /// Update seek drag position.
    pub fn update_seek_drag(&mut self, pos: f32) {
        if self.seeking {
            self.seek_drag_pos = pos.clamp(0.0, 1.0);
        }
    }

    /// End seek drag, returning the final position (0.0–1.0).
    pub fn end_seek_drag(&mut self) -> f32 {
        self.seeking = false;
        self.seek_drag_pos
    }

    /// Format time as "mm:ss".
    #[must_use]
    pub fn format_time(secs: f64) -> String {
        let total_secs = secs.max(0.0) as u64;
        let m = total_secs / 60;
        let s = total_secs % 60;
        format!("{m}:{s:02}")
    }
}

impl Default for MediaControls {
    fn default() -> Self {
        Self::new()
    }
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.origin.x
        && x <= rect.origin.x + rect.size.width
        && y >= rect.origin.y
        && y <= rect.origin.y + rect.size.height
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controls_initially_hidden() {
        let c = MediaControls::new();
        assert!(!c.is_visible());
    }

    #[test]
    fn test_show_hide() {
        let mut c = MediaControls::new();
        c.show();
        assert!(c.is_visible());
        c.hide();
        assert!(!c.is_visible());
    }

    #[test]
    fn test_mouse_enter_shows() {
        let mut c = MediaControls::new();
        c.on_mouse_enter();
        assert!(c.is_visible());
    }

    #[test]
    fn test_format_time() {
        assert_eq!(MediaControls::format_time(0.0), "0:00");
        assert_eq!(MediaControls::format_time(65.0), "1:05");
        assert_eq!(MediaControls::format_time(3661.0), "61:01");
        assert_eq!(MediaControls::format_time(-5.0), "0:00");
    }

    #[test]
    fn test_compute_layout() {
        let mut c = MediaControls::new();
        c.compute_layout(Rect::new(0.0, 0.0, 800.0, 450.0));
        assert!(c.layout().is_some());
        let l = c.layout().unwrap();
        assert!((l.bar.size.width - 800.0).abs() < 0.1);
    }

    #[test]
    fn test_hit_test_play_button() {
        let mut c = MediaControls::new();
        c.show();
        c.compute_layout(Rect::new(0.0, 0.0, 800.0, 450.0));
        let l = c.layout().unwrap();
        let cx = l.play_btn.origin.x + l.play_btn.size.width / 2.0;
        let cy = l.play_btn.origin.y + l.play_btn.size.height / 2.0;
        assert_eq!(c.hit_test(cx, cy), Some(ControlAction::PlayPause));
    }

    #[test]
    fn test_hit_test_outside_returns_none() {
        let mut c = MediaControls::new();
        c.show();
        c.compute_layout(Rect::new(0.0, 0.0, 800.0, 450.0));
        assert_eq!(c.hit_test(-10.0, -10.0), None);
    }

    #[test]
    fn test_seek_drag() {
        let mut c = MediaControls::new();
        assert!(!c.is_seeking());
        c.start_seek_drag(0.5);
        assert!(c.is_seeking());
        c.update_seek_drag(0.75);
        let final_pos = c.end_seek_drag();
        assert!((final_pos - 0.75).abs() < 0.01);
        assert!(!c.is_seeking());
    }
}
