// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! WebGL parameter masking to reduce fingerprinting surface.
//!
//! Replaces real GPU vendor/renderer strings and limits reported
//! capabilities to a plausible baseline, making it harder for
//! scripts to uniquely identify a device through
//! `WEBGL_debug_renderer_info`.

use std::collections::HashMap;

/// Masked WebGL parameter values returned instead of real hardware info.
#[derive(Debug, Clone)]
pub struct WebGlMask {
    /// Whether masking is enabled.
    pub enabled: bool,
    /// Spoofed vendor string (e.g. "Google Inc. (ANGLE)").
    pub vendor: String,
    /// Spoofed renderer string.
    pub renderer: String,
    /// Spoofed unmasked vendor (from `WEBGL_debug_renderer_info`).
    pub unmasked_vendor: String,
    /// Spoofed unmasked renderer.
    pub unmasked_renderer: String,
    /// Clamped extension set (only reveal what is on the allowlist).
    pub allowed_extensions: Vec<String>,
}

impl Default for WebGlMask {
    fn default() -> Self {
        Self {
            enabled: true,
            vendor: "WebKit".to_owned(),
            renderer: "WebKit WebGL".to_owned(),
            unmasked_vendor: "Google Inc. (ANGLE)".to_owned(),
            unmasked_renderer: "ANGLE (Generic, Vulkan 1.3, D3D11)".to_owned(),
            allowed_extensions: default_allowed_extensions(),
        }
    }
}

/// Apply masking to a raw parameter map from the GPU.
///
/// Keys are lowercase GL parameter names. Modified in place.
pub fn mask_webgl_params(mask: &WebGlMask, params: &mut HashMap<String, String>) {
    if !mask.enabled {
        return;
    }

    if let Some(v) = params.get_mut("vendor") {
        *v = mask.vendor.clone();
    }
    if let Some(v) = params.get_mut("renderer") {
        *v = mask.renderer.clone();
    }
    if let Some(v) = params.get_mut("unmasked_vendor_webgl") {
        *v = mask.unmasked_vendor.clone();
    }
    if let Some(v) = params.get_mut("unmasked_renderer_webgl") {
        *v = mask.unmasked_renderer.clone();
    }
}

/// Filter an extension list down to the allowed set.
#[must_use]
pub fn filter_extensions(mask: &WebGlMask, reported: &[String]) -> Vec<String> {
    if !mask.enabled {
        return reported.to_vec();
    }
    reported
        .iter()
        .filter(|ext| mask.allowed_extensions.contains(ext))
        .cloned()
        .collect()
}

/// Common WebGL extensions that don't leak fingerprint data.
fn default_allowed_extensions() -> Vec<String> {
    [
        "ANGLE_instanced_arrays",
        "EXT_blend_minmax",
        "EXT_color_buffer_half_float",
        "EXT_shader_texture_lod",
        "EXT_texture_filter_anisotropic",
        "OES_element_index_uint",
        "OES_standard_derivatives",
        "OES_texture_float",
        "OES_texture_float_linear",
        "OES_texture_half_float",
        "OES_texture_half_float_linear",
        "OES_vertex_array_object",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_depth_texture",
        "WEBGL_draw_buffers",
        "WEBGL_lose_context",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_real_params() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("vendor".into(), "NVIDIA Corporation".into());
        m.insert("renderer".into(), "GeForce RTX 4090/PCIe/SSE2".into());
        m.insert("unmasked_vendor_webgl".into(), "NVIDIA Corporation".into());
        m.insert(
            "unmasked_renderer_webgl".into(),
            "NVIDIA GeForce RTX 4090".into(),
        );
        m
    }

    #[test]
    fn masking_replaces_vendor_and_renderer() {
        let mask = WebGlMask::default();
        let mut params = make_real_params();
        mask_webgl_params(&mask, &mut params);
        assert_eq!(params["vendor"], "WebKit");
        assert_eq!(params["renderer"], "WebKit WebGL");
        assert_eq!(params["unmasked_vendor_webgl"], "Google Inc. (ANGLE)");
        assert!(params["unmasked_renderer_webgl"].contains("ANGLE"));
    }

    #[test]
    fn disabled_mask_is_noop() {
        let mask = WebGlMask {
            enabled: false,
            ..Default::default()
        };
        let mut params = make_real_params();
        let original = params.clone();
        mask_webgl_params(&mask, &mut params);
        assert_eq!(params, original);
    }

    #[test]
    fn extension_filtering() {
        let mask = WebGlMask::default();
        let reported = vec![
            "WEBGL_lose_context".into(),
            "EXT_weird_fingerprint_ext".into(),
            "OES_texture_float".into(),
        ];
        let filtered = filter_extensions(&mask, &reported);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"WEBGL_lose_context".to_owned()));
        assert!(filtered.contains(&"OES_texture_float".to_owned()));
    }

    #[test]
    fn disabled_filter_returns_all() {
        let mask = WebGlMask {
            enabled: false,
            ..Default::default()
        };
        let reported = vec!["ANYTHING".into(), "GOES".into()];
        assert_eq!(filter_extensions(&mask, &reported).len(), 2);
    }

    #[test]
    fn custom_mask_values_applied() {
        let mask = WebGlMask {
            enabled: true,
            vendor: "Custom".into(),
            renderer: "Custom GL".into(),
            unmasked_vendor: "Custom Inc.".into(),
            unmasked_renderer: "Custom GPU".into(),
            allowed_extensions: vec![],
        };
        let mut params = make_real_params();
        mask_webgl_params(&mask, &mut params);
        assert_eq!(params["vendor"], "Custom");
        assert_eq!(params["renderer"], "Custom GL");
    }
}
