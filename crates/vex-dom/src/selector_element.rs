// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `selectors::Element` implementation that wraps our arena-based DOM,
//! enabling CSS selector matching (querySelector, querySelectorAll).

use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::context::MatchingContext;
use selectors::matching::{matches_selector_list, ElementSelectorFlags};
use selectors::OpaqueElement;

use crate::arena::NodeArena;
use crate::node::{Namespace, NodeData};
use crate::selector_impl::{
    VexAttrValue, VexIdentifier, VexLocalName, VexNamespaceUrl, VexPseudoClass, VexPseudoElement,
    VexSelectorImpl,
};
use crate::traversal::Children;
use vex_core::VexId;

/// A lightweight handle referencing an element in a [`NodeArena`].
///
/// Implements `selectors::Element` so the Mozilla selector-matching engine can
/// walk our tree.  Cheap to clone (two words: pointer + u32).
#[derive(Clone, Debug)]
pub struct VexElement<'a> {
    pub(crate) arena: &'a NodeArena,
    pub(crate) id: VexId,
}

impl<'a> VexElement<'a> {
    /// Create a new element handle.
    ///
    /// # Panics
    /// Panics if `id` does not refer to an `Element` node.
    pub fn new(arena: &'a NodeArena, id: VexId) -> Self {
        debug_assert!(
            matches!(arena.get(id).data, NodeData::Element(_)),
            "VexElement must wrap an Element node"
        );
        Self { arena, id }
    }

    /// Try to construct; returns `None` if the node is not an element.
    pub fn try_new(arena: &'a NodeArena, id: VexId) -> Option<Self> {
        if matches!(arena.get(id).data, NodeData::Element(_)) {
            Some(Self { arena, id })
        } else {
            None
        }
    }

    /// The underlying `VexId`.
    pub fn id(&self) -> VexId {
        self.id
    }

    /// Borrow element data (panics if not an element — which can't happen
    /// if the handle was built correctly).
    fn elem_data(&self) -> &crate::node::ElementData {
        match &self.arena.get(self.id).data {
            NodeData::Element(e) => e,
            _ => unreachable!(),
        }
    }
}

// ── selectors::Element ──────────────────────────────────────────────

impl<'a> selectors::Element for VexElement<'a> {
    type Impl = VexSelectorImpl;

    fn opaque(&self) -> OpaqueElement {
        // Safe: we only need identity comparison.  We use the stable pointer
        // returned by `NodeArena::get` which points into the Vec<Node>.
        OpaqueElement::new(self.arena.get(self.id))
    }

    fn parent_element(&self) -> Option<Self> {
        let mut pid = self.arena.get(self.id).parent;
        while let Some(p) = pid {
            if matches!(self.arena.get(p).data, NodeData::Element(_)) {
                return Some(Self { arena: self.arena, id: p });
            }
            pid = self.arena.get(p).parent;
        }
        None
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false // No shadow DOM.
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let mut sib = self.arena.get(self.id).prev_sibling;
        while let Some(s) = sib {
            if matches!(self.arena.get(s).data, NodeData::Element(_)) {
                return Some(Self { arena: self.arena, id: s });
            }
            sib = self.arena.get(s).prev_sibling;
        }
        None
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let mut sib = self.arena.get(self.id).next_sibling;
        while let Some(s) = sib {
            if matches!(self.arena.get(s).data, NodeData::Element(_)) {
                return Some(Self { arena: self.arena, id: s });
            }
            sib = self.arena.get(s).next_sibling;
        }
        None
    }

    fn first_element_child(&self) -> Option<Self> {
        let mut child = self.arena.get(self.id).first_child;
        while let Some(c) = child {
            if matches!(self.arena.get(c).data, NodeData::Element(_)) {
                return Some(Self { arena: self.arena, id: c });
            }
            child = self.arena.get(c).next_sibling;
        }
        None
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.elem_data().namespace == Namespace::Html
    }

    fn has_local_name(&self, local_name: &str) -> bool {
        self.elem_data().tag_name == *local_name
    }

    fn has_namespace(&self, ns: &str) -> bool {
        if ns.is_empty() {
            // Empty string = no namespace constraint.
            return true;
        }
        let own = match self.elem_data().namespace {
            Namespace::Html => "http://www.w3.org/1999/xhtml",
            Namespace::Svg => "http://www.w3.org/2000/svg",
            Namespace::MathMl => "http://www.w3.org/1998/Math/MathML",
        };
        own == ns
    }

    fn is_same_type(&self, other: &Self) -> bool {
        let a = self.elem_data();
        let b = other.elem_data();
        a.tag_name == b.tag_name && a.namespace == b.namespace
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&VexNamespaceUrl>,
        local_name: &VexLocalName,
        operation: &AttrSelectorOperation<&VexAttrValue>,
    ) -> bool {
        let el = self.elem_data();
        el.attributes.iter().any(|attr| {
            // Check attribute name.
            if attr.name != local_name.0 {
                return false;
            }
            // Namespace constraint (simplified: attributes are unnamespaced).
            match ns {
                NamespaceConstraint::Any => {}
                NamespaceConstraint::Specific(url) => {
                    if !url.0.is_empty() {
                        return false; // We don't store attribute namespaces.
                    }
                }
            }
            operation.eval_str(&attr.value)
        })
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &VexPseudoClass,
        _context: &mut MatchingContext<VexSelectorImpl>,
    ) -> bool {
        // VexPseudoClass is uninhabited.
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &VexPseudoElement,
        _context: &mut MatchingContext<VexSelectorImpl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {
        // No invalidation system yet — ignore.
    }

    fn is_link(&self) -> bool {
        let tag = &self.elem_data().tag_name;
        (tag == "a" || tag == "area" || tag == "link")
            && self.elem_data().attributes.iter().any(|a| a.name == "href")
    }

    fn is_html_slot_element(&self) -> bool {
        self.elem_data().tag_name == "slot" && self.elem_data().namespace == Namespace::Html
    }

    fn has_id(&self, id: &VexIdentifier, case_sensitivity: CaseSensitivity) -> bool {
        self.elem_data().attributes.iter().any(|a| {
            a.name == "id" && case_sensitivity.eq(a.value.as_bytes(), id.0.as_bytes())
        })
    }

    fn has_class(&self, name: &VexIdentifier, case_sensitivity: CaseSensitivity) -> bool {
        self.elem_data().attributes.iter().any(|a| {
            a.name == "class"
                && a.value
                    .split_whitespace()
                    .any(|c| case_sensitivity.eq(c.as_bytes(), name.0.as_bytes()))
        })
    }

    fn has_custom_state(&self, _name: &VexIdentifier) -> bool {
        false
    }

    fn imported_part(&self, _name: &VexIdentifier) -> Option<VexIdentifier> {
        None
    }

    fn is_part(&self, _name: &VexIdentifier) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        // Empty = no child elements and no non-zero-length text nodes.
        Children::new(self.arena, self.id).all(|cid| {
            match &self.arena.get(cid).data {
                NodeData::Element(_) => false,
                NodeData::Text(t) => t.is_empty(),
                _ => true,
            }
        })
    }

    fn is_root(&self) -> bool {
        // Root if parent is the Document node.
        match self.arena.get(self.id).parent {
            Some(pid) => matches!(self.arena.get(pid).data, NodeData::Document),
            None => false,
        }
    }

    fn add_element_unique_hashes(&self, filter: &mut BloomFilter) -> bool {
        use precomputed_hash::PrecomputedHash;
        use crate::selector_impl::{VexLocalName, VexIdentifier};

        let el = self.elem_data();
        let name = VexLocalName(el.tag_name.clone());
        filter.insert_hash(name.precomputed_hash());

        let added = true;
        for attr in &el.attributes {
            if attr.name == "id" {
                let id = VexIdentifier(attr.value.clone());
                filter.insert_hash(id.precomputed_hash());
            }
        }
        added
    }
}

// ── querySelector / querySelectorAll helpers ────────────────────────

use selectors::context::{
    MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode, SelectorCaches,
};

/// Match all elements under `root` that match `selector_str`.
pub fn query_selector_all(
    arena: &NodeArena,
    root: VexId,
    selector_str: &str,
) -> Result<Vec<VexId>, String> {
    let selectors = crate::selector_impl::parse_selector(selector_str)
        .map_err(|()| format!("invalid selector: {selector_str}"))?;

    let mut caches = SelectorCaches::default();
    let mut results = Vec::new();

    // Walk every descendant element.
    let mut stack = vec![root];
    while let Some(nid) = stack.pop() {
        // Push children in reverse so we visit them in document order.
        let mut children: Vec<VexId> = Children::new(arena, nid).collect();
        children.reverse();
        stack.extend(children);

        // Only match element nodes (skip root itself if it's Document).
        if let Some(el) = VexElement::try_new(arena, nid) {
            let mut ctx = MatchingContext::new(
                MatchingMode::Normal,
                None,
                &mut caches,
                QuirksMode::NoQuirks,
                NeedsSelectorFlags::No,
                MatchingForInvalidation::No,
            );
            if matches_selector_list(&selectors, &el, &mut ctx) {
                results.push(nid);
            }
        }
    }

    Ok(results)
}

/// Match the first element under `root` that matches `selector_str`.
pub fn query_selector(
    arena: &NodeArena,
    root: VexId,
    selector_str: &str,
) -> Result<Option<VexId>, String> {
    let selectors = crate::selector_impl::parse_selector(selector_str)
        .map_err(|()| format!("invalid selector: {selector_str}"))?;

    let mut caches = SelectorCaches::default();
    let mut stack = vec![root];

    while let Some(nid) = stack.pop() {
        let mut children: Vec<VexId> = Children::new(arena, nid).collect();
        children.reverse();
        stack.extend(children);

        if let Some(el) = VexElement::try_new(arena, nid) {
            let mut ctx = MatchingContext::new(
                MatchingMode::Normal,
                None,
                &mut caches,
                QuirksMode::NoQuirks,
                NeedsSelectorFlags::No,
                MatchingForInvalidation::No,
            );
            if matches_selector_list(&selectors, &el, &mut ctx) {
                return Ok(Some(nid));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{attributes, tree};
    use crate::node::{ElementData, Namespace, NodeData};

    fn make_element(arena: &mut NodeArena, tag: &str) -> VexId {
        arena.alloc(NodeData::Element(ElementData {
            tag_name: tag.to_string(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }))
    }

    fn build_simple_tree() -> (NodeArena, VexId) {
        // <doc>
        //   <html>
        //     <body>
        //       <div id="main" class="container">
        //         <p class="intro">Hello</p>
        //         <p>World</p>
        //       </div>
        //       <span>Bye</span>
        //     </body>
        //   </html>
        // </doc>
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let html = make_element(&mut arena, "html");
        let body = make_element(&mut arena, "body");
        let div = make_element(&mut arena, "div");
        let p1 = make_element(&mut arena, "p");
        let p2 = make_element(&mut arena, "p");
        let span = make_element(&mut arena, "span");
        let t1 = arena.alloc(NodeData::Text("Hello".into()));
        let t2 = arena.alloc(NodeData::Text("World".into()));
        let t3 = arena.alloc(NodeData::Text("Bye".into()));

        tree::append_child(&mut arena, doc, html);
        tree::append_child(&mut arena, html, body);
        tree::append_child(&mut arena, body, div);
        tree::append_child(&mut arena, body, span);

        attributes::set_attribute(&mut arena, div, "id", "main");
        attributes::set_attribute(&mut arena, div, "class", "container");
        attributes::set_attribute(&mut arena, p1, "class", "intro");

        tree::append_child(&mut arena, div, p1);
        tree::append_child(&mut arena, div, p2);
        tree::append_child(&mut arena, p1, t1);
        tree::append_child(&mut arena, p2, t2);
        tree::append_child(&mut arena, span, t3);

        (arena, doc)
    }

    #[test]
    fn query_by_tag() {
        let (arena, doc) = build_simple_tree();
        let ps = query_selector_all(&arena, doc, "p").unwrap();
        assert_eq!(ps.len(), 2);
    }

    #[test]
    fn query_by_id() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector(&arena, doc, "#main").unwrap();
        assert!(r.is_some());
        let el = r.unwrap();
        if let NodeData::Element(ref e) = arena.get(el).data {
            assert_eq!(e.tag_name, "div");
        }
    }

    #[test]
    fn query_by_class() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, ".intro").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn query_descendant_combinator() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, "div p").unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn query_child_combinator() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, "body > p").unwrap();
        assert_eq!(r.len(), 0); // p is child of div, not body
    }

    #[test]
    fn query_compound() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, "p.intro").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn query_no_match() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector(&arena, doc, "h1").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn query_comma_list() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, "div, span").unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn query_attribute_selector() {
        let (arena, doc) = build_simple_tree();
        let r = query_selector_all(&arena, doc, "[id=\"main\"]").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn query_invalid_selector_returns_err() {
        let (arena, doc) = build_simple_tree();
        assert!(query_selector_all(&arena, doc, "!!!").is_err());
    }
}
