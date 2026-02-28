// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Declaration matching: collect all CSS declarations that apply to an element.

use vex_core::VexId;
use vex_dom::{query_selector_all, NodeArena};

use crate::cascade::origin::Origin;
use crate::cascade::specificity::{estimate_specificity, Specificity};
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
) -> Vec<MatchedDeclaration> {
    let mut result = Vec::new();
    let mut source_order = 0;

    for stylesheet in stylesheets {
        for rule in &stylesheet.rules {
            for selector_str in &rule.selectors {
                // Check if this selector matches our element
                let root_id = VexId::new(0); // document root
                let matches = query_selector_all(arena, root_id, selector_str);

                if let Ok(matched_ids) = matches {
                    if matched_ids.contains(&element_id) {
                        let specificity = estimate_specificity(selector_str);

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
                .map(|(i, Declaration { property, important })| {
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
                })
                .collect();
        }
    }
    Vec::new()
}
