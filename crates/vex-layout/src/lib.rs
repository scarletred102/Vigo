// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-layout
//!
//! Layout engine for the Vex browser engine.
//!
//! Converts a styled DOM into a positioned `LayoutBox` tree with
//! concrete pixel coordinates. Implements block, inline, flexbox,
//! and positioned layouts. Uses `cosmic-text` for text shaping.
//!
//! # Usage
//!
//! ```ignore
//! let tree = vex_layout::layout_document(&doc, &styles, viewport);
//! let dump = vex_layout::debug_dump(&tree);
//! ```

pub mod block;
pub mod box_model;
pub mod flex;
pub mod float;
pub mod grid;
pub mod hit_test;
pub mod inline;
pub mod positioned;
pub mod reflow;
pub mod stacking;
pub mod text;
pub mod tree_builder;

// Re-exports
pub use block::ContainingBlock;
pub use box_model::{BoxType, Dimensions, LayoutBox};
pub use hit_test::hit_test;
pub use reflow::{reflow_document, ReflowPlan};
pub use stacking::{build_stacking_order, StackingEntry};
pub use text::TextEngine;
pub use tree_builder::build_layout_tree;

use std::collections::HashMap;
use std::fmt::Write;

use vex_core::{Rect, Size, VexId};
use vex_css::ComputedStyle;
use vex_dom::{Document, NodeArena};

/// Perform full layout on a document.
///
/// This is the main entry point for the layout engine:
/// 1. Build the layout tree from DOM + styles.
/// 2. Run block/flex layout to compute dimensions.
/// 3. Apply positioned offsets (relative/absolute/fixed).
///
/// Returns the root `LayoutBox` with all positions resolved.
pub fn layout_document(
    document: &Document,
    styles: &HashMap<VexId, ComputedStyle>,
    viewport: Size,
) -> LayoutBox {
    // Step 1: Build layout tree
    let mut root = build_layout_tree(document, styles);

    // Step 2: Set up the initial containing block (viewport)
    let containing = ContainingBlock {
        width: viewport.width,
        height: viewport.height,
    };

    let mut text_engine = TextEngine::new();

    root.dimensions.content.origin.x = 0.0;
    root.dimensions.content.origin.y = 0.0;

    // Step 3: Layout pass — dispatch by box type
    layout_recursive(
        &mut root,
        containing,
        styles,
        document.arena(),
        &mut text_engine,
    );

    // Step 4: Position pass — apply relative/absolute/fixed offsets
    let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
    positioned::apply_positions(&mut root, styles, viewport_rect);

    root
}

/// Recursively layout a box and its children.
fn layout_recursive(
    layout_box: &mut LayoutBox,
    containing: ContainingBlock,
    styles: &HashMap<VexId, ComputedStyle>,
    arena: &NodeArena,
    text_engine: &mut TextEngine,
) {
    match layout_box.box_type {
        BoxType::Block | BoxType::Anonymous | BoxType::InlineBlock => {
            block::layout_block(layout_box, containing, styles, arena, text_engine);
        }
        BoxType::Flex => {
            flex::layout_flex(layout_box, containing, styles, arena, text_engine);
        }
        BoxType::Grid => {
            grid::layout_grid(layout_box, containing, styles, arena, text_engine);
        }
        BoxType::Inline => {
            // Inline boxes are sized during inline formatting context.
            // If standalone, just layout children in block mode as fallback.
            block::layout_block(layout_box, containing, styles, arena, text_engine);
        }
    }
}

/// Generate a debug dump of the layout tree as indented text.
///
/// Useful for visual debugging and integration tests.
pub fn debug_dump(root: &LayoutBox) -> String {
    let mut output = String::new();
    dump_recursive(root, 0, &mut output);
    output
}

fn dump_recursive(layout_box: &LayoutBox, depth: usize, output: &mut String) {
    let indent = "  ".repeat(depth);
    let node_label = match layout_box.node_id {
        Some(id) => format!("#{}", id.index()),
        None => "anon".to_string(),
    };
    let d = &layout_box.dimensions;
    let _ = writeln!(
        output,
        "{indent}{:?} {node_label} @ ({:.1}, {:.1}) {:.1}×{:.1}",
        layout_box.box_type,
        d.content.origin.x,
        d.content.origin.y,
        d.content.size.width,
        d.content.size.height,
    );

    for child in &layout_box.children {
        dump_recursive(child, depth + 1, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_model::BoxType;

    fn any_inline_with_width(root: &LayoutBox) -> bool {
        if root.box_type == BoxType::Inline && root.dimensions.content.size.width > 0.0 {
            return true;
        }
        root.children.iter().any(any_inline_with_width)
    }

    #[test]
    fn layout_document_measures_inline_text_runs() {
        let doc = vex_html::parse_html("<html><body><p>Hello Vigo layout engine</p></body></html>");
        let styles = vex_css::compute_styles(&doc, &[], Size::new(800.0, 600.0));

        let tree = layout_document(&doc, &styles, Size::new(800.0, 600.0));
        assert!(any_inline_with_width(&tree));
    }

    #[test]
    fn nested_block_descendants_follow_their_positioned_parent() {
        fn find_box(root: &LayoutBox, node_id: VexId) -> Option<&LayoutBox> {
            if root.node_id == Some(node_id) {
                return Some(root);
            }
            root.children
                .iter()
                .find_map(|child| find_box(child, node_id))
        }

        let doc = vex_html::parse_html(
            "<html><body><div><p>Nested browser content</p></div></body></html>",
        );
        let styles = vex_css::compute_styles(&doc, &[], Size::new(800.0, 600.0));
        let tree = layout_document(&doc, &styles, Size::new(800.0, 600.0));
        let div = doc.get_elements_by_tag_name("div")[0];
        let paragraph = doc.get_elements_by_tag_name("p")[0];
        let div_box = find_box(&tree, div).expect("div layout box");
        let paragraph_box = find_box(&tree, paragraph).expect("paragraph layout box");

        assert!(
            paragraph_box.dimensions.content.origin.y >= div_box.dimensions.content.origin.y,
            "nested paragraph was not translated with its parent"
        );
    }
}
