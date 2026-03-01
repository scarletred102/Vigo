// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Flexbox layout algorithm (CSS Flexible Box Layout Module Level 1).
//!
//! Handles `display: flex` containers: main-axis sizing, flex-grow/shrink
//! distribution, cross-axis alignment, and wrapping.

use std::collections::HashMap;

use vex_core::{Insets, VexId};
use vex_css::values::box_model::BoxSizing;
use vex_css::values::flex::{AlignItems, AlignSelf, FlexDirection, FlexWrap, JustifyContent};
use vex_css::ComputedStyle;

use crate::block::{layout_block, ContainingBlock};
use crate::box_model::{BoxType, LayoutBox};

/// Perform flex layout on a flex container and its children.
pub fn layout_flex(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    resolve_flex_container_size(layout_box, containing, styles);

    let style = layout_box.node_id.and_then(|id| styles.get(&id));
    let direction = style
        .map(|s| s.flex_direction)
        .unwrap_or(FlexDirection::Row);
    let wrap = style.map(|s| s.flex_wrap).unwrap_or(FlexWrap::NoWrap);
    let justify = style
        .map(|s| s.justify_content)
        .unwrap_or(JustifyContent::FlexStart);
    let align_items = style.map(|s| s.align_items).unwrap_or(AlignItems::Stretch);
    let is_row = direction.is_row();
    let is_reverse = matches!(
        direction,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    );

    let container_main = if is_row {
        layout_box.dimensions.content.size.width
    } else {
        layout_box.dimensions.content.size.height
    };

    // ── Step 1: Determine hypothetical main sizes ────────────────────
    let mut items: Vec<FlexItem> = Vec::new();

    let mut children = std::mem::take(&mut layout_box.children);
    for (i, child) in children.iter_mut().enumerate() {
        let child_style = child.node_id.and_then(|id| styles.get(&id));
        let flex_basis = child_style.map(|s| s.flex_basis).unwrap_or(f32::NAN);
        let flex_grow = child_style.map(|s| s.flex_grow).unwrap_or(0.0);
        let flex_shrink = child_style.map(|s| s.flex_shrink).unwrap_or(1.0);

        // Determine base size from flex-basis or width/height
        let base_size = if !flex_basis.is_nan() {
            flex_basis
        } else {
            let dim = if is_row {
                child_style.map(|s| s.width).unwrap_or(f32::NAN)
            } else {
                child_style.map(|s| s.height).unwrap_or(f32::NAN)
            };
            if dim.is_nan() {
                0.0
            } else {
                dim
            }
        };

        // Resolve padding/border/margin for main axis edges
        resolve_child_edges(child, child_style);

        let edges = if is_row {
            child.dimensions.padding.left
                + child.dimensions.padding.right
                + child.dimensions.border.left
                + child.dimensions.border.right
                + child.dimensions.margin.left
                + child.dimensions.margin.right
        } else {
            child.dimensions.padding.top
                + child.dimensions.padding.bottom
                + child.dimensions.border.top
                + child.dimensions.border.bottom
                + child.dimensions.margin.top
                + child.dimensions.margin.bottom
        };

        items.push(FlexItem {
            index: i,
            base_size,
            flex_grow,
            flex_shrink,
            main_edges: edges,
            final_main: base_size,
            final_cross: 0.0,
        });
    }

    // ── Step 2: Wrap into flex lines ─────────────────────────────────
    let lines = if wrap == FlexWrap::NoWrap {
        vec![FlexLine {
            items: (0..items.len()).collect(),
        }]
    } else {
        wrap_into_lines(&items, container_main)
    };

    // ── Step 3: Resolve flexible lengths per line ────────────────────
    for line in &lines {
        let total_base: f32 = line
            .items
            .iter()
            .map(|&idx| items[idx].base_size + items[idx].main_edges)
            .sum();
        let free_space = container_main - total_base;

        if free_space > 0.0 {
            // Distribute with flex-grow
            let total_grow: f32 = line.items.iter().map(|&idx| items[idx].flex_grow).sum();
            if total_grow > 0.0 {
                for &idx in &line.items {
                    items[idx].final_main =
                        items[idx].base_size + free_space * items[idx].flex_grow / total_grow;
                }
            }
        } else if free_space < 0.0 {
            // Distribute with flex-shrink
            let total_shrink: f32 = line.items.iter().map(|&idx| items[idx].flex_shrink).sum();
            if total_shrink > 0.0 {
                for &idx in &line.items {
                    let shrink_amount = free_space.abs() * items[idx].flex_shrink / total_shrink;
                    items[idx].final_main = (items[idx].base_size - shrink_amount).max(0.0);
                }
            }
        }
    }

    // ── Step 4: Determine cross sizes ────────────────────────────────
    let container_cross = if is_row {
        layout_box.dimensions.content.size.height
    } else {
        layout_box.dimensions.content.size.width
    };

    for item in &mut items {
        let child_style = children[item.index].node_id.and_then(|id| styles.get(&id));
        let cross_dim = if is_row {
            child_style.map(|s| s.height).unwrap_or(f32::NAN)
        } else {
            child_style.map(|s| s.width).unwrap_or(f32::NAN)
        };
        item.final_cross = if cross_dim.is_nan() {
            // Auto cross: use main size ratio or stretch
            item.final_main.min(container_cross)
        } else {
            cross_dim
        };
    }

    // ── Step 5: Position items ───────────────────────────────────────
    let content_x = layout_box.dimensions.content.origin.x;
    let content_y = layout_box.dimensions.content.origin.y;
    let mut cross_cursor = 0.0_f32;

    for line in &lines {
        // Calculate line cross size
        let line_cross = line
            .items
            .iter()
            .map(|&idx| {
                let cross_edges = if is_row {
                    items[idx].main_edges // Approximate — use actual cross edges
                } else {
                    0.0
                };
                items[idx].final_cross + cross_edges
            })
            .fold(0.0_f32, f32::max);

        // Justify content: distribute free main-axis space
        let total_main: f32 = line
            .items
            .iter()
            .map(|&idx| items[idx].final_main + items[idx].main_edges)
            .sum();
        let free_main = (container_main - total_main).max(0.0);
        let item_count = line.items.len();

        let (mut main_cursor, gap) = justify_offsets(justify, free_main, item_count);

        let ordered_items: Vec<usize> = if is_reverse {
            line.items.iter().rev().copied().collect()
        } else {
            line.items.clone()
        };

        for &idx in &ordered_items {
            let item = &items[idx];
            let child = &mut children[item.index];

            // Cross-axis alignment for this item
            let child_style = child.node_id.and_then(|id| styles.get(&id));
            let align_self = child_style.map(|s| s.align_self).unwrap_or(AlignSelf::Auto);
            let effective_align = if align_self == AlignSelf::Auto {
                align_items
            } else {
                align_self_to_items(align_self)
            };

            let cross_offset = cross_align_offset(effective_align, item.final_cross, line_cross);

            let m = &child.dimensions.margin;
            let p = &child.dimensions.padding;
            let b = &child.dimensions.border;

            if is_row {
                child.dimensions.content.size.width = item.final_main;
                child.dimensions.content.size.height = item.final_cross;
                child.dimensions.content.origin.x =
                    content_x + main_cursor + m.left + p.left + b.left;
                child.dimensions.content.origin.y =
                    content_y + cross_cursor + cross_offset + m.top + p.top + b.top;
            } else {
                child.dimensions.content.size.width = item.final_cross;
                child.dimensions.content.size.height = item.final_main;
                child.dimensions.content.origin.x =
                    content_x + cross_cursor + cross_offset + m.left + p.left + b.left;
                child.dimensions.content.origin.y = content_y + main_cursor + m.top + p.top + b.top;
            }

            main_cursor += item.final_main + item.main_edges + gap;

            // Recursively layout block children inside flex items
            if !child.children.is_empty() {
                let child_containing = ContainingBlock {
                    width: child.dimensions.content.size.width,
                    height: child.dimensions.content.size.height,
                };
                for grandchild in &mut child.children {
                    if matches!(grandchild.box_type, BoxType::Block | BoxType::Flex) {
                        layout_block(grandchild, child_containing, styles);
                    }
                }
            }
        }

        cross_cursor += line_cross;
    }

    // ── Step 6: Auto height for flex container ───────────────────────
    let auto_h = layout_box
        .node_id
        .and_then(|id| styles.get(&id))
        .map(|s| s.height)
        .unwrap_or(f32::NAN);

    if auto_h.is_nan() {
        if is_row {
            layout_box.dimensions.content.size.height = cross_cursor;
        } else {
            // Column flex: main axis is vertical — use last position
            let last_main: f32 = items
                .iter()
                .map(|item| {
                    let c = &children[item.index];
                    if is_row {
                        c.dimensions.content.origin.x + c.dimensions.content.size.width
                    } else {
                        c.dimensions.content.origin.y + c.dimensions.content.size.height
                    }
                })
                .fold(0.0_f32, f32::max);
            layout_box.dimensions.content.size.height =
                (last_main - layout_box.dimensions.content.origin.y).max(0.0);
        }
    }

    layout_box.children = children;
}

// ── Internal types ───────────────────────────────────────────────────

struct FlexItem {
    index: usize,
    base_size: f32,
    flex_grow: f32,
    flex_shrink: f32,
    main_edges: f32,
    final_main: f32,
    final_cross: f32,
}

struct FlexLine {
    items: Vec<usize>, // Indices into the FlexItem vec
}

// ── Helpers ──────────────────────────────────────────────────────────

fn resolve_flex_container_size(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
) {
    let style = layout_box.node_id.and_then(|id| styles.get(&id));

    let padding = style
        .map(|s| {
            Insets::new(
                s.padding_top,
                s.padding_right,
                s.padding_bottom,
                s.padding_left,
            )
        })
        .unwrap_or_default();
    let border = style
        .map(|s| {
            Insets::new(
                s.border_top_width,
                s.border_right_width,
                s.border_bottom_width,
                s.border_left_width,
            )
        })
        .unwrap_or_default();
    let margin = style
        .map(|s| {
            Insets::new(
                if s.margin_top.is_nan() {
                    0.0
                } else {
                    s.margin_top
                },
                if s.margin_right.is_nan() {
                    0.0
                } else {
                    s.margin_right
                },
                if s.margin_bottom.is_nan() {
                    0.0
                } else {
                    s.margin_bottom
                },
                if s.margin_left.is_nan() {
                    0.0
                } else {
                    s.margin_left
                },
            )
        })
        .unwrap_or_default();

    layout_box.dimensions.padding = padding;
    layout_box.dimensions.border = border;
    layout_box.dimensions.margin = margin;

    // Width
    let raw_w = style.map(|s| s.width).unwrap_or(f32::NAN);
    let box_sizing = style.map(|s| s.box_sizing).unwrap_or(BoxSizing::ContentBox);
    let horiz = padding.left + padding.right + border.left + border.right;
    let content_w = if raw_w.is_nan() {
        containing.width - horiz - margin.left - margin.right
    } else {
        match box_sizing {
            BoxSizing::ContentBox => raw_w,
            BoxSizing::BorderBox => (raw_w - horiz).max(0.0),
        }
    };
    layout_box.dimensions.content.size.width = content_w.max(0.0);

    // Height — set explicit or leave 0 (auto resolved later)
    let raw_h = style.map(|s| s.height).unwrap_or(f32::NAN);
    if !raw_h.is_nan() {
        let vert = padding.top + padding.bottom + border.top + border.bottom;
        let content_h = match box_sizing {
            BoxSizing::ContentBox => raw_h,
            BoxSizing::BorderBox => (raw_h - vert).max(0.0),
        };
        layout_box.dimensions.content.size.height = content_h;
    }
}

fn resolve_child_edges(child: &mut LayoutBox, style: Option<&ComputedStyle>) {
    if let Some(s) = style {
        child.dimensions.padding = Insets::new(
            s.padding_top,
            s.padding_right,
            s.padding_bottom,
            s.padding_left,
        );
        child.dimensions.border = Insets::new(
            s.border_top_width,
            s.border_right_width,
            s.border_bottom_width,
            s.border_left_width,
        );
        child.dimensions.margin = Insets::new(
            if s.margin_top.is_nan() {
                0.0
            } else {
                s.margin_top
            },
            if s.margin_right.is_nan() {
                0.0
            } else {
                s.margin_right
            },
            if s.margin_bottom.is_nan() {
                0.0
            } else {
                s.margin_bottom
            },
            if s.margin_left.is_nan() {
                0.0
            } else {
                s.margin_left
            },
        );
    }
}

fn wrap_into_lines(items: &[FlexItem], container_main: f32) -> Vec<FlexLine> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut line_main = 0.0_f32;

    for (i, item) in items.iter().enumerate() {
        let item_main = item.base_size + item.main_edges;
        if line_main + item_main > container_main && !current.is_empty() {
            lines.push(FlexLine {
                items: std::mem::take(&mut current),
            });
            line_main = 0.0;
        }
        current.push(i);
        line_main += item_main;
    }
    if !current.is_empty() {
        lines.push(FlexLine { items: current });
    }
    lines
}

fn justify_offsets(justify: JustifyContent, free_space: f32, count: usize) -> (f32, f32) {
    if count == 0 {
        return (0.0, 0.0);
    }
    match justify {
        JustifyContent::FlexStart => (0.0, 0.0),
        JustifyContent::FlexEnd => (free_space, 0.0),
        JustifyContent::Center => (free_space / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free_space / (count - 1) as f32)
            }
        }
        JustifyContent::SpaceAround => {
            let gap = free_space / count as f32;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly => {
            let gap = free_space / (count + 1) as f32;
            (gap, gap)
        }
    }
}

fn align_self_to_items(align_self: AlignSelf) -> AlignItems {
    match align_self {
        AlignSelf::Auto => AlignItems::Stretch,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart => AlignItems::FlexStart,
        AlignSelf::FlexEnd => AlignItems::FlexEnd,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
    }
}

fn cross_align_offset(align: AlignItems, item_cross: f32, line_cross: f32) -> f32 {
    let free = (line_cross - item_cross).max(0.0);
    match align {
        AlignItems::Stretch | AlignItems::FlexStart | AlignItems::Baseline => 0.0,
        AlignItems::FlexEnd => free,
        AlignItems::Center => free / 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::Rect;

    fn make_flex_container(width: f32) -> LayoutBox {
        let mut b = LayoutBox::new(None, BoxType::Flex);
        b.dimensions.content = Rect::new(0.0, 0.0, width, 0.0);
        b
    }

    fn make_flex_child(
        id: u32,
        basis: f32,
        grow: f32,
        shrink: f32,
    ) -> (VexId, LayoutBox, ComputedStyle) {
        let vid = VexId::new(id);
        let b = LayoutBox::new(Some(vid), BoxType::Block);
        let s = ComputedStyle {
            flex_basis: basis,
            flex_grow: grow,
            flex_shrink: shrink,
            ..Default::default()
        };
        (vid, b, s)
    }

    #[test]
    fn flex_grow_distributes_space() {
        let mut styles = HashMap::new();
        let (id1, child1, s1) = make_flex_child(1, 100.0, 1.0, 0.0);
        let (id2, child2, s2) = make_flex_child(2, 100.0, 3.0, 0.0);
        styles.insert(id1, s1);
        styles.insert(id2, s2);

        let mut container = make_flex_container(600.0);
        container.children = vec![child1, child2];

        layout_flex(
            &mut container,
            ContainingBlock {
                width: 600.0,
                height: 400.0,
            },
            &styles,
        );

        // 400 free space: child1 gets +100, child2 gets +300
        let w1 = container.children[0].dimensions.content.size.width;
        let w2 = container.children[1].dimensions.content.size.width;
        assert!((w1 - 200.0).abs() < 1.0, "child1 width: {w1}");
        assert!((w2 - 400.0).abs() < 1.0, "child2 width: {w2}");
    }

    #[test]
    fn flex_shrink_reduces_overflow() {
        let mut styles = HashMap::new();
        let (id1, child1, s1) = make_flex_child(1, 400.0, 0.0, 1.0);
        let (id2, child2, s2) = make_flex_child(2, 400.0, 0.0, 1.0);
        styles.insert(id1, s1);
        styles.insert(id2, s2);

        let mut container = make_flex_container(600.0);
        container.children = vec![child1, child2];

        layout_flex(
            &mut container,
            ContainingBlock {
                width: 600.0,
                height: 400.0,
            },
            &styles,
        );

        let w1 = container.children[0].dimensions.content.size.width;
        let w2 = container.children[1].dimensions.content.size.width;
        assert!(
            (w1 - 300.0).abs() < 1.0,
            "child1 should shrink to 300, got {w1}"
        );
        assert!(
            (w2 - 300.0).abs() < 1.0,
            "child2 should shrink to 300, got {w2}"
        );
    }

    #[test]
    fn justify_center() {
        let (offset, gap) = justify_offsets(JustifyContent::Center, 100.0, 2);
        assert_eq!(offset, 50.0);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn justify_space_between() {
        let (offset, gap) = justify_offsets(JustifyContent::SpaceBetween, 100.0, 3);
        assert_eq!(offset, 0.0);
        assert_eq!(gap, 50.0);
    }

    #[test]
    fn justify_space_evenly() {
        let (offset, gap) = justify_offsets(JustifyContent::SpaceEvenly, 120.0, 3);
        assert_eq!(offset, 30.0);
        assert_eq!(gap, 30.0);
    }

    #[test]
    fn cross_align_center() {
        assert_eq!(cross_align_offset(AlignItems::Center, 40.0, 100.0), 30.0);
    }

    #[test]
    fn cross_align_flex_end() {
        assert_eq!(cross_align_offset(AlignItems::FlexEnd, 40.0, 100.0), 60.0);
    }
}
