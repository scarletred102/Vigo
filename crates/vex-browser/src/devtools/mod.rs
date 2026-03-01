// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # DevTools
//!
//! Built-in developer tools for the Vex browser engine.
//! Custom-rendered UI — not a web page. Each panel produces
//! [`DisplayCommand`](vex_render::display_list::DisplayCommand) items
//! that are composited into the browser window.
//!
//! Toggle via `F12` or `Ctrl+Shift+I`.

pub mod console;
pub mod elements;
pub mod network;
pub mod performance;
pub mod sources;

use vex_core::geometry::Rect;

/// Which DevTools panel is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevToolsPanel {
    /// DOM tree + computed styles.
    Elements,
    /// Console log + REPL.
    Console,
    /// Network request inspector.
    Network,
    /// Page source viewer.
    Sources,
    /// Frame timing / performance.
    Performance,
}

impl DevToolsPanel {
    /// Display label for the panel tab.
    pub fn label(self) -> &'static str {
        match self {
            Self::Elements => "Elements",
            Self::Console => "Console",
            Self::Network => "Network",
            Self::Sources => "Sources",
            Self::Performance => "Performance",
        }
    }

    /// All panels in tab-bar order.
    pub const ALL: &'static [DevToolsPanel] = &[
        Self::Elements,
        Self::Console,
        Self::Network,
        Self::Sources,
        Self::Performance,
    ];
}

/// How the DevTools panel is docked relative to the content area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPosition {
    /// Docked to the bottom of the content area.
    Bottom,
    /// Docked to the right of the content area.
    Right,
}

/// Height (or width) of the DevTools panel in logical pixels.
pub const DEVTOOLS_DEFAULT_SIZE: f32 = 300.0;

/// Minimum size for the DevTools panel.
pub const DEVTOOLS_MIN_SIZE: f32 = 150.0;

/// Tab bar height inside DevTools.
pub const DEVTOOLS_TAB_HEIGHT: f32 = 30.0;

/// Master DevTools state — tracks open/closed, active panel, dock position.
#[derive(Debug, Clone)]
pub struct DevToolsState {
    /// Whether DevTools are currently open.
    open: bool,
    /// Which panel is active.
    active_panel: DevToolsPanel,
    /// Dock position.
    dock: DockPosition,
    /// Panel size (height if bottom-docked, width if right-docked).
    panel_size: f32,
}

impl Default for DevToolsState {
    fn default() -> Self {
        Self {
            open: false,
            active_panel: DevToolsPanel::Elements,
            dock: DockPosition::Bottom,
            panel_size: DEVTOOLS_DEFAULT_SIZE,
        }
    }
}

impl DevToolsState {
    /// Create a new closed DevTools state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle DevTools open/closed.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Open DevTools (no-op if already open).
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close DevTools.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Whether DevTools are currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The currently active panel.
    pub fn active_panel(&self) -> DevToolsPanel {
        self.active_panel
    }

    /// Switch to a different panel.
    pub fn set_panel(&mut self, panel: DevToolsPanel) {
        self.active_panel = panel;
    }

    /// Current dock position.
    pub fn dock(&self) -> DockPosition {
        self.dock
    }

    /// Change dock position.
    pub fn set_dock(&mut self, dock: DockPosition) {
        self.dock = dock;
    }

    /// Current panel size (height if bottom-docked, width if right-docked).
    pub fn panel_size(&self) -> f32 {
        self.panel_size
    }

    /// Resize the panel (clamped to minimum).
    pub fn set_panel_size(&mut self, size: f32) {
        self.panel_size = size.max(DEVTOOLS_MIN_SIZE);
    }

    /// Compute the layout regions when DevTools are open.
    ///
    /// Returns `(content_rect, devtools_rect)` — the remaining content area
    /// and the DevTools panel area. If DevTools are closed, `devtools_rect`
    /// is zero-sized and `content_rect` is the full available area.
    pub fn layout(&self, available: Rect) -> DevToolsLayout {
        if !self.open {
            return DevToolsLayout {
                content: available,
                devtools: Rect::new(available.origin.x, available.origin.y, 0.0, 0.0),
                tab_bar: Rect::new(0.0, 0.0, 0.0, 0.0),
                panel_body: Rect::new(0.0, 0.0, 0.0, 0.0),
            };
        }

        let size = self.panel_size.min(match self.dock {
            DockPosition::Bottom => (available.size.height * 0.8).max(DEVTOOLS_MIN_SIZE),
            DockPosition::Right => (available.size.width * 0.8).max(DEVTOOLS_MIN_SIZE),
        });

        let (content, devtools) = match self.dock {
            DockPosition::Bottom => {
                let content_h = (available.size.height - size).max(0.0);
                let content = Rect::new(
                    available.origin.x,
                    available.origin.y,
                    available.size.width,
                    content_h,
                );
                let devtools = Rect::new(
                    available.origin.x,
                    available.origin.y + content_h,
                    available.size.width,
                    size,
                );
                (content, devtools)
            }
            DockPosition::Right => {
                let content_w = (available.size.width - size).max(0.0);
                let content = Rect::new(
                    available.origin.x,
                    available.origin.y,
                    content_w,
                    available.size.height,
                );
                let devtools = Rect::new(
                    available.origin.x + content_w,
                    available.origin.y,
                    size,
                    available.size.height,
                );
                (content, devtools)
            }
        };

        let tab_bar = Rect::new(
            devtools.origin.x,
            devtools.origin.y,
            devtools.size.width,
            DEVTOOLS_TAB_HEIGHT,
        );
        let panel_body = Rect::new(
            devtools.origin.x,
            devtools.origin.y + DEVTOOLS_TAB_HEIGHT,
            devtools.size.width,
            (devtools.size.height - DEVTOOLS_TAB_HEIGHT).max(0.0),
        );

        DevToolsLayout {
            content,
            devtools,
            tab_bar,
            panel_body,
        }
    }
}

/// Layout regions for content + DevTools.
#[derive(Debug, Clone)]
pub struct DevToolsLayout {
    /// Remaining content area (page rendering).
    pub content: Rect,
    /// Full DevTools area (includes tab bar + panel body).
    pub devtools: Rect,
    /// DevTools tab bar region.
    pub tab_bar: Rect,
    /// DevTools panel body region (below tab bar).
    pub panel_body: Rect,
}

/// Hit-test result for a click inside the DevTools area.
#[derive(Debug, Clone, PartialEq)]
pub enum DevToolsAction {
    /// Clicked on a panel tab — switch to it.
    SwitchPanel(DevToolsPanel),
    /// Clicked inside the panel body — delegate to the active panel.
    PanelClick { x: f32, y: f32 },
    /// No action.
    None,
}

/// Hit-test a click within the DevTools region.
pub fn hit_test_devtools(
    layout: &DevToolsLayout,
    _state: &DevToolsState,
    x: f32,
    y: f32,
) -> DevToolsAction {
    // Check tab bar.
    let tb = &layout.tab_bar;
    if x >= tb.origin.x
        && x < tb.origin.x + tb.size.width
        && y >= tb.origin.y
        && y < tb.origin.y + tb.size.height
    {
        // Determine which tab was clicked.
        let tab_count = DevToolsPanel::ALL.len() as f32;
        let tab_width = tb.size.width / tab_count;
        let local_x = x - tb.origin.x;
        let idx = (local_x / tab_width) as usize;
        if let Some(&panel) = DevToolsPanel::ALL.get(idx) {
            return DevToolsAction::SwitchPanel(panel);
        }
    }

    // Check panel body.
    let pb = &layout.panel_body;
    if x >= pb.origin.x
        && x < pb.origin.x + pb.size.width
        && y >= pb.origin.y
        && y < pb.origin.y + pb.size.height
    {
        return DevToolsAction::PanelClick {
            x: x - pb.origin.x,
            y: y - pb.origin.y,
        };
    }

    DevToolsAction::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_closed() {
        let state = DevToolsState::new();
        assert!(!state.is_open());
        assert_eq!(state.active_panel(), DevToolsPanel::Elements);
        assert_eq!(state.dock(), DockPosition::Bottom);
    }

    #[test]
    fn toggle_opens_and_closes() {
        let mut state = DevToolsState::new();
        state.toggle();
        assert!(state.is_open());
        state.toggle();
        assert!(!state.is_open());
    }

    #[test]
    fn switch_panel() {
        let mut state = DevToolsState::new();
        state.set_panel(DevToolsPanel::Console);
        assert_eq!(state.active_panel(), DevToolsPanel::Console);
    }

    #[test]
    fn panel_labels() {
        assert_eq!(DevToolsPanel::Elements.label(), "Elements");
        assert_eq!(DevToolsPanel::Console.label(), "Console");
        assert_eq!(DevToolsPanel::Network.label(), "Network");
        assert_eq!(DevToolsPanel::Sources.label(), "Sources");
        assert_eq!(DevToolsPanel::Performance.label(), "Performance");
    }

    #[test]
    fn layout_closed_returns_full_content() {
        let state = DevToolsState::new();
        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);
        assert_eq!(layout.content, available);
        assert_eq!(layout.devtools.size.width, 0.0);
    }

    #[test]
    fn layout_bottom_splits_vertically() {
        let mut state = DevToolsState::new();
        state.open();
        state.set_dock(DockPosition::Bottom);
        state.set_panel_size(300.0);

        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);

        // Content gets remaining height.
        let expected_content_h = 643.0 - 300.0;
        assert!((layout.content.size.height - expected_content_h).abs() < 0.1);
        assert!((layout.devtools.size.height - 300.0).abs() < 0.1);
        assert_eq!(layout.devtools.size.width, 1280.0);
    }

    #[test]
    fn layout_right_splits_horizontally() {
        let mut state = DevToolsState::new();
        state.open();
        state.set_dock(DockPosition::Right);
        state.set_panel_size(400.0);

        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);

        let expected_content_w = 1280.0 - 400.0;
        assert!((layout.content.size.width - expected_content_w).abs() < 0.1);
        assert!((layout.devtools.size.width - 400.0).abs() < 0.1);
        assert_eq!(layout.devtools.size.height, 643.0);
    }

    #[test]
    fn panel_size_clamped_to_minimum() {
        let mut state = DevToolsState::new();
        state.set_panel_size(50.0);
        assert!((state.panel_size() - DEVTOOLS_MIN_SIZE).abs() < 0.1);
    }

    #[test]
    fn hit_test_tab_bar() {
        let mut state = DevToolsState::new();
        state.open();
        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);

        // Click on first tab (Elements).
        let tab_y = layout.tab_bar.origin.y + 5.0;
        let action = hit_test_devtools(&layout, &state, 10.0, tab_y);
        assert_eq!(action, DevToolsAction::SwitchPanel(DevToolsPanel::Elements));

        // Click on second tab (Console) — tab_width = 1280/5 = 256.
        let action = hit_test_devtools(&layout, &state, 260.0, tab_y);
        assert_eq!(action, DevToolsAction::SwitchPanel(DevToolsPanel::Console));
    }

    #[test]
    fn hit_test_panel_body() {
        let mut state = DevToolsState::new();
        state.open();
        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);

        let body_y = layout.panel_body.origin.y + 10.0;
        let action = hit_test_devtools(&layout, &state, 100.0, body_y);
        assert!(matches!(action, DevToolsAction::PanelClick { .. }));
    }

    #[test]
    fn dock_position_switching() {
        let mut state = DevToolsState::new();
        assert_eq!(state.dock(), DockPosition::Bottom);
        state.set_dock(DockPosition::Right);
        assert_eq!(state.dock(), DockPosition::Right);
    }

    #[test]
    fn tab_bar_within_devtools() {
        let mut state = DevToolsState::new();
        state.open();
        let available = Rect::new(0.0, 77.0, 1280.0, 643.0);
        let layout = state.layout(available);

        // Tab bar is at top of devtools region.
        assert!((layout.tab_bar.origin.y - layout.devtools.origin.y).abs() < 0.1);
        assert!((layout.tab_bar.size.height - DEVTOOLS_TAB_HEIGHT).abs() < 0.1);
    }
}
