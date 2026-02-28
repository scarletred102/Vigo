// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Selector engine types implementing `selectors::SelectorImpl`.
//!
//! These types plug our DOM into the Mozilla `selectors` crate so that
//! `querySelector` / `querySelectorAll` work with standard CSS selectors.

use std::borrow::Borrow;
use std::fmt;

use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;
use selectors::parser::{NonTSPseudoClass, PseudoElement, SelectorImpl};
use selectors::SelectorList;

// ── Thin string wrappers ────────────────────────────────────────────

/// A CSS identifier value (id, class name, etc.).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VexIdentifier(pub String);

/// A CSS local name (tag name).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VexLocalName(pub String);

/// A CSS namespace URL.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VexNamespaceUrl(pub String);

/// A CSS namespace prefix.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VexNamespacePrefix(pub String);

/// A CSS attribute value.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct VexAttrValue(pub String);

// ── From<&str> ──────────────────────────────────────────────────────

impl<'a> From<&'a str> for VexIdentifier {
    fn from(s: &'a str) -> Self {
        Self(s.to_owned())
    }
}

impl<'a> From<&'a str> for VexLocalName {
    fn from(s: &'a str) -> Self {
        Self(s.to_owned())
    }
}

impl<'a> From<&'a str> for VexNamespaceUrl {
    fn from(s: &'a str) -> Self {
        Self(s.to_owned())
    }
}

impl<'a> From<&'a str> for VexNamespacePrefix {
    fn from(s: &'a str) -> Self {
        Self(s.to_owned())
    }
}

impl<'a> From<&'a str> for VexAttrValue {
    fn from(s: &'a str) -> Self {
        Self(s.to_owned())
    }
}

// ── AsRef<str> (needed for AttrSelectorOperation::eval_str) ─────────

impl AsRef<str> for VexAttrValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ── ToCss ───────────────────────────────────────────────────────────

impl ToCss for VexIdentifier {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        cssparser::serialize_identifier(&self.0, dest)
    }
}

impl ToCss for VexLocalName {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(&self.0)
    }
}

impl ToCss for VexNamespaceUrl {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(&self.0)
    }
}

impl ToCss for VexNamespacePrefix {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        dest.write_str(&self.0)
    }
}

impl ToCss for VexAttrValue {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        cssparser::serialize_string(&self.0, dest)
    }
}

// ── PrecomputedHash ─────────────────────────────────────────────────

fn simple_hash(s: &str) -> u32 {
    // FNV-1a for a fast, stable hash.
    let mut h: u32 = 2_166_136_261;
    for b in s.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(16_777_619);
    }
    h
}

impl PrecomputedHash for VexIdentifier {
    fn precomputed_hash(&self) -> u32 {
        simple_hash(&self.0)
    }
}

impl PrecomputedHash for VexLocalName {
    fn precomputed_hash(&self) -> u32 {
        simple_hash(&self.0)
    }
}

impl PrecomputedHash for VexNamespaceUrl {
    fn precomputed_hash(&self) -> u32 {
        simple_hash(&self.0)
    }
}

// ── Borrow impls (BorrowedLocalName = str, BorrowedNamespaceUrl = str) ──

impl Borrow<str> for VexLocalName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for VexNamespaceUrl {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// ── Pseudo-class / pseudo-element (minimal, empty) ─────────────────

/// We don't support non-tree-structural pseudo-classes yet, but the
/// trait requires a type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VexPseudoClass {}

impl ToCss for VexPseudoClass {
    fn to_css<W: fmt::Write>(&self, _dest: &mut W) -> fmt::Result {
        match *self {} // Uninhabited
    }
}

impl NonTSPseudoClass for VexPseudoClass {
    type Impl = VexSelectorImpl;

    fn is_active_or_hover(&self) -> bool {
        match *self {}
    }

    fn is_user_action_state(&self) -> bool {
        match *self {}
    }
}

/// Same for pseudo-elements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VexPseudoElement {}

impl ToCss for VexPseudoElement {
    fn to_css<W: fmt::Write>(&self, _dest: &mut W) -> fmt::Result {
        match *self {}
    }
}

impl PseudoElement for VexPseudoElement {
    type Impl = VexSelectorImpl;
}

// ── VexSelectorImpl ─────────────────────────────────────────────────

/// The concrete `SelectorImpl` for Vex's DOM.
#[derive(Clone, Debug)]
pub struct VexSelectorImpl;

impl SelectorImpl for VexSelectorImpl {
    type ExtraMatchingData<'a> = ();

    type AttrValue = VexAttrValue;
    type Identifier = VexIdentifier;
    type LocalName = VexLocalName;
    type NamespaceUrl = VexNamespaceUrl;
    type NamespacePrefix = VexNamespacePrefix;

    type BorrowedNamespaceUrl = str;
    type BorrowedLocalName = str;

    type NonTSPseudoClass = VexPseudoClass;
    type PseudoElement = VexPseudoElement;
}

// ── Selector parser ─────────────────────────────────────────────────

/// A minimal selector parser for Vex.
pub struct VexParser;

impl<'i> selectors::Parser<'i> for VexParser {
    type Impl = VexSelectorImpl;
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }
}

/// Parse a CSS selector string into a `SelectorList`.
#[allow(clippy::result_unit_err)]
pub fn parse_selector(input: &str) -> Result<SelectorList<VexSelectorImpl>, ()> {
    let mut css_input = cssparser::ParserInput::new(input);
    let mut parser = cssparser::Parser::new(&mut css_input);
    SelectorList::parse(
        &VexParser,
        &mut parser,
        selectors::parser::ParseRelative::No,
    )
    .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_selector() {
        let sel = parse_selector("div").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_class_selector() {
        let sel = parse_selector(".foo").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_id_selector() {
        let sel = parse_selector("#main").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_compound_selector() {
        let sel = parse_selector("div.foo#bar").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_descendant_selector() {
        let sel = parse_selector("div p").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_child_combinator() {
        let sel = parse_selector("div > p").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_comma_list() {
        let sel = parse_selector("div, p, span").unwrap();
        assert_eq!(sel.len(), 3);
    }

    #[test]
    fn parse_attribute_selector() {
        let sel = parse_selector("input[type=\"text\"]").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_invalid_selector_fails() {
        assert!(parse_selector("!!!").is_err());
    }
}
