// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS length values with unit resolution.

use vex_core::Size;

/// A CSS length value that can be in different units.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LengthValue {
    Px(f32),
    Em(f32),
    Rem(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
    Auto,
    #[default]
    Zero,
}

impl LengthValue {
    /// Resolve this length to an absolute pixel value.
    ///
    /// - `font_size`: the computed font-size of the current element (for `em`)
    /// - `root_font_size`: the computed font-size of the root element (for `rem`)
    /// - `viewport`: viewport dimensions (for `vw`/`vh`)
    /// - `containing`: the relevant containing dimension (for `%`)
    pub fn resolve(
        &self,
        font_size: f32,
        root_font_size: f32,
        viewport: Size,
        containing: f32,
    ) -> f32 {
        match self {
            Self::Px(v) => *v,
            Self::Em(v) => v * font_size,
            Self::Rem(v) => v * root_font_size,
            Self::Percent(v) => v / 100.0 * containing,
            Self::Vw(v) => v / 100.0 * viewport.width,
            Self::Vh(v) => v / 100.0 * viewport.height,
            Self::Auto => 0.0,
            Self::Zero => 0.0,
        }
    }

    /// Returns true if this is `Auto`.
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Returns true if this is a zero-like value.
    pub fn is_zero(&self) -> bool {
        matches!(self, Self::Zero | Self::Px(0.0))
    }
}



/// Parse a CSS length string like "10px", "2em", "50%", "auto".
pub fn parse_length(input: &str) -> Option<LengthValue> {
    let input = input.trim();
    if input == "auto" {
        return Some(LengthValue::Auto);
    }
    if input == "0" {
        return Some(LengthValue::Zero);
    }

    // Try each unit suffix
    for (suffix, constructor) in &[
        ("px", LengthValue::Px as fn(f32) -> LengthValue),
        ("em", LengthValue::Em),
        ("rem", LengthValue::Rem),
        ("vw", LengthValue::Vw),
        ("vh", LengthValue::Vh),
    ] {
        if let Some(num_str) = input.strip_suffix(suffix) {
            if let Ok(v) = num_str.trim().parse::<f32>() {
                return Some(constructor(v));
            }
        }
    }

    // Percent
    if let Some(num_str) = input.strip_suffix('%') {
        if let Ok(v) = num_str.trim().parse::<f32>() {
            return Some(LengthValue::Percent(v));
        }
    }

    // Bare number → pixels
    if let Ok(v) = input.parse::<f32>() {
        return Some(LengthValue::Px(v));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_px() {
        let v = LengthValue::Px(16.0);
        assert_eq!(v.resolve(12.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 16.0);
    }

    #[test]
    fn resolve_em() {
        let v = LengthValue::Em(2.0);
        assert_eq!(v.resolve(14.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 28.0);
    }

    #[test]
    fn resolve_rem() {
        let v = LengthValue::Rem(1.5);
        assert_eq!(v.resolve(14.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 24.0);
    }

    #[test]
    fn resolve_percent() {
        let v = LengthValue::Percent(50.0);
        assert_eq!(v.resolve(14.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 400.0);
    }

    #[test]
    fn resolve_vw() {
        let v = LengthValue::Vw(10.0);
        assert_eq!(v.resolve(14.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 192.0);
    }

    #[test]
    fn resolve_vh() {
        let v = LengthValue::Vh(50.0);
        assert_eq!(v.resolve(14.0, 16.0, Size::new(1920.0, 1080.0), 800.0), 540.0);
    }

    #[test]
    fn resolve_auto_and_zero() {
        let auto = LengthValue::Auto;
        let zero = LengthValue::Zero;
        let vp = Size::new(1920.0, 1080.0);
        assert_eq!(auto.resolve(16.0, 16.0, vp, 800.0), 0.0);
        assert_eq!(zero.resolve(16.0, 16.0, vp, 800.0), 0.0);
        assert!(auto.is_auto());
        assert!(zero.is_zero());
    }

    #[test]
    fn parse_various_lengths() {
        assert_eq!(parse_length("10px"), Some(LengthValue::Px(10.0)));
        assert_eq!(parse_length("2em"), Some(LengthValue::Em(2.0)));
        assert_eq!(parse_length("1.5rem"), Some(LengthValue::Rem(1.5)));
        assert_eq!(parse_length("50%"), Some(LengthValue::Percent(50.0)));
        assert_eq!(parse_length("auto"), Some(LengthValue::Auto));
        assert_eq!(parse_length("0"), Some(LengthValue::Zero));
        assert_eq!(parse_length("garbage"), None);
    }
}
