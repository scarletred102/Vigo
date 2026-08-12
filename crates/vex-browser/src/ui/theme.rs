// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Shared browser chrome palette.
//!
//! A restrained midnight shell makes the browser chrome visually distinct from
//! web content, while the violet-blue accent provides a consistent focus cue.

use vex_core::color::Color;

pub const CHROME_BG: Color = Color::rgb(15, 23, 42);
pub const CHROME_BG_ALT: Color = Color::rgb(30, 41, 59);
pub const CHROME_BORDER: Color = Color::rgb(51, 65, 85);

pub const TAB_ACTIVE_BG: Color = Color::rgb(30, 41, 59);
pub const TAB_INACTIVE_BG: Color = Color::rgb(15, 23, 42);
pub const TAB_TEXT: Color = Color::rgb(248, 250, 252);
pub const TAB_TEXT_INACTIVE: Color = Color::rgb(148, 163, 184);
pub const TAB_ICON: Color = Color::rgb(129, 140, 248);
pub const TAB_NEW_BUTTON: Color = Color::rgb(226, 232, 240);
pub const TAB_ACTIVE_ACCENT: Color = Color::rgb(99, 102, 241);

pub const NAV_BUTTON_BG: Color = Color::rgb(30, 41, 59);
pub const NAV_BUTTON_BG_DISABLED: Color = Color::rgb(15, 23, 42);
pub const NAV_BUTTON_TEXT: Color = Color::rgb(241, 245, 249);
pub const NAV_BUTTON_TEXT_DISABLED: Color = Color::rgb(71, 85, 105);
pub const ADDRESS_BG: Color = Color::rgb(15, 23, 42);
pub const ADDRESS_BORDER: Color = Color::rgb(51, 65, 85);
pub const ADDRESS_BORDER_FOCUSED: Color = Color::rgb(99, 102, 241);
pub const ADDRESS_TEXT: Color = Color::rgb(248, 250, 252);
pub const ADDRESS_PLACEHOLDER: Color = Color::rgb(148, 163, 184);
pub const ADDRESS_SCHEME: Color = Color::rgb(129, 140, 248);
pub const ADDRESS_PATH: Color = Color::rgb(203, 213, 225);
pub const ADDRESS_QUERY: Color = Color::rgb(165, 180, 252);
pub const HTTPS_COLOR: Color = Color::rgb(34, 197, 94);
pub const HTTP_COLOR: Color = Color::rgb(249, 115, 22);
