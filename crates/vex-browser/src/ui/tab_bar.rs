// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tab bar rendering — draws the tab strip at the top of the browser window.

use vex_core::color::Color;
use vex_core::geometry::{Point, Rect};
use vex_render::display_list::{DisplayCommand, DisplayList};

use crate::tab::Tab;

use super::nav_bar::friendly_title_from_url;
use super::theme;

/// Colors for the tab bar.
const TAB_BAR_BG: Color = theme::CHROME_BG;
const ACTIVE_TAB_BG: Color = theme::TAB_ACTIVE_BG;
const INACTIVE_TAB_BG: Color = theme::TAB_INACTIVE_BG;
const TAB_TEXT_COLOR: Color = theme::TAB_TEXT;
const INACTIVE_TAB_TEXT: Color = theme::TAB_TEXT_INACTIVE;
const CLOSE_BTN_COLOR: Color = theme::TAB_ICON;
const NEW_TAB_BTN_COLOR: Color = theme::TAB_NEW_BUTTON;
const TAB_ACTIVE_ACCENT: Color = theme::TAB_ACTIVE_ACCENT;
const TAB_BAR_BOTTOM_LINE: Color = theme::CHROME_BORDER;

/// Maximum tab width in pixels.
const MAX_TAB_WIDTH: f32 = 250.0;
/// Minimum tab width for a visible tab.
const MIN_TAB_WIDTH: f32 = 120.0;
/// Absolute minimum width allowed under extreme tab counts.
const ABS_MIN_TAB_WIDTH: f32 = 56.0;
/// Padding between tabs.
const TAB_GAP: f32 = 7.0;
/// Tab height.
const TAB_HEIGHT: f32 = 36.0;
/// Top offset for tabs within the tab bar.
const TAB_TOP: f32 = 6.0;
/// Width of the new-tab (+) button.
const NEW_TAB_BTN_WIDTH: f32 = 32.0;
/// Left/right padding inside the tab strip.
const TAB_STRIP_PADDING: f32 = 4.0;

/// Render the tab bar into a display list.
pub fn render_tab_bar(dl: &mut DisplayList, tabs: &[Tab], active_index: usize, tab_bar_rect: Rect) {
    // Background.
    dl.push(DisplayCommand::FillRect {
        rect: tab_bar_rect,
        color: TAB_BAR_BG,
        border_radius: 0.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            tab_bar_rect.origin.x,
            tab_bar_rect.origin.y + tab_bar_rect.size.height - 1.0,
            tab_bar_rect.size.width,
            1.0,
        ),
        color: TAB_BAR_BOTTOM_LINE,
        border_radius: 0.0,
    });

    let geom = tab_strip_geometry(tab_bar_rect, tabs.len(), active_index);

    for (slot, tab_index) in (geom.start_index..geom.end_index).enumerate() {
        let tab = &tabs[tab_index];
        let is_active = tab_index == active_index;
        let x =
            tab_bar_rect.origin.x + TAB_STRIP_PADDING + (geom.tab_width + TAB_GAP) * slot as f32;
        let y = tab_bar_rect.origin.y + TAB_TOP;
        let tab_rect = Rect::new(x, y, geom.tab_width, TAB_HEIGHT);

        // Tab shell.
        dl.push(DisplayCommand::FillRect {
            rect: tab_rect,
            color: if is_active {
                theme::ADDRESS_BORDER_FOCUSED
            } else {
                theme::CHROME_BORDER
            },
            border_radius: 9.0,
        });

        // Tab background.
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(
                x + 1.0,
                y + 1.0,
                (geom.tab_width - 2.0).max(0.0),
                TAB_HEIGHT - 2.0,
            ),
            color: if is_active {
                ACTIVE_TAB_BG
            } else {
                INACTIVE_TAB_BG
            },
            border_radius: 8.0,
        });

        // Active tab accent.
        if is_active {
            dl.push(DisplayCommand::FillRect {
                rect: Rect::new(
                    x + 10.0,
                    y + TAB_HEIGHT - 4.0,
                    (geom.tab_width - 20.0).max(0.0),
                    2.0,
                ),
                color: TAB_ACTIVE_ACCENT,
                border_radius: 1.0,
            });
        }

        // Favicon placeholder dot.
        dl.push(DisplayCommand::DrawText {
            position: Point::new(x + 8.0, y + 8.0),
            text: "•".into(),
            color: if is_active {
                theme::TAB_ICON
            } else {
                Color::rgb(111, 130, 168)
            },
            font_size: 12.0,
            line_height: 14.0,
        });

        // Tab title (truncated).
        let max_text_width = geom.tab_width - 46.0; // favicon + close button + padding
        let raw_title = if tab.title.trim().is_empty() || tab.title == "New Tab" {
            friendly_title_from_url(tab.url.as_ref())
        } else {
            tab.title.clone()
        };
        let title = truncate_title(&raw_title, max_text_width);
        dl.push(DisplayCommand::DrawText {
            position: Point::new(x + 18.0, y + 8.3),
            text: title,
            color: if is_active {
                TAB_TEXT_COLOR
            } else {
                INACTIVE_TAB_TEXT
            },
            font_size: if is_active { 12.5 } else { 12.0 },
            line_height: 16.0,
        });

        // Close button chip.
        let close_x = x + geom.tab_width - 20.0;
        let close_y = y + 7.0;
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(close_x, close_y, 14.0, 14.0),
            color: if is_active {
                Color::rgb(48, 65, 102)
            } else {
                Color::rgb(34, 48, 77)
            },
            border_radius: 7.0,
        });

        // Close button (×).
        dl.push(DisplayCommand::DrawText {
            position: Point::new(x + geom.tab_width - 16.0, y + 6.8),
            text: "×".into(),
            color: CLOSE_BTN_COLOR,
            font_size: 12.0,
            line_height: 18.0,
        });
    }

    // New tab (+) button.
    let plus_x = geom.plus_x;
    let plus_y = tab_bar_rect.origin.y + TAB_TOP;
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(plus_x, plus_y, NEW_TAB_BTN_WIDTH, TAB_HEIGHT),
        color: theme::ADDRESS_BORDER_FOCUSED,
        border_radius: 8.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            plus_x + 1.0,
            plus_y + 1.0,
            NEW_TAB_BTN_WIDTH - 2.0,
            TAB_HEIGHT - 2.0,
        ),
        color: Color::rgb(36, 51, 83),
        border_radius: 7.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(plus_x + 10.0, plus_y + 6.8),
        text: "+".into(),
        color: NEW_TAB_BTN_COLOR,
        font_size: 15.0,
        line_height: 18.0,
    });
}

/// Hit-test a click position against tab bar elements.
/// Returns the action to take.
#[derive(Debug, Clone, PartialEq)]
pub enum TabBarAction {
    /// Switch to this tab index.
    SwitchTab(usize),
    /// Close this tab index.
    CloseTab(usize),
    /// Create a new tab.
    NewTab,
    /// Not in the tab bar.
    None,
}

/// Determine what was clicked in the tab bar.
pub fn hit_test_tab_bar(
    x: f32,
    y: f32,
    tab_count: usize,
    active_index: usize,
    tab_bar_rect: Rect,
) -> TabBarAction {
    // Check if click is within the tab bar.
    if y < tab_bar_rect.origin.y || y > tab_bar_rect.origin.y + tab_bar_rect.size.height {
        return TabBarAction::None;
    }

    let geom = tab_strip_geometry(tab_bar_rect, tab_count, active_index);

    for (slot, tab_index) in (geom.start_index..geom.end_index).enumerate() {
        let tab_x =
            tab_bar_rect.origin.x + TAB_STRIP_PADDING + (geom.tab_width + TAB_GAP) * slot as f32;
        let tab_y = tab_bar_rect.origin.y + TAB_TOP;

        if x >= tab_x && x <= tab_x + geom.tab_width && y >= tab_y && y <= tab_y + TAB_HEIGHT {
            // Check if close button was clicked (rightmost 20px).
            if x >= tab_x + geom.tab_width - 20.0 {
                return TabBarAction::CloseTab(tab_index);
            }
            return TabBarAction::SwitchTab(tab_index);
        }
    }

    // Check new-tab button.
    if x >= geom.plus_x
        && x <= geom.plus_x + NEW_TAB_BTN_WIDTH
        && y >= tab_bar_rect.origin.y + TAB_TOP
        && y <= tab_bar_rect.origin.y + TAB_TOP + TAB_HEIGHT
    {
        return TabBarAction::NewTab;
    }

    TabBarAction::None
}

#[derive(Debug, Clone, Copy)]
struct TabStripGeometry {
    start_index: usize,
    end_index: usize,
    tab_width: f32,
    plus_x: f32,
}

fn tab_strip_geometry(
    tab_bar_rect: Rect,
    tab_count: usize,
    active_index: usize,
) -> TabStripGeometry {
    let total = tab_count.max(1);
    let tabs_area_width =
        (tab_bar_rect.size.width - TAB_STRIP_PADDING * 2.0 - NEW_TAB_BTN_WIDTH - TAB_GAP).max(0.0);

    let max_visible = ((tabs_area_width + TAB_GAP) / (MIN_TAB_WIDTH + TAB_GAP)).floor() as usize;
    let visible = max_visible.max(1).min(total);

    let mut start = active_index.saturating_add(1).saturating_sub(visible);
    if start + visible > total {
        start = total.saturating_sub(visible);
    }
    let end = (start + visible).min(total);
    let visible_count = (end - start).max(1) as f32;

    let raw_width = (tabs_area_width - TAB_GAP * (visible_count - 1.0)).max(0.0) / visible_count;
    let tab_width = raw_width.clamp(ABS_MIN_TAB_WIDTH, MAX_TAB_WIDTH);

    let plus_x = tab_bar_rect.origin.x + TAB_STRIP_PADDING + (tab_width + TAB_GAP) * visible_count;

    TabStripGeometry {
        start_index: start,
        end_index: end,
        tab_width,
        plus_x,
    }
}

/// Truncate a title to approximately fit within `max_width` pixels.
/// Uses a rough estimate of 7 pixels per character.
fn truncate_title(title: &str, max_width: f32) -> String {
    let max_chars = (max_width / 7.0).max(3.0) as usize;
    if title.len() <= max_chars {
        title.to_string()
    } else {
        format!("{}…", &title[..max_chars.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_tab_bar_produces_commands() {
        let mut dl = DisplayList::new();
        let tab = Tab::blank(crate::TabId::new(1));
        let rect = Rect::new(0.0, 0.0, 1280.0, 36.0);
        render_tab_bar(&mut dl, &[tab], 0, rect);
        // Should have: bg + tab bg + tab text + close btn + plus bg + plus text >= 6
        assert!(dl.len() >= 6);
    }

    #[test]
    fn hit_test_switch_tab() {
        let rect = Rect::new(0.0, 0.0, 1280.0, 36.0);
        let action = hit_test_tab_bar(20.0, 15.0, 2, 0, rect);
        assert_eq!(action, TabBarAction::SwitchTab(0));
    }

    #[test]
    fn hit_test_new_tab_button() {
        let rect = Rect::new(0.0, 0.0, 1280.0, 36.0);
        let action = hit_test_tab_bar(280.0, 8.0, 1, 0, rect);
        assert_eq!(action, TabBarAction::NewTab);
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let rect = Rect::new(0.0, 0.0, 1280.0, 36.0);
        let action = hit_test_tab_bar(50.0, 100.0, 1, 0, rect);
        assert_eq!(action, TabBarAction::None);
    }

    #[test]
    fn truncate_long_title() {
        let title = "This is a very long page title that should be truncated";
        let truncated = truncate_title(title, 100.0);
        assert!(truncated.len() < title.len());
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn geometry_keeps_plus_button_inside_bar() {
        let rect = Rect::new(0.0, 0.0, 1024.0, 40.0);
        let geom = tab_strip_geometry(rect, 24, 12);
        assert!(geom.plus_x + NEW_TAB_BTN_WIDTH <= rect.origin.x + rect.size.width + 0.1);
    }

    #[test]
    fn geometry_keeps_active_tab_visible_windowed() {
        let rect = Rect::new(0.0, 0.0, 900.0, 40.0);
        let geom = tab_strip_geometry(rect, 30, 22);
        assert!(22 >= geom.start_index && 22 < geom.end_index);
    }
}
