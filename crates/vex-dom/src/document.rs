// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! High-level `Document` facade over the DOM.

use vex_core::VexId;

use crate::arena::NodeArena;
use crate::forms::FormStateMap;
use crate::node::{ElementData, ElementState, Namespace, NodeData};
use crate::traversal::{Children, Descendants};
use crate::tree;

/// An in-memory DOM document.
///
/// Owns its [`NodeArena`] and provides factory / query helpers.
pub struct Document {
    arena: NodeArena,
    root: VexId,
    form_states: FormStateMap,
}

impl Document {
    /// Create an empty document (one `Document` root node).
    pub fn new() -> Self {
        let mut arena = NodeArena::new();
        let root = arena.alloc(NodeData::Document);
        Self {
            arena,
            root,
            form_states: FormStateMap::new(),
        }
    }

    /// The root `#document` node id.
    pub fn root(&self) -> VexId {
        self.root
    }

    /// The first `Element` child of the root (usually `<html>`).
    pub fn root_element(&self) -> Option<VexId> {
        Children::new(&self.arena, self.root)
            .find(|&id| matches!(self.arena.get(id).data, NodeData::Element(_)))
    }

    /// Alias for [`Self::root_element`], matching browser terminology.
    pub fn document_element(&self) -> Option<VexId> {
        self.root_element()
    }

    /// The `<head>` element if present.
    pub fn head(&self) -> Option<VexId> {
        let html = self.root_element()?;
        Children::new(&self.arena, html).find(|&id| {
            matches!(
                &self.arena.get(id).data,
                NodeData::Element(el) if el.tag_name == "head"
            )
        })
    }

    /// The `<body>` element if present.
    pub fn body(&self) -> Option<VexId> {
        let html = self.root_element()?;
        Children::new(&self.arena, html).find(|&id| {
            matches!(
                &self.arena.get(id).data,
                NodeData::Element(el) if el.tag_name == "body"
            )
        })
    }

    /// Borrow the node arena.
    pub fn arena(&self) -> &NodeArena {
        &self.arena
    }

    /// Mutably borrow the node arena.
    pub fn arena_mut(&mut self) -> &mut NodeArena {
        &mut self.arena
    }

    /// Borrow the form state map.
    pub fn form_states(&self) -> &FormStateMap {
        &self.form_states
    }

    /// Mutably borrow the form state map.
    pub fn form_states_mut(&mut self) -> &mut FormStateMap {
        &mut self.form_states
    }

    // ── Factory methods ──────────────────────────────────────────────

    /// Create a detached element node.
    pub fn create_element(&mut self, tag: &str, ns: Namespace) -> VexId {
        self.arena.alloc(NodeData::Element(ElementData {
            tag_name: tag.to_string(),
            namespace: ns,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
            state: ElementState::default(),
        }))
    }

    /// Create a detached text node.
    pub fn create_text(&mut self, text: &str) -> VexId {
        self.arena.alloc(NodeData::Text(text.to_string()))
    }

    /// Create a detached comment node.
    pub fn create_comment(&mut self, text: &str) -> VexId {
        self.arena.alloc(NodeData::Comment(text.to_string()))
    }

    // ── Tree ops ─────────────────────────────────────────────────────

    pub fn append_child(&mut self, parent: VexId, child: VexId) {
        tree::append_child(&mut self.arena, parent, child);
    }

    pub fn insert_before(&mut self, parent: VexId, child: VexId, reference: VexId) {
        tree::insert_before(&mut self.arena, parent, child, reference);
    }

    pub fn remove_child(&mut self, parent: VexId, child: VexId) {
        tree::remove_child(&mut self.arena, parent, child);
    }

    // ── Queries ──────────────────────────────────────────────────────

    /// First element whose `id` attribute equals `id_value`.
    pub fn get_element_by_id(&self, id_value: &str) -> Option<VexId> {
        Descendants::new(&self.arena, self.root).find(|&nid| {
            if let NodeData::Element(ref el) = self.arena.get(nid).data {
                el.attributes
                    .iter()
                    .any(|a| a.name == "id" && a.value == id_value)
            } else {
                false
            }
        })
    }

    /// All elements with the given tag name (case-insensitive).
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<VexId> {
        let tag_lower = tag.to_ascii_lowercase();
        Descendants::new(&self.arena, self.root)
            .filter(|&nid| {
                if let NodeData::Element(ref el) = self.arena.get(nid).data {
                    el.tag_name == tag_lower
                } else {
                    false
                }
            })
            .collect()
    }

    /// All elements that have `class` in their space-separated class list.
    pub fn get_elements_by_class_name(&self, class: &str) -> Vec<VexId> {
        Descendants::new(&self.arena, self.root)
            .filter(|&nid| {
                if let NodeData::Element(ref el) = self.arena.get(nid).data {
                    el.attributes.iter().any(|a| {
                        a.name == "class" && a.value.split_whitespace().any(|c| c == class)
                    })
                } else {
                    false
                }
            })
            .collect()
    }

    /// First element matching a CSS selector string.
    pub fn query_selector(&self, selector: &str) -> Result<Option<VexId>, String> {
        crate::selector_element::query_selector(&self.arena, self.root, selector)
    }

    /// All elements matching a CSS selector string, in document order.
    pub fn query_selector_all(&self, selector: &str) -> Result<Vec<VexId>, String> {
        crate::selector_element::query_selector_all(&self.arena, self.root, selector)
    }

    /// Concatenate all descendant text nodes under `node`.
    pub fn text_content(&self, node: VexId) -> String {
        let mut out = String::new();
        self.collect_text(&mut out, node);
        out
    }

    fn collect_text(&self, out: &mut String, node: VexId) {
        match &self.arena.get(node).data {
            NodeData::Text(t) => out.push_str(t),
            _ => {
                let mut child = self.arena.get(node).first_child;
                while let Some(id) = child {
                    self.collect_text(out, id);
                    child = self.arena.get(id).next_sibling;
                }
            }
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes;
    use crate::node::Namespace;

    #[test]
    fn root_element_is_html() {
        let mut doc = Document::new();
        let html = doc.create_element("html", Namespace::Html);
        doc.append_child(doc.root(), html);
        assert_eq!(doc.root_element(), Some(html));
    }

    #[test]
    fn get_element_by_id_found() {
        let mut doc = Document::new();
        let root = doc.root();
        let div = doc.create_element("div", Namespace::Html);
        attributes::set_attribute(doc.arena_mut(), div, "id", "main");
        doc.append_child(root, div);

        assert_eq!(doc.get_element_by_id("main"), Some(div));
        assert_eq!(doc.get_element_by_id("nope"), None);
    }

    #[test]
    fn get_elements_by_tag_name_multiple() {
        let mut doc = Document::new();
        let root = doc.root();
        let p1 = doc.create_element("p", Namespace::Html);
        let p2 = doc.create_element("p", Namespace::Html);
        let div = doc.create_element("div", Namespace::Html);
        doc.append_child(root, p1);
        doc.append_child(root, div);
        doc.append_child(root, p2);

        let ps = doc.get_elements_by_tag_name("p");
        assert_eq!(ps, vec![p1, p2]);
    }

    #[test]
    fn get_elements_by_class_name_space_separated() {
        let mut doc = Document::new();
        let root = doc.root();
        let div = doc.create_element("div", Namespace::Html);
        attributes::set_attribute(doc.arena_mut(), div, "class", "foo bar baz");
        doc.append_child(root, div);

        assert_eq!(doc.get_elements_by_class_name("bar"), vec![div]);
        assert!(doc.get_elements_by_class_name("qux").is_empty());
    }

    #[test]
    fn text_content_nested() {
        let mut doc = Document::new();
        let root = doc.root();
        let p = doc.create_element("p", Namespace::Html);
        let t1 = doc.create_text("Hello ");
        let span = doc.create_element("span", Namespace::Html);
        let t2 = doc.create_text("world");
        doc.append_child(root, p);
        doc.append_child(p, t1);
        doc.append_child(p, span);
        doc.append_child(span, t2);

        assert_eq!(doc.text_content(p), "Hello world");
    }

    #[test]
    fn head_and_body_helpers() {
        let mut doc = Document::new();
        let root = doc.root();

        let html = doc.create_element("html", Namespace::Html);
        let head = doc.create_element("head", Namespace::Html);
        let body = doc.create_element("body", Namespace::Html);

        doc.append_child(root, html);
        doc.append_child(html, head);
        doc.append_child(html, body);

        assert_eq!(doc.document_element(), Some(html));
        assert_eq!(doc.head(), Some(head));
        assert_eq!(doc.body(), Some(body));
    }
}
