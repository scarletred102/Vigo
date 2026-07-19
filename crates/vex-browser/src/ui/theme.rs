// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Shared browser chrome palette.
//!
//! A restrained midnight shell makes the browser chrome visually distinct from
//! web content, while the violet-blue accent provides a consistent focus cue.

use vex_core::color::Color;

pub const CHROME_BG: Color = Color::rgb(15, 23, 42);
pub const CHROME_BG_ALT: Color = Color::rgb(20, 31, 54);
pub const CHROME_BORDER: Color = Color::rgb(48, 64, 92);

pub const TAB_ACTIVE_BG: Color = Color::rgb(30, 43, 71);
pub const TAB_INACTIVE_BG: Color = Color::rgb(18, 29, 50);
pub const TAB_TEXT: Color = Color::rgb(241, 245, 249);
pub const TAB_TEXT_INACTIVE: Color = Color::rgb(163, 177, 204);
pub const TAB_ICON: Color = Color::rgb(151, 180, 255);
pub const TAB_NEW_BUTTON: Color = Color::rgb(201, 214, 255);
pub const TAB_ACTIVE_ACCENT: Color = Color::rgb(116, 142, 255);

pub const NAV_BUTTON_BG: Color = Color::rgb(37, 52, 83);
pub const NAV_BUTTON_BG_DISABLED: Color = Color::rgb(28, 40, 64);
pub const NAV_BUTTON_TEXT: Color = Color::rgb(222, 231, 250);
pub const NAV_BUTTON_TEXT_DISABLED: Color = Color::rgb(103, 121, 153);
pub const ADDRESS_BG: Color = Color::rgb(9, 16, 31);
pub const ADDRESS_BORDER: Color = Color::rgb(56, 75, 111);
pub const ADDRESS_BORDER_FOCUSED: Color = Color::rgb(120, 143, 255);
pub const ADDRESS_TEXT: Color = Color::rgb(235, 241, 255);
pub const ADDRESS_PLACEHOLDER: Color = Color::rgb(142, 158, 190);
pub const ADDRESS_SCHEME: Color = Color::rgb(139, 158, 203);
pub const ADDRESS_PATH: Color = Color::rgb(185, 198, 226);
pub const ADDRESS_QUERY: Color = Color::rgb(149, 167, 207);
pub const HTTPS_COLOR: Color = Color::rgb(92, 221, 150);
pub const HTTP_COLOR: Color = Color::rgb(255, 154, 124);
