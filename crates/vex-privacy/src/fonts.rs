// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Font enumeration restriction to reduce fingerprinting surface.
//!
//! Instead of exposing all system-installed fonts, this module provides
//! a curated allowlist of common web-safe fonts. Scripts calling
//! `document.fonts.check()` or attempting to detect installed fonts via
//! width probing will only see fonts on the allowlist.

/// Configuration for font enumeration restriction.
#[derive(Debug, Clone)]
pub struct FontRestrictionConfig {
    /// Whether restriction is enabled.
    pub enabled: bool,
    /// Allowed font family names (case-insensitive matching).
    pub allowed_families: Vec<String>,
}

impl Default for FontRestrictionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_families: default_allowed_fonts(),
        }
    }
}

/// Check whether a font family is allowed to be reported as available.
#[must_use]
pub fn is_font_allowed(config: &FontRestrictionConfig, family: &str) -> bool {
    if !config.enabled {
        return true;
    }
    let lower = family.to_ascii_lowercase();
    config
        .allowed_families
        .iter()
        .any(|f| f.to_ascii_lowercase() == lower)
}

/// Filter a list of font families down to the allowed set.
#[must_use]
pub fn filter_fonts(config: &FontRestrictionConfig, families: &[String]) -> Vec<String> {
    if !config.enabled {
        return families.to_vec();
    }
    families
        .iter()
        .filter(|f| is_font_allowed(config, f))
        .cloned()
        .collect()
}

/// Curated list of common web-safe fonts that don't leak OS/user info.
fn default_allowed_fonts() -> Vec<String> {
    [
        // Generic families
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        // Common web-safe fonts (cross-platform)
        "Arial",
        "Helvetica",
        "Times New Roman",
        "Times",
        "Courier New",
        "Courier",
        "Georgia",
        "Verdana",
        "Trebuchet MS",
        "Tahoma",
        "Palatino",
        "Garamond",
        // Common on macOS + Windows
        "Segoe UI",
        "Roboto",
        "Liberation Sans",
        "Liberation Serif",
        "Liberation Mono",
        // Monospace
        "Consolas",
        "Menlo",
        "Monaco",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_font_passes() {
        let config = FontRestrictionConfig::default();
        assert!(is_font_allowed(&config, "Arial"));
        assert!(is_font_allowed(&config, "arial")); // case insensitive
    }

    #[test]
    fn disallowed_font_blocked() {
        let config = FontRestrictionConfig::default();
        assert!(!is_font_allowed(&config, "Papyrus"));
        assert!(!is_font_allowed(
            &config,
            "Some Unique Font Only On This Machine"
        ));
    }

    #[test]
    fn disabled_allows_everything() {
        let config = FontRestrictionConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(is_font_allowed(&config, "Anything Goes"));
    }

    #[test]
    fn filter_fonts_works() {
        let config = FontRestrictionConfig::default();
        let input = vec![
            "Arial".into(),
            "Papyrus".into(),
            "Courier New".into(),
            "SomeRareFont".into(),
        ];
        let result = filter_fonts(&config, &input);
        assert_eq!(result, vec!["Arial", "Courier New"]);
    }

    #[test]
    fn generic_families_allowed() {
        let config = FontRestrictionConfig::default();
        for fam in &["serif", "sans-serif", "monospace", "system-ui"] {
            assert!(is_font_allowed(&config, fam), "{fam} should be allowed");
        }
    }

    #[test]
    fn custom_allowlist() {
        let config = FontRestrictionConfig {
            enabled: true,
            allowed_families: vec!["OnlyThis".into()],
        };
        assert!(is_font_allowed(&config, "OnlyThis"));
        assert!(!is_font_allowed(&config, "Arial"));
    }
}
