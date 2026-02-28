// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! RGBA color type with hex and CSS named-color parsing.

use serde::{Deserialize, Serialize};

use crate::VexError;

/// An 8-bit-per-channel RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// Parse `#rgb`, `#rrggbb`, or `#rrggbbaa`.
    pub fn from_hex(s: &str) -> Result<Self, VexError> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            3 => {
                let r = hex_digit(s.as_bytes()[0])?;
                let g = hex_digit(s.as_bytes()[1])?;
                let b = hex_digit(s.as_bytes()[2])?;
                Ok(Self::rgb(r << 4 | r, g << 4 | g, b << 4 | b))
            }
            6 => Ok(Self::rgb(hex_byte(&s[0..2])?, hex_byte(&s[2..4])?, hex_byte(&s[4..6])?)),
            8 => Ok(Self::rgba(
                hex_byte(&s[0..2])?,
                hex_byte(&s[2..4])?,
                hex_byte(&s[4..6])?,
                hex_byte(&s[6..8])?,
            )),
            _ => Err(VexError::Parse(format!("invalid hex color: #{s}"))),
        }
    }

    /// Resolve one of the 17 CSS basic color keywords.
    pub fn from_css_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "black" => Self::rgb(0, 0, 0),
            "silver" => Self::rgb(192, 192, 192),
            "gray" | "grey" => Self::rgb(128, 128, 128),
            "white" => Self::rgb(255, 255, 255),
            "maroon" => Self::rgb(128, 0, 0),
            "red" => Self::rgb(255, 0, 0),
            "purple" => Self::rgb(128, 0, 128),
            "fuchsia" => Self::rgb(255, 0, 255),
            "green" => Self::rgb(0, 128, 0),
            "lime" => Self::rgb(0, 255, 0),
            "olive" => Self::rgb(128, 128, 0),
            "yellow" => Self::rgb(255, 255, 0),
            "navy" => Self::rgb(0, 0, 128),
            "blue" => Self::rgb(0, 0, 255),
            "teal" => Self::rgb(0, 128, 128),
            "aqua" => Self::rgb(0, 255, 255),
            "transparent" => Self::TRANSPARENT,
            _ => return None,
        })
    }

    /// Normalized `[0.0..1.0]` array for GPU uniforms.
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

fn hex_digit(b: u8) -> Result<u8, VexError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(VexError::Parse(format!("invalid hex digit: {}", b as char))),
    }
}

fn hex_byte(s: &str) -> Result<u8, VexError> {
    let hi = hex_digit(s.as_bytes()[0])?;
    let lo = hex_digit(s.as_bytes()[1])?;
    Ok(hi << 4 | lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_rrggbb() {
        assert_eq!(Color::from_hex("#1a1a2e").unwrap(), Color::rgb(26, 26, 46));
    }

    #[test]
    fn hex_rrggbbaa() {
        assert_eq!(
            Color::from_hex("#ff000080").unwrap(),
            Color::rgba(255, 0, 0, 128)
        );
    }

    #[test]
    fn hex_short() {
        assert_eq!(Color::from_hex("#fff").unwrap(), Color::WHITE);
    }

    #[test]
    fn hex_invalid() {
        assert!(Color::from_hex("#zz").is_err());
    }

    #[test]
    fn css_named() {
        assert_eq!(Color::from_css_name("red"), Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn css_transparent() {
        assert_eq!(Color::from_css_name("transparent"), Some(Color::TRANSPARENT));
    }

    #[test]
    fn css_unknown() {
        assert_eq!(Color::from_css_name("not-a-color"), None);
    }

    #[test]
    fn to_f32() {
        let c = Color::rgb(255, 0, 128);
        let f = c.to_f32_array();
        assert!((f[0] - 1.0).abs() < f32::EPSILON);
        assert!((f[1]).abs() < f32::EPSILON);
        assert!((f[2] - 128.0 / 255.0).abs() < 0.001);
        assert!((f[3] - 1.0).abs() < f32::EPSILON);
    }
}
