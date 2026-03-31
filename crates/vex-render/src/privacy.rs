// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Privacy hooks for the render pipeline.
//!
//! Bridges `vex-privacy` modules into the GPU render path:
//! - **Canvas noise**: applied to pixel readback (toDataURL / getImageData).
//! - **Font restriction**: filters font families before glyph shaping.
//! - **WebGL masking**: intercepts WebGL parameter queries.

use std::collections::HashMap;

use vex_privacy::canvas::{apply_canvas_noise, CanvasFingerprintConfig};
use vex_privacy::fonts::{is_font_allowed, FontRestrictionConfig};
use vex_privacy::webgl::{filter_extensions, mask_webgl_params, WebGlMask};

/// Aggregated privacy configuration for the render pipeline.
#[derive(Debug, Clone, Default)]
pub struct RenderPrivacyConfig {
    /// Canvas fingerprint noise applied to pixel readback.
    pub canvas: CanvasFingerprintConfig,
    /// Font enumeration restriction (limits which families are available).
    pub fonts: FontRestrictionConfig,
    /// WebGL parameter masking.
    pub webgl: WebGlMask,
}

impl RenderPrivacyConfig {
    /// Check whether a font family is allowed by the restriction policy.
    #[must_use]
    pub fn is_font_allowed(&self, family: &str) -> bool {
        is_font_allowed(&self.fonts, family)
    }

    /// Apply canvas fingerprint noise to RGBA pixel data in-place.
    ///
    /// Called after pixel readback (screenshot / toDataURL / getImageData).
    pub fn apply_canvas_noise(&self, pixels: &mut [u8]) {
        apply_canvas_noise(&self.canvas, pixels);
    }

    /// Mask WebGL parameter values to prevent GPU fingerprinting.
    pub fn mask_webgl_params(&self, params: &mut HashMap<String, String>) {
        mask_webgl_params(&self.webgl, params);
    }

    /// Filter a reported WebGL extension list to the allowed subset.
    #[must_use]
    pub fn filter_webgl_extensions(&self, reported: &[String]) -> Vec<String> {
        filter_extensions(&self.webgl, reported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_protections_enabled() {
        let cfg = RenderPrivacyConfig::default();
        assert!(cfg.canvas.enabled);
        assert!(cfg.fonts.enabled);
        assert!(cfg.webgl.enabled);
    }

    #[test]
    fn font_restriction_filters_unknown_families() {
        let cfg = RenderPrivacyConfig::default();
        assert!(cfg.is_font_allowed("Arial"));
        assert!(cfg.is_font_allowed("serif"));
        assert!(!cfg.is_font_allowed("SuperRareFont2000"));
    }

    #[test]
    fn canvas_noise_modifies_pixels() {
        let mut cfg = RenderPrivacyConfig::default();
        cfg.canvas.session_seed = 42;
        cfg.canvas.noise_magnitude = 3;

        let original = vec![128u8; 16]; // 4 RGBA pixels
        let mut noised = original.clone();
        cfg.apply_canvas_noise(&mut noised);
        assert_ne!(original, noised);
    }

    #[test]
    fn webgl_masking_replaces_vendor() {
        let cfg = RenderPrivacyConfig::default();
        let mut params = HashMap::new();
        params.insert("vendor".to_string(), "NVIDIA Corporation".to_string());
        params.insert("renderer".to_string(), "GeForce RTX 4090".to_string());

        cfg.mask_webgl_params(&mut params);
        assert_eq!(params["vendor"], "WebKit");
        assert_eq!(params["renderer"], "WebKit WebGL");
    }

    #[test]
    fn webgl_extension_filtering() {
        let cfg = RenderPrivacyConfig::default();
        let reported = vec![
            "OES_texture_float".to_string(),
            "WEBGL_super_secret_extension".to_string(),
        ];
        let filtered = cfg.filter_webgl_extensions(&reported);
        assert!(filtered.contains(&"OES_texture_float".to_string()));
        assert!(!filtered.contains(&"WEBGL_super_secret_extension".to_string()));
    }
}
