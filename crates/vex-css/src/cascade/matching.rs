// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Declaration matching: collect all CSS declarations that apply to an element.

use vex_core::VexId;
use vex_core::Size;
use vex_dom::{matches_selector_list, NodeArena};

use crate::cascade::origin::Origin;
use crate::cascade::specificity::{estimate_specificity, specificity_from_selector_list, Specificity};
use crate::media::{evaluate_media, parse_media_condition};
use crate::parser::Stylesheet;
use crate::properties::{Declaration, Property};

/// A matched declaration with its priority metadata.
#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    pub property: Property,
    pub specificity: Specificity,
    pub origin: Origin,
    pub source_order: usize,
}

/// Collect all declarations from stylesheets that match a given element.
pub fn collect_matching_declarations(
    element_id: VexId,
    arena: &NodeArena,
    stylesheets: &[Stylesheet],
    viewport: Size,
) -> Vec<MatchedDeclaration> {
    let mut result = Vec::new();
    let mut source_order = 0;

    for stylesheet in stylesheets {
        for rule in &stylesheet.rules {
            if let Some(condition) = &rule.media_condition {
                let media_matches = parse_media_condition(condition)
                    .map(|cond| evaluate_media(&cond, viewport))
                    .unwrap_or(false);
                if !media_matches {
                    continue;
                }
            }

            for selector_str in &rule.selectors {
                let parsed = vex_dom::selector_impl::parse_selector(selector_str);

                // Match directly against element to avoid full-tree scans.
                let is_match = parsed
                    .as_ref()
                    .map(|selectors| matches_selector_list(arena, element_id, selectors))
                    .unwrap_or(false);

                if is_match {
                    let specificity = parsed
                        .as_ref()
                        .map(specificity_from_selector_list)
                        .unwrap_or_else(|_| estimate_specificity(selector_str));

                    for decl in &rule.declarations {
                        let origin = if decl.important {
                            Origin::AuthorImportant
                        } else {
                            Origin::Author
                        };

                        result.push(MatchedDeclaration {
                            property: decl.property.clone(),
                            specificity,
                            origin,
                            source_order,
                        });
                        source_order += 1;
                    }
                }
            }
        }
    }

    result
}

/// Collect inline style declarations for an element.
pub fn collect_inline_declarations(
    element_id: VexId,
    arena: &NodeArena,
) -> Vec<MatchedDeclaration> {
    use vex_dom::NodeData;

    let node = arena.get(element_id);
    if let NodeData::Element(ref el) = node.data {
        if let Some(style_attr) = el.attributes.iter().find(|a| a.name == "style") {
            let decls = crate::parser::parse_inline_style(&style_attr.value);
            return decls
                .into_iter()
                .enumerate()
                .map(
                    |(
                        i,
                        Declaration {
                            property,
                            important,
                        },
                    )| {
                        MatchedDeclaration {
                            property,
                            specificity: Specificity::INLINE,
                            origin: if important {
                                Origin::InlineImportant
                            } else {
                                Origin::Inline
                            },
                            source_order: i,
                        }
                    },
                )
                .collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_stylesheet;
    use vex_html::parse_html;

    #[test]
    fn media_filtered_rules_only_apply_when_matching_viewport() {
        let doc = parse_html("<div class='mobile'></div>");
        let arena = doc.arena();
        let div = doc.get_elements_by_class_name("mobile")[0];

        let css = parse_stylesheet(
            "@media (max-width: 600px) { .mobile { color: red; } } .mobile { color: blue; }",
        );

        let wide = collect_matching_declarations(div, arena, std::slice::from_ref(&css), Size::new(1200.0, 800.0));
        let narrow = collect_matching_declarations(div, arena, &[css], Size::new(500.0, 800.0));

        // Narrow should include extra declaration from media rule.
        assert!(narrow.len() > wide.len());
    }
}
