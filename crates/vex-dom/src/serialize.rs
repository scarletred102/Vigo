// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Serialize a DOM subtree back to HTML.

use vex_core::VexId;

use crate::arena::NodeArena;
use crate::node::NodeData;

/// Serialize the subtree rooted at `id` to an HTML string.
pub fn serialize(arena: &NodeArena, id: VexId) -> String {
    let mut out = String::new();
    serialize_node(arena, id, &mut out);
    out
}

fn serialize_node(arena: &NodeArena, id: VexId, out: &mut String) {
    match &arena.get(id).data {
        NodeData::Document => serialize_children(arena, id, out),
        NodeData::Element(el) => {
            out.push('<');
            out.push_str(&el.tag_name);
            for attr in &el.attributes {
                out.push(' ');
                out.push_str(&attr.name);
                out.push_str("=\"");
                escape_attr(&attr.value, out);
                out.push('"');
            }
            out.push('>');
            if !is_void(&el.tag_name) {
                serialize_children(arena, id, out);
                out.push_str("</");
                out.push_str(&el.tag_name);
                out.push('>');
            }
        }
        NodeData::Text(text) => escape_text(text, out),
        NodeData::Comment(text) => {
            out.push_str("<!--");
            out.push_str(text);
            out.push_str("-->");
        }
        NodeData::Doctype { name, .. } => {
            out.push_str("<!DOCTYPE ");
            out.push_str(name);
            out.push('>');
        }
    }
}

fn serialize_children(arena: &NodeArena, id: VexId, out: &mut String) {
    let mut child = arena.get(id).first_child;
    while let Some(cid) = child {
        serialize_node(arena, cid, out);
        child = arena.get(cid).next_sibling;
    }
}

fn escape_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

fn escape_attr(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::NodeArena;
    use crate::node::{Attribute, ElementData, Namespace, NodeData};
    use crate::tree::append_child;

    fn make_element(
        arena: &mut NodeArena,
        tag: &str,
        attrs: Vec<(&str, &str)>,
    ) -> VexId {
        arena.alloc(NodeData::Element(ElementData {
            tag_name: tag.into(),
            namespace: Namespace::Html,
            attributes: attrs
                .into_iter()
                .map(|(n, v)| Attribute {
                    name: n.into(),
                    value: v.into(),
                })
                .collect(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }))
    }

    #[test]
    fn simple_element() {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let div = make_element(&mut arena, "div", vec![("class", "box")]);
        let text = arena.alloc(NodeData::Text("hello".into()));
        append_child(&mut arena, root, div);
        append_child(&mut arena, div, text);

        assert_eq!(serialize(&arena, root), r#"<div class="box">hello</div>"#);
    }

    #[test]
    fn void_element() {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let br = make_element(&mut arena, "br", vec![]);
        append_child(&mut arena, root, br);

        assert_eq!(serialize(&arena, root), "<br>");
    }

    #[test]
    fn escaping() {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let text = arena.alloc(NodeData::Text("a < b & c > d".into()));
        append_child(&mut arena, root, text);

        assert_eq!(serialize(&arena, root), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn comment_and_doctype() {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let dt = arena.alloc(NodeData::Doctype {
            name: "html".into(),
            public_id: String::new(),
            system_id: String::new(),
        });
        let comment = arena.alloc(NodeData::Comment(" hi ".into()));
        append_child(&mut arena, root, dt);
        append_child(&mut arena, root, comment);

        assert_eq!(serialize(&arena, root), "<!DOCTYPE html><!-- hi -->");
    }

    #[test]
    fn nested_structure() {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        let ul = make_element(&mut arena, "ul", vec![]);
        let li1 = make_element(&mut arena, "li", vec![]);
        let li2 = make_element(&mut arena, "li", vec![]);
        let t1 = arena.alloc(NodeData::Text("one".into()));
        let t2 = arena.alloc(NodeData::Text("two".into()));
        append_child(&mut arena, root, ul);
        append_child(&mut arena, ul, li1);
        append_child(&mut arena, li1, t1);
        append_child(&mut arena, ul, li2);
        append_child(&mut arena, li2, t2);

        assert_eq!(
            serialize(&arena, root),
            "<ul><li>one</li><li>two</li></ul>"
        );
    }
}
