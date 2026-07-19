// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Navigation bar rendering — address bar, back/forward, reload buttons.

use std::fmt::Write;

use super::theme;
use vex_core::color::Color;
use vex_core::geometry::{Point, Rect};
use vex_render::display_list::{DisplayCommand, DisplayList};

/// Button sizes.
const NAV_BUTTON_SIZE: f32 = 30.0;
const NAV_BUTTON_GAP: f32 = 6.0;
const BUTTON_TOP_OFFSET: f32 = 10.0;
const NAV_LEFT_PADDING: f32 = 12.0;
const NAV_RIGHT_PADDING: f32 = 8.0;
const ADDRESS_SCHEME_ICON_WIDTH: f32 = 16.0;

/// Colors.
const NAV_BAR_BG: Color = theme::CHROME_BG_ALT;
const NAV_BAR_BOTTOM_LINE: Color = theme::CHROME_BORDER;
const BUTTON_BG: Color = theme::NAV_BUTTON_BG;
const BUTTON_TEXT: Color = theme::NAV_BUTTON_TEXT;
const BUTTON_DISABLED: Color = theme::NAV_BUTTON_TEXT_DISABLED;
const BUTTON_ACTIVE: Color = theme::NAV_BUTTON_BG_DISABLED;
const ADDRESS_BAR_BORDER: Color = theme::ADDRESS_BORDER;
const ADDRESS_BAR_BORDER_FOCUSED: Color = theme::ADDRESS_BORDER_FOCUSED;
const ADDRESS_BAR_BG: Color = theme::ADDRESS_BG;
const ADDRESS_TEXT: Color = theme::ADDRESS_TEXT;
const ADDRESS_PLACEHOLDER: Color = theme::ADDRESS_PLACEHOLDER;
const HTTPS_COLOR: Color = theme::HTTPS_COLOR;
const HTTP_COLOR: Color = theme::HTTP_COLOR;
const URL_PATH_COLOR: Color = theme::ADDRESS_PATH;
const URL_SCHEME_COLOR: Color = theme::ADDRESS_SCHEME;
const URL_QUERY_COLOR: Color = theme::ADDRESS_QUERY;

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
    /// Whether the address bar currently has keyboard focus.
    pub is_address_focused: bool,
    /// Pixels reserved on the right side (extension action controls, etc.).
    pub right_inset: f32,
}

/// Render the navigation bar into a display list.
pub fn render_nav_bar(dl: &mut DisplayList, state: &NavBarState<'_>, nav_rect: Rect) {
    // Background.
    dl.push(DisplayCommand::FillRect {
        rect: nav_rect,
        color: NAV_BAR_BG,
        border_radius: 0.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            nav_rect.origin.x,
            nav_rect.origin.y + nav_rect.size.height - 1.0,
            nav_rect.size.width,
            1.0,
        ),
        color: NAV_BAR_BOTTOM_LINE,
        border_radius: 0.0,
    });

    let left = nav_rect.origin.x + NAV_LEFT_PADDING;
    let btn_y = nav_rect.origin.y + BUTTON_TOP_OFFSET;
    let mut x = left;

    // Back button (←).
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: ADDRESS_BAR_BORDER,
        border_radius: 8.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            x + 1.0,
            btn_y + 1.0,
            NAV_BUTTON_SIZE - 2.0,
            NAV_BUTTON_SIZE - 2.0,
        ),
        color: if state.can_go_back {
            BUTTON_BG
        } else {
            BUTTON_ACTIVE
        },
        border_radius: 7.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 9.5, btn_y + 7.5),
        text: "←".into(),
        color: if state.can_go_back {
            BUTTON_TEXT
        } else {
            BUTTON_DISABLED
        },
        font_size: 14.0,
        line_height: 16.0,
    });
    x += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Forward button (→).
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: ADDRESS_BAR_BORDER,
        border_radius: 8.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            x + 1.0,
            btn_y + 1.0,
            NAV_BUTTON_SIZE - 2.0,
            NAV_BUTTON_SIZE - 2.0,
        ),
        color: if state.can_go_forward {
            BUTTON_BG
        } else {
            BUTTON_ACTIVE
        },
        border_radius: 7.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 9.5, btn_y + 7.5),
        text: "→".into(),
        color: if state.can_go_forward {
            BUTTON_TEXT
        } else {
            BUTTON_DISABLED
        },
        font_size: 14.0,
        line_height: 16.0,
    });
    x += NAV_BUTTON_SIZE + NAV_BUTTON_GAP;

    // Reload / stop button.
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(x, btn_y, NAV_BUTTON_SIZE, NAV_BUTTON_SIZE),
        color: ADDRESS_BAR_BORDER,
        border_radius: 8.0,
    });
    dl.push(DisplayCommand::FillRect {
        rect: Rect::new(
            x + 1.0,
            btn_y + 1.0,
            NAV_BUTTON_SIZE - 2.0,
            NAV_BUTTON_SIZE - 2.0,
        ),
        color: if state.is_loading {
            BUTTON_ACTIVE
        } else {
            BUTTON_BG
        },
        border_radius: 7.0,
    });
    dl.push(DisplayCommand::DrawText {
        position: Point::new(x + 8.8, btn_y + 7.5),
        text: if state.is_loading { "✕" } else { "↻" }.into(),
        color: BUTTON_TEXT,
        font_size: 14.0,
        line_height: 16.0,
    });
    // Address bar (outer border + inner fill).
    let address_outer = address_bar_rect_with_right_inset(nav_rect, state.right_inset);
    let address_bar_width = address_outer.size.width;
    dl.push(DisplayCommand::FillRect {
        rect: address_outer,
        color: if state.is_address_focused {
            ADDRESS_BAR_BORDER_FOCUSED
        } else {
            ADDRESS_BAR_BORDER
        },
        border_radius: 10.0,
    });

    let address_inner = Rect::new(
        address_outer.origin.x + 1.0,
        address_outer.origin.y + 1.0,
        (address_outer.size.width - 2.0).max(0.0),
        (address_outer.size.height - 2.0).max(0.0),
    );
    dl.push(DisplayCommand::FillRect {
        rect: address_inner,
        color: ADDRESS_BAR_BG,
        border_radius: 9.0,
    });

    // HTTPS indicator.
    let scheme_color = if state.is_https {
        HTTPS_COLOR
    } else {
        HTTP_COLOR
    };
    // Text symbols are deliberately used instead of emoji: the renderer's
    // system fallback does not guarantee colored emoji glyphs on every host.
    let scheme_text = if state.is_https { "●" } else { "!" };
    dl.push(DisplayCommand::DrawText {
        position: Point::new(address_inner.origin.x + 8.0, btn_y + 7.0),
        text: scheme_text.into(),
        color: scheme_color,
        font_size: 11.0,
        line_height: 16.0,
    });

    // URL / placeholder text.
    let url_x = address_inner.origin.x + 8.0 + ADDRESS_SCHEME_ICON_WIDTH;
    let max_url_width = (address_bar_width - 36.0).max(0.0);
    let display_source = if state.is_address_focused {
        state.url.to_string()
    } else {
        pretty_display_url(state.url)
    };
    let show_placeholder = display_source.trim().is_empty();
    let display_text = if show_placeholder {
        "Search or enter address".to_string()
    } else {
        display_source
    };
    if show_placeholder {
        let url_text = truncate_url(&display_text, max_url_width);
        dl.push(DisplayCommand::DrawText {
            position: Point::new(url_x, btn_y + 7.0),
            text: url_text,
            color: ADDRESS_PLACEHOLDER,
            font_size: 13.0,
            line_height: 16.0,
        });
    } else {
        let styled = styled_url_segments(&display_text);
        let max_chars = (max_url_width / 7.5).max(10.0) as usize;
        let mut cursor_x = url_x;
        let mut consumed = 0usize;
        for (segment, color) in styled {
            if consumed >= max_chars {
                break;
            }
            let remaining = max_chars - consumed;
            let rendered = if segment.chars().count() <= remaining {
                segment
            } else {
                truncate_to_chars(&segment, remaining.saturating_sub(1)) + "…"
            };
            let seg_width = estimate_text_width(&rendered);
            dl.push(DisplayCommand::DrawText {
                position: Point::new(cursor_x, btn_y + 7.0),
                text: rendered.clone(),
                color,
                font_size: 13.0,
                line_height: 16.0,
            });
            cursor_x += seg_width;
            consumed += rendered.chars().count();
            if rendered.ends_with('…') {
                break;
            }
        }
    }
}

/// Compute the address bar rectangle for this nav bar rect.
#[must_use]
pub fn address_bar_rect(nav_rect: Rect) -> Rect {
    address_bar_rect_with_right_inset(nav_rect, 0.0)
}

/// Compute the address bar rectangle while reserving right-side width.
#[must_use]
pub fn address_bar_rect_with_right_inset(nav_rect: Rect, right_inset: f32) -> Rect {
    let left = nav_rect.origin.x + NAV_LEFT_PADDING;
    let btn_y = nav_rect.origin.y + BUTTON_TOP_OFFSET;
    let x = left + (NAV_BUTTON_SIZE + NAV_BUTTON_GAP) * 3.0 + 4.0;
    let right_inset = right_inset.max(0.0);
    let width =
        (nav_rect.size.width - (x - nav_rect.origin.x) - NAV_RIGHT_PADDING - right_inset).max(0.0);
    Rect::new(x, btn_y, width, NAV_BUTTON_SIZE)
}

/// Estimate x-position for text cursor inside the address bar.
#[must_use]
pub fn address_cursor_x(nav_rect: Rect, text: &str) -> f32 {
    address_cursor_x_with_right_inset(nav_rect, text, 0.0)
}

/// Estimate x-position for text cursor inside the address bar with right inset.
#[must_use]
pub fn address_cursor_x_with_right_inset(nav_rect: Rect, text: &str, right_inset: f32) -> f32 {
    let address_rect = address_bar_rect_with_right_inset(nav_rect, right_inset);
    let text_start = address_rect.origin.x + 8.0 + ADDRESS_SCHEME_ICON_WIDTH;
    let content_right = address_rect.origin.x + address_rect.size.width - 10.0;
    (text_start + estimate_text_width(text)).min(content_right)
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

    let left = nav_rect.origin.x + NAV_LEFT_PADDING;
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

    // Address bar region.
    let address = address_bar_rect(nav_rect);
    if x >= address.origin.x
        && x <= address.origin.x + address.size.width
        && y >= address.origin.y
        && y <= address.origin.y + address.size.height
    {
        return NavBarAction::AddressBar;
    }

    // Fallback: right side where the address bar is expected.
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

fn pretty_display_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut out = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string();
    if out.ends_with('/') {
        out.pop();
    }
    out
}

fn styled_url_segments(url: &str) -> Vec<(String, Color)> {
    if url.starts_with("vex://") || url.starts_with("about:") {
        return vec![(url.to_string(), ADDRESS_TEXT)];
    }

    let mut out = Vec::new();
    let mut rest = url;
    if let Some(stripped) = rest.strip_prefix("https://") {
        out.push(("https://".to_string(), URL_SCHEME_COLOR));
        rest = stripped;
    } else if let Some(stripped) = rest.strip_prefix("http://") {
        out.push(("http://".to_string(), URL_SCHEME_COLOR));
        rest = stripped;
    }

    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if host_end > 0 {
        out.push((rest[..host_end].to_string(), ADDRESS_TEXT));
        rest = &rest[host_end..];
    }

    if rest.is_empty() {
        return out;
    }

    let query_start = rest.find('?').unwrap_or(rest.len());
    let hash_start = rest.find('#').unwrap_or(rest.len());
    let path_end = query_start.min(hash_start);

    if path_end > 0 {
        out.push((rest[..path_end].to_string(), URL_PATH_COLOR));
    }

    if query_start < rest.len() {
        let query_end = hash_start.min(rest.len());
        if query_end > query_start {
            out.push((rest[query_start..query_end].to_string(), URL_QUERY_COLOR));
        }
    }

    if hash_start < rest.len() {
        out.push((rest[hash_start..].to_string(), URL_PATH_COLOR));
    }

    out
}

fn truncate_to_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Format a host/path pair into a friendlier title when tab has no title.
#[must_use]
pub fn friendly_title_from_url(url: &str) -> String {
    let pretty = pretty_display_url(url);
    if pretty.is_empty() {
        return "New Tab".to_string();
    }

    if let Some((host, _path)) = pretty.split_once('/') {
        let mut title = host.to_string();
        if title.starts_with("www.") {
            title = title.trim_start_matches("www.").to_string();
        }
        if let Some(dot) = title.find('.') {
            let mut first = title[..dot].to_string();
            if let Some(ch) = first.chars().next() {
                first.replace_range(..ch.len_utf8(), &ch.to_uppercase().to_string());
            }
            return first;
        }
        return title;
    }

    let mut out = String::new();
    let _ = write!(&mut out, "{}", pretty);
    out
}

fn estimate_text_width(text: &str) -> f32 {
    text.chars()
        .map(|ch| if ch == 'i' || ch == 'l' { 4.0 } else { 7.2 })
        .sum()
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
            is_address_focused: false,
            right_inset: 0.0,
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
        let action = hit_test_nav_bar(52.0, 48.0, rect);
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

    #[test]
    fn pretty_display_strips_scheme() {
        assert_eq!(
            pretty_display_url("https://example.com/path"),
            "example.com/path"
        );
        assert_eq!(pretty_display_url("http://example.com/"), "example.com");
    }

    #[test]
    fn address_rect_is_inside_nav_rect() {
        let nav = Rect::new(0.0, 40.0, 1200.0, 46.0);
        let addr = address_bar_rect(nav);
        assert!(addr.origin.x >= nav.origin.x);
        assert!(addr.origin.y >= nav.origin.y);
        assert!(addr.origin.x + addr.size.width <= nav.origin.x + nav.size.width);
    }

    #[test]
    fn cursor_x_clamps_to_address_right_edge() {
        let nav = Rect::new(0.0, 40.0, 360.0, 46.0);
        let x = address_cursor_x(nav, "very long address text that should clamp hard");
        let addr = address_bar_rect(nav);
        assert!(x <= addr.origin.x + addr.size.width);
    }

    #[test]
    fn address_rect_respects_right_inset() {
        let nav = Rect::new(0.0, 40.0, 1200.0, 46.0);
        let no_inset = address_bar_rect_with_right_inset(nav, 0.0);
        let inset = address_bar_rect_with_right_inset(nav, 140.0);
        assert!(inset.size.width < no_inset.size.width);
    }
}
