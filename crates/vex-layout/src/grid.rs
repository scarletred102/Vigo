// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Grid Layout algorithm.
//!
//! Implements `display: grid` containers: explicit track sizing,
//! auto-placement, `fr` unit distribution, and gap support.
//!
//! Reference: CSS Grid Layout Module Level 1.

use std::collections::HashMap;

use vex_core::{Insets, VexId};
use vex_css::values::box_model::BoxSizing;
use vex_css::values::grid::{GridAutoFlow, GridLine, TrackSize};
use vex_css::ComputedStyle;

use crate::block::{layout_block, ContainingBlock};
use crate::box_model::LayoutBox;

/// Perform grid layout on a grid container and its children.
pub fn layout_grid(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    resolve_grid_container_size(layout_box, containing, styles);

    let style = layout_box.node_id.and_then(|id| styles.get(&id));

    let col_tracks = style
        .map(|s| s.grid_template_columns.0.clone())
        .unwrap_or_default();
    let row_tracks = style
        .map(|s| s.grid_template_rows.0.clone())
        .unwrap_or_default();
    let auto_flow = style.map(|s| s.grid_auto_flow).unwrap_or(GridAutoFlow::Row);
    let col_gap = style.map(|s| s.grid_column_gap).unwrap_or(0.0);
    let row_gap = style.map(|s| s.grid_row_gap).unwrap_or(0.0);

    let container_width = layout_box.dimensions.content.size.width;
    let container_height = layout_box.dimensions.content.size.height;

    let child_count = layout_box.children.len();
    if child_count == 0 {
        return;
    }

    // ── Step 1: Determine grid dimensions ────────────────────────────
    let explicit_cols = col_tracks.len();
    let explicit_rows = row_tracks.len();

    // Collect explicit placements to determine grid size.
    let mut placements: Vec<GridPlacement> = Vec::with_capacity(child_count);
    let mut max_col: usize = explicit_cols;
    let mut max_row: usize = explicit_rows;

    for child in layout_box.children.iter() {
        let child_style = child.node_id.and_then(|id| styles.get(&id));
        let col_start = child_style
            .map(|s| s.grid_column_start)
            .unwrap_or(GridLine::Auto);
        let col_end = child_style
            .map(|s| s.grid_column_end)
            .unwrap_or(GridLine::Auto);
        let row_start = child_style
            .map(|s| s.grid_row_start)
            .unwrap_or(GridLine::Auto);
        let row_end = child_style
            .map(|s| s.grid_row_end)
            .unwrap_or(GridLine::Auto);

        let placement = resolve_placement(col_start, col_end, row_start, row_end);
        if let Some(c) = placement.col_end {
            max_col = max_col.max(c);
        }
        if let Some(r) = placement.row_end {
            max_row = max_row.max(r);
        }
        placements.push(placement);
    }

    // If no explicit tracks, compute from item count and auto-flow.
    let num_cols = if explicit_cols > 0 {
        max_col.max(explicit_cols)
    } else if auto_flow.is_row() {
        // Default: auto rows → we need to decide column count.
        // Use ceil(sqrt(n)) or just 1 column if no template.
        let n = child_count;
        (n as f64).sqrt().ceil() as usize
    } else {
        1
    };
    let num_cols = num_cols.max(1);

    // Auto-place items that don't have explicit positions.
    auto_place_items(&mut placements, num_cols, auto_flow);

    // Determine total rows from placements.
    let mut num_rows = explicit_rows;
    for p in &placements {
        if let Some(end) = p.row_end {
            num_rows = num_rows.max(end);
        }
    }
    num_rows = num_rows.max(1);

    // ── Step 2: Resolve track sizes ──────────────────────────────────
    let total_col_gaps = if num_cols > 1 {
        col_gap * (num_cols - 1) as f32
    } else {
        0.0
    };
    let total_row_gaps = if num_rows > 1 {
        row_gap * (num_rows - 1) as f32
    } else {
        0.0
    };

    let col_sizes = resolve_track_sizes(&col_tracks, num_cols, container_width - total_col_gaps);

    // For row sizing: use explicit container height if set, otherwise use containing
    // block height as available space (allows auto rows to have non-zero height).
    let available_height = if container_height > 0.0 {
        container_height
    } else {
        containing.height
    };
    let row_sizes = resolve_track_sizes(
        &row_tracks,
        num_rows,
        if available_height.is_finite() && !available_height.is_nan() {
            (available_height - total_row_gaps).max(0.0)
        } else {
            0.0
        },
    );

    // ── Step 3: Compute cumulative offsets ────────────────────────────
    let col_offsets = cumulative_offsets(&col_sizes, col_gap);
    let row_offsets = cumulative_offsets(&row_sizes, row_gap);

    // ── Step 4: Position each child ──────────────────────────────────
    let container_x = layout_box.dimensions.content.origin.x;
    let container_y = layout_box.dimensions.content.origin.y;

    let mut children = std::mem::take(&mut layout_box.children);

    for (i, child) in children.iter_mut().enumerate() {
        let p = &placements[i];
        let col_start = p.col_start.unwrap_or(0);
        let col_end = p.col_end.unwrap_or(col_start + 1);
        let row_start = p.row_start.unwrap_or(0);
        let row_end = p.row_end.unwrap_or(row_start + 1);

        let x = col_offsets.get(col_start).copied().unwrap_or(0.0);
        let y = row_offsets.get(row_start).copied().unwrap_or(0.0);

        // Width = sum of spanned columns + gaps between them.
        let mut w: f32 = col_sizes[col_start..col_end.min(col_sizes.len())]
            .iter()
            .sum();
        if col_end > col_start + 1 {
            w += col_gap * (col_end - col_start - 1) as f32;
        }

        // Height = sum of spanned rows + gaps between them.
        let mut h: f32 = row_sizes[row_start..row_end.min(row_sizes.len())]
            .iter()
            .sum();
        if row_end > row_start + 1 {
            h += row_gap * (row_end - row_start - 1) as f32;
        }

        // Resolve child margins/padding/borders.
        let child_style = child.node_id.and_then(|id| styles.get(&id));
        let (margins, padding, borders) = resolve_edges(child_style);

        child.dimensions.margin = margins;
        child.dimensions.padding = padding;
        child.dimensions.border = borders;

        let content_w = (w
            - margins.left
            - margins.right
            - padding.left
            - padding.right
            - borders.left
            - borders.right)
            .max(0.0);
        let content_h = (h
            - margins.top
            - margins.bottom
            - padding.top
            - padding.bottom
            - borders.top
            - borders.bottom)
            .max(0.0);

        child.dimensions.content.origin.x =
            container_x + x + margins.left + padding.left + borders.left;
        child.dimensions.content.origin.y =
            container_y + y + margins.top + padding.top + borders.top;
        child.dimensions.content.size.width = content_w;
        child.dimensions.content.size.height = content_h;

        // Recursively layout child contents.
        let child_containing = ContainingBlock {
            width: content_w,
            height: content_h,
        };
        for grandchild in &mut child.children {
            layout_block(grandchild, child_containing, styles);
        }
    }

    layout_box.children = children;

    // ── Step 5: Update container height if auto ──────────────────────
    if layout_box.dimensions.content.size.height.is_nan()
        || layout_box.dimensions.content.size.height == 0.0
    {
        let total_h: f32 = row_sizes.iter().sum::<f32>() + total_row_gaps;
        layout_box.dimensions.content.size.height = total_h;
    }
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Partially resolved grid placement for a single item.
#[derive(Debug, Default)]
struct GridPlacement {
    col_start: Option<usize>,
    col_end: Option<usize>,
    row_start: Option<usize>,
    row_end: Option<usize>,
}

/// Resolve explicit GridLine values into zero-based indices.
fn resolve_placement(
    col_start: GridLine,
    col_end: GridLine,
    row_start: GridLine,
    row_end: GridLine,
) -> GridPlacement {
    let mut p = GridPlacement::default();

    match col_start {
        GridLine::Line(n) if n > 0 => p.col_start = Some((n - 1) as usize),
        _ => {}
    }
    match col_end {
        GridLine::Line(n) if n > 0 => p.col_end = Some((n - 1) as usize),
        GridLine::Span(n) => {
            if let Some(start) = p.col_start {
                p.col_end = Some(start + n as usize);
            }
        }
        _ => {
            if let Some(start) = p.col_start {
                p.col_end = Some(start + 1);
            }
        }
    }

    match row_start {
        GridLine::Line(n) if n > 0 => p.row_start = Some((n - 1) as usize),
        _ => {}
    }
    match row_end {
        GridLine::Line(n) if n > 0 => p.row_end = Some((n - 1) as usize),
        GridLine::Span(n) => {
            if let Some(start) = p.row_start {
                p.row_end = Some(start + n as usize);
            }
        }
        _ => {
            if let Some(start) = p.row_start {
                p.row_end = Some(start + 1);
            }
        }
    }

    p
}

/// Auto-place items that don't have explicit positions.
fn auto_place_items(placements: &mut [GridPlacement], num_cols: usize, flow: GridAutoFlow) {
    let mut cursor_col: usize = 0;
    let mut cursor_row: usize = 0;
    let total_items = placements.len();

    for p in placements.iter_mut() {
        if p.col_start.is_some() && p.row_start.is_some() {
            // Fully placed — skip.
            continue;
        }

        if flow.is_row() {
            // Row-major auto-placement.
            if p.col_start.is_none() {
                p.col_start = Some(cursor_col);
                p.col_end = Some(cursor_col + 1);
            }
            if p.row_start.is_none() {
                p.row_start = Some(cursor_row);
                p.row_end = Some(cursor_row + 1);
            }

            cursor_col += 1;
            if cursor_col >= num_cols {
                cursor_col = 0;
                cursor_row += 1;
            }
        } else {
            // Column-major auto-placement.
            if p.row_start.is_none() {
                p.row_start = Some(cursor_row);
                p.row_end = Some(cursor_row + 1);
            }
            if p.col_start.is_none() {
                p.col_start = Some(cursor_col);
                p.col_end = Some(cursor_col + 1);
            }

            cursor_row += 1;
            // Estimate row count for column-major flow.
            let estimated_rows = total_items.div_ceil(num_cols);
            if cursor_row >= estimated_rows.max(1) {
                cursor_row = 0;
                cursor_col += 1;
            }
        }
    }
}

/// Resolve track sizes: honor fixed/fr/auto, distribute fr space.
fn resolve_track_sizes(template: &[TrackSize], count: usize, available: f32) -> Vec<f32> {
    let mut sizes = Vec::with_capacity(count);
    let mut total_fixed: f32 = 0.0;
    let mut total_fr: f32 = 0.0;

    // First pass: collect fixed sizes and fr amounts.
    for i in 0..count {
        let track = template.get(i).cloned().unwrap_or(TrackSize::Auto);
        match &track {
            TrackSize::Px(v) => {
                sizes.push(SizeEntry::Fixed(*v));
                total_fixed += *v;
            }
            TrackSize::Fr(v) => {
                sizes.push(SizeEntry::Fractional(*v));
                total_fr += *v;
            }
            TrackSize::Auto | TrackSize::MinContent | TrackSize::MaxContent => {
                // Auto tracks get a fair share of remaining space.
                sizes.push(SizeEntry::Auto);
            }
            TrackSize::MinMax(min, max) => {
                let min_v = track_to_px(min, available);
                let max_v = track_to_px(max, available);
                sizes.push(SizeEntry::MinMax(min_v, max_v));
                total_fixed += min_v;
            }
        }
    }

    // Count auto tracks.
    let auto_count = sizes
        .iter()
        .filter(|s| matches!(s, SizeEntry::Auto))
        .count();

    // Available space for fr and auto tracks.
    let remaining = (available - total_fixed).max(0.0);

    // Second pass: resolve sizes.
    let mut result = Vec::with_capacity(count);
    for entry in &sizes {
        match entry {
            SizeEntry::Fixed(v) => result.push(*v),
            SizeEntry::Fractional(fr) => {
                if total_fr > 0.0 {
                    // Fr tracks share the remaining space (after fixed and auto minimum).
                    let auto_reserved = if auto_count > 0 {
                        // Reserve a small portion for auto tracks.
                        0.0
                    } else {
                        0.0
                    };
                    let fr_space = (remaining - auto_reserved).max(0.0);
                    result.push(fr_space * (fr / total_fr));
                } else {
                    result.push(0.0);
                }
            }
            SizeEntry::Auto => {
                // Auto tracks share remaining space not taken by fr tracks.
                if total_fr > 0.0 {
                    // If there are fr tracks, auto gets 0 (content-based is deferred).
                    result.push(0.0);
                } else if auto_count > 0 {
                    result.push(remaining / auto_count as f32);
                } else {
                    result.push(0.0);
                }
            }
            SizeEntry::MinMax(min, max) => {
                // MinMax: use min as base, but clamp to max.
                // If there's remaining space and no fr tracks, expand toward max.
                if total_fr > 0.0 {
                    result.push(*min);
                } else {
                    let share = remaining / (auto_count + 1).max(1) as f32;
                    result.push(share.clamp(*min, *max));
                }
            }
        }
    }

    result
}

#[derive(Debug)]
enum SizeEntry {
    Fixed(f32),
    Fractional(f32),
    Auto,
    MinMax(f32, f32),
}

fn track_to_px(track: &TrackSize, available: f32) -> f32 {
    match track {
        TrackSize::Px(v) => *v,
        TrackSize::Fr(_) => available, // fr in minmax is treated as max
        TrackSize::Auto | TrackSize::MinContent | TrackSize::MaxContent => 0.0,
        TrackSize::MinMax(min, _) => track_to_px(min, available),
    }
}

/// Cumulative offsets from track sizes + gaps.
fn cumulative_offsets(sizes: &[f32], gap: f32) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut offset = 0.0_f32;
    for (i, &size) in sizes.iter().enumerate() {
        offsets.push(offset);
        offset += size;
        if i + 1 < sizes.len() {
            offset += gap;
        }
    }
    offsets
}

/// Resolve container size (width from containing block, margins/padding/border).
fn resolve_grid_container_size(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    let style = layout_box.node_id.and_then(|id| styles.get(&id));

    let (margins, padding, borders) = resolve_edges(style);
    layout_box.dimensions.margin = margins;
    layout_box.dimensions.padding = padding;
    layout_box.dimensions.border = borders;

    // Width
    let specified_width = style.and_then(|s| {
        if s.width.is_nan() {
            None
        } else {
            Some(s.width)
        }
    });

    let box_sizing = style.map(|s| s.box_sizing).unwrap_or(BoxSizing::ContentBox);

    let content_width = if let Some(w) = specified_width {
        match box_sizing {
            BoxSizing::ContentBox => w,
            BoxSizing::BorderBox => {
                (w - padding.left - padding.right - borders.left - borders.right).max(0.0)
            }
        }
    } else {
        (containing.width
            - margins.left
            - margins.right
            - padding.left
            - padding.right
            - borders.left
            - borders.right)
            .max(0.0)
    };

    layout_box.dimensions.content.size.width = content_width;

    // Height
    let specified_height = style.and_then(|s| {
        if s.height.is_nan() {
            None
        } else {
            Some(s.height)
        }
    });
    layout_box.dimensions.content.size.height = specified_height.unwrap_or(0.0);
}

fn resolve_edges(style: Option<&ComputedStyle>) -> (Insets, Insets, Insets) {
    let style = match style {
        Some(s) => s,
        None => return (Insets::default(), Insets::default(), Insets::default()),
    };

    let margins = Insets::new(
        if style.margin_top.is_nan() {
            0.0
        } else {
            style.margin_top
        },
        if style.margin_right.is_nan() {
            0.0
        } else {
            style.margin_right
        },
        if style.margin_bottom.is_nan() {
            0.0
        } else {
            style.margin_bottom
        },
        if style.margin_left.is_nan() {
            0.0
        } else {
            style.margin_left
        },
    );

    let padding = Insets::new(
        style.padding_top,
        style.padding_right,
        style.padding_bottom,
        style.padding_left,
    );

    let borders = Insets::new(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    );

    (margins, padding, borders)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;
    use vex_core::{Point, VexId};
    use vex_css::values::grid::TrackList;

    fn make_grid_container(
        cols: &str,
        rows: &str,
        child_count: usize,
    ) -> (LayoutBox, HashMap<VexId, ComputedStyle>) {
        let container_id = VexId::new(1);
        let mut container = LayoutBox::new(Some(container_id), BoxType::Grid);

        let mut styles = HashMap::new();
        let mut container_style = ComputedStyle::default();
        container_style.grid_template_columns = TrackList::parse(cols);
        container_style.grid_template_rows = TrackList::parse(rows);
        styles.insert(container_id, container_style);

        for i in 0..child_count {
            let child_id = VexId::new(10 + i as u32);
            let child = LayoutBox::new(Some(child_id), BoxType::Block);
            container.children.push(child);
            styles.insert(child_id, ComputedStyle::default());
        }

        (container, styles)
    }

    #[test]
    fn three_column_equal_fr() {
        let (mut container, styles) = make_grid_container("1fr 1fr 1fr", "", 6);
        let containing = ContainingBlock {
            width: 300.0,
            height: 600.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        assert_eq!(container.children.len(), 6);
        // First row: 3 items, each ~100px wide.
        let c0 = &container.children[0];
        assert!((c0.dimensions.content.size.width - 100.0).abs() < 1.0);
        let c1 = &container.children[1];
        assert!((c1.dimensions.content.origin.x - 100.0).abs() < 1.0);
        let c2 = &container.children[2];
        assert!((c2.dimensions.content.origin.x - 200.0).abs() < 1.0);

        // Second row should start on new row.
        let c3 = &container.children[3];
        assert!(c3.dimensions.content.origin.y > 0.0);
    }

    #[test]
    fn fixed_and_fr_columns() {
        let (mut container, styles) = make_grid_container("200px 1fr", "", 2);
        let containing = ContainingBlock {
            width: 500.0,
            height: 400.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        let c0 = &container.children[0];
        assert!((c0.dimensions.content.size.width - 200.0).abs() < 1.0);
        let c1 = &container.children[1];
        assert!((c1.dimensions.content.size.width - 300.0).abs() < 1.0);
    }

    #[test]
    fn explicit_rows() {
        let (mut container, styles) = make_grid_container("1fr 1fr", "100px 200px", 4);
        let containing = ContainingBlock {
            width: 400.0,
            height: 300.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        // Row 1 items should have height ~100px, row 2 ~200px.
        let c0 = &container.children[0];
        assert!((c0.dimensions.content.size.height - 100.0).abs() < 1.0);
        let c2 = &container.children[2];
        assert!((c2.dimensions.content.size.height - 200.0).abs() < 1.0);
    }

    #[test]
    fn gap_spacing() {
        let (mut container, mut styles) = make_grid_container("1fr 1fr", "", 2);
        let container_id = VexId::new(1);
        styles.get_mut(&container_id).unwrap().grid_column_gap = 20.0;
        let containing = ContainingBlock {
            width: 220.0,
            height: 100.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        // With 20px gap between 2 cols in 220px: each col = (220-20)/2 = 100px.
        let c0 = &container.children[0];
        assert!((c0.dimensions.content.size.width - 100.0).abs() < 1.0);
        let c1 = &container.children[1];
        // Second column starts at 100 + 20 = 120.
        assert!((c1.dimensions.content.origin.x - 120.0).abs() < 1.0);
    }

    #[test]
    fn row_gap_spacing() {
        let (mut container, mut styles) = make_grid_container("1fr", "50px 50px", 2);
        let container_id = VexId::new(1);
        styles.get_mut(&container_id).unwrap().grid_row_gap = 10.0;
        let containing = ContainingBlock {
            width: 200.0,
            height: 200.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        let c1 = &container.children[1];
        // Row 2 starts at 50 + 10 = 60.
        assert!((c1.dimensions.content.origin.y - 60.0).abs() < 1.0);
    }

    #[test]
    fn explicit_placement() {
        let container_id = VexId::new(1);
        let child_id = VexId::new(10);

        let mut container = LayoutBox::new(Some(container_id), BoxType::Grid);
        let mut styles = HashMap::new();

        let mut container_style = ComputedStyle::default();
        container_style.grid_template_columns = TrackList::parse("100px 100px 100px");
        container_style.grid_template_rows = TrackList::parse("50px 50px");
        styles.insert(container_id, container_style);

        let mut child_style = ComputedStyle::default();
        child_style.grid_column_start = GridLine::Line(2);
        child_style.grid_column_end = GridLine::Line(4);
        child_style.grid_row_start = GridLine::Line(1);
        child_style.grid_row_end = GridLine::Line(2);
        styles.insert(child_id, child_style);

        container
            .children
            .push(LayoutBox::new(Some(child_id), BoxType::Block));

        let containing = ContainingBlock {
            width: 300.0,
            height: 100.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        let c = &container.children[0];
        // Should span columns 2-3 (index 1-2), starting at x=100.
        assert!((c.dimensions.content.origin.x - 100.0).abs() < 1.0);
        // Width should be 200 (two columns).
        assert!((c.dimensions.content.size.width - 200.0).abs() < 1.0);
    }

    #[test]
    fn span_placement() {
        let container_id = VexId::new(1);
        let child_id = VexId::new(10);

        let mut container = LayoutBox::new(Some(container_id), BoxType::Grid);
        let mut styles = HashMap::new();

        let mut container_style = ComputedStyle::default();
        container_style.grid_template_columns = TrackList::parse("100px 100px 100px");
        styles.insert(container_id, container_style);

        let mut child_style = ComputedStyle::default();
        child_style.grid_column_start = GridLine::Line(1);
        child_style.grid_column_end = GridLine::Span(2);
        styles.insert(child_id, child_style);

        container
            .children
            .push(LayoutBox::new(Some(child_id), BoxType::Block));

        let containing = ContainingBlock {
            width: 300.0,
            height: 100.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        let c = &container.children[0];
        assert!((c.dimensions.content.size.width - 200.0).abs() < 1.0);
    }

    #[test]
    fn auto_height_from_rows() {
        let (mut container, styles) = make_grid_container("1fr", "80px 80px", 2);
        let containing = ContainingBlock {
            width: 200.0,
            height: 600.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        assert!((container.dimensions.content.size.height - 160.0).abs() < 1.0);
    }

    #[test]
    fn empty_grid_no_crash() {
        let (mut container, styles) = make_grid_container("1fr 1fr", "", 0);
        let containing = ContainingBlock {
            width: 400.0,
            height: 300.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);
        assert_eq!(container.children.len(), 0);
    }

    #[test]
    fn single_item_no_template() {
        let (mut container, styles) = make_grid_container("", "", 1);
        let containing = ContainingBlock {
            width: 300.0,
            height: 200.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);
        // Should not crash and item should get some space.
        assert_eq!(container.children.len(), 1);
    }

    #[test]
    fn resolve_track_sizes_all_fixed() {
        let sizes = resolve_track_sizes(&[TrackSize::Px(100.0), TrackSize::Px(200.0)], 2, 400.0);
        assert_eq!(sizes, vec![100.0, 200.0]);
    }

    #[test]
    fn resolve_track_sizes_all_fr() {
        let sizes = resolve_track_sizes(&[TrackSize::Fr(1.0), TrackSize::Fr(2.0)], 2, 300.0);
        assert!((sizes[0] - 100.0).abs() < 0.1);
        assert!((sizes[1] - 200.0).abs() < 0.1);
    }

    #[test]
    fn cumulative_offsets_with_gap() {
        let offsets = cumulative_offsets(&[100.0, 100.0, 100.0], 10.0);
        assert_eq!(offsets.len(), 3);
        assert!((offsets[0] - 0.0).abs() < 0.1);
        assert!((offsets[1] - 110.0).abs() < 0.1);
        assert!((offsets[2] - 220.0).abs() < 0.1);
    }

    #[test]
    fn repeat_track_list_layout() {
        let (mut container, styles) = make_grid_container("repeat(4, 1fr)", "50px", 4);
        let containing = ContainingBlock {
            width: 400.0,
            height: 200.0,
        };
        container.dimensions.content.origin = Point::default();
        layout_grid(&mut container, containing, &styles);

        // 4 columns of 100px each.
        for (i, child) in container.children.iter().enumerate() {
            let expected_x = i as f32 * 100.0;
            assert!(
                (child.dimensions.content.origin.x - expected_x).abs() < 1.0,
                "child {} x: {} expected {}",
                i,
                child.dimensions.content.origin.x,
                expected_x
            );
            assert!((child.dimensions.content.size.width - 100.0).abs() < 1.0);
        }
    }
}
