// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! `selectors::Element` implementation that wraps our arena-based DOM,
//! enabling CSS selector matching (querySelector, querySelectorAll).

use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::context::MatchingContext;
use selectors::matching::{
    matches_selector_list as selectors_matches_selector_list, ElementSelectorFlags,
};
use selectors::OpaqueElement;

use crate::arena::NodeArena;
use crate::node::{ElementState, Namespace, NodeData};
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

    /// Walk up the DOM to find the nearest `lang` attribute value.
    fn find_lang(&self) -> String {
        let mut nid = Some(self.id);
        while let Some(current) = nid {
            if let NodeData::Element(ref el) = self.arena.get(current).data {
                if let Some(attr) = el.attributes.iter().find(|a| a.name == "lang") {
                    return attr.value.clone();
                }
            }
            nid = self.arena.get(current).parent;
        }
        String::new()
    }
}

/// Check if `actual_lang` matches or is a sub-tag of `expected_lang`.
///
/// Per BCP 47 / CSS :lang() semantics: `"en-US"` matches `"en"`.
fn lang_matches(actual: &str, expected: &str) -> bool {
    if actual.is_empty() {
        return false;
    }
    if actual.eq_ignore_ascii_case(expected) {
        return true;
    }
    // Check prefix: "en-US" starts with "en-"
    if actual.len() > expected.len() {
        let prefix = &actual[..expected.len()];
        let separator = actual.as_bytes()[expected.len()];
        return prefix.eq_ignore_ascii_case(expected) && separator == b'-';
    }
    false
}

fn split_attr_name(name: &str) -> (Option<&str>, &str) {
    if let Some((prefix, local)) = name.split_once(':') {
        (Some(prefix), local)
    } else {
        (None, name)
    }
}

fn attribute_namespace_uri(prefix: Option<&str>) -> &'static str {
    match prefix {
        Some("xlink") => "http://www.w3.org/1999/xlink",
        Some("xml") => "http://www.w3.org/XML/1998/namespace",
        Some("xmlns") => "http://www.w3.org/2000/xmlns/",
        _ => "",
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
                return Some(Self {
                    arena: self.arena,
                    id: p,
                });
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
                return Some(Self {
                    arena: self.arena,
                    id: s,
                });
            }
            sib = self.arena.get(s).prev_sibling;
        }
        None
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let mut sib = self.arena.get(self.id).next_sibling;
        while let Some(s) = sib {
            if matches!(self.arena.get(s).data, NodeData::Element(_)) {
                return Some(Self {
                    arena: self.arena,
                    id: s,
                });
            }
            sib = self.arena.get(s).next_sibling;
        }
        None
    }

    fn first_element_child(&self) -> Option<Self> {
        let mut child = self.arena.get(self.id).first_child;
        while let Some(c) = child {
            if matches!(self.arena.get(c).data, NodeData::Element(_)) {
                return Some(Self {
                    arena: self.arena,
                    id: c,
                });
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
            let (prefix, local) = split_attr_name(&attr.name);
            if local != local_name.0 {
                return false;
            }

            let attr_ns = attribute_namespace_uri(prefix);
            match ns {
                NamespaceConstraint::Any => {}
                NamespaceConstraint::Specific(url) => {
                    if attr_ns != url.0 {
                        return false;
                    }
                }
            }

            operation.eval_str(&attr.value)
        })
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &VexPseudoClass,
        _context: &mut MatchingContext<VexSelectorImpl>,
    ) -> bool {
        let el = self.elem_data();
        match pc {
            // Link pseudo-classes — :link and :any-link match unvisited links.
            VexPseudoClass::AnyLink | VexPseudoClass::Link => self.is_link(),
            // We never track visited state (privacy), so :visited always false.
            VexPseudoClass::Visited => false,
            // :lang() — walk up DOM looking for lang attribute.
            VexPseudoClass::Lang(ref expected_lang) => {
                let actual = self.find_lang();
                lang_matches(&actual, expected_lang)
            }
            // :read-only is the inverse of :read-write
            VexPseudoClass::ReadOnly => !el.state.contains(ElementState::READ_WRITE),
            // All other pseudo-classes map to ElementState flags.
            other => el.state.contains(other.state_flag()),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &VexPseudoElement,
        _context: &mut MatchingContext<VexSelectorImpl>,
    ) -> bool {
        // Pseudo-elements are matched structurally by the `selectors` crate
        // itself — we return true if the element supports the pseudo-element.
        // For now, all elements can have ::before/::after/etc.
        true
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
        self.elem_data()
            .attributes
            .iter()
            .any(|a| a.name == "id" && case_sensitivity.eq(a.value.as_bytes(), id.0.as_bytes()))
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
        Children::new(self.arena, self.id).all(|cid| match &self.arena.get(cid).data {
            NodeData::Element(_) => false,
            NodeData::Text(t) => t.is_empty(),
            _ => true,
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
        use crate::selector_impl::{VexIdentifier, VexLocalName};
        use precomputed_hash::PrecomputedHash;

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

fn build_matching_context<'a>(
    caches: &'a mut SelectorCaches,
) -> MatchingContext<'a, VexSelectorImpl> {
    MatchingContext::new(
        MatchingMode::Normal,
        None,
        caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    )
}

/// Check whether a specific element matches a pre-parsed selector list.
pub fn matches_selector_list(
    arena: &NodeArena,
    element_id: VexId,
    selectors: &selectors::parser::SelectorList<VexSelectorImpl>,
) -> bool {
    let Some(el) = VexElement::try_new(arena, element_id) else {
        return false;
    };

    let mut caches = SelectorCaches::default();
    let mut ctx = build_matching_context(&mut caches);
    selectors_matches_selector_list(selectors, &el, &mut ctx)
}

/// Check whether a specific element matches a selector string.
pub fn matches_selector(
    arena: &NodeArena,
    element_id: VexId,
    selector_str: &str,
) -> Result<bool, String> {
    let selectors = crate::selector_impl::parse_selector(selector_str)
        .map_err(|()| format!("invalid selector: {selector_str}"))?;
    Ok(matches_selector_list(arena, element_id, &selectors))
}

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
            let mut ctx = build_matching_context(&mut caches);
            if selectors_matches_selector_list(&selectors, &el, &mut ctx) {
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
            let mut ctx = build_matching_context(&mut caches);
            if selectors_matches_selector_list(&selectors, &el, &mut ctx) {
                return Ok(Some(nid));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{ElementData, ElementState, Namespace, NodeData};
    use crate::{attributes, tree};

    fn make_element(arena: &mut NodeArena, tag: &str) -> VexId {
        arena.alloc(NodeData::Element(ElementData {
            tag_name: tag.to_string(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
            state: ElementState::default(),
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
    fn matches_selector_single_element() {
        let (arena, doc) = build_simple_tree();
        let p = query_selector(&arena, doc, "p.intro").unwrap().unwrap();
        assert!(matches_selector(&arena, p, "p.intro").unwrap());
        assert!(!matches_selector(&arena, p, "div").unwrap());
    }

    #[test]
    fn matches_parsed_selector_list_single_element() {
        let (arena, doc) = build_simple_tree();
        let p = query_selector(&arena, doc, "p.intro").unwrap().unwrap();
        let selectors = crate::selector_impl::parse_selector("p.intro").unwrap();
        assert!(matches_selector_list(&arena, p, &selectors));
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

    #[test]
    fn split_attribute_name_handles_prefix() {
        assert_eq!(split_attr_name("href"), (None, "href"));
        assert_eq!(split_attr_name("xlink:href"), (Some("xlink"), "href"));
    }

    #[test]
    fn attribute_namespace_mapping_known_prefixes() {
        assert_eq!(
            attribute_namespace_uri(Some("xlink")),
            "http://www.w3.org/1999/xlink"
        );
        assert_eq!(attribute_namespace_uri(None), "");
    }

    // ── Pseudo-class matching tests ─────────────────────────────────

    #[test]
    fn hover_matches_when_state_set() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let div = make_element(&mut arena, "div");
        tree::append_child(&mut arena, doc, div);

        // Set hover state
        if let NodeData::Element(ref mut el) = arena.get_mut(div).data {
            el.state.insert(ElementState::HOVER);
        }

        let result = query_selector_all(&arena, doc, "div:hover").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn hover_does_not_match_without_state() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let div = make_element(&mut arena, "div");
        tree::append_child(&mut arena, doc, div);

        let result = query_selector_all(&arena, doc, "div:hover").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn focus_matches_when_state_set() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let input = make_element(&mut arena, "input");
        tree::append_child(&mut arena, doc, input);

        if let NodeData::Element(ref mut el) = arena.get_mut(input).data {
            el.state.insert(ElementState::FOCUS);
        }

        let result = query_selector(&arena, doc, "input:focus").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn checked_matches_when_state_set() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let input = make_element(&mut arena, "input");
        tree::append_child(&mut arena, doc, input);
        attributes::set_attribute(&mut arena, input, "type", "checkbox");

        if let NodeData::Element(ref mut el) = arena.get_mut(input).data {
            el.state.insert(ElementState::CHECKED);
        }

        let result = query_selector(&arena, doc, "input:checked").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn disabled_matches_when_state_set() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let input = make_element(&mut arena, "input");
        tree::append_child(&mut arena, doc, input);

        if let NodeData::Element(ref mut el) = arena.get_mut(input).data {
            el.state.insert(ElementState::DISABLED);
        }

        let result = query_selector(&arena, doc, "input:disabled").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn link_matches_anchor_with_href() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let a = make_element(&mut arena, "a");
        tree::append_child(&mut arena, doc, a);
        attributes::set_attribute(&mut arena, a, "href", "https://example.com");

        let result = query_selector(&arena, doc, "a:link").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn link_does_not_match_anchor_without_href() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let a = make_element(&mut arena, "a");
        tree::append_child(&mut arena, doc, a);

        let result = query_selector(&arena, doc, "a:link").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn any_link_matches_anchor_with_href() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let a = make_element(&mut arena, "a");
        tree::append_child(&mut arena, doc, a);
        attributes::set_attribute(&mut arena, a, "href", "#foo");

        let result = query_selector(&arena, doc, "a:any-link").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn visited_never_matches() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let a = make_element(&mut arena, "a");
        tree::append_child(&mut arena, doc, a);
        attributes::set_attribute(&mut arena, a, "href", "https://example.com");

        let result = query_selector(&arena, doc, "a:visited").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_only_matches_when_not_read_write() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let input = make_element(&mut arena, "input");
        tree::append_child(&mut arena, doc, input);
        // No READ_WRITE state → :read-only matches.

        let result = query_selector(&arena, doc, "input:read-only").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn read_only_does_not_match_when_read_write() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let input = make_element(&mut arena, "input");
        tree::append_child(&mut arena, doc, input);

        if let NodeData::Element(ref mut el) = arena.get_mut(input).data {
            el.state.insert(ElementState::READ_WRITE);
        }

        let result = query_selector(&arena, doc, "input:read-only").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn lang_matches_element_with_lang_attr() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let html = make_element(&mut arena, "html");
        let p = make_element(&mut arena, "p");
        tree::append_child(&mut arena, doc, html);
        tree::append_child(&mut arena, html, p);
        attributes::set_attribute(&mut arena, html, "lang", "en-US");

        let result = query_selector(&arena, doc, "p:lang(en)").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn lang_matches_exact() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let p = make_element(&mut arena, "p");
        tree::append_child(&mut arena, doc, p);
        attributes::set_attribute(&mut arena, p, "lang", "fr");

        let result = query_selector(&arena, doc, "p:lang(fr)").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn lang_does_not_match_different_lang() {
        let mut arena = NodeArena::new();
        let doc = arena.alloc(NodeData::Document);
        let p = make_element(&mut arena, "p");
        tree::append_child(&mut arena, doc, p);
        attributes::set_attribute(&mut arena, p, "lang", "fr");

        let result = query_selector(&arena, doc, "p:lang(de)").unwrap();
        assert!(result.is_none());
    }

    // ── ElementState unit tests ─────────────────────────────────────

    #[test]
    fn element_state_insert_contains() {
        let mut s = ElementState::default();
        assert!(!s.contains(ElementState::HOVER));
        s.insert(ElementState::HOVER);
        assert!(s.contains(ElementState::HOVER));
    }

    #[test]
    fn element_state_remove() {
        let mut s = ElementState::default();
        s.insert(ElementState::FOCUS);
        s.remove(ElementState::FOCUS);
        assert!(!s.contains(ElementState::FOCUS));
    }

    #[test]
    fn element_state_set() {
        let mut s = ElementState::default();
        s.set(ElementState::ACTIVE, true);
        assert!(s.contains(ElementState::ACTIVE));
        s.set(ElementState::ACTIVE, false);
        assert!(!s.contains(ElementState::ACTIVE));
    }

    #[test]
    fn element_state_multiple_flags() {
        let mut s = ElementState::default();
        s.insert(ElementState::HOVER);
        s.insert(ElementState::FOCUS);
        assert!(s.contains(ElementState::HOVER));
        assert!(s.contains(ElementState::FOCUS));
        assert!(!s.contains(ElementState::ACTIVE));
    }
}
