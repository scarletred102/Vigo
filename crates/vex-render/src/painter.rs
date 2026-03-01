// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Painter — walks the layout tree to produce a display list.
//!
//! The painter traverses layout boxes in paint order and emits display
//! commands for backgrounds, borders, text, and structural operations
//! like clipping and opacity layers.

use std::collections::HashMap;

use vex_core::color::Color;
use vex_core::geometry::{Insets, Point, Rect, Size};
use vex_core::VexId;
use vex_css::computed::ComputedStyle;
use vex_css::values::box_model::{BorderStyle, Visibility};
use vex_dom::node::NodeData;
use vex_dom::Document;
use vex_layout::LayoutBox;

use crate::display_list::{DisplayCommand, DisplayList, RenderBorderStyle};

/// Build a display list from a laid-out tree.
///
/// Walks the layout tree in paint order, emitting drawing commands for
/// backgrounds, borders, and text. Skips boxes entirely outside the viewport.
pub fn build_display_list(
    root: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    document: &Document,
    viewport: Size,
) -> DisplayList {
    let mut dl = DisplayList::with_capacity(root.children.len() * 4);
    let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
    paint_box(root, styles, document, &viewport_rect, &mut dl);
    dl
}

/// Recursively paint a layout box and its children.
fn paint_box(
    layout_box: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    document: &Document,
    viewport: &Rect,
    dl: &mut DisplayList,
) {
    let border_box = layout_box.dimensions.border_box();

    // P6.1.3 — Viewport culling: skip if entirely off-screen.
    if !viewport.intersects(&border_box) {
        return;
    }

    let style = layout_box.node_id.and_then(|id| styles.get(&id));

    // Check visibility.
    if let Some(s) = style {
        if s.visibility == Visibility::Hidden {
            // Still takes space but doesn't paint.
            paint_children(layout_box, styles, document, viewport, dl);
            return;
        }
    }

    // Opacity layer.
    let needs_opacity = style.is_some_and(|s| s.opacity < 1.0);
    if needs_opacity {
        dl.push(DisplayCommand::PushOpacity {
            opacity: style.unwrap().opacity,
        });
    }

    // Clip region for overflow: hidden.
    let needs_clip = layout_box.clip_rect.is_some();
    if needs_clip {
        dl.push(DisplayCommand::PushClip {
            rect: layout_box.clip_rect.unwrap(),
        });
    }

    // 1. Background color.
    paint_background(layout_box, style, dl);

    // 2. Borders.
    paint_borders(layout_box, style, dl);

    // 3. Text content.
    paint_text(layout_box, style, document, dl);

    // 4. Children (recursive).
    paint_children(layout_box, styles, document, viewport, dl);

    // Close clip/opacity in reverse order.
    if needs_clip {
        dl.push(DisplayCommand::PopClip);
    }
    if needs_opacity {
        dl.push(DisplayCommand::PopOpacity);
    }
}

/// Paint the background color for a box.
fn paint_background(layout_box: &LayoutBox, style: Option<&ComputedStyle>, dl: &mut DisplayList) {
    let bg_color = style.map_or(Color::TRANSPARENT, |s| s.background_color);
    if bg_color.a == 0 {
        return;
    }

    dl.push(DisplayCommand::FillRect {
        rect: layout_box.dimensions.border_box(),
        color: bg_color,
    });
}

/// Paint borders if they have non-zero widths and visible styles.
fn paint_borders(layout_box: &LayoutBox, style: Option<&ComputedStyle>, dl: &mut DisplayList) {
    let s = match style {
        Some(s) => s,
        None => return,
    };

    let widths = Insets::new(
        s.border_top_width,
        s.border_right_width,
        s.border_bottom_width,
        s.border_left_width,
    );

    // Skip if all borders are zero.
    if widths.top == 0.0 && widths.right == 0.0 && widths.bottom == 0.0 && widths.left == 0.0 {
        return;
    }

    let colors = [
        s.border_top_color,
        s.border_right_color,
        s.border_bottom_color,
        s.border_left_color,
    ];

    let styles = [
        convert_border_style(s.border_top_style),
        convert_border_style(s.border_right_style),
        convert_border_style(s.border_bottom_style),
        convert_border_style(s.border_left_style),
    ];

    // Skip if all styles are None.
    if styles.iter().all(|s| *s == RenderBorderStyle::None) {
        return;
    }

    dl.push(DisplayCommand::DrawBorder {
        rect: layout_box.dimensions.border_box(),
        widths,
        colors,
        styles,
    });
}

/// Paint text content for text nodes.
fn paint_text(
    layout_box: &LayoutBox,
    style: Option<&ComputedStyle>,
    document: &Document,
    dl: &mut DisplayList,
) {
    let node_id = match layout_box.node_id {
        Some(id) => id,
        None => return,
    };

    // Bounds check — anonymous boxes may have IDs beyond the arena.
    let arena = document.arena();
    if (node_id.index() as usize) >= arena.len() {
        return;
    }

    // Only paint text for text nodes.
    let node = arena.get(node_id);
    let text = match &node.data {
        NodeData::Text(t) => t.as_str(),
        _ => return,
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    let fallback = default_text_style();
    let s = style.unwrap_or(&fallback);
    let content = &layout_box.dimensions.content;

    dl.push(DisplayCommand::DrawText {
        position: Point::new(content.origin.x, content.origin.y),
        text: trimmed.to_string(),
        color: s.color,
        font_size: s.font_size,
        line_height: s.line_height,
    });
}

/// Recursively paint child boxes.
fn paint_children(
    layout_box: &LayoutBox,
    styles: &HashMap<VexId, ComputedStyle>,
    document: &Document,
    viewport: &Rect,
    dl: &mut DisplayList,
) {
    for child in &layout_box.children {
        paint_box(child, styles, document, viewport, dl);
    }
}

/// Convert CSS border style to render border style.
fn convert_border_style(css: BorderStyle) -> RenderBorderStyle {
    match css {
        BorderStyle::Solid
        | BorderStyle::Double
        | BorderStyle::Groove
        | BorderStyle::Ridge
        | BorderStyle::Inset
        | BorderStyle::Outset => RenderBorderStyle::Solid,
        BorderStyle::Dashed => RenderBorderStyle::Dashed,
        BorderStyle::Dotted => RenderBorderStyle::Dotted,
        BorderStyle::None | BorderStyle::Hidden => RenderBorderStyle::None,
    }
}

/// Fallback style for text nodes without explicit styles.
fn default_text_style() -> ComputedStyle {
    ComputedStyle::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::geometry::Insets as CoreInsets;
    use vex_layout::{BoxType, Dimensions};

    fn make_style(bg: Color) -> ComputedStyle {
        ComputedStyle {
            background_color: bg,
            ..ComputedStyle::default()
        }
    }

    fn make_box(id: u32, content: Rect) -> LayoutBox {
        LayoutBox {
            node_id: Some(VexId::new(id)),
            box_type: BoxType::Block,
            dimensions: Dimensions {
                content,
                padding: CoreInsets::new(0.0, 0.0, 0.0, 0.0),
                border: CoreInsets::new(0.0, 0.0, 0.0, 0.0),
                margin: CoreInsets::new(0.0, 0.0, 0.0, 0.0),
            },
            children: vec![],
            clip_rect: None,
            scroll_offset: Point::new(0.0, 0.0),
        }
    }

    #[test]
    fn empty_tree_produces_empty_list() {
        let root = make_box(0, Rect::new(0.0, 0.0, 100.0, 100.0));
        let styles = HashMap::new();
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        // No styles → no background → empty.
        assert!(dl.is_empty());
    }

    #[test]
    fn background_color_emits_fill_rect() {
        let root = make_box(0, Rect::new(0.0, 0.0, 200.0, 100.0));
        let mut styles = HashMap::new();
        styles.insert(VexId::new(0), make_style(Color::rgb(255, 0, 0)));
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        assert_eq!(dl.len(), 1);
        match &dl.commands()[0] {
            DisplayCommand::FillRect { color, .. } => {
                assert_eq!(color.r, 255);
            }
            _ => panic!("expected FillRect"),
        }
    }

    #[test]
    fn off_screen_box_is_culled() {
        // Box at y=1000 with viewport height=600 → culled.
        let root = make_box(0, Rect::new(0.0, 1000.0, 100.0, 50.0));
        let mut styles = HashMap::new();
        styles.insert(VexId::new(0), make_style(Color::rgb(0, 255, 0)));
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        assert!(dl.is_empty(), "off-screen box should be culled");
    }

    #[test]
    fn children_are_painted() {
        let child = make_box(1, Rect::new(10.0, 10.0, 80.0, 40.0));
        let mut root = make_box(0, Rect::new(0.0, 0.0, 200.0, 100.0));
        root.children.push(child);

        let mut styles = HashMap::new();
        styles.insert(VexId::new(0), make_style(Color::rgb(100, 100, 100)));
        styles.insert(VexId::new(1), make_style(Color::rgb(200, 200, 200)));
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        // Root background + child background.
        assert_eq!(dl.len(), 2);
    }

    #[test]
    fn transparent_background_skipped() {
        let root = make_box(0, Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut styles = HashMap::new();
        styles.insert(VexId::new(0), make_style(Color::TRANSPARENT));
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        assert!(dl.is_empty(), "transparent bg should not emit FillRect");
    }

    #[test]
    fn opacity_wraps_content() {
        let root = make_box(0, Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut styles = HashMap::new();
        let mut s = make_style(Color::rgb(255, 0, 0));
        s.opacity = 0.5;
        styles.insert(VexId::new(0), s);
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        assert_eq!(dl.len(), 3); // PushOpacity + FillRect + PopOpacity
        assert!(matches!(
            dl.commands()[0],
            DisplayCommand::PushOpacity { .. }
        ));
        assert!(matches!(dl.commands()[1], DisplayCommand::FillRect { .. }));
        assert!(matches!(dl.commands()[2], DisplayCommand::PopOpacity));
    }

    #[test]
    fn borders_emitted_when_nonzero() {
        let root = make_box(0, Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut styles = HashMap::new();
        let s = ComputedStyle {
            border_top_width: 2.0,
            border_top_style: BorderStyle::Solid,
            border_top_color: Color::BLACK,
            ..ComputedStyle::default()
        };
        styles.insert(VexId::new(0), s);
        let doc = Document::new();
        let viewport = Size::new(800.0, 600.0);

        let dl = build_display_list(&root, &styles, &doc, viewport);
        assert!(
            dl.commands()
                .iter()
                .any(|c| matches!(c, DisplayCommand::DrawBorder { .. })),
            "should emit DrawBorder for solid 2px border"
        );
    }
}
