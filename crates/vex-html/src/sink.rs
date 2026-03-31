// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! html5ever `TreeSink` that builds a `vex-dom` `Document`.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use html5ever::tree_builder::{ElemName, ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{
    namespace_url, ns, Attribute as HtmlAttribute, LocalName, Namespace as MkNs, QualName,
};
use tendril::StrTendril;

use vex_core::VexId;
use vex_dom::node::{Attribute, ElementData, ElementState, Namespace, NodeData};
use vex_dom::{tree, Document};

// ── Owned ElemName wrapper (avoids lifetime issues with RefCell) ──────

/// An owned element-name that satisfies html5ever's `ElemName` trait.
#[derive(Debug)]
pub struct VexElemName {
    local: LocalName,
    ns: MkNs,
}

impl ElemName for VexElemName {
    fn local_name(&self) -> &LocalName {
        &self.local
    }
    fn ns(&self) -> &MkNs {
        &self.ns
    }
}

// ── Conversions ──────────────────────────────────────────────────────

fn convert_ns(ns: &MkNs) -> Namespace {
    if *ns == ns!(html) {
        Namespace::Html
    } else if *ns == ns!(svg) {
        Namespace::Svg
    } else if *ns == ns!(mathml) {
        Namespace::MathMl
    } else {
        Namespace::Html // fallback
    }
}

fn convert_attrs(attrs: Vec<HtmlAttribute>) -> Vec<Attribute> {
    attrs
        .into_iter()
        .map(|a| {
            let name = match a.name.prefix {
                Some(ref prefix) => format!("{}:{}", prefix, a.name.local),
                None => a.name.local.to_string(),
            };
            Attribute {
                name,
                value: a.value.to_string(),
            }
        })
        .collect()
}

/// Append `child` to `parent`, merging adjacent text nodes.
fn sink_append(doc: &mut Document, parent: VexId, child: NodeOrText<VexId>) {
    match child {
        NodeOrText::AppendNode(id) => {
            doc.append_child(parent, id);
        }
        NodeOrText::AppendText(text) => {
            let last = doc.arena().get(parent).last_child;
            if let Some(last_id) = last {
                if let NodeData::Text(ref mut existing) = doc.arena_mut().get_mut(last_id).data {
                    existing.push_str(&text);
                    return;
                }
            }
            let text_id = doc.create_text(&text);
            doc.append_child(parent, text_id);
        }
    }
}

// ── The TreeSink ─────────────────────────────────────────────────────

/// html5ever tree-builder sink that constructs a [`Document`].
pub struct VexSink {
    doc: RefCell<Document>,
    /// QualName stored per element id (for `elem_name`).
    names: RefCell<HashMap<u32, QualName>>,
    quirks_mode: Cell<QuirksMode>,
}

impl VexSink {
    pub fn new() -> Self {
        Self {
            doc: RefCell::new(Document::new()),
            names: RefCell::new(HashMap::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
        }
    }
}

impl TreeSink for VexSink {
    type Handle = VexId;
    type Output = Document;
    type ElemName<'a> = VexElemName;

    fn finish(self) -> Document {
        self.doc.into_inner()
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {
        // Silently swallow — this is normal for real-world HTML.
    }

    fn get_document(&self) -> VexId {
        self.doc.borrow().root()
    }

    fn elem_name<'a>(&'a self, target: &'a VexId) -> VexElemName {
        let names = self.names.borrow();
        let qn = match names.get(&target.index()) {
            Some(qn) => qn,
            None => panic!("elem_name called on a node without a recorded QualName"),
        };
        VexElemName {
            local: qn.local.clone(),
            ns: qn.ns.clone(),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<HtmlAttribute>,
        flags: ElementFlags,
    ) -> VexId {
        let mut doc = self.doc.borrow_mut();
        let ns = convert_ns(&name.ns);
        let tag = name.local.to_string();
        let dom_attrs = convert_attrs(attrs);

        let id = doc.arena_mut().alloc(NodeData::Element(ElementData {
            tag_name: tag,
            namespace: ns,
            attributes: dom_attrs,
            template_contents: None,
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
            state: ElementState::default(),
        }));

        // Store QualName for later `elem_name` lookups.
        self.names.borrow_mut().insert(id.index(), name);

        // <template> elements get an associated document fragment.
        if flags.template {
            let frag_id = doc.arena_mut().alloc(NodeData::Document);
            if let NodeData::Element(ref mut el) = doc.arena_mut().get_mut(id).data {
                el.template_contents = Some(frag_id);
            }
        }

        id
    }

    fn create_comment(&self, text: StrTendril) -> VexId {
        self.doc.borrow_mut().create_comment(&text)
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> VexId {
        // Processing instructions are exotic in HTML; store as a comment.
        self.doc
            .borrow_mut()
            .create_comment(&format!("?{} {}", target, data))
    }

    fn append(&self, parent: &VexId, child: NodeOrText<VexId>) {
        let mut doc = self.doc.borrow_mut();
        sink_append(&mut doc, *parent, child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &VexId,
        prev_element: &VexId,
        child: NodeOrText<VexId>,
    ) {
        let has_parent = self.doc.borrow().arena().get(*element).parent.is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let mut doc = self.doc.borrow_mut();
        let root = doc.root();
        let dt = doc.arena_mut().alloc(NodeData::Doctype {
            name: name.to_string(),
            public_id: public_id.to_string(),
            system_id: system_id.to_string(),
        });
        doc.append_child(root, dt);
    }

    fn get_template_contents(&self, target: &VexId) -> VexId {
        let doc = self.doc.borrow();
        if let NodeData::Element(ref el) = doc.arena().get(*target).data {
            match el.template_contents {
                Some(contents) => contents,
                None => panic!("get_template_contents on non-template element"),
            }
        } else {
            panic!("get_template_contents called on a non-element node");
        }
    }

    fn same_node(&self, x: &VexId, y: &VexId) -> bool {
        *x == *y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn append_before_sibling(&self, sibling: &VexId, new_node: NodeOrText<VexId>) {
        let mut doc = self.doc.borrow_mut();
        let parent = match doc.arena().get(*sibling).parent {
            Some(parent) => parent,
            None => panic!("append_before_sibling: sibling has no parent"),
        };

        match new_node {
            NodeOrText::AppendNode(id) => {
                tree::insert_before(doc.arena_mut(), parent, id, *sibling);
            }
            NodeOrText::AppendText(text) => {
                // Merge with preceding text node if possible.
                let prev = doc.arena().get(*sibling).prev_sibling;
                if let Some(prev_id) = prev {
                    if let NodeData::Text(ref mut existing) = doc.arena_mut().get_mut(prev_id).data
                    {
                        existing.push_str(&text);
                        return;
                    }
                }
                let text_id = doc.create_text(&text);
                tree::insert_before(doc.arena_mut(), parent, text_id, *sibling);
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &VexId, attrs: Vec<HtmlAttribute>) {
        let mut doc = self.doc.borrow_mut();
        if let NodeData::Element(ref mut el) = doc.arena_mut().get_mut(*target).data {
            for attr in attrs {
                let name = attr.name.local.to_string();
                if !el.attributes.iter().any(|a| a.name == name) {
                    el.attributes.push(Attribute {
                        name,
                        value: attr.value.to_string(),
                    });
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &VexId) {
        tree::detach(self.doc.borrow_mut().arena_mut(), *target);
    }

    fn reparent_children(&self, node: &VexId, new_parent: &VexId) {
        tree::reparent_children(self.doc.borrow_mut().arena_mut(), *node, *new_parent);
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &VexId) -> bool {
        let doc = self.doc.borrow();
        if let NodeData::Element(ref el) = doc.arena().get(*handle).data {
            el.mathml_annotation_xml_integration_point
        } else {
            false
        }
    }
}
