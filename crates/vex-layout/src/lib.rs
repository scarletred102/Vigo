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
pub mod hit_test;
pub mod inline;
pub mod positioned;
pub mod stacking;
pub mod text;
pub mod tree_builder;

// Re-exports
pub use block::ContainingBlock;
pub use box_model::{BoxType, Dimensions, LayoutBox};
pub use hit_test::hit_test;
pub use stacking::{build_stacking_order, StackingEntry};
pub use text::TextEngine;
pub use tree_builder::build_layout_tree;

use std::collections::HashMap;
use std::fmt::Write;

use vex_core::{Rect, Size, VexId};
use vex_css::ComputedStyle;
use vex_dom::Document;

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

    root.dimensions.content.origin.x = 0.0;
    root.dimensions.content.origin.y = 0.0;

    // Step 3: Layout pass — dispatch by box type
    layout_recursive(&mut root, containing, styles);

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
) {
    match layout_box.box_type {
        BoxType::Block | BoxType::Anonymous | BoxType::InlineBlock => {
            block::layout_block(layout_box, containing, styles);
        }
        BoxType::Flex => {
            flex::layout_flex(layout_box, containing, styles);
        }
        BoxType::Inline => {
            // Inline boxes are sized during inline formatting context.
            // If standalone, just layout children in block mode as fallback.
            block::layout_block(layout_box, containing, styles);
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
