// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Tile grid management for raster/compositing scalability.
//!
//! This is a lightweight tile scheduler inspired by browser compositors:
//! - fixed-size tile grid over content space,
//! - dirty tracking by rect,
//! - viewport-visible tile selection for prioritized rasterization.

use vex_core::geometry::{Rect, Size};

/// One raster tile.
#[derive(Debug, Clone)]
pub struct Tile {
    pub id: u32,
    pub rect: Rect,
    pub dirty: bool,
}

/// Tile grid over a 2D content area.
#[derive(Debug, Clone)]
pub struct TileGrid {
    pub tile_size: u32,
    pub content_size: Size,
    tiles: Vec<Tile>,
}

impl TileGrid {
    /// Create a tile grid for `content_size`.
    pub fn new(content_size: Size, tile_size: u32) -> Self {
        let tile_size = tile_size.max(16);
        let cols = (content_size.width.max(0.0) / tile_size as f32).ceil() as u32;
        let rows = (content_size.height.max(0.0) / tile_size as f32).ceil() as u32;

        let mut tiles = Vec::with_capacity((cols * rows) as usize);
        let mut id = 0u32;
        for row in 0..rows {
            for col in 0..cols {
                let x = col as f32 * tile_size as f32;
                let y = row as f32 * tile_size as f32;
                let w = (content_size.width - x).min(tile_size as f32).max(0.0);
                let h = (content_size.height - y).min(tile_size as f32).max(0.0);
                tiles.push(Tile {
                    id,
                    rect: Rect::new(x, y, w, h),
                    dirty: true,
                });
                id += 1;
            }
        }

        Self {
            tile_size,
            content_size,
            tiles,
        }
    }

    /// Number of tiles in the grid.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Whether the grid has no tiles.
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Mark tiles intersecting `damage` as dirty.
    pub fn mark_dirty(&mut self, damage: Rect) {
        for tile in &mut self.tiles {
            if tile.rect.intersects(&damage) {
                tile.dirty = true;
            }
        }
    }

    /// Clear all dirty flags.
    pub fn clear_dirty(&mut self) {
        for tile in &mut self.tiles {
            tile.dirty = false;
        }
    }

    /// Count dirty tiles.
    pub fn dirty_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.dirty).count()
    }

    /// Return IDs of tiles intersecting the viewport rect.
    pub fn visible_tile_ids(&self, viewport: Rect) -> Vec<u32> {
        self.tiles
            .iter()
            .filter(|t| t.rect.intersects(&viewport))
            .map(|t| t.id)
            .collect()
    }

    /// Return dirty tile IDs intersecting viewport (priority raster set).
    pub fn dirty_visible_tile_ids(&self, viewport: Rect) -> Vec<u32> {
        self.tiles
            .iter()
            .filter(|t| t.dirty && t.rect.intersects(&viewport))
            .map(|t| t.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_builds_tiles() {
        let grid = TileGrid::new(Size::new(512.0, 512.0), 256);
        assert_eq!(grid.len(), 4);
    }

    #[test]
    fn mark_dirty_hits_intersecting_tiles() {
        let mut grid = TileGrid::new(Size::new(512.0, 512.0), 256);
        grid.clear_dirty();
        assert_eq!(grid.dirty_count(), 0);

        grid.mark_dirty(Rect::new(300.0, 20.0, 40.0, 40.0));
        assert_eq!(grid.dirty_count(), 1);
    }

    #[test]
    fn visible_tiles_for_viewport() {
        let grid = TileGrid::new(Size::new(1024.0, 1024.0), 256);
        let ids = grid.visible_tile_ids(Rect::new(0.0, 0.0, 400.0, 400.0));
        assert_eq!(ids.len(), 4);
    }
}
