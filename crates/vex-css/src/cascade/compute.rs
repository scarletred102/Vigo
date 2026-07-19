// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Full style computation pipeline.

use std::collections::HashMap;

use vex_core::{Size, VexId};
use vex_dom::{Document, NodeData};

use crate::cascade::inheritance::apply_inheritance;
use crate::cascade::matching::{collect_inline_declarations, collect_matching_declarations};
use crate::cascade::resolve::resolve_cascade;
use crate::cascade::value_resolution::{resolve_line_height, resolve_property};
use crate::computed::ComputedStyle;
use crate::parser::Stylesheet;
use crate::properties::Property;
use crate::ua_stylesheet;

/// Default root font-size in pixels (matches browser spec default).
pub const DEFAULT_ROOT_FONT_SIZE: f32 = 16.0;

/// Compute styles for every element in the document.
///
/// Algorithm:
/// 1. Collect user-agent stylesheet defaults.
/// 2. For each element (tree order), collect matching declarations,
///    resolve cascade, apply inheritance, resolve all values to px.
///
/// Uses [`DEFAULT_ROOT_FONT_SIZE`] for `rem` resolution. Call
/// [`compute_styles_with_font_size`] to override.
pub fn compute_styles(
    document: &Document,
    stylesheets: &[Stylesheet],
    viewport: Size,
) -> HashMap<VexId, ComputedStyle> {
    compute_styles_with_font_size(document, stylesheets, viewport, DEFAULT_ROOT_FONT_SIZE)
}

/// Like [`compute_styles`] but allows specifying a custom root font-size
/// (e.g. from `BrowserSettings.default_font_size`).
pub fn compute_styles_with_font_size(
    document: &Document,
    stylesheets: &[Stylesheet],
    viewport: Size,
    root_font_size: f32,
) -> HashMap<VexId, ComputedStyle> {
    let mut styles: HashMap<VexId, ComputedStyle> = HashMap::new();
    let arena = document.arena();

    // Combine user-agent + author stylesheets
    let ua = ua_stylesheet::ua_stylesheet();
    let mut all_sheets = vec![ua];
    all_sheets.extend_from_slice(stylesheets);

    // Walk tree in document order
    let root = document.root();
    compute_recursive(
        root,
        None,
        arena,
        &all_sheets,
        viewport,
        root_font_size,
        &mut styles,
    );

    styles
}

fn compute_recursive(
    node_id: VexId,
    parent_id: Option<VexId>,
    arena: &vex_dom::NodeArena,
    stylesheets: &[Stylesheet],
    viewport: Size,
    root_font_size: f32,
    styles: &mut HashMap<VexId, ComputedStyle>,
) {
    let node = arena.get(node_id);

    if let NodeData::Element(_) = &node.data {
        let parent_style = parent_id
            .and_then(|pid| styles.get(&pid))
            .cloned()
            .unwrap_or_default();

        let parent_font_size = parent_style.font_size;

        // 1. Collect all matching declarations
        let mut matched = collect_matching_declarations(node_id, arena, stylesheets, viewport);

        // 2. Collect inline style declarations
        let inline = collect_inline_declarations(node_id, arena);
        matched.extend(inline);

        // 3. Resolve cascade (which declaration wins for each property)
        let winning_props = resolve_cascade(&matched);

        // 4. Resolve values to px
        let resolved: Vec<_> = winning_props
            .iter()
            .map(|p| {
                resolve_property(
                    p,
                    parent_font_size,
                    root_font_size,
                    viewport,
                    viewport.width,
                )
            })
            .collect();

        // 5. Start from parent's inherited values + defaults
        let mut computed = ComputedStyle::default();
        apply_inheritance(&mut computed, &parent_style);

        // 6. Apply winning properties
        for prop in &resolved {
            computed.apply(prop);
        }

        // General length resolution uses the parent font size, which is right
        // for inherited font-size values but wrong for line-height. Unitless
        // and percentage line-heights are based on this element's final font
        // size, so resolve them after font-size has been applied.
        if let Some(Property::LineHeight(line_height)) = winning_props
            .iter()
            .find(|property| matches!(property, Property::LineHeight(_)))
        {
            computed.apply(&Property::LineHeight(resolve_line_height(
                line_height,
                computed.font_size,
                root_font_size,
                viewport,
            )));
        }

        styles.insert(node_id, computed);
    }

    // Recurse into children
    let mut child = arena.get(node_id).first_child;
    while let Some(child_id) = child {
        let effective_parent = match &arena.get(node_id).data {
            NodeData::Element(_) => Some(node_id),
            _ => parent_id,
        };
        compute_recursive(
            child_id,
            effective_parent,
            arena,
            stylesheets,
            viewport,
            root_font_size,
            styles,
        );
        child = arena.get(child_id).next_sibling;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_stylesheet;
    use crate::values::display::Display;

    fn make_test_doc() -> Document {
        // <html><body><div id="main"><p>Hello</p></div></body></html>
        let html = r#"<html><body><div id="main"><p>Hello</p></div></body></html>"#;
        vex_html::parse_html(html)
    }

    #[test]
    fn basic_style_application() {
        let doc = make_test_doc();
        let css = parse_stylesheet("div { display: flex; }");
        let viewport = Size::new(1280.0, 720.0);
        let styles = compute_styles(&doc, &[css], viewport);

        // Find the div element
        let divs = doc.get_elements_by_tag_name("div");
        assert!(!divs.is_empty());
        let div_style = styles.get(&divs[0]).unwrap();
        assert_eq!(div_style.display, Display::Flex);
    }

    #[test]
    fn inheritance_chain() {
        let doc = make_test_doc();
        let css = parse_stylesheet("body { color: red; }");
        let viewport = Size::new(1280.0, 720.0);
        let styles = compute_styles(&doc, &[css], viewport);

        // p should inherit color from body
        let ps = doc.get_elements_by_tag_name("p");
        if let Some(p_style) = ps.first().and_then(|id| styles.get(id)) {
            // Color should have been inherited through div → p
            assert_eq!(
                p_style.color,
                vex_core::Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255
                }
            );
        }
    }

    #[test]
    fn ua_defaults_applied() {
        let doc = make_test_doc();
        let viewport = Size::new(1280.0, 720.0);
        let styles = compute_styles(&doc, &[], viewport);

        // body should have display: block from UA stylesheet
        let bodies = doc.get_elements_by_tag_name("body");
        if let Some(body_style) = bodies.first().and_then(|id| styles.get(id)) {
            assert_eq!(body_style.display, Display::Block);
        }
    }

    #[test]
    fn unitless_line_height_uses_each_elements_font_size() {
        let doc =
            vex_html::parse_html("<html><body><h1>Vigo</h1><p>Readable text</p></body></html>");
        let css = parse_stylesheet(
            "body { font-size: 20px; line-height: 1.5; } h1 { font-size: 34px; line-height: 1.2; }",
        );
        let styles = compute_styles(&doc, &[css], Size::new(1280.0, 720.0));

        let body = doc.get_elements_by_tag_name("body")[0];
        let h1 = doc.get_elements_by_tag_name("h1")[0];
        let p = doc.get_elements_by_tag_name("p")[0];

        assert!((styles[&body].line_height - 30.0).abs() < f32::EPSILON);
        assert!((styles[&h1].line_height - 40.8).abs() < 0.001);
        assert!((styles[&p].line_height - 30.0).abs() < f32::EPSILON);
    }

    #[test]
    fn media_query_rules_apply_only_when_matching_viewport() {
        let doc = make_test_doc();
        let css = parse_stylesheet(
            "#main { display: block; } @media (max-width: 700px) { #main { display: none; } }",
        );

        let wide = compute_styles(&doc, std::slice::from_ref(&css), Size::new(1200.0, 800.0));
        let narrow = compute_styles(&doc, &[css], Size::new(600.0, 800.0));

        let main = doc.get_element_by_id("main").expect("main element");
        assert_eq!(wide.get(&main).unwrap().display, Display::Block);
        assert_eq!(narrow.get(&main).unwrap().display, Display::None);
    }
}
