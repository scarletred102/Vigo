// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Image atlas — CPU-side texture atlas for decoded images.
//!
//! Shelf-packed RGBA8 texture. The renderer uploads [`pixels()`] to a
//! GPU texture whenever [`is_dirty()`] returns true.

use std::collections::HashMap;

use crate::display_list::ImageId;
use crate::image_decode::DecodedImage;

/// Default image atlas size: 4096 × 4096 (64 MB RGBA).
const ATLAS_WIDTH: u32 = 4096;
const ATLAS_HEIGHT: u32 = 4096;

/// Padding between packed images (prevents texture bleeding).
const IMAGE_PAD: u32 = 1;

/// Atlas entry for a single uploaded image.
#[derive(Debug, Clone, Copy)]
pub struct ImageEntry {
    /// UV coordinates [u0, v0, u1, v1] normalised 0..1.
    pub uv: [f32; 4],
    /// Pixel dimensions of the image in the atlas.
    pub width: u32,
    pub height: u32,
}

/// A horizontal shelf in the atlas.
#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

/// CPU-side image atlas with shelf-based packing.
pub struct ImageAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    shelves: Vec<Shelf>,
    entries: HashMap<ImageId, ImageEntry>,
    dirty: bool,
}

impl ImageAtlas {
    /// Create an empty image atlas.
    pub fn new() -> Self {
        let pixel_count = (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize; // RGBA
        Self {
            pixels: vec![0u8; pixel_count],
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            shelves: Vec::new(),
            entries: HashMap::new(),
            dirty: false,
        }
    }

    /// Upload a decoded image into the atlas. Returns the atlas entry with UVs.
    ///
    /// If this `ImageId` was already uploaded, returns the existing entry.
    /// Returns `None` if the image doesn't fit.
    pub fn upload(&mut self, id: ImageId, image: &DecodedImage) -> Option<ImageEntry> {
        if let Some(entry) = self.entries.get(&id) {
            return Some(*entry);
        }

        if image.width == 0 || image.height == 0 {
            return None;
        }

        let (px, py) = self.pack(image.width, image.height)?;

        // Blit RGBA pixels into the atlas.
        for row in 0..image.height {
            let src_start = (row * image.width * 4) as usize;
            let src_end = src_start + (image.width * 4) as usize;
            let dst_start = ((py + row) * self.width * 4 + px * 4) as usize;

            if src_end <= image.pixels.len() {
                let dst_end = dst_start + (image.width * 4) as usize;
                if dst_end <= self.pixels.len() {
                    self.pixels[dst_start..dst_end]
                        .copy_from_slice(&image.pixels[src_start..src_end]);
                }
            }
        }
        self.dirty = true;

        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        let entry = ImageEntry {
            uv: [
                px as f32 * inv_w,
                py as f32 * inv_h,
                (px + image.width) as f32 * inv_w,
                (py + image.height) as f32 * inv_h,
            ],
            width: image.width,
            height: image.height,
        };

        self.entries.insert(id, entry);
        Some(entry)
    }

    /// Look up UVs for an already-uploaded image.
    pub fn get(&self, id: ImageId) -> Option<&ImageEntry> {
        self.entries.get(&id)
    }

    /// Returns `true` if pixel data has changed since last [`mark_clean`].
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Raw RGBA8 pixel data for GPU upload.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Atlas dimensions in pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Mark atlas as clean after uploading to GPU.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Number of cached images.
    pub fn cached_count(&self) -> usize {
        self.entries.len()
    }

    // ── internal ─────────────────────────────────────────

    fn pack(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let padded_w = w + IMAGE_PAD;
        let padded_h = h + IMAGE_PAD;

        // Best-fit existing shelf.
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

        // New shelf.
        let shelf_y = self.shelves.last().map(|s| s.y + s.height).unwrap_or(0);

        if shelf_y + padded_h > self.height {
            return None;
        }

        self.shelves.push(Shelf {
            y: shelf_y,
            height: padded_h,
            next_x: padded_w,
        });

        Some((0, shelf_y))
    }
}

impl Default for ImageAtlas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(w: u32, h: u32) -> DecodedImage {
        // Solid red RGBA.
        let pixels = [255, 0, 0, 255].repeat((w * h) as usize);
        DecodedImage {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn upload_and_retrieve() {
        let mut atlas = ImageAtlas::new();
        let img = make_test_image(64, 64);
        let id = ImageId(1);

        let entry = atlas.upload(id, &img).expect("should fit");
        assert_eq!(entry.width, 64);
        assert_eq!(entry.height, 64);
        assert!(entry.uv[2] > entry.uv[0]);
        assert!(atlas.is_dirty());
    }

    #[test]
    fn duplicate_upload_returns_same_entry() {
        let mut atlas = ImageAtlas::new();
        let img = make_test_image(32, 32);
        let id = ImageId(42);

        let e1 = atlas.upload(id, &img).unwrap();
        let e2 = atlas.upload(id, &img).unwrap();
        assert_eq!(e1.uv, e2.uv);
        assert_eq!(atlas.cached_count(), 1);
    }

    #[test]
    fn multiple_images_pack() {
        let mut atlas = ImageAtlas::new();
        for i in 0..10 {
            let img = make_test_image(100, 100);
            assert!(atlas.upload(ImageId(i), &img).is_some());
        }
        assert_eq!(atlas.cached_count(), 10);
    }

    #[test]
    fn get_returns_entry() {
        let mut atlas = ImageAtlas::new();
        let id = ImageId(7);
        assert!(atlas.get(id).is_none());

        let img = make_test_image(16, 16);
        atlas.upload(id, &img);
        assert!(atlas.get(id).is_some());
    }

    #[test]
    fn zero_size_image_rejected() {
        let mut atlas = ImageAtlas::new();
        let img = DecodedImage {
            width: 0,
            height: 0,
            pixels: vec![],
        };
        assert!(atlas.upload(ImageId(1), &img).is_none());
    }

    #[test]
    fn dirty_clears_after_mark_clean() {
        let mut atlas = ImageAtlas::new();
        let img = make_test_image(8, 8);
        atlas.upload(ImageId(1), &img);
        assert!(atlas.is_dirty());
        atlas.mark_clean();
        assert!(!atlas.is_dirty());
    }
}
