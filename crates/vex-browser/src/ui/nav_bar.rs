// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Navigation bar rendering — address bar, back/forward, reload buttons.

use vex_core::color::Color;
use vex_core::geometry::{Point, Rect};
use vex_render::display_list::{DisplayCommand, DisplayList};

/// Button sizes.
const NAV_BUTTON_SIZE: f32 = 28.0;
const NAV_BUTTON_GAP: f32 = 4.0;
const BUTTON_TOP_OFFSET: f32 = 6.0;

/// Colors.
const NAV_BAR_BG: Color = Color::rgb(38, 38, 46);
const BUTTON_BG: Color = Color::rgb(52, 52, 64);
const BUTTON_TEXT: Color = Color::rgb(180, 180, 200);
const BUTTON_DISABLED: Color = Color::rgb(80, 80, 90);
const ADDRESS_BAR_BG: Color = Color::rgb(30, 30, 38);
const ADDRESS_TEXT: Color = Color::rgb(200, 200, 220);
const HTTPS_COLOR: Color = Color::rgb(80, 200, 120);
const HTTP_COLOR: Color = Color::rgb(200, 80, 80);

/// State for rendering the navigation bar.
pub struct NavBarState<'a> {
    /// The current URL text to display.
    pub url: &'a str,
    /// Whether the back button is enabled.
    pub can_go_back: bool,
    /// Whether the forward button is enabled.
    pub can_go_forward: bool,
    /// Whether the page is currently loading (shows stop instead of reload).
    pub is_loading: bool,
    /// Whether the URL uses HTTPS.
    pub is_https: bool,
}

/// Render the navigation bar into a display list.
pub fn render_nav_bar(dl: &mut DisplayList, state: &NavBarState<'_>, nav_rect: Rect) {
    // Background.
    dl.push(DisplayCommand::FillRect {
        rect: nav_rect,
        color: NAV_BAR_BG,
        border_radius: 0.0,
    });

    let left = nav_rect.origin.x + 8.0;
    let btn_y = nav_rect.origin.y + BUTTON_TOP_OFFSET;
    let mut x = left;

    // Back button (◀).
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: BUTTON_BG,
        border_radius: 0.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 8.0, btn_y + 6.0),
        text: "◀".into(),
        color: if state.can_go_back {
            BUTTON_TEXT
        } else {
            BUTTON_DISABLED
        },
        font_size: 13.0,
        line_height: 16.0,
    });
    x += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Forward button (▶).
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: BUTTON_BG,
        border_radius: 0.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 8.0, btn_y + 6.0),
        text: "▶".into(),
        color: if state.can_go_forward {
            BUTTON_TEXT
        } else {
            BUTTON_DISABLED
        },
        font_size: 13.0,
        line_height: 16.0,
    });
    x += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Reload / stop button.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: BUTTON_BG,
        border_radius: 0.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 8.0, btn_y + 6.0),
        text: if state.is_loading { "✕" } else { "↻" }.into(),
        color: BUTTON_TEXT,
        font_size: 13.0,
        line_height: 16.0,
    });
    x += NAV_BUTTON_SIZE + NAV_BUTTON_GAP + 4.0;

    // Address bar.
    let address_bar_width = nav_rect.size.width - (x - nav_rect.origin.x) - 8.0;
    let address_bar_height = NAV_BUTTON_SIZE;
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, address_bar_width, address_bar_height),
        color: ADDRESS_BAR_BG,
        border_radius: 0.0,
    });

    // HTTPS indicator.
    let scheme_color = if state.is_https {
        HTTPS_COLOR
    } else {
        HTTP_COLOR
    };
    let scheme_text = if state.is_https { "🔒 " } else { "⚠ " };
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 8.0, btn_y + 6.0),
        text: scheme_text.into(),
        color: scheme_color,
        font_size: 12.0,
        line_height: 16.0,
    });

    // URL text.
    let url_x = x + 26.0;
    let max_url_width = address_bar_width - 36.0;
    let url_text = truncate_url(state.url, max_url_width);
    dl.push(DisplayCommand::DrawText {
        position: Point::new(url_x, btn_y + 6.0),
        text: url_text,
        color: ADDRESS_TEXT,
        font_size: 13.0,
        line_height: 16.0,
    });
}

/// What was clicked in the navigation bar.
#[derive(Debug, Clone, PartialEq)]
pub enum NavBarAction {
    /// Back button clicked.
    Back,
    /// Forward button clicked.
    Forward,
    /// Reload/Stop button clicked.
    ReloadOrStop,
    /// Address bar clicked (should focus for editing).
    AddressBar,
    /// Not a nav bar click.
    None,
}

/// Hit-test a click in the navigation bar.
pub fn hit_test_nav_bar(x: f32, y: f32, nav_rect: Rect) -> NavBarAction {
    if y < nav_rect.origin.y || y > nav_rect.origin.y + nav_rect.size.height {
        return NavBarAction::None;
    }

    let left = nav_rect.origin.x + 8.0;
    let btn_y = nav_rect.origin.y + BUTTON_TOP_OFFSET;
    let mut bx = left;

    // Back button.
    if hit_button(x, y, bx, btn_y) {
        return NavBarAction::Back;
    }
    bx += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Forward button.
    if hit_button(x, y, bx, btn_y) {
        return NavBarAction::Forward;
    }
    bx += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Reload button.
    if hit_button(x, y, bx, btn_y) {
        return NavBarAction::ReloadOrStop;
    }
    bx += NAV_BUTTON_SIZE + NAV_BUTTON_GAP + 4.0;

    // Address bar — everything to the right.
    if x >= bx {
        return NavBarAction::AddressBar;
    }

    NavBarAction::None
}

fn hit_button(x: f32, y: f32, bx: f32, by: f32) -> bool {
    x >= bx && x <= bx + NAV_BUTTON_SIZE && y >= by && y <= by + NAV_BUTTON_SIZE
}

/// Truncate a URL string to fit approximately within pixel width.
fn truncate_url(url: &str, max_width: f32) -> String {
    let max_chars = (max_width / 7.5).max(10.0) as usize;
    if url.len() <= max_chars {
        url.to_string()
    } else {
        format!("{}…", &url[..max_chars.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_nav_bar_produces_commands() {
        let mut dl = DisplayList::new();
        let state = NavBarState {
            url: "https://example.com",
            can_go_back: true,
            can_go_forward: false,
            is_loading: false,
            is_https: true,
        };
        let rect = Rect::new(0.0, 36.0, 1280.0, 40.0);
        render_nav_bar(&mut dl, &state, rect);
        // bg + 3 buttons(bg+text each) + address bg + scheme text + url text >= 10
        assert!(dl.len() >= 10);
    }

    #[test]
    fn hit_test_back_button() {
        let rect = Rect::new(0.0, 36.0, 1280.0, 40.0);
        let action = hit_test_nav_bar(16.0, 48.0, rect);
        assert_eq!(action, NavBarAction::Back);
    }

    #[test]
    fn hit_test_forward_button() {
        let rect = Rect::new(0.0, 36.0, 1280.0, 40.0);
        let action = hit_test_nav_bar(44.0, 48.0, rect);
        assert_eq!(action, NavBarAction::Forward);
    }

    #[test]
    fn hit_test_address_bar() {
        let rect = Rect::new(0.0, 36.0, 1280.0, 40.0);
        let action = hit_test_nav_bar(200.0, 48.0, rect);
        assert_eq!(action, NavBarAction::AddressBar);
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let rect = Rect::new(0.0, 36.0, 1280.0, 40.0);
        let action = hit_test_nav_bar(200.0, 200.0, rect);
        assert_eq!(action, NavBarAction::None);
    }

    #[test]
    fn truncate_long_url() {
        let url = "https://very-long-domain.example.com/some/very/deep/path/to/resource?with=params&more=stuff";
        let truncated = truncate_url(url, 200.0);
        assert!(truncated.len() < url.len());
        assert!(truncated.ends_with('…'));
    }
}
