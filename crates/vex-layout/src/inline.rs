// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Inline-level layout: line boxes, text fragments, text-align.
//!
//! When a block container has only inline children, they are laid
//! out left-to-right, wrapping into line boxes when the available
//! width is exhausted. Text measurement is delegated to [`crate::text::TextEngine`].

use std::collections::HashMap;

use vex_core::VexId;
use vex_css::values::text::{TextAlign, WhiteSpace};
use vex_css::ComputedStyle;
use vex_dom::{NodeArena, NodeData};

use crate::box_model::LayoutBox;
use crate::text::TextEngine;

/// A single horizontal line box containing fragments.
#[derive(Debug)]
pub struct LineBox {
    /// Fragments on this line (index into parent's children vec).
    pub fragments: Vec<InlineFragment>,
    /// Total width consumed.
    pub width: f32,
    /// Height of the tallest fragment.
    pub height: f32,
}

/// A chunk of content placed on a line.
#[derive(Debug, Clone)]
pub struct InlineFragment {
    /// Index into the parent layout box's `children`.
    pub child_index: usize,
    /// Position within the line box.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Lay out inline children of a block layout box into line boxes.
///
/// This modifies the children's positions in-place and returns the
/// total height consumed by the inline formatting context.
pub fn layout_inline_children(
    layout_box: &mut LayoutBox,
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
) -> f32 {
    let available_width = layout_box.dimensions.content.size.width;
    let content_x = layout_box.dimensions.content.origin.x;
    let content_y = layout_box.dimensions.content.origin.y;

    // Determine text-align from the parent's style
    let text_align = layout_box
        .node_id
        .and_then(|id| styles.get(&id))
        .map(|s| s.text_align)
        .unwrap_or(TextAlign::Left);

    // Build line boxes from children
    let mut lines = build_line_boxes(
        &mut layout_box.children,
        arena,
        styles,
        text_engine,
        available_width,
    );

    // Apply positions
    let mut cursor_y = 0.0_f32;
    let line_count = lines.len();
    for (line_idx, line) in lines.iter_mut().enumerate() {
        // For justify: distribute extra space between fragments.
        if text_align == TextAlign::Justify {
            let is_last = line_idx == line_count - 1;
            apply_justify(line, available_width, is_last);
        }

        let offset_x = align_offset(text_align, line.width, available_width);

        for frag in &line.fragments {
            let child = &mut layout_box.children[frag.child_index];
            let old_origin = child.dimensions.content.origin;
            let target_x = content_x + offset_x + frag.x;
            let target_y = content_y + cursor_y + frag.y;
            child.translate_subtree(target_x - old_origin.x, target_y - old_origin.y);
            child.dimensions.content.size.width = frag.width;
            child.dimensions.content.size.height = frag.height;
        }
        cursor_y += line.height;
    }

    cursor_y
}

/// Build line boxes from a list of inline children.
fn build_line_boxes(
    children: &mut [LayoutBox],
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
    available_width: f32,
) -> Vec<LineBox> {
    let mut lines: Vec<LineBox> = Vec::new();
    let mut current_line = LineBox {
        fragments: Vec::new(),
        width: 0.0,
        height: 0.0,
    };

    for (i, child) in children.iter().enumerate() {
        // Measure the child
        let (child_w, child_h) =
            measure_inline_child(child, arena, styles, text_engine, available_width);

        // Does it fit on the current line?
        if current_line.width + child_w > available_width && !current_line.fragments.is_empty() {
            // Wrap: finish current line, start new one
            lines.push(current_line);
            current_line = LineBox {
                fragments: Vec::new(),
                width: 0.0,
                height: 0.0,
            };
        }

        current_line.fragments.push(InlineFragment {
            child_index: i,
            x: current_line.width,
            y: 0.0,
            width: child_w,
            height: child_h,
        });
        current_line.width += child_w;
        current_line.height = current_line.height.max(child_h);
    }

    // Don't forget the last line
    if !current_line.fragments.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Measure the natural size of an inline child.
fn measure_inline_child(
    child: &LayoutBox,
    arena: &NodeArena,
    styles: &HashMap<VexId, ComputedStyle>,
    text_engine: &mut TextEngine,
    available_width: f32,
) -> (f32, f32) {
    let node_id = match child.node_id {
        Some(id) => id,
        None => return (0.0, 0.0),
    };

    let node = arena.get(node_id);

    match &node.data {
        NodeData::Text(text) => {
            let style = styles.get(&node_id);
            // Walk up to parent for font info if the text node doesn't have its own style
            let (font_size, line_height, white_space) = if let Some(s) = style {
                (s.font_size, s.line_height, s.white_space)
            } else {
                // Use the parent's style as fallback
                let parent_style = node.parent.and_then(|pid| styles.get(&pid));
                match parent_style {
                    Some(ps) => (ps.font_size, ps.line_height, ps.white_space),
                    None => (16.0, 19.2, WhiteSpace::Normal),
                }
            };

            let normalized = normalize_text_for_layout(text, white_space);
            if normalized.is_empty() {
                return (0.0, 0.0);
            }

            let wrap_width = if matches!(white_space, WhiteSpace::NoWrap | WhiteSpace::Pre) {
                f32::MAX / 4.0
            } else {
                available_width
            };

            text_engine.measure(&normalized, font_size, line_height, wrap_width)
        }
        NodeData::Element(_) => {
            // Element — use its explicit width/height or default
            let style = styles.get(&node_id);
            let w = style.map(|s| s.width).unwrap_or(f32::NAN);
            let h = style.map(|s| s.height).unwrap_or(f32::NAN);

            if !w.is_nan() && !h.is_nan() {
                return (w, h);
            }

            let (font_size, line_height, white_space) = style
                .map(|s| (s.font_size, s.line_height, s.white_space))
                .or_else(|| {
                    node.parent
                        .and_then(|pid| styles.get(&pid))
                        .map(|s| (s.font_size, s.line_height, s.white_space))
                })
                .unwrap_or((16.0, 19.2, WhiteSpace::Normal));

            let mut text = String::new();
            collect_text_content(arena, node_id, &mut text);
            let normalized = normalize_text_for_layout(&text, white_space);
            let (measured_w, measured_h) = if normalized.is_empty() {
                (0.0, 0.0)
            } else {
                let wrap_width = if matches!(white_space, WhiteSpace::NoWrap | WhiteSpace::Pre) {
                    f32::MAX / 4.0
                } else {
                    available_width
                };
                text_engine.measure(&normalized, font_size, line_height, wrap_width)
            };

            (
                if w.is_nan() { measured_w } else { w },
                if h.is_nan() { measured_h } else { h },
            )
        }
        _ => (0.0, 0.0),
    }
}

fn collect_text_content(arena: &NodeArena, node_id: VexId, out: &mut String) {
    let node = arena.get(node_id);
    match &node.data {
        NodeData::Text(t) => out.push_str(t),
        _ => {
            let mut child = node.first_child;
            while let Some(cid) = child {
                collect_text_content(arena, cid, out);
                child = arena.get(cid).next_sibling;
            }
        }
    }
}

fn normalize_text_for_layout(text: &str, white_space: WhiteSpace) -> String {
    match white_space {
        WhiteSpace::Pre | WhiteSpace::PreWrap => text.to_string(),
        WhiteSpace::PreLine => collapse_whitespace_preserve_newlines(text),
        WhiteSpace::Normal | WhiteSpace::NoWrap => collapse_whitespace(text),
    }
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_ws = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }

    out
}

fn collapse_whitespace_preserve_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_ws = false;

    for ch in text.chars() {
        if ch == '\n' || ch == '\r' {
            out.push('\n');
            in_ws = false;
        } else if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }

    out
}

/// Compute the x offset for line alignment.
fn align_offset(align: TextAlign, line_width: f32, container_width: f32) -> f32 {
    let free = (container_width - line_width).max(0.0);
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Right => free,
        TextAlign::Center => free / 2.0,
        // Justify uses 0 offset — spacing is distributed in apply_justify().
        TextAlign::Justify => 0.0,
    }
}

/// Distribute extra horizontal space between fragments for justify alignment.
///
/// Only applies to lines with more than one fragment that are NOT the last line.
fn apply_justify(line: &mut LineBox, available_width: f32, is_last_line: bool) {
    // Don't justify the last line — it should remain left-aligned.
    if is_last_line || line.fragments.len() < 2 {
        return;
    }

    let free = (available_width - line.width).max(0.0);
    if free <= 0.0 {
        return;
    }

    let gaps = (line.fragments.len() - 1) as f32;
    let extra_per_gap = free / gaps;

    // Shift each fragment by cumulative extra space.
    for (i, frag) in line.fragments.iter_mut().enumerate() {
        frag.x += extra_per_gap * i as f32;
    }
    line.width = available_width;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;
    use vex_core::Rect;

    #[test]
    fn align_offset_left() {
        assert_eq!(align_offset(TextAlign::Left, 200.0, 800.0), 0.0);
    }

    #[test]
    fn align_offset_right() {
        assert_eq!(align_offset(TextAlign::Right, 200.0, 800.0), 600.0);
    }

    #[test]
    fn align_offset_center() {
        assert_eq!(align_offset(TextAlign::Center, 200.0, 800.0), 300.0);
    }

    #[test]
    fn align_offset_overflow_clamps() {
        // Line wider than container → offset = 0
        assert_eq!(align_offset(TextAlign::Center, 900.0, 800.0), 0.0);
    }

    #[test]
    fn empty_children_produce_no_lines() {
        let mut parent = LayoutBox::new(None, BoxType::Block);
        parent.dimensions.content = Rect::new(0.0, 0.0, 800.0, 0.0);
        let arena = vex_dom::NodeArena::default();
        let styles = HashMap::new();
        let mut engine = TextEngine::new();
        let h = layout_inline_children(&mut parent, &arena, &styles, &mut engine);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn justify_distributes_space_between_fragments() {
        let mut line = LineBox {
            fragments: vec![
                InlineFragment {
                    child_index: 0,
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                },
                InlineFragment {
                    child_index: 1,
                    x: 100.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                },
                InlineFragment {
                    child_index: 2,
                    x: 200.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                },
            ],
            width: 300.0,
            height: 20.0,
        };
        apply_justify(&mut line, 500.0, false);
        // 200px free / 2 gaps = 100px per gap.
        assert!((line.fragments[0].x - 0.0).abs() < 0.01);
        assert!((line.fragments[1].x - 200.0).abs() < 0.01);
        assert!((line.fragments[2].x - 400.0).abs() < 0.01);
    }

    #[test]
    fn justify_last_line_not_justified() {
        let mut line = LineBox {
            fragments: vec![
                InlineFragment {
                    child_index: 0,
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                },
                InlineFragment {
                    child_index: 1,
                    x: 100.0,
                    y: 0.0,
                    width: 100.0,
                    height: 20.0,
                },
            ],
            width: 200.0,
            height: 20.0,
        };
        let original_x1 = line.fragments[1].x;
        apply_justify(&mut line, 500.0, true);
        // Last line: should not change.
        assert_eq!(line.fragments[1].x, original_x1);
    }

    #[test]
    fn collapse_whitespace_preserves_single_space_runs() {
        assert_eq!(collapse_whitespace("a   b\t\t c"), "a b c");
        assert_eq!(collapse_whitespace("   "), " ");
    }

    #[test]
    fn pre_mode_keeps_whitespace() {
        let s = normalize_text_for_layout(" a\n  b ", WhiteSpace::Pre);
        assert_eq!(s, " a\n  b ");
    }
}
