// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Browser settings — user-configurable preferences with JSON persistence.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vex_core::error::VexError;
use vex_core::VexResult;
use vex_privacy::canvas::CanvasFingerprintConfig;
use vex_privacy::fonts::FontRestrictionConfig;
use vex_privacy::webgl::WebGlMask;

/// All browser settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    // -- General --
    /// Home page URL.
    pub home_page: String,
    /// What to show on new tab.
    pub new_tab_page: NewTabPage,
    /// Default search engine URL template (use `{query}` as placeholder).
    pub search_engine: String,

    // -- Appearance --
    /// Theme mode.
    pub theme: ThemeMode,
    /// Default zoom percentage (100 = default).
    pub default_zoom: u32,
    /// Show bookmarks bar.
    pub show_bookmarks_bar: bool,
    /// Default window width in logical pixels.
    pub window_width: u32,
    /// Default window height in logical pixels.
    pub window_height: u32,
    /// Default root font-size in pixels (used for `rem` unit resolution).
    pub default_font_size: f32,

    // -- Privacy --
    /// Block third-party cookies.
    pub block_third_party_cookies: bool,
    /// Enable tracking protection.
    pub tracking_protection: bool,
    /// Send Do Not Track header.
    pub do_not_track: bool,
    /// HTTPS-only mode.
    pub https_only: bool,
    /// Clear browsing data on exit.
    pub clear_on_exit: bool,

    // -- Content --
    /// Enable JavaScript.
    pub javascript_enabled: bool,
    /// Enable images.
    pub images_enabled: bool,
    /// Default text encoding.
    pub default_encoding: String,

    // -- Downloads --
    /// Download directory path.
    pub download_dir: String,
    /// Always ask where to save downloads.
    pub ask_download_location: bool,

    // -- Privacy configs (runtime-only, not serialized) --
    /// Canvas fingerprint protection config (session seed randomized at startup).
    #[serde(skip)]
    pub canvas_fingerprint: CanvasFingerprintConfig,
    /// WebGL parameter masking config.
    #[serde(skip)]
    pub webgl_mask: WebGlMask,
    /// Font enumeration restriction config.
    #[serde(skip)]
    pub font_restriction: FontRestrictionConfig,
}

/// What to show on the new-tab page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NewTabPage {
    /// Blank page.
    Blank,
    /// The home page.
    HomePage,
    /// Custom URL.
    Custom(String),
}

/// UI theme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Dark theme (default for Vex).
    Dark,
    /// Light theme.
    Light,
    /// Follow OS setting.
    System,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            home_page: "vex://newtab".to_string(),
            new_tab_page: NewTabPage::Blank,
            search_engine: "https://duckduckgo.com/?q={query}".to_string(),
            theme: ThemeMode::Dark,
            default_zoom: 100,
            show_bookmarks_bar: true,
            window_width: 1280,
            window_height: 720,
            default_font_size: 16.0,
            block_third_party_cookies: true,
            tracking_protection: true,
            do_not_track: true,
            https_only: true,
            clear_on_exit: false,
            javascript_enabled: true,
            images_enabled: true,
            default_encoding: "UTF-8".to_string(),
            download_dir: default_download_dir(),
            ask_download_location: false,
            canvas_fingerprint: CanvasFingerprintConfig::default(),
            webgl_mask: WebGlMask::default(),
            font_restriction: FontRestrictionConfig::default(),
        }
    }
}

impl BrowserSettings {
    /// Create settings with all defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a search URL from a query string.
    pub fn search_url(&self, query: &str) -> String {
        self.search_engine.replace("{query}", &url_encode(query))
    }

    /// Save settings to a JSON file.
    pub fn save(&self, path: &Path) -> VexResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| VexError::Internal(format!("Failed to serialize settings: {e}")))?;
        std::fs::write(path, json)
            .map_err(|e| VexError::Internal(format!("Failed to write settings: {e}")))?;
        Ok(())
    }

    /// Load settings from a JSON file. Returns defaults if file doesn't exist.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
}

/// Simple percent-encoding for query strings.
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{b:02X}"));
            }
        }
    }
    result
}

/// Default download directory.
fn default_download_dir() -> String {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let mut path = std::path::PathBuf::from(home);
        path.push("Downloads");
        path.to_string_lossy().to_string()
    } else {
        "Downloads".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let s = BrowserSettings::new();
        assert_eq!(s.default_zoom, 100);
        assert!(s.tracking_protection);
        assert!(s.https_only);
        assert!(s.javascript_enabled);
        assert_eq!(s.theme, ThemeMode::Dark);
        assert_eq!(s.window_width, 1280);
        assert_eq!(s.window_height, 720);
        assert!((s.default_font_size - 16.0).abs() < f32::EPSILON);
        // Privacy configs are present with sensible defaults.
        assert!(s.canvas_fingerprint.enabled);
        assert_ne!(s.canvas_fingerprint.session_seed, 0);
        assert!(s.webgl_mask.enabled);
        assert!(s.font_restriction.enabled);
    }

    #[test]
    fn search_url_replaces_query() {
        let s = BrowserSettings::new();
        let url = s.search_url("rust browser");
        assert!(url.contains("rust+browser"));
        assert!(url.starts_with("https://duckduckgo.com/"));
    }

    #[test]
    fn url_encode_special_chars() {
        assert_eq!(url_encode("hello world"), "hello+world");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn save_and_load() {
        let mut s = BrowserSettings::new();
        s.default_zoom = 150;
        s.show_bookmarks_bar = false;

        let dir = std::env::temp_dir().join("vex_settings_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        s.save(&path).unwrap();

        let loaded = BrowserSettings::load(&path);
        assert_eq!(loaded.default_zoom, 150);
        assert!(!loaded.show_bookmarks_bar);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let loaded = BrowserSettings::load(Path::new("/nonexistent/path/settings.json"));
        assert_eq!(loaded.default_zoom, 100);
    }

    #[test]
    fn new_tab_page_variants() {
        let s = BrowserSettings::new();
        assert_eq!(s.new_tab_page, NewTabPage::Blank);
    }
}
