// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Context menu — right-click menu model and hit testing.

use vex_core::color::Color;
use vex_core::geometry::{Point, Rect};
use vex_render::display_list::{DisplayCommand, DisplayList};

/// A context menu positioned at a screen location.
#[derive(Debug, Clone)]
pub struct ContextMenu {
    /// Screen position where the menu should appear.
    pub position: Point,
    /// Menu items.
    pub items: Vec<MenuItem>,
    /// Whether the menu is currently visible.
    pub visible: bool,
}

/// A single item in the context menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Display label.
    pub label: String,
    /// Action to take when clicked.
    pub action: MenuAction,
    /// Whether the item is enabled.
    pub enabled: bool,
    /// Whether this is a separator line.
    pub separator: bool,
}

/// Actions a context menu can trigger.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    /// No action (separator).
    None,
    /// Navigate back.
    Back,
    /// Navigate forward.
    Forward,
    /// Reload the page.
    Reload,
    /// Copy the selected text.
    Copy,
    /// Select all text.
    SelectAll,
    /// Open link in new tab.
    OpenLinkInNewTab,
    /// Copy link URL.
    CopyLinkUrl,
    /// Save image as.
    SaveImageAs,
    /// Copy image.
    CopyImage,
    /// View page source.
    ViewSource,
    /// Inspect element (dev tools).
    InspectElement,
}

/// Menu item height and width constants.
const ITEM_HEIGHT: f32 = 28.0;
const SEPARATOR_HEIGHT: f32 = 8.0;
const MENU_WIDTH: f32 = 180.0;
const MENU_PADDING: f32 = 4.0;

/// Colors.
const MENU_BG: Color = Color::rgb(42, 42, 52);
const MENU_BORDER: Color = Color::rgb(60, 60, 75);
// ITEM_HOVER_BG will be used when hover state tracking is implemented.
#[allow(dead_code)]
const ITEM_HOVER_BG: Color = Color::rgb(55, 55, 70);
const ITEM_TEXT: Color = Color::rgb(210, 210, 225);
const DISABLED_TEXT: Color = Color::rgb(90, 90, 110);
const SEPARATOR_COLOR: Color = Color::rgb(55, 55, 65);

impl ContextMenu {
    /// Create a page context menu (right-click on page background).
    pub fn page_menu(position: Point, can_go_back: bool, can_go_forward: bool) -> Self {
        Self {
            position,
            visible: true,
            items: vec![
                MenuItem::action("Back", MenuAction::Back, can_go_back),
                MenuItem::action("Forward", MenuAction::Forward, can_go_forward),
                MenuItem::action("Reload", MenuAction::Reload, true),
                MenuItem::separator(),
                MenuItem::action("Select All", MenuAction::SelectAll, true),
                MenuItem::action("Copy", MenuAction::Copy, false),
                MenuItem::separator(),
                MenuItem::action("View Page Source", MenuAction::ViewSource, true),
                MenuItem::action("Inspect", MenuAction::InspectElement, true),
            ],
        }
    }

    /// Create a link context menu (right-click on link).
    pub fn link_menu(position: Point) -> Self {
        Self {
            position,
            visible: true,
            items: vec![
                MenuItem::action("Open Link in New Tab", MenuAction::OpenLinkInNewTab, true),
                MenuItem::action("Copy Link Address", MenuAction::CopyLinkUrl, true),
                MenuItem::separator(),
                MenuItem::action("Inspect", MenuAction::InspectElement, true),
            ],
        }
    }

    /// Create an image context menu (right-click on image).
    pub fn image_menu(position: Point) -> Self {
        Self {
            position,
            visible: true,
            items: vec![
                MenuItem::action("Save Image As…", MenuAction::SaveImageAs, true),
                MenuItem::action("Copy Image", MenuAction::CopyImage, true),
                MenuItem::separator(),
                MenuItem::action("Inspect", MenuAction::InspectElement, true),
            ],
        }
    }

    /// Dismiss the menu.
    pub fn dismiss(&mut self) {
        self.visible = false;
    }

    /// Calculate the total menu height.
    pub fn menu_height(&self) -> f32 {
        MENU_PADDING * 2.0
            + self
                .items
                .iter()
                .map(|item| {
                    if item.separator {
                        SEPARATOR_HEIGHT
                    } else {
                        ITEM_HEIGHT
                    }
                })
                .sum::<f32>()
    }

    /// Render the context menu into a display list.
    pub fn render(&self, dl: &mut DisplayList) {
        if !self.visible {
            return;
        }

        let menu_rect = Rect::new(
            self.position.x,
            self.position.y,
            MENU_WIDTH,
            self.menu_height(),
        );

        // Border.
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(
                menu_rect.origin.x - 1.0,
                menu_rect.origin.y - 1.0,
                menu_rect.size.width + 2.0,
                menu_rect.size.height + 2.0,
            ),
            color: MENU_BORDER,
        });

        // Background.
        dl.push(DisplayCommand::FillRect {
            rect: menu_rect,
            color: MENU_BG,
        });

        let mut y = self.position.y + MENU_PADDING;
        for item in &self.items {
            if item.separator {
                dl.push(DisplayCommand::FillRect {
                    rect: Rect::new(
                        self.position.x + 8.0,
                        y + SEPARATOR_HEIGHT / 2.0 - 0.5,
                        MENU_WIDTH - 16.0,
                        1.0,
                    ),
                    color: SEPARATOR_COLOR,
                });
                y += SEPARATOR_HEIGHT;
            } else {
                dl.push(DisplayCommand::DrawText {
                    position: Point::new(self.position.x + 12.0, y + 6.0),
                    text: item.label.clone(),
                    color: if item.enabled {
                        ITEM_TEXT
                    } else {
                        DISABLED_TEXT
                    },
                    font_size: 12.0,
                    line_height: 16.0,
                });
                y += ITEM_HEIGHT;
            }
        }
    }

    /// Hit-test a click against the menu. Returns the action if an enabled item was clicked.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<MenuAction> {
        if !self.visible {
            return None;
        }

        let menu_rect = Rect::new(
            self.position.x,
            self.position.y,
            MENU_WIDTH,
            self.menu_height(),
        );

        if x < menu_rect.origin.x
            || x > menu_rect.origin.x + menu_rect.size.width
            || y < menu_rect.origin.y
            || y > menu_rect.origin.y + menu_rect.size.height
        {
            return None;
        }

        let mut item_y = self.position.y + MENU_PADDING;
        for item in &self.items {
            let height = if item.separator {
                SEPARATOR_HEIGHT
            } else {
                ITEM_HEIGHT
            };

            if y >= item_y && y < item_y + height && !item.separator && item.enabled {
                return Some(item.action.clone());
            }
            item_y += height;
        }

        None
    }
}

impl MenuItem {
    fn action(label: &str, action: MenuAction, enabled: bool) -> Self {
        Self {
            label: label.to_string(),
            action,
            enabled,
            separator: false,
        }
    }

    fn separator() -> Self {
        Self {
            label: String::new(),
            action: MenuAction::None,
            enabled: false,
            separator: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_menu_has_items() {
        let menu = ContextMenu::page_menu(Point::new(100.0, 100.0), true, false);
        assert!(menu.visible);
        assert!(!menu.items.is_empty());
        assert!(menu.items.len() >= 8);
    }

    #[test]
    fn link_menu_has_items() {
        let menu = ContextMenu::link_menu(Point::new(0.0, 0.0));
        assert!(menu.visible);
        assert!(menu.items.len() >= 3);
    }

    #[test]
    fn dismiss_hides_menu() {
        let mut menu = ContextMenu::page_menu(Point::new(100.0, 100.0), true, true);
        assert!(menu.visible);
        menu.dismiss();
        assert!(!menu.visible);
    }

    #[test]
    fn hit_test_enabled_item() {
        let menu = ContextMenu::page_menu(Point::new(0.0, 0.0), true, false);
        // First item "Back" at y ~ 4 to 32.
        let action = menu.hit_test(50.0, 15.0);
        assert_eq!(action, Some(MenuAction::Back));
    }

    #[test]
    fn hit_test_disabled_item() {
        let menu = ContextMenu::page_menu(Point::new(0.0, 0.0), true, false);
        // "Forward" is disabled (can_go_forward = false), at y ~ 32 to 60.
        let action = menu.hit_test(50.0, 40.0);
        assert_eq!(action, None);
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let menu = ContextMenu::page_menu(Point::new(100.0, 100.0), true, true);
        let action = menu.hit_test(0.0, 0.0);
        assert_eq!(action, None);
    }

    #[test]
    fn hidden_menu_renders_nothing() {
        let mut menu = ContextMenu::page_menu(Point::new(0.0, 0.0), true, true);
        menu.dismiss();
        let mut dl = DisplayList::new();
        menu.render(&mut dl);
        assert_eq!(dl.len(), 0);
    }

    #[test]
    fn render_produces_commands() {
        let menu = ContextMenu::page_menu(Point::new(50.0, 50.0), true, true);
        let mut dl = DisplayList::new();
        menu.render(&mut dl);
        // border bg + menu bg + text items >= 3
        assert!(dl.len() >= 3);
    }
}
