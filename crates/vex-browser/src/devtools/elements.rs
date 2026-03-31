// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Elements panel — DOM tree view, computed styles, box model overlay.
//!
//! The DOM tree is rendered as a collapsible list of nodes. Each element
//! shows `<tag class="..." id="...">`. Clicking selects the node.
//! A separate styles panel shows computed styles for the selected element.

use std::collections::HashSet;

use vex_core::id::VexId;
use vex_dom::node::NodeData;
use vex_dom::traversal::Children;
use vex_dom::Document;

/// A single row in the rendered DOM tree view.
#[derive(Debug, Clone, PartialEq)]
pub struct DomTreeRow {
    /// The DOM node this row represents.
    pub node_id: VexId,
    /// Indentation depth (0 = root).
    pub depth: u32,
    /// Display text (e.g. `<div class="main" id="app">`).
    pub label: String,
    /// Whether this node is expanded (children visible).
    pub expanded: bool,
    /// Whether this node has children at all.
    pub has_children: bool,
    /// Whether this is a closing tag row (e.g. `</div>`).
    pub is_closing_tag: bool,
}

/// State for the Elements panel DOM tree.
#[derive(Debug, Clone, Default)]
pub struct DomTreeState {
    /// Set of expanded node IDs.
    expanded: HashSet<VexId>,
    /// Currently selected node (highlighted on page).
    selected: Option<VexId>,
    /// Currently hovered node (for box model overlay).
    hovered: Option<VexId>,
    /// Scroll offset within the tree view (in rows).
    scroll_offset: usize,
}

impl DomTreeState {
    /// Create a new empty tree state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle expand/collapse of a node.
    pub fn toggle_expand(&mut self, node_id: VexId) {
        if self.expanded.contains(&node_id) {
            self.expanded.remove(&node_id);
        } else {
            self.expanded.insert(node_id);
        }
    }

    /// Expand a node.
    pub fn expand(&mut self, node_id: VexId) {
        self.expanded.insert(node_id);
    }

    /// Collapse a node.
    pub fn collapse(&mut self, node_id: VexId) {
        self.expanded.remove(&node_id);
    }

    /// Whether a node is expanded.
    pub fn is_expanded(&self, node_id: VexId) -> bool {
        self.expanded.contains(&node_id)
    }

    /// Select a node (for style inspection).
    pub fn select(&mut self, node_id: VexId) {
        self.selected = Some(node_id);
    }

    /// Clear selection.
    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// Currently selected node.
    pub fn selected(&self) -> Option<VexId> {
        self.selected
    }

    /// Set the hovered node (for box model overlay on page).
    pub fn set_hovered(&mut self, node_id: Option<VexId>) {
        self.hovered = node_id;
    }

    /// Currently hovered node.
    pub fn hovered(&self) -> Option<VexId> {
        self.hovered
    }

    /// Current scroll offset (row index).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Scroll by delta rows (clamped to 0).
    pub fn scroll_by(&mut self, delta: i32, max_rows: usize) {
        let new = self.scroll_offset as i32 + delta;
        self.scroll_offset = new.max(0).min(max_rows.saturating_sub(1) as i32) as usize;
    }

    /// Expand the path from root to a given node (so it becomes visible).
    pub fn reveal_node(&mut self, doc: &Document, node_id: VexId) {
        let arena = doc.arena();
        let mut current = node_id;
        while let Some(parent) = arena.get(current).parent {
            self.expand(parent);
            current = parent;
        }
    }

    /// Build the visible rows for the DOM tree.
    ///
    /// Walks the DOM starting from the root element, respecting expanded state.
    /// Returns a flat list of rows to render.
    pub fn build_rows(&self, doc: &Document) -> Vec<DomTreeRow> {
        let mut rows = Vec::new();
        let root = doc.root();
        self.build_rows_recursive(doc, root, 0, &mut rows);
        rows
    }

    fn build_rows_recursive(
        &self,
        doc: &Document,
        node_id: VexId,
        depth: u32,
        rows: &mut Vec<DomTreeRow>,
    ) {
        let arena = doc.arena();
        let node = arena.get(node_id);
        let has_children = node.first_child.is_some();
        let expanded = self.is_expanded(node_id);

        // Build label based on node type.
        let (label, show_closing) = match &node.data {
            NodeData::Document => {
                // Don't show a row for the #document node itself,
                // just walk its children.
                let children: Vec<VexId> = Children::new(arena, node_id).collect();
                for child in children {
                    self.build_rows_recursive(doc, child, depth, rows);
                }
                return;
            }
            NodeData::Element(elem) => {
                let tag = &elem.tag_name;
                let mut label = format!("<{tag}");

                // Show id attribute first, then class.
                for attr in &elem.attributes {
                    if attr.name == "id" {
                        label.push_str(&format!(" id=\"{}\"", attr.value));
                    }
                }
                for attr in &elem.attributes {
                    if attr.name == "class" {
                        label.push_str(&format!(" class=\"{}\"", attr.value));
                    }
                }

                label.push('>');
                (label, has_children && expanded)
            }
            NodeData::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return; // Skip whitespace-only text nodes.
                }
                let display = if trimmed.len() > 60 {
                    format!("\"{}…\"", &trimmed[..60])
                } else {
                    format!("\"{trimmed}\"")
                };
                (display, false)
            }
            NodeData::Comment(text) => {
                let display = if text.len() > 50 {
                    format!("<!--{}…-->", &text[..50])
                } else {
                    format!("<!--{text}-->")
                };
                (display, false)
            }
            NodeData::Doctype { name, .. } => (format!("<!DOCTYPE {name}>"), false),
        };

        rows.push(DomTreeRow {
            node_id,
            depth,
            label,
            expanded,
            has_children,
            is_closing_tag: false,
        });

        // If expanded, recurse into children.
        if expanded && has_children {
            let children: Vec<VexId> = Children::new(arena, node_id).collect();
            for child in children {
                self.build_rows_recursive(doc, child, depth + 1, rows);
            }
        }

        // Add closing tag row for expanded elements.
        if show_closing {
            if let NodeData::Element(elem) = &node.data {
                rows.push(DomTreeRow {
                    node_id,
                    depth,
                    label: format!("</{}>", elem.tag_name),
                    expanded: false,
                    has_children: false,
                    is_closing_tag: true,
                });
            }
        }
    }
}

/// Format a node label for display in the tree (utility for renderers).
pub fn format_node_label(doc: &Document, node_id: VexId) -> Option<String> {
    let arena = doc.arena();
    let node = arena.get(node_id);
    match &node.data {
        NodeData::Document => Some("#document".to_owned()),
        NodeData::Element(elem) => {
            let mut s = format!("<{}", elem.tag_name);
            for attr in &elem.attributes {
                if attr.name == "id" || attr.name == "class" {
                    s.push_str(&format!(" {}=\"{}\"", attr.name, attr.value));
                }
            }
            s.push('>');
            Some(s)
        }
        NodeData::Text(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("\"{trimmed}\""))
            }
        }
        NodeData::Comment(c) => Some(format!("<!--{c}-->")),
        NodeData::Doctype { name, .. } => Some(format!("<!DOCTYPE {name}>")),
    }
}

// ── Computed Styles Panel ──────────────────────────────────────

use std::collections::HashMap;

use vex_css::computed::ComputedStyle;

/// Category of CSS properties for display grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleCategory {
    /// Box model: margin, padding, border widths.
    BoxModel,
    /// Dimensions: width, height, min/max.
    Dimensions,
    /// Typography: font family/size/weight, text-align, line-height.
    Typography,
    /// Colors: color, background-color, border colors.
    Colors,
    /// Layout: display, position, z-index, overflow.
    Layout,
    /// Flexbox: flex-direction, flex-wrap, align, justify.
    Flexbox,
    /// Position offsets: top, right, bottom, left.
    Positioning,
    /// Visual: opacity, visibility, cursor.
    Visual,
}

impl StyleCategory {
    /// Display label for the category.
    pub fn label(self) -> &'static str {
        match self {
            Self::BoxModel => "Box Model",
            Self::Dimensions => "Dimensions",
            Self::Typography => "Typography",
            Self::Colors => "Colors",
            Self::Layout => "Layout",
            Self::Flexbox => "Flexbox",
            Self::Positioning => "Positioning",
            Self::Visual => "Visual",
        }
    }

    /// All categories in display order.
    pub const ALL: &'static [StyleCategory] = &[
        Self::Layout,
        Self::Dimensions,
        Self::BoxModel,
        Self::Typography,
        Self::Colors,
        Self::Flexbox,
        Self::Positioning,
        Self::Visual,
    ];
}

/// A single property entry in the computed styles list.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleEntry {
    /// Property name (e.g. "margin-top").
    pub name: &'static str,
    /// Computed value as a string (e.g. "8px", "block", "#000000").
    pub value: String,
    /// Whether this property was inherited from a parent.
    pub inherited: bool,
}

/// Extract computed style entries grouped by category.
///
/// Returns a map from category to list of style entries.
pub fn extract_style_entries(style: &ComputedStyle) -> HashMap<StyleCategory, Vec<StyleEntry>> {
    let mut groups: HashMap<StyleCategory, Vec<StyleEntry>> = HashMap::new();

    // Helper: insert an entry into a group.
    let mut add = |cat: StyleCategory, name: &'static str, value: String, inherited: bool| {
        groups.entry(cat).or_default().push(StyleEntry {
            name,
            value,
            inherited,
        });
    };

    // ── Layout ──
    add(
        StyleCategory::Layout,
        "display",
        format!("{:?}", style.display),
        false,
    );
    add(
        StyleCategory::Layout,
        "position",
        format!("{:?}", style.position),
        false,
    );
    add(
        StyleCategory::Layout,
        "z-index",
        format!("{}", style.z_index),
        false,
    );
    add(
        StyleCategory::Layout,
        "overflow",
        format!("{:?}", style.overflow),
        false,
    );
    add(
        StyleCategory::Layout,
        "box-sizing",
        format!("{:?}", style.box_sizing),
        false,
    );

    // ── Dimensions ──
    add(
        StyleCategory::Dimensions,
        "width",
        format_auto_px(style.width),
        false,
    );
    add(
        StyleCategory::Dimensions,
        "height",
        format_auto_px(style.height),
        false,
    );
    add(
        StyleCategory::Dimensions,
        "min-width",
        format_px(style.min_width),
        false,
    );
    add(
        StyleCategory::Dimensions,
        "min-height",
        format_px(style.min_height),
        false,
    );
    add(
        StyleCategory::Dimensions,
        "max-width",
        format_auto_px(style.max_width),
        false,
    );
    add(
        StyleCategory::Dimensions,
        "max-height",
        format_auto_px(style.max_height),
        false,
    );

    // ── Box Model ──
    add(
        StyleCategory::BoxModel,
        "margin-top",
        format_px(style.margin_top),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "margin-right",
        format_px(style.margin_right),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "margin-bottom",
        format_px(style.margin_bottom),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "margin-left",
        format_px(style.margin_left),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "padding-top",
        format_px(style.padding_top),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "padding-right",
        format_px(style.padding_right),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "padding-bottom",
        format_px(style.padding_bottom),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "padding-left",
        format_px(style.padding_left),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "border-top-width",
        format_px(style.border_top_width),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "border-right-width",
        format_px(style.border_right_width),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "border-bottom-width",
        format_px(style.border_bottom_width),
        false,
    );
    add(
        StyleCategory::BoxModel,
        "border-left-width",
        format_px(style.border_left_width),
        false,
    );

    // ── Typography ──
    add(
        StyleCategory::Typography,
        "font-family",
        format!("{:?}", style.font_family),
        true,
    );
    add(
        StyleCategory::Typography,
        "font-size",
        format_px(style.font_size),
        true,
    );
    add(
        StyleCategory::Typography,
        "font-weight",
        format!("{:?}", style.font_weight),
        true,
    );
    add(
        StyleCategory::Typography,
        "font-style",
        format!("{:?}", style.font_style),
        true,
    );
    add(
        StyleCategory::Typography,
        "line-height",
        format_px(style.line_height),
        true,
    );
    add(
        StyleCategory::Typography,
        "text-align",
        format!("{:?}", style.text_align),
        true,
    );
    add(
        StyleCategory::Typography,
        "text-decoration",
        format!("{:?}", style.text_decoration),
        false,
    );
    add(
        StyleCategory::Typography,
        "white-space",
        format!("{:?}", style.white_space),
        true,
    );

    // ── Colors ──
    add(
        StyleCategory::Colors,
        "color",
        format_color(&style.color),
        true,
    );
    add(
        StyleCategory::Colors,
        "background-color",
        format_color(&style.background_color),
        false,
    );

    // ── Flexbox ──
    add(
        StyleCategory::Flexbox,
        "flex-direction",
        format!("{:?}", style.flex_direction),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "flex-wrap",
        format!("{:?}", style.flex_wrap),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "justify-content",
        format!("{:?}", style.justify_content),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "align-items",
        format!("{:?}", style.align_items),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "align-self",
        format!("{:?}", style.align_self),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "flex-grow",
        format!("{}", style.flex_grow),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "flex-shrink",
        format!("{}", style.flex_shrink),
        false,
    );
    add(
        StyleCategory::Flexbox,
        "flex-basis",
        format_auto_px(style.flex_basis),
        false,
    );

    // ── Positioning ──
    add(
        StyleCategory::Positioning,
        "top",
        format_auto_px(style.top),
        false,
    );
    add(
        StyleCategory::Positioning,
        "right",
        format_auto_px(style.right),
        false,
    );
    add(
        StyleCategory::Positioning,
        "bottom",
        format_auto_px(style.bottom),
        false,
    );
    add(
        StyleCategory::Positioning,
        "left",
        format_auto_px(style.left),
        false,
    );

    // ── Visual ──
    add(
        StyleCategory::Visual,
        "opacity",
        format!("{}", style.opacity),
        false,
    );
    add(
        StyleCategory::Visual,
        "visibility",
        format!("{:?}", style.visibility),
        false,
    );
    add(
        StyleCategory::Visual,
        "cursor",
        format!("{:?}", style.cursor),
        true,
    );

    groups
}

/// Format a pixel value (never auto).
fn format_px(v: f32) -> String {
    if v == f32::INFINITY {
        "none".to_owned()
    } else {
        format!("{v:.1}px")
    }
}

/// Format a value that can be `auto` (represented by NAN) or px.
fn format_auto_px(v: f32) -> String {
    if v.is_nan() {
        "auto".to_owned()
    } else if v == f32::INFINITY {
        "none".to_owned()
    } else {
        format!("{v:.1}px")
    }
}

/// Format a Color as hex string.
fn format_color(c: &vex_core::Color) -> String {
    if c.a == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        format!("rgba({}, {}, {}, {:.2})", c.r, c.g, c.b, c.a as f32 / 255.0)
    }
}

// ── Box Model Overlay ──────────────────────────────────────────

use vex_core::geometry::{Insets, Rect};
use vex_core::Color;

/// Semi-transparent overlay colors for the box model visualizer.
pub struct BoxModelColors;

impl BoxModelColors {
    /// Margin overlay — orange with alpha.
    pub const MARGIN: Color = Color {
        r: 255,
        g: 165,
        b: 0,
        a: 80,
    };
    /// Border overlay — yellow with alpha.
    pub const BORDER: Color = Color {
        r: 255,
        g: 255,
        b: 0,
        a: 80,
    };
    /// Padding overlay — green with alpha.
    pub const PADDING: Color = Color {
        r: 0,
        g: 200,
        b: 0,
        a: 80,
    };
    /// Content overlay — blue with alpha.
    pub const CONTENT: Color = Color {
        r: 100,
        g: 150,
        b: 255,
        a: 80,
    };
}

/// A set of colored rectangles for the box model overlay on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxModelOverlay {
    /// Margin area (outermost).
    pub margin_rect: Rect,
    pub margin_color: Color,
    /// Border area.
    pub border_rect: Rect,
    pub border_color: Color,
    /// Padding area.
    pub padding_rect: Rect,
    pub padding_color: Color,
    /// Content area (innermost).
    pub content_rect: Rect,
    pub content_color: Color,
}

/// Compute a box model overlay from layout dimensions.
///
/// The content rect is the innermost; padding/border/margin expand outward.
/// Each colored rect is the *full* area for that layer — the renderer
/// draws them back-to-front so inner layers paint on top.
pub fn compute_box_model_overlay(
    content: Rect,
    padding: Insets,
    border: Insets,
    margin: Insets,
) -> BoxModelOverlay {
    let padding_rect = expand_rect(content, padding);
    let border_rect = expand_rect(padding_rect, border);
    let margin_rect = expand_rect(border_rect, margin);

    BoxModelOverlay {
        margin_rect,
        margin_color: BoxModelColors::MARGIN,
        border_rect,
        border_color: BoxModelColors::BORDER,
        padding_rect,
        padding_color: BoxModelColors::PADDING,
        content_rect: content,
        content_color: BoxModelColors::CONTENT,
    }
}

/// Expand a rect outward by given insets.
fn expand_rect(r: Rect, i: Insets) -> Rect {
    Rect::new(
        r.origin.x - i.left,
        r.origin.y - i.top,
        r.size.width + i.left + i.right,
        r.size.height + i.top + i.bottom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_dom::node::{ElementState, Namespace};

    fn build_test_doc() -> Document {
        let mut doc = Document::new();
        let html = doc.create_element("html", Namespace::Html);
        doc.append_child(doc.root(), html);

        let head = doc.create_element("head", Namespace::Html);
        doc.append_child(html, head);

        let body = doc.create_element("body", Namespace::Html);
        doc.append_child(html, body);

        let div = doc.create_element("div", Namespace::Html);
        doc.arena_mut().get_mut(div).data = NodeData::Element(vex_dom::node::ElementData {
            tag_name: "div".into(),
            namespace: Namespace::Html,
            attributes: vec![
                vex_dom::node::Attribute {
                    name: "id".into(),
                    value: "main".into(),
                },
                vex_dom::node::Attribute {
                    name: "class".into(),
                    value: "container".into(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
            state: ElementState::default(),
        });
        doc.append_child(body, div);

        let text = doc.create_text("Hello, world!");
        doc.append_child(div, text);

        doc
    }

    #[test]
    fn default_state_has_no_selection() {
        let state = DomTreeState::new();
        assert!(state.selected().is_none());
        assert!(state.hovered().is_none());
    }

    #[test]
    fn toggle_expand() {
        let mut state = DomTreeState::new();
        let id = VexId::new(5);
        assert!(!state.is_expanded(id));
        state.toggle_expand(id);
        assert!(state.is_expanded(id));
        state.toggle_expand(id);
        assert!(!state.is_expanded(id));
    }

    #[test]
    fn select_and_deselect() {
        let mut state = DomTreeState::new();
        let id = VexId::new(3);
        state.select(id);
        assert_eq!(state.selected(), Some(id));
        state.deselect();
        assert!(state.selected().is_none());
    }

    #[test]
    fn build_rows_collapsed() {
        let doc = build_test_doc();
        let state = DomTreeState::new();
        let rows = state.build_rows(&doc);

        // With everything collapsed, we should see the top-level elements:
        // <html> (depth 0)
        assert!(!rows.is_empty());
        assert_eq!(rows[0].label, "<html>");
        assert!(rows[0].has_children);
        assert!(!rows[0].expanded);
    }

    #[test]
    fn build_rows_expanded() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let html_id = doc.root_element().unwrap();
        state.expand(html_id);

        let rows = state.build_rows(&doc);
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"<html>"));
        assert!(labels.contains(&"<head>"));
        assert!(labels.contains(&"<body>"));
        assert!(labels.contains(&"</html>"));
    }

    #[test]
    fn build_rows_deep_expansion() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let html_id = doc.root_element().unwrap();
        state.expand(html_id);

        let body_ids = doc.get_elements_by_tag_name("body");
        assert!(!body_ids.is_empty());
        state.expand(body_ids[0]);

        let div_ids = doc.get_elements_by_tag_name("div");
        assert!(!div_ids.is_empty());
        state.expand(div_ids[0]);

        let rows = state.build_rows(&doc);
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();

        assert!(labels.contains(&"<div id=\"main\" class=\"container\">"));
        assert!(labels.contains(&"\"Hello, world!\""));
        assert!(labels.contains(&"</div>"));
    }

    #[test]
    fn element_label_includes_id_and_class() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let html_id = doc.root_element().unwrap();
        state.expand(html_id);
        let body_ids = doc.get_elements_by_tag_name("body");
        state.expand(body_ids[0]);

        let rows = state.build_rows(&doc);
        let div_row = rows
            .iter()
            .find(|r| r.label.starts_with("<div"))
            .expect("should have div row");
        assert!(div_row.label.contains("id=\"main\""));
        assert!(div_row.label.contains("class=\"container\""));
    }

    #[test]
    fn closing_tag_rows() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let html_id = doc.root_element().unwrap();
        state.expand(html_id);

        let rows = state.build_rows(&doc);
        let closing = rows.iter().filter(|r| r.is_closing_tag).count();
        assert!(closing > 0, "expanded elements should have closing tags");
    }

    #[test]
    fn reveal_node_expands_ancestors() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let div_ids = doc.get_elements_by_tag_name("div");
        let div_id = div_ids[0];

        state.reveal_node(&doc, div_id);

        let html_id = doc.root_element().unwrap();
        let body_ids = doc.get_elements_by_tag_name("body");

        assert!(state.is_expanded(html_id));
        assert!(state.is_expanded(body_ids[0]));
    }

    #[test]
    fn scroll_by_clamped() {
        let mut state = DomTreeState::new();
        state.scroll_by(-5, 10);
        assert_eq!(state.scroll_offset(), 0);
        state.scroll_by(100, 10);
        assert_eq!(state.scroll_offset(), 9);
    }

    #[test]
    fn format_node_label_element() {
        let doc = build_test_doc();
        let html_id = doc.root_element().unwrap();
        let label = format_node_label(&doc, html_id);
        assert_eq!(label, Some("<html>".to_owned()));
    }

    #[test]
    fn whitespace_text_nodes_skipped() {
        let mut doc = Document::new();
        let html = doc.create_element("html", Namespace::Html);
        doc.append_child(doc.root(), html);
        let ws = doc.create_text("   \n\t  ");
        doc.append_child(html, ws);

        let mut state = DomTreeState::new();
        state.expand(html);
        let rows = state.build_rows(&doc);

        let text_rows: Vec<_> = rows.iter().filter(|r| r.label.starts_with('"')).collect();
        assert!(text_rows.is_empty());
    }

    #[test]
    fn depth_increases_with_nesting() {
        let doc = build_test_doc();
        let mut state = DomTreeState::new();

        let html_id = doc.root_element().unwrap();
        state.expand(html_id);
        let body_ids = doc.get_elements_by_tag_name("body");
        state.expand(body_ids[0]);

        let rows = state.build_rows(&doc);
        let body_row = rows
            .iter()
            .find(|r| r.label == "<body>")
            .expect("should have body");
        let div_row = rows
            .iter()
            .find(|r| r.label.starts_with("<div"))
            .expect("should have div");

        assert!(div_row.depth > body_row.depth);
    }

    // ── Computed Styles Tests ──

    #[test]
    fn style_category_labels() {
        assert_eq!(StyleCategory::BoxModel.label(), "Box Model");
        assert_eq!(StyleCategory::Typography.label(), "Typography");
        assert_eq!(StyleCategory::Layout.label(), "Layout");
    }

    #[test]
    fn extract_default_style_entries() {
        let style = ComputedStyle::default();
        let groups = extract_style_entries(&style);

        // Should have entries in multiple categories.
        assert!(groups.contains_key(&StyleCategory::Layout));
        assert!(groups.contains_key(&StyleCategory::Dimensions));
        assert!(groups.contains_key(&StyleCategory::Typography));
    }

    #[test]
    fn layout_group_has_display() {
        let style = ComputedStyle::default();
        let groups = extract_style_entries(&style);
        let layout = &groups[&StyleCategory::Layout];
        let display_entry = layout.iter().find(|e| e.name == "display");
        assert!(display_entry.is_some());
    }

    #[test]
    fn dimensions_auto_formatting() {
        let style = ComputedStyle::default();
        let groups = extract_style_entries(&style);
        let dims = &groups[&StyleCategory::Dimensions];
        let width = dims.iter().find(|e| e.name == "width").unwrap();
        assert_eq!(width.value, "auto"); // NAN → "auto"
    }

    #[test]
    fn color_formatting_opaque() {
        let c = vex_core::Color {
            r: 255,
            g: 0,
            b: 128,
            a: 255,
        };
        let s = format_color(&c);
        assert_eq!(s, "#ff0080");
    }

    #[test]
    fn color_formatting_with_alpha() {
        let c = vex_core::Color {
            r: 255,
            g: 0,
            b: 128,
            a: 128,
        };
        let s = format_color(&c);
        assert!(s.starts_with("rgba(255, 0, 128,"));
    }

    #[test]
    fn format_px_infinity_is_none() {
        assert_eq!(format_auto_px(f32::INFINITY), "none");
    }

    #[test]
    fn inherited_properties_marked() {
        let style = ComputedStyle::default();
        let groups = extract_style_entries(&style);
        let typography = &groups[&StyleCategory::Typography];

        // font-family should be marked inherited.
        let ff = typography.iter().find(|e| e.name == "font-family").unwrap();
        assert!(ff.inherited);

        // font-size should be inherited.
        let fs = typography.iter().find(|e| e.name == "font-size").unwrap();
        assert!(fs.inherited);
    }

    #[test]
    fn all_categories_covered() {
        let style = ComputedStyle::default();
        let groups = extract_style_entries(&style);

        for cat in StyleCategory::ALL {
            assert!(groups.contains_key(cat), "missing category: {:?}", cat);
        }
    }

    // ── Box Model Overlay Tests ──

    #[test]
    fn overlay_colors_are_semitransparent() {
        // Verify our overlay colors are semi-transparent (alpha < 255).
        let colors = [
            BoxModelColors::MARGIN,
            BoxModelColors::BORDER,
            BoxModelColors::PADDING,
            BoxModelColors::CONTENT,
        ];
        for c in &colors {
            assert!(
                c.a < 255,
                "overlay color should be semi-transparent: {:?}",
                c
            );
        }
    }

    #[test]
    fn compute_overlay_no_edges() {
        let content = Rect::new(100.0, 100.0, 200.0, 50.0);
        let zero = Insets::uniform(0.0);
        let overlay = compute_box_model_overlay(content, zero, zero, zero);

        assert_eq!(overlay.content_rect, content);
        assert_eq!(overlay.padding_rect, content);
        assert_eq!(overlay.border_rect, content);
        assert_eq!(overlay.margin_rect, content);
    }

    #[test]
    fn compute_overlay_with_padding() {
        let content = Rect::new(100.0, 100.0, 200.0, 50.0);
        let padding = Insets::uniform(10.0);
        let zero = Insets::uniform(0.0);
        let overlay = compute_box_model_overlay(content, padding, zero, zero);

        assert_eq!(overlay.content_rect, content);
        assert!((overlay.padding_rect.origin.x - 90.0).abs() < 0.1);
        assert!((overlay.padding_rect.size.width - 220.0).abs() < 0.1);
    }

    #[test]
    fn compute_overlay_all_layers() {
        let content = Rect::new(50.0, 50.0, 100.0, 40.0);
        let padding = Insets::new(5.0, 5.0, 5.0, 5.0);
        let border = Insets::new(2.0, 2.0, 2.0, 2.0);
        let margin = Insets::new(10.0, 10.0, 10.0, 10.0);
        let overlay = compute_box_model_overlay(content, padding, border, margin);

        // Each layer should be progressively larger.
        assert!(overlay.padding_rect.size.width > overlay.content_rect.size.width);
        assert!(overlay.border_rect.size.width > overlay.padding_rect.size.width);
        assert!(overlay.margin_rect.size.width > overlay.border_rect.size.width);

        // Margin rect should be 50 - 5 - 2 - 10 = 33 origin x.
        assert!((overlay.margin_rect.origin.x - 33.0).abs() < 0.1);
    }

    #[test]
    fn expand_rect_symmetric() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        let insets = Insets::uniform(5.0);
        let expanded = expand_rect(r, insets);
        assert!((expanded.origin.x - 5.0).abs() < 0.1);
        assert!((expanded.origin.y - 15.0).abs() < 0.1);
        assert!((expanded.size.width - 110.0).abs() < 0.1);
        assert!((expanded.size.height - 60.0).abs() < 0.1);
    }
}
