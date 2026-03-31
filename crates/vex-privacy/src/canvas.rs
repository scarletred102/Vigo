// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Canvas fingerprint protection.
//!
//! When enabled, adds subtle per-session noise to pixel data returned by
//! `toDataURL()` / `getImageData()` to defeat canvas-based fingerprinting
//! while preserving visual fidelity.

/// Configuration for canvas fingerprint protection.
#[derive(Debug, Clone)]
pub struct CanvasFingerprintConfig {
    /// Whether protection is enabled.
    pub enabled: bool,
    /// Per-session random seed (changes each browser session).
    pub session_seed: u64,
    /// Maximum noise magnitude per colour channel (0–5 recommended).
    pub noise_magnitude: u8,
}

impl Default for CanvasFingerprintConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            session_seed: rand::random::<u64>(),
            noise_magnitude: 2,
        }
    }
}

/// Apply subtle deterministic noise to RGBA pixel data.
///
/// The noise is deterministic given the same `session_seed` and pixel
/// position so that repeated reads return the same (noised) value within
/// a session, but differ across sessions.
///
/// `pixels` must be in RGBA format (4 bytes per pixel).
pub fn apply_canvas_noise(config: &CanvasFingerprintConfig, pixels: &mut [u8]) {
    if !config.enabled || config.noise_magnitude == 0 {
        return;
    }

    let mag = i16::from(config.noise_magnitude);

    for (i, chunk) in pixels.chunks_exact_mut(4).enumerate() {
        // Simple hash combining seed + pixel index for deterministic noise.
        let h = cheap_hash(config.session_seed, i as u64);

        // Apply noise to R, G, B channels only (leave alpha alone).
        for (j, byte) in chunk.iter_mut().take(3).enumerate() {
            let noise = ((h >> (j * 8)) & 0xFF) as i16 % (2 * mag + 1) - mag;
            *byte = (*byte as i16 + noise).clamp(0, 255) as u8;
        }
    }
}

/// Check whether a `toDataURL()` or `getImageData()` call should be noised.
///
/// Returns `true` when protection is active.
#[must_use]
pub fn should_noise_canvas(config: &CanvasFingerprintConfig) -> bool {
    config.enabled && config.noise_magnitude > 0
}

/// Cheap non-cryptographic hash for noise generation.
fn cheap_hash(seed: u64, index: u64) -> u64 {
    let mut h = seed.wrapping_mul(0x517c_c1b7_2722_0a95);
    h ^= index;
    h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 32;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_config_is_noop() {
        let config = CanvasFingerprintConfig {
            enabled: false,
            ..Default::default()
        };
        let mut pixels = [128u8; 16]; // 4 pixels
        let original = pixels;
        apply_canvas_noise(&config, &mut pixels);
        assert_eq!(pixels, original);
    }

    #[test]
    fn zero_magnitude_is_noop() {
        let config = CanvasFingerprintConfig {
            noise_magnitude: 0,
            ..Default::default()
        };
        let mut pixels = [128u8; 16];
        let original = pixels;
        apply_canvas_noise(&config, &mut pixels);
        assert_eq!(pixels, original);
    }

    #[test]
    fn noise_modifies_pixels() {
        let config = CanvasFingerprintConfig {
            enabled: true,
            session_seed: 42,
            noise_magnitude: 2,
        };
        let mut pixels = [128u8; 400]; // 100 pixels
        let original = pixels;
        apply_canvas_noise(&config, &mut pixels);
        // At least some pixels should differ.
        assert_ne!(pixels, original);
    }

    #[test]
    fn noise_is_deterministic() {
        let config = CanvasFingerprintConfig {
            enabled: true,
            session_seed: 123,
            noise_magnitude: 3,
        };
        let mut p1 = [100u8; 40];
        let mut p2 = [100u8; 40];
        apply_canvas_noise(&config, &mut p1);
        apply_canvas_noise(&config, &mut p2);
        assert_eq!(p1, p2);
    }

    #[test]
    fn different_seeds_different_results() {
        let c1 = CanvasFingerprintConfig {
            enabled: true,
            session_seed: 1,
            noise_magnitude: 2,
        };
        let c2 = CanvasFingerprintConfig {
            enabled: true,
            session_seed: 2,
            noise_magnitude: 2,
        };
        let mut p1 = [128u8; 40];
        let mut p2 = [128u8; 40];
        apply_canvas_noise(&c1, &mut p1);
        apply_canvas_noise(&c2, &mut p2);
        assert_ne!(p1, p2);
    }

    #[test]
    fn alpha_channel_untouched() {
        let config = CanvasFingerprintConfig {
            enabled: true,
            session_seed: 99,
            noise_magnitude: 5,
        };
        let mut pixels = [128u8; 8]; // 2 pixels
        apply_canvas_noise(&config, &mut pixels);
        // Alpha bytes are at indices 3 and 7.
        assert_eq!(pixels[3], 128);
        assert_eq!(pixels[7], 128);
    }

    #[test]
    fn should_noise_respects_config() {
        let on = CanvasFingerprintConfig::default();
        assert!(should_noise_canvas(&on));

        let off = CanvasFingerprintConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_noise_canvas(&off));
    }
}
