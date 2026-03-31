// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Shared browser chrome palette.
//!
//! Palette direction is derived from the local Servo shell references:
//! `refs/servo-shell-master/servo-shell-master/css/shell.css`
//! (`#ececec`, `#d4d4d4`, `#aaaaaa` family).

use vex_core::color::Color;

pub const CHROME_BG: Color = Color::rgb(236, 236, 236);
pub const CHROME_BG_ALT: Color = Color::rgb(220, 220, 220);
pub const CHROME_BORDER: Color = Color::rgb(170, 170, 170);

pub const TAB_ACTIVE_BG: Color = Color::rgb(250, 250, 250);
pub const TAB_INACTIVE_BG: Color = Color::rgb(216, 216, 216);
pub const TAB_TEXT: Color = Color::rgb(36, 36, 36);
pub const TAB_TEXT_INACTIVE: Color = Color::rgb(98, 98, 98);
pub const TAB_ICON: Color = Color::rgb(106, 106, 106);
pub const TAB_NEW_BUTTON: Color = Color::rgb(80, 80, 80);
pub const TAB_ACTIVE_ACCENT: Color = Color::rgb(166, 166, 166);

pub const NAV_BUTTON_BG: Color = Color::rgb(224, 224, 224);
pub const NAV_BUTTON_BG_DISABLED: Color = Color::rgb(209, 209, 209);
pub const NAV_BUTTON_TEXT: Color = Color::rgb(58, 58, 58);
pub const NAV_BUTTON_TEXT_DISABLED: Color = Color::rgb(140, 140, 140);
pub const ADDRESS_BG: Color = Color::rgb(255, 255, 255);
pub const ADDRESS_BORDER: Color = Color::rgb(177, 177, 177);
pub const ADDRESS_BORDER_FOCUSED: Color = Color::rgb(122, 122, 122);
pub const ADDRESS_TEXT: Color = Color::rgb(42, 42, 42);
pub const ADDRESS_PLACEHOLDER: Color = Color::rgb(128, 128, 128);
pub const ADDRESS_SCHEME: Color = Color::rgb(120, 120, 120);
pub const ADDRESS_PATH: Color = Color::rgb(90, 90, 90);
pub const ADDRESS_QUERY: Color = Color::rgb(74, 74, 74);
pub const HTTPS_COLOR: Color = Color::rgb(44, 130, 71);
pub const HTTP_COLOR: Color = Color::rgb(160, 66, 66);
