// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Float layout.
//!
//! Implements the CSS 2.1 §9.5.1 float positioning model:
//! - Left and right floats are placed in the available space.
//! - Non-floated content flows around floats.
//! - `clear` property moves content below cleared floats.
//!
//! This module provides a [`FloatContext`] that tracks placed floats
//! and computes available widths for both floated and non-floated boxes.

use std::collections::HashMap;

use vex_core::VexId;
use vex_css::values::box_model::{Clear, Float};
use vex_css::ComputedStyle;
use vex_dom::NodeArena;

use crate::block::{layout_block, ContainingBlock};
use crate::box_model::LayoutBox;
use crate::text::TextEngine;

/// A placed float — records the occupied region.
#[derive(Debug, Clone, Copy)]
pub struct PlacedFloat {
    /// Top edge of the float in the containing block's coordinate space.
    pub top: f32,
    /// Bottom edge of the float.
    pub bottom: f32,
    /// Left edge of the float.
    pub left: f32,
    /// Right edge of the float.
    pub right: f32,
    /// Whether this is a left or right float.
    pub side: Float,
}

/// Tracks all floats in the current block formatting context (BFC).
///
/// Used during block layout to position floated elements and compute
/// available widths for non-floated content.
#[derive(Debug, Clone, Default)]
pub struct FloatContext {
    /// All placed floats in this BFC.
    floats: Vec<PlacedFloat>,
    /// Width of the containing block.
    containing_width: f32,
}

/// The available horizontal band at a given vertical position.
#[derive(Debug, Clone, Copy)]
pub struct AvailableBand {
    /// Left edge of available space.
    pub left: f32,
    /// Right edge of available space.
    pub right: f32,
}

impl AvailableBand {
    /// Width of the available band.
    pub fn width(&self) -> f32 {
        (self.right - self.left).max(0.0)
    }
}

impl FloatContext {
    /// Create a new float context for a containing block of the given width.
    pub fn new(containing_width: f32) -> Self {
        Self {
            floats: Vec::new(),
            containing_width,
        }
    }

    /// Compute the available horizontal band at the given y position.
    ///
    /// Returns the left and right edges taking into account all floats
    /// whose vertical range includes `y`.
    pub fn available_at(&self, y: f32) -> AvailableBand {
        let mut left = 0.0_f32;
        let mut right = self.containing_width;

        for f in &self.floats {
            if y >= f.top && y < f.bottom {
                match f.side {
                    Float::Left => left = left.max(f.right),
                    Float::Right => right = right.min(f.left),
                    Float::None => {}
                }
            }
        }

        AvailableBand { left, right }
    }

    /// Find the y position at which the given width is available.
    ///
    /// Starting from `start_y`, looks for a vertical position where
    /// the available width is at least `needed_width`.
    pub fn find_available_y(&self, start_y: f32, needed_width: f32) -> f32 {
        let mut y = start_y;

        // Collect all unique transition points (float bottoms)
        let mut transitions: Vec<f32> = self.floats.iter().map(|f| f.bottom).collect();
        transitions.push(start_y);
        transitions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        transitions.dedup();

        for &ty in &transitions {
            if ty < y {
                continue;
            }
            let band = self.available_at(ty);
            if band.width() >= needed_width {
                return ty;
            }
            y = ty;
        }

        // If all floats are narrower, try below them all
        if let Some(max_bottom) = self.floats.iter().map(|f| f.bottom).reduce(f32::max) {
            if y < max_bottom {
                return max_bottom;
            }
        }

        y
    }

    /// Place a float and record it.
    pub fn place_float(&mut self, float: PlacedFloat) {
        self.floats.push(float);
    }

    /// Compute the y position needed to clear floats of the given side.
    ///
    /// Returns the y value that is below all floats on the cleared side.
    pub fn clear_y(&self, clear: Clear) -> f32 {
        let mut y = 0.0_f32;
        for f in &self.floats {
            let dominated = match clear {
                Clear::Left => f.side == Float::Left,
                Clear::Right => f.side == Float::Right,
                Clear::Both => true,
                Clear::None => false,
            };
            if dominated {
                y = y.max(f.bottom);
            }
        }
        y
    }

    /// Get the bottom of all floats (for auto height calculation).
    pub fn float_bottom(&self) -> f32 {
        self.floats
            .iter()
            .map(|f| f.bottom)
            .reduce(f32::max)
            .unwrap_or(0.0)
    }
}

/// Layout block children with float support.
///
/// This replaces the simple vertical stacking when any child has
/// `float` or `clear` properties set. It creates a [`FloatContext`]
/// and positions children accordingly.
pub fn layout_block_children_with_floats(
    layout_box: &mut LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    arena: &NodeArena,
    text_engine: &mut TextEngine,
) {
    let content_x = layout_box.dimensions.content.origin.x;
    let content_y = layout_box.dimensions.content.origin.y;
    let content_width = layout_box.dimensions.content.size.width;

    let containing = ContainingBlock {
        width: content_width,
        height: layout_box.dimensions.content.size.height,
    };

    let mut float_ctx = FloatContext::new(content_width);
    let mut cursor_y = 0.0_f32;
    let mut prev_margin_bottom = 0.0_f32;

    let mut children = std::mem::take(&mut layout_box.children);

    for child in &mut children {
        let child_style = child.node_id.and_then(|id| styles.get(&id));
        let float_side = child_style.map(|s| s.float).unwrap_or(Float::None);
        let clear_side = child_style.map(|s| s.clear).unwrap_or(Clear::None);

        // Handle clear — move cursor below cleared floats.
        if clear_side != Clear::None {
            let clear_y = float_ctx.clear_y(clear_side);
            if clear_y > cursor_y {
                cursor_y = clear_y;
                prev_margin_bottom = 0.0;
            }
        }

        if float_side != Float::None {
            // ── Float layout ─────────────────────────────────────
            layout_block(child, containing, styles, arena, text_engine);

            let float_width = child.dimensions.margin_box().size.width;
            let float_height = child.dimensions.margin_box().size.height;

            // Find the y position where the float fits.
            let place_y = float_ctx.find_available_y(cursor_y, float_width);
            let band = float_ctx.available_at(place_y);

            let (float_x, placed_left, placed_right) = match float_side {
                Float::Left => {
                    let x = band.left;
                    (x, x, x + float_width)
                }
                Float::Right => {
                    let x = band.right - float_width;
                    (x, x, band.right)
                }
                Float::None => unreachable!(),
            };

            // Position the float's content area.
            child.dimensions.content.origin.x = content_x
                + float_x
                + child.dimensions.margin.left
                + child.dimensions.padding.left
                + child.dimensions.border.left;
            child.dimensions.content.origin.y = content_y
                + place_y
                + child.dimensions.margin.top
                + child.dimensions.padding.top
                + child.dimensions.border.top;

            float_ctx.place_float(PlacedFloat {
                top: place_y,
                bottom: place_y + float_height,
                left: placed_left,
                right: placed_right,
                side: float_side,
            });
        } else {
            // ── Normal flow (respecting floats) ──────────────────
            layout_block(child, containing, styles, arena, text_engine);

            // Margin collapsing.
            let child_margin_top = child.dimensions.margin.top;
            let collapsed = crate::block::collapse_margins(prev_margin_bottom, child_margin_top);
            cursor_y += collapsed - prev_margin_bottom;

            // Find a y position where the child fits alongside floats.
            let child_width = child.dimensions.margin_box().size.width;
            let place_y = float_ctx.find_available_y(cursor_y, child_width);
            let band = float_ctx.available_at(place_y);

            child.dimensions.content.origin.x = content_x
                + band.left
                + child.dimensions.margin.left
                + child.dimensions.padding.left
                + child.dimensions.border.left;
            child.dimensions.content.origin.y = content_y
                + place_y
                + child.dimensions.margin.top
                + child.dimensions.padding.top
                + child.dimensions.border.top;

            cursor_y = place_y + child.dimensions.margin_box().size.height;
            prev_margin_bottom = child.dimensions.margin.bottom;
        }
    }

    layout_box.children = children;

    // Extend auto-height to contain floats (CSS 2.1 §10.6.7).
    let float_bottom = float_ctx.float_bottom();
    if float_bottom > cursor_y {
        // The containing block needs to be tall enough for floats.
        // This is stored for height calculation.
        layout_box.dimensions.content.size.height =
            layout_box.dimensions.content.size.height.max(float_bottom);
    }
}

/// Check if any direct child of the layout box has float or clear.
pub fn has_floats_or_clears(
    layout_box: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
) -> bool {
    layout_box.children.iter().any(|child| {
        child
            .node_id
            .and_then(|id| styles.get(&id))
            .is_some_and(|s| s.float != Float::None || s.clear != Clear::None)
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_float_context() {
        let ctx = FloatContext::new(800.0);
        let band = ctx.available_at(0.0);
        assert_eq!(band.left, 0.0);
        assert_eq!(band.right, 800.0);
        assert_eq!(band.width(), 800.0);
    }

    #[test]
    fn left_float_reduces_available_width() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 100.0,
            left: 0.0,
            right: 200.0,
            side: Float::Left,
        });

        let band = ctx.available_at(50.0);
        assert_eq!(band.left, 200.0);
        assert_eq!(band.right, 800.0);
        assert_eq!(band.width(), 600.0);

        // Below the float, full width available.
        let band_below = ctx.available_at(100.0);
        assert_eq!(band_below.left, 0.0);
        assert_eq!(band_below.right, 800.0);
    }

    #[test]
    fn right_float_reduces_available_width() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 100.0,
            left: 600.0,
            right: 800.0,
            side: Float::Right,
        });

        let band = ctx.available_at(50.0);
        assert_eq!(band.left, 0.0);
        assert_eq!(band.right, 600.0);
        assert_eq!(band.width(), 600.0);
    }

    #[test]
    fn both_floats_narrow_available_band() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 80.0,
            left: 0.0,
            right: 150.0,
            side: Float::Left,
        });
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 60.0,
            left: 650.0,
            right: 800.0,
            side: Float::Right,
        });

        let band = ctx.available_at(30.0);
        assert_eq!(band.left, 150.0);
        assert_eq!(band.right, 650.0);
        assert_eq!(band.width(), 500.0);

        // After right float clears, left float still active.
        let band_mid = ctx.available_at(70.0);
        assert_eq!(band_mid.left, 150.0);
        assert_eq!(band_mid.right, 800.0);
    }

    #[test]
    fn clear_left_moves_below_left_float() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 120.0,
            left: 0.0,
            right: 200.0,
            side: Float::Left,
        });

        assert_eq!(ctx.clear_y(Clear::Left), 120.0);
        assert_eq!(ctx.clear_y(Clear::Right), 0.0);
        assert_eq!(ctx.clear_y(Clear::Both), 120.0);
        assert_eq!(ctx.clear_y(Clear::None), 0.0);
    }

    #[test]
    fn clear_both_moves_below_all_floats() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 100.0,
            left: 0.0,
            right: 200.0,
            side: Float::Left,
        });
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 150.0,
            left: 600.0,
            right: 800.0,
            side: Float::Right,
        });

        assert_eq!(ctx.clear_y(Clear::Both), 150.0);
        assert_eq!(ctx.clear_y(Clear::Left), 100.0);
        assert_eq!(ctx.clear_y(Clear::Right), 150.0);
    }

    #[test]
    fn find_available_y_with_narrow_band() {
        let mut ctx = FloatContext::new(400.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 80.0,
            left: 0.0,
            right: 300.0,
            side: Float::Left,
        });

        // 200px wide element doesn't fit in the 100px gap.
        let y = ctx.find_available_y(0.0, 200.0);
        assert!(y >= 80.0);

        // 90px wide element fits alongside the float.
        let y2 = ctx.find_available_y(0.0, 90.0);
        assert_eq!(y2, 0.0);
    }

    #[test]
    fn float_bottom_returns_lowest_float() {
        let mut ctx = FloatContext::new(800.0);
        assert_eq!(ctx.float_bottom(), 0.0);

        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 50.0,
            left: 0.0,
            right: 100.0,
            side: Float::Left,
        });
        ctx.place_float(PlacedFloat {
            top: 20.0,
            bottom: 150.0,
            left: 700.0,
            right: 800.0,
            side: Float::Right,
        });

        assert_eq!(ctx.float_bottom(), 150.0);
    }

    #[test]
    fn stacked_left_floats() {
        let mut ctx = FloatContext::new(800.0);
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 50.0,
            left: 0.0,
            right: 200.0,
            side: Float::Left,
        });
        ctx.place_float(PlacedFloat {
            top: 0.0,
            bottom: 50.0,
            left: 200.0,
            right: 400.0,
            side: Float::Left,
        });

        let band = ctx.available_at(25.0);
        assert_eq!(band.left, 400.0);
        assert_eq!(band.right, 800.0);
    }
}
