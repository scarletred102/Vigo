// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS specificity calculation.
//!
//! Specificity is a triple (a, b, c) where:
//! - a = number of ID selectors
//! - b = number of class/attribute/pseudo-class selectors
//! - c = number of type/pseudo-element selectors

/// Specificity as a comparable triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

const SPECIFICITY_COMPONENT_MASK: u32 = (1 << 10) - 1;

impl Specificity {
    /// Inline styles have the highest specificity.
    pub const INLINE: Self = Self(u32::MAX, 0, 0);

    /// Compute a numeric value for comparison (a * 1000000 + b * 1000 + c).
    pub fn numeric(&self) -> u64 {
        (self.0 as u64) * 1_000_000 + (self.1 as u64) * 1_000 + (self.2 as u64)
    }
}

/// Extract specificity from a selector string using simple heuristic counting.
/// This is a quick estimation — the `selectors` crate provides the canonical value.
pub fn estimate_specificity(selector: &str) -> Specificity {
    let mut ids = 0u32;
    let mut classes = 0u32;
    let mut types = 0u32;

    // Simple tokenization for specificity counting
    let s = selector.trim();
    let mut chars = s.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '#' => {
                chars.next();
                ids += 1;
                // Consume identifier
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                {
                    chars.next();
                }
            }
            '.' => {
                chars.next();
                classes += 1;
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                {
                    chars.next();
                }
            }
            '[' => {
                classes += 1;
                while chars.peek().is_some_and(|c| *c != ']') {
                    chars.next();
                }
                chars.next(); // consume ']'
            }
            ':' => {
                chars.next();
                if chars.peek() == Some(&':') {
                    // Pseudo-element
                    chars.next();
                    types += 1;
                } else {
                    // Pseudo-class
                    classes += 1;
                }
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '-')
                {
                    chars.next();
                }
            }
            ' ' | '>' | '+' | '~' | ',' => {
                chars.next();
            }
            _ if ch.is_alphabetic() => {
                types += 1;
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                {
                    chars.next();
                }
            }
            '*' => {
                chars.next();
                // Universal selector has no specificity
            }
            _ => {
                chars.next();
            }
        }
    }

    Specificity(ids, classes, types)
}

/// Decode selectors-crate packed specificity (`a<<20 | b<<10 | c`) into
/// our tuple representation.
pub fn decode_packed_specificity(packed: u32) -> Specificity {
    let a = (packed >> 20) & SPECIFICITY_COMPONENT_MASK;
    let b = (packed >> 10) & SPECIFICITY_COMPONENT_MASK;
    let c = packed & SPECIFICITY_COMPONENT_MASK;
    Specificity(a, b, c)
}

/// Canonical specificity from a parsed selector list.
pub fn specificity_from_selector_list(
    selectors: &selectors::parser::SelectorList<vex_dom::selector_impl::VexSelectorImpl>,
) -> Specificity {
    selectors
        .slice()
        .iter()
        .map(|s| decode_packed_specificity(s.specificity()))
        .max()
        .unwrap_or_default()
}

/// Canonical specificity for a selector string if it can be parsed by our
/// selector engine.
pub fn canonical_specificity(selector: &str) -> Option<Specificity> {
    let parsed = vex_dom::selector_impl::parse_selector(selector).ok()?;
    Some(specificity_from_selector_list(&parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_selector() {
        assert_eq!(estimate_specificity("#id"), Specificity(1, 0, 0));
    }

    #[test]
    fn class_selector() {
        assert_eq!(estimate_specificity(".class"), Specificity(0, 1, 0));
    }

    #[test]
    fn type_selector() {
        assert_eq!(estimate_specificity("div"), Specificity(0, 0, 1));
    }

    #[test]
    fn combined_selector() {
        assert_eq!(estimate_specificity("#id .class div"), Specificity(1, 1, 1));
    }

    #[test]
    fn complex_selector() {
        assert_eq!(
            estimate_specificity("div.foo > p.bar"),
            Specificity(0, 2, 2)
        );
    }

    #[test]
    fn specificity_ordering() {
        let a = Specificity(1, 0, 0);
        let b = Specificity(0, 1, 0);
        let c = Specificity(0, 0, 1);
        assert!(a > b);
        assert!(b > c);
    }

    #[test]
    fn decode_packed_specificity_roundtrip() {
        let packed = (2u32 << 20) | (5u32 << 10) | 7u32;
        assert_eq!(decode_packed_specificity(packed), Specificity(2, 5, 7));
    }

    #[test]
    fn canonical_specificity_respects_where_zeroing() {
        // :where(...) contributes zero specificity.
        assert_eq!(
            canonical_specificity(":where(#id, .cls, div)"),
            Some(Specificity(0, 0, 0))
        );
    }

    #[test]
    fn canonical_specificity_for_is_takes_most_specific_argument() {
        // :is() takes max argument specificity.
        assert_eq!(
            canonical_specificity(":is(.a, #id, div)"),
            Some(Specificity(1, 0, 0))
        );
    }
}
