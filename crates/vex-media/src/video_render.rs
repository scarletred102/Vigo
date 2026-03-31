// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Video frame rendering integration.
//!
//! Decoded video frames are uploaded as GPU textures and rendered as
//! `DrawImage` commands in the display list at the `<video>` element's
//! layout position. The texture is updated each frame at playback rate.

use vex_core::geometry::Rect;

/// A decoded video frame ready for GPU upload.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Presentation timestamp in milliseconds.
    pub pts_ms: i64,
}

impl DecodedFrame {
    /// Create a new decoded frame.
    #[must_use]
    pub fn new(pixels: Vec<u8>, width: u32, height: u32, pts_ms: i64) -> Self {
        Self {
            pixels,
            width,
            height,
            pts_ms,
        }
    }

    /// Expected byte size for the pixel data (width × height × 4 for RGBA).
    #[must_use]
    pub fn expected_size(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    /// Whether the pixel data matches the expected size.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.pixels.len() == self.expected_size() && self.width > 0 && self.height > 0
    }
}

/// Manages the current video frame for rendering.
///
/// Holds the latest decoded frame and tracks whether the GPU texture
/// needs to be updated.
#[derive(Debug)]
pub struct VideoSurface {
    /// The current frame to display.
    current_frame: Option<DecodedFrame>,
    /// Whether the texture needs re-upload.
    dirty: bool,
    /// Layout rect where the video is rendered.
    layout_rect: Rect,
    /// Aspect ratio (width / height) for letterboxing.
    aspect_ratio: f32,
}

impl VideoSurface {
    /// Create a new video surface.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_frame: None,
            dirty: false,
            layout_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            aspect_ratio: 16.0 / 9.0,
        }
    }

    /// Submit a new decoded frame for display.
    pub fn submit_frame(&mut self, frame: DecodedFrame) {
        if frame.width > 0 && frame.height > 0 {
            self.aspect_ratio = frame.width as f32 / frame.height as f32;
        }
        self.current_frame = Some(frame);
        self.dirty = true;
    }

    /// Get the current frame, if any.
    #[must_use]
    pub fn current_frame(&self) -> Option<&DecodedFrame> {
        self.current_frame.as_ref()
    }

    /// Whether the GPU texture needs updating.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark texture as uploaded (clear dirty flag).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Set the layout rectangle where the video is positioned.
    pub fn set_layout_rect(&mut self, rect: Rect) {
        self.layout_rect = rect;
    }

    /// Get the layout rectangle.
    #[must_use]
    pub fn layout_rect(&self) -> Rect {
        self.layout_rect
    }

    /// Compute the actual video rectangle within the layout rect,
    /// preserving aspect ratio (letterboxed/pillarboxed).
    #[must_use]
    pub fn compute_video_rect(&self) -> Rect {
        let container = self.layout_rect;
        if container.size.width <= 0.0 || container.size.height <= 0.0 {
            return container;
        }

        let container_aspect = container.size.width / container.size.height;

        if (container_aspect - self.aspect_ratio).abs() < 0.01 {
            // Aspect ratios match — fill the container
            container
        } else if container_aspect > self.aspect_ratio {
            // Container is wider — pillarbox (black bars on sides)
            let video_w = container.size.height * self.aspect_ratio;
            let offset_x = (container.size.width - video_w) / 2.0;
            Rect::new(
                container.origin.x + offset_x,
                container.origin.y,
                video_w,
                container.size.height,
            )
        } else {
            // Container is taller — letterbox (black bars top/bottom)
            let video_h = container.size.width / self.aspect_ratio;
            let offset_y = (container.size.height - video_h) / 2.0;
            Rect::new(
                container.origin.x,
                container.origin.y + offset_y,
                container.size.width,
                video_h,
            )
        }
    }

    /// Clear the surface (e.g., when video is unloaded).
    pub fn clear(&mut self) {
        self.current_frame = None;
        self.dirty = true;
    }

    /// Video aspect ratio.
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }
}

impl Default for VideoSurface {
    fn default() -> Self {
        Self::new()
    }
}

// ──── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frame(w: u32, h: u32) -> DecodedFrame {
        DecodedFrame::new(vec![0u8; (w * h * 4) as usize], w, h, 0)
    }

    #[test]
    fn test_decoded_frame_valid() {
        let f = test_frame(320, 240);
        assert!(f.is_valid());
        assert_eq!(f.expected_size(), 320 * 240 * 4);
    }

    #[test]
    fn test_decoded_frame_invalid_size() {
        let f = DecodedFrame::new(vec![0u8; 10], 320, 240, 0);
        assert!(!f.is_valid());
    }

    #[test]
    fn test_decoded_frame_zero_dims() {
        let f = DecodedFrame::new(vec![], 0, 0, 0);
        assert!(!f.is_valid());
    }

    #[test]
    fn test_surface_new() {
        let s = VideoSurface::new();
        assert!(s.current_frame().is_none());
        assert!(!s.is_dirty());
    }

    #[test]
    fn test_surface_submit_frame() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(640, 480));
        assert!(s.current_frame().is_some());
        assert!(s.is_dirty());

        s.mark_clean();
        assert!(!s.is_dirty());
    }

    #[test]
    fn test_surface_aspect_ratio() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(1920, 1080));
        assert!((s.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_video_rect_exact_fit() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(1600, 900));
        s.set_layout_rect(Rect::new(0.0, 0.0, 800.0, 450.0));
        let r = s.compute_video_rect();
        assert!((r.size.width - 800.0).abs() < 1.0);
        assert!((r.size.height - 450.0).abs() < 1.0);
    }

    #[test]
    fn test_video_rect_letterbox() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(1920, 1080)); // 16:9
        s.set_layout_rect(Rect::new(0.0, 0.0, 400.0, 400.0)); // square
        let r = s.compute_video_rect();
        // Should be pillarboxed: video width = 400, video height = 400/1.78 ≈ 225
        assert!((r.size.width - 400.0).abs() < 1.0);
        assert!(r.size.height < 400.0);
    }

    #[test]
    fn test_video_rect_pillarbox() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(100, 200)); // 0.5 narrow
        s.set_layout_rect(Rect::new(0.0, 0.0, 400.0, 400.0));
        let r = s.compute_video_rect();
        // Narrow video in square container → pillarbox
        assert!(r.size.width < 400.0);
        assert!((r.size.height - 400.0).abs() < 1.0);
    }

    #[test]
    fn test_surface_clear() {
        let mut s = VideoSurface::new();
        s.submit_frame(test_frame(320, 240));
        s.mark_clean();
        s.clear();
        assert!(s.current_frame().is_none());
        assert!(s.is_dirty());
    }
}
