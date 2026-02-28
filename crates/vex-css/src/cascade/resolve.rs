// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Cascade resolution: sort declarations by origin, specificity, and source order.

use std::collections::HashMap;

use crate::cascade::matching::MatchedDeclaration;
use crate::properties::Property;

/// Resolve the cascade: given all matched declarations, determine the winning
/// value for each property.
pub fn resolve_cascade(declarations: &[MatchedDeclaration]) -> Vec<Property> {
    // Group by property name, keeping all declarations for each property
    let mut by_property: HashMap<&str, Vec<&MatchedDeclaration>> = HashMap::new();

    for decl in declarations {
        by_property
            .entry(decl.property.name())
            .or_default()
            .push(decl);
    }

    let mut result = Vec::new();

    for (_name, mut decls) in by_property {
        // Sort by (origin, specificity, source_order) — last one wins
        decls.sort_by(|a, b| {
            a.origin
                .cmp(&b.origin)
                .then_with(|| a.specificity.cmp(&b.specificity))
                .then_with(|| a.source_order.cmp(&b.source_order))
        });

        // The last declaration (highest priority) wins
        if let Some(winner) = decls.last() {
            result.push(winner.property.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cascade::origin::Origin;
    use crate::cascade::specificity::Specificity;
    use crate::values::color::ColorValue;

    fn make_decl(prop: Property, origin: Origin, spec: Specificity, order: usize) -> MatchedDeclaration {
        MatchedDeclaration {
            property: prop,
            specificity: spec,
            origin,
            source_order: order,
        }
    }

    #[test]
    fn higher_specificity_wins() {
        let decls = vec![
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)), Origin::Author, Specificity(0, 0, 1), 0),
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)), Origin::Author, Specificity(0, 1, 0), 1),
        ];
        let result = resolve_cascade(&decls);
        assert_eq!(result.len(), 1);
        // The class selector (0,1,0) should win over type (0,0,1)
        assert_eq!(result[0], Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)));
    }

    #[test]
    fn important_overrides() {
        let decls = vec![
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)), Origin::AuthorImportant, Specificity(0, 0, 1), 0),
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)), Origin::Author, Specificity(1, 0, 0), 1),
        ];
        let result = resolve_cascade(&decls);
        assert_eq!(result[0], Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)));
    }

    #[test]
    fn source_order_tiebreak() {
        let decls = vec![
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)), Origin::Author, Specificity(0, 1, 0), 0),
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)), Origin::Author, Specificity(0, 1, 0), 1),
        ];
        let result = resolve_cascade(&decls);
        // Later source order wins
        assert_eq!(result[0], Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)));
    }

    #[test]
    fn inline_beats_author() {
        let decls = vec![
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)), Origin::Author, Specificity(1, 1, 1), 0),
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)), Origin::Inline, Specificity::INLINE, 1),
        ];
        let result = resolve_cascade(&decls);
        assert_eq!(result[0], Property::Color(ColorValue::Rgba(vex_core::Color::WHITE)));
    }

    #[test]
    fn multiple_properties_resolved() {
        let decls = vec![
            make_decl(Property::Color(ColorValue::Rgba(vex_core::Color::BLACK)), Origin::Author, Specificity(0, 0, 1), 0),
            make_decl(Property::Opacity(0.5), Origin::Author, Specificity(0, 0, 1), 1),
        ];
        let result = resolve_cascade(&decls);
        assert_eq!(result.len(), 2);
    }
}
