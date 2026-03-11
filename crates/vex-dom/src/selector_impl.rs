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

use crate::node::ElementState;

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

// ── Pseudo-class / pseudo-element ───────────────────────────────────

/// CSS pseudo-classes supported by the Vex engine.
///
/// Tree-structural pseudo-classes (`:first-child`, `:nth-child()`, etc.) are
/// handled directly by the `selectors` crate. This enum covers the
/// *non-tree-structural* pseudo-classes that require element state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VexPseudoClass {
    // User-action pseudo-classes
    Hover,
    Active,
    Focus,
    FocusVisible,
    FocusWithin,

    // Link pseudo-classes
    AnyLink,
    Link,
    Visited,

    // Form state pseudo-classes
    Enabled,
    Disabled,
    Checked,
    Indeterminate,
    Default,
    Required,
    Optional,
    ReadOnly,
    ReadWrite,
    PlaceholderShown,
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    Autofill,

    // Target pseudo-class
    Target,

    // Defined pseudo-class
    Defined,

    // Fullscreen pseudo-class
    Fullscreen,

    // Open pseudo-class (for <details>/<dialog>)
    Open,

    // :lang() functional pseudo-class
    Lang(String),
}

impl VexPseudoClass {
    /// Return the `ElementState` flag corresponding to this pseudo-class,
    /// or `0` if the pseudo-class doesn't map to a single flag.
    pub fn state_flag(&self) -> u32 {
        match self {
            Self::Hover => ElementState::HOVER,
            Self::Active => ElementState::ACTIVE,
            Self::Focus => ElementState::FOCUS,
            Self::FocusVisible => ElementState::FOCUS_VISIBLE,
            Self::FocusWithin => ElementState::FOCUS_WITHIN,
            Self::Enabled => ElementState::ENABLED,
            Self::Disabled => ElementState::DISABLED,
            Self::Checked => ElementState::CHECKED,
            Self::Indeterminate => ElementState::INDETERMINATE,
            Self::Default => ElementState::DEFAULT,
            Self::Required => ElementState::REQUIRED,
            Self::Optional => ElementState::OPTIONAL,
            Self::ReadOnly => ElementState::READ_ONLY,
            Self::ReadWrite => ElementState::READ_WRITE,
            Self::PlaceholderShown => ElementState::PLACEHOLDER_SHOWN,
            Self::Valid => ElementState::VALID,
            Self::Invalid => ElementState::INVALID,
            Self::InRange => ElementState::IN_RANGE,
            Self::OutOfRange => ElementState::OUT_OF_RANGE,
            Self::Visited => ElementState::VISITED,
            Self::Target => ElementState::TARGET,
            Self::Open => ElementState::OPEN,
            Self::Defined => ElementState::DEFINED,
            Self::Fullscreen => ElementState::FULLSCREEN,
            Self::Autofill => ElementState::AUTOFILL,
            // These don't map to a single state flag.
            Self::AnyLink | Self::Link | Self::Lang(_) => 0,
        }
    }
}

impl ToCss for VexPseudoClass {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        match self {
            Self::Hover => dest.write_str(":hover"),
            Self::Active => dest.write_str(":active"),
            Self::Focus => dest.write_str(":focus"),
            Self::FocusVisible => dest.write_str(":focus-visible"),
            Self::FocusWithin => dest.write_str(":focus-within"),
            Self::AnyLink => dest.write_str(":any-link"),
            Self::Link => dest.write_str(":link"),
            Self::Visited => dest.write_str(":visited"),
            Self::Enabled => dest.write_str(":enabled"),
            Self::Disabled => dest.write_str(":disabled"),
            Self::Checked => dest.write_str(":checked"),
            Self::Indeterminate => dest.write_str(":indeterminate"),
            Self::Default => dest.write_str(":default"),
            Self::Required => dest.write_str(":required"),
            Self::Optional => dest.write_str(":optional"),
            Self::ReadOnly => dest.write_str(":read-only"),
            Self::ReadWrite => dest.write_str(":read-write"),
            Self::PlaceholderShown => dest.write_str(":placeholder-shown"),
            Self::Valid => dest.write_str(":valid"),
            Self::Invalid => dest.write_str(":invalid"),
            Self::InRange => dest.write_str(":in-range"),
            Self::OutOfRange => dest.write_str(":out-of-range"),
            Self::Target => dest.write_str(":target"),
            Self::Defined => dest.write_str(":defined"),
            Self::Fullscreen => dest.write_str(":fullscreen"),
            Self::Open => dest.write_str(":open"),
            Self::Autofill => dest.write_str(":autofill"),
            Self::Lang(ref lang) => {
                dest.write_str(":lang(")?;
                dest.write_str(lang)?;
                dest.write_str(")")
            }
        }
    }
}

impl NonTSPseudoClass for VexPseudoClass {
    type Impl = VexSelectorImpl;

    fn is_active_or_hover(&self) -> bool {
        matches!(self, Self::Active | Self::Hover)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(
            self,
            Self::Active | Self::Hover | Self::Focus | Self::FocusVisible | Self::FocusWithin
        )
    }
}

/// CSS pseudo-elements supported by the Vex engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VexPseudoElement {
    Before,
    After,
    FirstLine,
    FirstLetter,
    Selection,
    Placeholder,
    Marker,
}

impl ToCss for VexPseudoElement {
    fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
        match self {
            Self::Before => dest.write_str("::before"),
            Self::After => dest.write_str("::after"),
            Self::FirstLine => dest.write_str("::first-line"),
            Self::FirstLetter => dest.write_str("::first-letter"),
            Self::Selection => dest.write_str("::selection"),
            Self::Placeholder => dest.write_str("::placeholder"),
            Self::Marker => dest.write_str("::marker"),
        }
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

/// A CSS selector parser for Vex with pseudo-class/element support.
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

    fn parse_non_ts_pseudo_class(
        &self,
        _location: cssparser::SourceLocation,
        name: cssparser::CowRcStr<'i>,
    ) -> Result<VexPseudoClass, cssparser::ParseError<'i, Self::Error>> {
        let pc = match &*name.to_ascii_lowercase() {
            "hover" => VexPseudoClass::Hover,
            "active" => VexPseudoClass::Active,
            "focus" => VexPseudoClass::Focus,
            "focus-visible" => VexPseudoClass::FocusVisible,
            "focus-within" => VexPseudoClass::FocusWithin,
            "any-link" => VexPseudoClass::AnyLink,
            "link" => VexPseudoClass::Link,
            "visited" => VexPseudoClass::Visited,
            "enabled" => VexPseudoClass::Enabled,
            "disabled" => VexPseudoClass::Disabled,
            "checked" => VexPseudoClass::Checked,
            "indeterminate" => VexPseudoClass::Indeterminate,
            "default" => VexPseudoClass::Default,
            "required" => VexPseudoClass::Required,
            "optional" => VexPseudoClass::Optional,
            "read-only" => VexPseudoClass::ReadOnly,
            "read-write" => VexPseudoClass::ReadWrite,
            "placeholder-shown" => VexPseudoClass::PlaceholderShown,
            "valid" => VexPseudoClass::Valid,
            "invalid" => VexPseudoClass::Invalid,
            "in-range" => VexPseudoClass::InRange,
            "out-of-range" => VexPseudoClass::OutOfRange,
            "target" => VexPseudoClass::Target,
            "defined" => VexPseudoClass::Defined,
            "fullscreen" => VexPseudoClass::Fullscreen,
            "open" => VexPseudoClass::Open,
            "autofill" => VexPseudoClass::Autofill,
            _ => {
                return Err(_location.new_custom_error(
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                        name,
                    ),
                ));
            }
        };
        Ok(pc)
    }

    fn parse_non_ts_functional_pseudo_class<'t>(
        &self,
        name: cssparser::CowRcStr<'i>,
        parser: &mut cssparser::Parser<'i, 't>,
        _after_part: bool,
    ) -> Result<VexPseudoClass, cssparser::ParseError<'i, Self::Error>> {
        match &*name.to_ascii_lowercase() {
            "lang" => {
                let lang = parser.expect_ident_or_string()?.as_ref().to_owned();
                Ok(VexPseudoClass::Lang(lang))
            }
            _ => Err(parser.new_custom_error(
                selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name),
            )),
        }
    }

    fn parse_pseudo_element(
        &self,
        _location: cssparser::SourceLocation,
        name: cssparser::CowRcStr<'i>,
    ) -> Result<VexPseudoElement, cssparser::ParseError<'i, Self::Error>> {
        let pe = match &*name.to_ascii_lowercase() {
            "before" => VexPseudoElement::Before,
            "after" => VexPseudoElement::After,
            "first-line" => VexPseudoElement::FirstLine,
            "first-letter" => VexPseudoElement::FirstLetter,
            "selection" => VexPseudoElement::Selection,
            "placeholder" => VexPseudoElement::Placeholder,
            "marker" => VexPseudoElement::Marker,
            _ => {
                return Err(_location.new_custom_error(
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                        name,
                    ),
                ));
            }
        };
        Ok(pe)
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

    // ── Pseudo-class parsing tests ──────────────────────────────────

    #[test]
    fn parse_hover_pseudo_class() {
        let sel = parse_selector("a:hover").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_focus_pseudo_class() {
        let sel = parse_selector("input:focus").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_active_pseudo_class() {
        let sel = parse_selector("button:active").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_visited_pseudo_class() {
        let sel = parse_selector("a:visited").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_checked_pseudo_class() {
        let sel = parse_selector("input:checked").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_disabled_pseudo_class() {
        let sel = parse_selector("input:disabled").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_enabled_pseudo_class() {
        let sel = parse_selector("input:enabled").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_required_pseudo_class() {
        let sel = parse_selector("input:required").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_placeholder_shown_pseudo_class() {
        let sel = parse_selector("input:placeholder-shown").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_read_only_pseudo_class() {
        let sel = parse_selector("input:read-only").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_valid_invalid_pseudo_class() {
        assert!(parse_selector("input:valid").is_ok());
        assert!(parse_selector("input:invalid").is_ok());
    }

    #[test]
    fn parse_any_link_pseudo_class() {
        let sel = parse_selector("a:any-link").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_target_pseudo_class() {
        let sel = parse_selector("#section:target").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_focus_within_pseudo_class() {
        let sel = parse_selector("form:focus-within").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_focus_visible_pseudo_class() {
        let sel = parse_selector("input:focus-visible").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_lang_functional_pseudo_class() {
        let sel = parse_selector("p:lang(en)").unwrap();
        assert_eq!(sel.len(), 1);
    }

    // ── Pseudo-element parsing tests ────────────────────────────────

    #[test]
    fn parse_before_pseudo_element() {
        let sel = parse_selector("p::before").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_after_pseudo_element() {
        let sel = parse_selector("p::after").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_first_line_pseudo_element() {
        let sel = parse_selector("p::first-line").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_selection_pseudo_element() {
        let sel = parse_selector("::selection").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_placeholder_pseudo_element() {
        let sel = parse_selector("input::placeholder").unwrap();
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn parse_marker_pseudo_element() {
        let sel = parse_selector("li::marker").unwrap();
        assert_eq!(sel.len(), 1);
    }

    // ── Pseudo-class property tests ─────────────────────────────────

    #[test]
    fn hover_is_active_or_hover() {
        assert!(VexPseudoClass::Hover.is_active_or_hover());
        assert!(VexPseudoClass::Active.is_active_or_hover());
        assert!(!VexPseudoClass::Focus.is_active_or_hover());
    }

    #[test]
    fn user_action_states() {
        assert!(VexPseudoClass::Active.is_user_action_state());
        assert!(VexPseudoClass::Hover.is_user_action_state());
        assert!(VexPseudoClass::Focus.is_user_action_state());
        assert!(VexPseudoClass::FocusVisible.is_user_action_state());
        assert!(VexPseudoClass::FocusWithin.is_user_action_state());
        assert!(!VexPseudoClass::Checked.is_user_action_state());
    }

    #[test]
    fn state_flag_mapping() {
        assert_eq!(VexPseudoClass::Hover.state_flag(), ElementState::HOVER);
        assert_eq!(VexPseudoClass::Focus.state_flag(), ElementState::FOCUS);
        assert_eq!(VexPseudoClass::Checked.state_flag(), ElementState::CHECKED);
        assert_eq!(VexPseudoClass::Link.state_flag(), 0);
    }

    #[test]
    fn pseudo_class_to_css() {
        let mut s = String::new();
        VexPseudoClass::Hover.to_css(&mut s).unwrap();
        assert_eq!(s, ":hover");

        s.clear();
        VexPseudoClass::Lang("en".into()).to_css(&mut s).unwrap();
        assert_eq!(s, ":lang(en)");
    }

    #[test]
    fn pseudo_element_to_css() {
        let mut s = String::new();
        VexPseudoElement::Before.to_css(&mut s).unwrap();
        assert_eq!(s, "::before");

        s.clear();
        VexPseudoElement::Selection.to_css(&mut s).unwrap();
        assert_eq!(s, "::selection");
    }

    #[test]
    fn parse_unsupported_pseudo_class_fails() {
        assert!(parse_selector("div:magic").is_err());
    }

    #[test]
    fn parse_unsupported_pseudo_element_fails() {
        assert!(parse_selector("div::rainbow").is_err());
    }
}
