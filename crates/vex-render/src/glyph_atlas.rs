// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Glyph atlas — CPU-side texture atlas for rasterized text glyphs.
//!
//! Uses `cosmic_text` for font loading / shaping and `SwashCache` for
//! glyph rasterization. Glyphs are packed into a single-channel (R8)
//! texture using a shelf-based bin-packing algorithm.
//!
//! The renderer uploads [`pixels()`] to a GPU texture whenever
//! [`is_dirty()`] returns true.

use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, CacheKey, FontSystem, Metrics, Placement, Shaping, SwashCache, SwashContent,
};

/// Default atlas size: 2048 × 2048 (4 MB single-channel).
const ATLAS_WIDTH: u32 = 2048;
const ATLAS_HEIGHT: u32 = 2048;

/// Padding between packed glyphs (prevents texture bleeding).
const GLYPH_PAD: u32 = 1;

/// A positioned glyph ready for GPU instanced rendering.
#[derive(Debug, Clone, Copy)]
pub struct PositionedGlyph {
    /// Screen-space rect: [x, y, width, height] in pixels.
    pub rect: [f32; 4],
    /// Atlas UV rect: [u0, v0, u1, v1] normalised 0..1.
    pub uv: [f32; 4],
    /// Text color: [r, g, b, a] normalised 0..1.
    pub color: [f32; 4],
}

/// Cached atlas entry for a single glyph.
#[derive(Debug, Clone, Copy)]
struct GlyphEntry {
    /// UV coordinates in [u0, v0, u1, v1].
    uv: [f32; 4],
    /// Pixel placement from swash (left/top offsets).
    placement: Placement,
    /// Pixel width/height of the rasterized glyph.
    width: u32,
    height: u32,
    /// Frame counter when this glyph was last used.
    last_used_frame: u64,
}

/// A horizontal shelf in the atlas for packing glyphs.
#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

/// CPU-side glyph atlas with shelf-based packing and cosmic-text rasterization.
pub struct GlyphAtlas {
    font_system: FontSystem,
    swash_cache: SwashCache,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    shelves: Vec<Shelf>,
    entries: HashMap<CacheKey, GlyphEntry>,
    dirty: bool,
    /// Current frame counter, incremented each call to `begin_frame`.
    frame: u64,
}

impl GlyphAtlas {
    /// Create a new glyph atlas, loading system fonts.
    pub fn new() -> Self {
        let pixel_count = (ATLAS_WIDTH * ATLAS_HEIGHT) as usize;
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            pixels: vec![0u8; pixel_count],
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            shelves: Vec::new(),
            entries: HashMap::new(),
            dirty: false,
            frame: 0,
        }
    }

    /// Shape a text string, rasterize any missing glyphs into the atlas,
    /// and return positioned glyph instances ready for the GPU.
    pub fn prepare_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        line_height: f32,
        color: [f32; 4],
    ) -> Vec<PositionedGlyph> {
        if text.is_empty() {
            return Vec::new();
        }

        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(&mut self.font_system, Some(f32::MAX), None);
        buffer.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            let run_y = y + run.line_y;

            for layout_glyph in run.glyphs.iter() {
                let physical = layout_glyph.physical((0.0, 0.0), 1.0);

                // Ensure glyph is rasterized and packed into the atlas.
                let entry = match self.ensure_glyph(physical.cache_key) {
                    Some(e) => e,
                    None => continue, // skip glyphs that fail to rasterize
                };

                // Screen position = text origin + physical glyph offset + placement offset.
                let gx = x + physical.x as f32 + entry.placement.left as f32;
                let gy = run_y + physical.y as f32 - entry.placement.top as f32;

                glyphs.push(PositionedGlyph {
                    rect: [gx, gy, entry.width as f32, entry.height as f32],
                    uv: entry.uv,
                    color,
                });
            }
        }

        glyphs
    }

    /// Returns `true` if the atlas pixel data has changed since the last
    /// call to [`mark_clean`].
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Raw R8 pixel data for GPU upload.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Atlas dimensions in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Mark atlas as clean after uploading to the GPU.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Number of cached glyph entries.
    pub fn cached_count(&self) -> usize {
        self.entries.len()
    }

    /// Call at the start of each frame to advance the frame counter.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Evict glyphs unused for `stale_frames` frames.
    ///
    /// Clears the atlas completely and re-packs remaining glyphs on the next
    /// `prepare_text` call. Returns the number of evicted entries.
    pub fn evict_stale(&mut self, stale_frames: u64) -> usize {
        let threshold = self.frame.saturating_sub(stale_frames);
        let before = self.entries.len();

        self.entries
            .retain(|_, entry| entry.last_used_frame >= threshold);

        let evicted = before - self.entries.len();
        if evicted > 0 {
            // Full atlas rebuild required — clear pixels and shelves.
            self.pixels.fill(0);
            self.shelves.clear();
            self.entries.clear(); // All entries lose their UV positions.
            self.dirty = true;
        }
        evicted
    }

    /// Fraction of atlas capacity used (approximate, based on shelf coverage).
    pub fn usage_fraction(&self) -> f32 {
        let used_height: u32 = self.shelves.iter().map(|s| s.height).sum();
        used_height as f32 / self.height as f32
    }

    // ── internal ─────────────────────────────────────────────

    /// Ensure a glyph is rasterized and packed in the atlas. Returns its entry.
    fn ensure_glyph(&mut self, cache_key: CacheKey) -> Option<GlyphEntry> {
        // Already cached? Update usage timestamp and return.
        if let Some(entry) = self.entries.get_mut(&cache_key) {
            entry.last_used_frame = self.frame;
            return Some(*entry);
        }

        // Rasterize via cosmic-text's swash integration.
        // Use `get_image_uncached` to get an owned `SwashImage` —
        // avoids borrow conflicts with `self.pixels` / `self.shelves`.
        let image = self
            .swash_cache
            .get_image_uncached(&mut self.font_system, cache_key)?;

        // Only handle alpha (grayscale) content.
        match image.content {
            SwashContent::Mask => {}
            SwashContent::Color | SwashContent::SubpixelMask => {
                // For now, skip color emoji and subpixel glyphs.
                return None;
            }
        }

        let gw = image.placement.width;
        let gh = image.placement.height;

        // Zero-size glyphs (e.g. space) — cache a dummy entry.
        if gw == 0 || gh == 0 {
            let entry = GlyphEntry {
                uv: [0.0; 4],
                placement: image.placement,
                width: 0,
                height: 0,
                last_used_frame: self.frame,
            };
            self.entries.insert(cache_key, entry);
            return Some(entry);
        }

        // Pack into the atlas.
        let (px, py) = self.pack(gw, gh)?;

        // Blit glyph pixels into the atlas.
        for row in 0..gh {
            for col in 0..gw {
                let src_idx = (row * gw + col) as usize;
                let dst_idx = ((py + row) * self.width + (px + col)) as usize;
                if src_idx < image.data.len() && dst_idx < self.pixels.len() {
                    self.pixels[dst_idx] = image.data[src_idx];
                }
            }
        }
        self.dirty = true;

        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        let entry = GlyphEntry {
            uv: [
                px as f32 * inv_w,
                py as f32 * inv_h,
                (px + gw) as f32 * inv_w,
                (py + gh) as f32 * inv_h,
            ],
            placement: image.placement,
            width: gw,
            height: gh,
            last_used_frame: self.frame,
        };

        self.entries.insert(cache_key, entry);
        Some(entry)
    }

    /// Shelf-based packing: find or create a shelf that fits the glyph.
    fn pack(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let padded_w = w + GLYPH_PAD;
        let padded_h = h + GLYPH_PAD;

        // Try existing shelves (best-fit by height).
        let mut best_idx = None;
        let mut best_waste = u32::MAX;

        for (i, shelf) in self.shelves.iter().enumerate() {
            if shelf.height >= padded_h && shelf.next_x + padded_w <= self.width {
                let waste = shelf.height - padded_h;
                if waste < best_waste {
                    best_waste = waste;
                    best_idx = Some(i);
                }
            }
        }

        if let Some(idx) = best_idx {
            let shelf = &mut self.shelves[idx];
            let px = shelf.next_x;
            let py = shelf.y;
            shelf.next_x += padded_w;
            return Some((px, py));
        }

        // Create a new shelf.
        let shelf_y = self.shelves.last().map(|s| s.y + s.height).unwrap_or(0);

        if shelf_y + padded_h > self.height {
            // Atlas full.
            return None;
        }

        let px = 0;
        let py = shelf_y;
        self.shelves.push(Shelf {
            y: shelf_y,
            height: padded_h,
            next_x: padded_w,
        });

        Some((px, py))
    }
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_atlas_is_clean() {
        let atlas = GlyphAtlas::new();
        assert!(!atlas.is_dirty());
        assert_eq!(atlas.cached_count(), 0);
        assert_eq!(atlas.dimensions(), (ATLAS_WIDTH, ATLAS_HEIGHT));
    }

    #[test]
    fn prepare_empty_text_returns_empty() {
        let mut atlas = GlyphAtlas::new();
        let glyphs = atlas.prepare_text("", 0.0, 0.0, 16.0, 20.0, [1.0; 4]);
        assert!(glyphs.is_empty());
    }

    #[test]
    fn prepare_text_produces_glyphs() {
        let mut atlas = GlyphAtlas::new();
        let glyphs = atlas.prepare_text("Hello", 10.0, 20.0, 16.0, 20.0, [1.0, 1.0, 1.0, 1.0]);
        // "Hello" should produce at least 4 visible glyphs (l has same shape but different positions)
        assert!(!glyphs.is_empty(), "expected glyphs for 'Hello', got none");
        // All glyphs should have valid UV coordinates.
        for g in &glyphs {
            assert!(g.uv[2] >= g.uv[0], "u1 >= u0");
            assert!(g.uv[3] >= g.uv[1], "v1 >= v0");
        }
    }

    #[test]
    fn atlas_caches_glyphs() {
        let mut atlas = GlyphAtlas::new();
        let _ = atlas.prepare_text("AB", 0.0, 0.0, 16.0, 20.0, [1.0; 4]);
        let count_after_first = atlas.cached_count();
        assert!(count_after_first > 0);

        // Same text again — cache should not grow.
        let _ = atlas.prepare_text("AB", 50.0, 0.0, 16.0, 20.0, [1.0; 4]);
        assert_eq!(atlas.cached_count(), count_after_first);
    }

    #[test]
    fn atlas_dirty_after_new_glyphs() {
        let mut atlas = GlyphAtlas::new();
        let _ = atlas.prepare_text("X", 0.0, 0.0, 16.0, 20.0, [1.0; 4]);
        assert!(atlas.is_dirty());

        atlas.mark_clean();
        assert!(!atlas.is_dirty());

        // Same glyph — should not re-dirty.
        let _ = atlas.prepare_text("X", 50.0, 0.0, 16.0, 20.0, [1.0; 4]);
        assert!(!atlas.is_dirty());
    }

    #[test]
    fn shelf_packing_fills_rows() {
        let mut atlas = GlyphAtlas::new();
        // Prepare many different characters to exercise shelf packing.
        let _ = atlas.prepare_text(
            "abcdefghijklmnopqrstuvwxyz0123456789",
            0.0,
            0.0,
            24.0,
            30.0,
            [1.0; 4],
        );
        assert!(
            atlas.cached_count() > 10,
            "many unique glyphs should be cached"
        );
        assert!(atlas.is_dirty());
    }

    #[test]
    fn positioned_glyphs_have_correct_color() {
        let mut atlas = GlyphAtlas::new();
        let color = [0.5, 0.3, 0.8, 1.0];
        let glyphs = atlas.prepare_text("Test", 0.0, 0.0, 16.0, 20.0, color);
        for g in &glyphs {
            assert_eq!(g.color, color);
        }
    }

    #[test]
    fn evict_stale_clears_atlas() {
        let mut atlas = GlyphAtlas::new();
        atlas.begin_frame(); // frame 1
        let _ = atlas.prepare_text("Old", 0.0, 0.0, 16.0, 20.0, [1.0; 4]);
        let count = atlas.cached_count();
        assert!(count > 0);

        // Advance many frames without using those glyphs.
        for _ in 0..120 {
            atlas.begin_frame();
        }

        let evicted = atlas.evict_stale(60);
        assert!(evicted > 0, "stale glyphs should be evicted");
        assert_eq!(atlas.cached_count(), 0);
        assert!(atlas.is_dirty());
    }

    #[test]
    fn recently_used_glyphs_survive_eviction() {
        let mut atlas = GlyphAtlas::new();
        atlas.begin_frame();
        let _ = atlas.prepare_text("Keep", 0.0, 0.0, 16.0, 20.0, [1.0; 4]);

        // Only 5 frames later, try to evict with 60-frame threshold.
        for _ in 0..5 {
            atlas.begin_frame();
        }

        let evicted = atlas.evict_stale(60);
        assert_eq!(evicted, 0, "recent glyphs should survive");
    }

    #[test]
    fn usage_fraction_increases() {
        let mut atlas = GlyphAtlas::new();
        let before = atlas.usage_fraction();
        let _ = atlas.prepare_text("ABCDEFGHIJ", 0.0, 0.0, 48.0, 60.0, [1.0; 4]);
        let after = atlas.usage_fraction();
        assert!(after > before, "usage should increase after adding glyphs");
    }
}
