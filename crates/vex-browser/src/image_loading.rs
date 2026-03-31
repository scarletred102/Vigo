// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Async image loading pattern for browser pipeline integration.
//!
//! Pattern:
//! 1) fetch image bytes via `vex-net`
//! 2) decode on a background thread with `tokio::task::spawn_blocking`
//! 3) consume decoded frames on main/render path and upload into atlas

use tokio::task::JoinHandle;
use vex_render::display_list::ImageId;
use vex_render::image_decode::DecodedImage;

// ── srcset support ────────────────────────────────────────────────────

/// A single candidate from an `<img srcset>` attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct SrcsetCandidate {
    /// Image URL.
    pub url: String,
    /// Width descriptor in pixels (`w`), or `None`.
    pub width: Option<u32>,
    /// Pixel-density descriptor (`x`), or `None`.
    pub density: Option<f32>,
}

/// Parse an `srcset` attribute value into a list of candidates.
///
/// Handles both `w` (width) and `x` (density) descriptors.
/// If no descriptor is present, defaults to `1x`.
pub fn parse_srcset(srcset: &str) -> Vec<SrcsetCandidate> {
    srcset
        .split(',')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split_whitespace().collect();
            let url = parts.first()?.to_string();
            if url.is_empty() {
                return None;
            }

            let mut width = None;
            let mut density = None;

            if let Some(desc) = parts.get(1) {
                if let Some(w) = desc.strip_suffix('w') {
                    width = w.parse().ok();
                } else if let Some(x) = desc.strip_suffix('x') {
                    density = x.parse().ok();
                }
            }

            // Default density if neither descriptor present.
            if width.is_none() && density.is_none() {
                density = Some(1.0);
            }

            Some(SrcsetCandidate {
                url,
                width,
                density,
            })
        })
        .collect()
}

/// Pick the best srcset candidate for a given viewport width and device
/// pixel ratio.
///
/// Strategy:
/// - If candidates use `w` descriptors, pick the smallest image that is
///   at least as wide as `viewport_width`.
/// - If candidates use `x` descriptors, pick the closest density match.
/// - Falls back to the first candidate if nothing beats it.
pub fn pick_best_source(
    candidates: &[SrcsetCandidate],
    viewport_width: f32,
    device_pixel_ratio: f32,
) -> Option<&SrcsetCandidate> {
    if candidates.is_empty() {
        return None;
    }

    // Prefer `w` descriptors.
    let width_candidates: Vec<&SrcsetCandidate> =
        candidates.iter().filter(|c| c.width.is_some()).collect();

    if !width_candidates.is_empty() {
        // Pick smallest w >= viewport_width * dpr, or the largest available.
        let target = viewport_width * device_pixel_ratio;
        let mut best = width_candidates[0];
        for &c in &width_candidates {
            let w = c.width.unwrap_or(0) as f32;
            let bw = best.width.unwrap_or(0) as f32;
            if (w >= target && (bw < target || w < bw)) || (bw < target && w > bw) {
                best = c;
            }
        }
        return Some(best);
    }

    // Density descriptors: pick closest to device_pixel_ratio.
    let mut best = &candidates[0];
    let mut best_diff = f32::MAX;
    for c in candidates {
        let d = c.density.unwrap_or(1.0);
        let diff = (d - device_pixel_ratio).abs();
        if diff < best_diff {
            best_diff = diff;
            best = c;
        }
    }
    Some(best)
}

/// Result of a background image decode task.
#[derive(Debug)]
pub enum ImageLoadResult {
    Ready { id: ImageId, image: DecodedImage },
    Failed { id: ImageId, error: String },
}

/// Tracks pending image decode tasks and drains completed results.
#[derive(Default)]
pub struct AsyncImageLoader {
    pending: Vec<JoinHandle<ImageLoadResult>>,
}

impl AsyncImageLoader {
    /// Create an empty async loader queue.
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Queue a decode job for fetched image bytes.
    pub fn request_decode(&mut self, id: ImageId, bytes: Vec<u8>) {
        let task =
            tokio::task::spawn_blocking(move || {
                match vex_render::image_decode::decode_image(&bytes) {
                    Ok(image) => ImageLoadResult::Ready { id, image },
                    Err(error) => ImageLoadResult::Failed {
                        id,
                        error: error.to_string(),
                    },
                }
            });
        self.pending.push(task);
    }

    /// Number of decode tasks still pending completion.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Drain all finished decode tasks without blocking on unfinished ones.
    pub async fn drain_finished(&mut self) -> Vec<ImageLoadResult> {
        let mut finished = Vec::new();
        let mut i = 0;

        while i < self.pending.len() {
            if self.pending[i].is_finished() {
                let handle = self.pending.swap_remove(i);
                match handle.await {
                    Ok(result) => finished.push(result),
                    Err(error) => {
                        tracing::error!(
                            target: "vex_browser::image_loading",
                            "image decode task join error: {error}"
                        );
                    }
                }
            } else {
                i += 1;
            }
        }

        finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Vec<u8> {
        // Minimal 1×1 PNG.
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }

    #[tokio::test]
    async fn decode_success_path() {
        let mut loader = AsyncImageLoader::new();
        loader.request_decode(ImageId(1), tiny_png());

        let mut result = Vec::new();
        for _ in 0..200 {
            result = loader.drain_finished().await;
            if !result.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        assert_eq!(result.len(), 1);
        match &result[0] {
            ImageLoadResult::Ready { id, image } => {
                assert_eq!(*id, ImageId(1));
                assert_eq!(image.width, 1);
                assert_eq!(image.height, 1);
            }
            ImageLoadResult::Failed { error, .. } => panic!("unexpected decode failure: {error}"),
        }
    }

    #[tokio::test]
    async fn decode_failure_path() {
        let mut loader = AsyncImageLoader::new();
        loader.request_decode(ImageId(2), b"not-an-image".to_vec());

        let mut result = Vec::new();
        for _ in 0..200 {
            result = loader.drain_finished().await;
            if !result.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        assert_eq!(result.len(), 1);
        match &result[0] {
            ImageLoadResult::Failed { id, .. } => assert_eq!(*id, ImageId(2)),
            ImageLoadResult::Ready { .. } => panic!("expected decode failure"),
        }
    }

    #[test]
    fn parse_srcset_width_descriptors() {
        let parsed = parse_srcset("small.jpg 320w, medium.jpg 640w, large.jpg 1024w");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].url, "small.jpg");
        assert_eq!(parsed[0].width, Some(320));
        assert_eq!(parsed[1].url, "medium.jpg");
        assert_eq!(parsed[1].width, Some(640));
        assert_eq!(parsed[2].url, "large.jpg");
        assert_eq!(parsed[2].width, Some(1024));
    }

    #[test]
    fn parse_srcset_density_descriptors() {
        let parsed = parse_srcset("photo.jpg 1x, photo@2x.jpg 2x");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].density, Some(1.0));
        assert_eq!(parsed[1].density, Some(2.0));
    }

    #[test]
    fn parse_srcset_no_descriptor_defaults_1x() {
        let parsed = parse_srcset("image.jpg");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].url, "image.jpg");
        assert_eq!(parsed[0].density, Some(1.0));
    }

    #[test]
    fn pick_best_width_candidate() {
        let candidates = parse_srcset("small.jpg 320w, medium.jpg 640w, large.jpg 1024w");
        let best = pick_best_source(&candidates, 500.0, 1.0).unwrap();
        assert_eq!(best.url, "medium.jpg"); // 640w >= 500*1.0
    }

    #[test]
    fn pick_best_density_candidate() {
        let candidates = parse_srcset("photo.jpg 1x, photo@2x.jpg 2x, photo@3x.jpg 3x");
        let best = pick_best_source(&candidates, 400.0, 2.0).unwrap();
        assert_eq!(best.url, "photo@2x.jpg");
    }

    #[test]
    fn pick_best_empty_returns_none() {
        let candidates: Vec<SrcsetCandidate> = vec![];
        assert!(pick_best_source(&candidates, 400.0, 1.0).is_none());
    }
}
