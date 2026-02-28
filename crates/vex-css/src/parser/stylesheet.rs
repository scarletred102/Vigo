// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Hand-rolled CSS stylesheet parser.
//!
//! Parses stylesheets into a list of rules, each with selectors + declarations.
//! Handles `@media` and `@import` at-rules. Also parses inline `style=""` attributes.

use crate::properties::{parse_declaration, Declaration};

/// A parsed CSS stylesheet.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<CssRule>,
}

/// A CSS rule: one or more selectors + a list of declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selectors: Vec<String>,
    pub declarations: Vec<Declaration>,
    /// If inside a `@media` block, the condition string.
    pub media_condition: Option<String>,
}

/// Parse a full CSS stylesheet string.
pub fn parse_stylesheet(css: &str) -> Stylesheet {
    let mut rules = Vec::new();
    let mut pos = 0;
    let bytes = css.as_bytes();

    while pos < bytes.len() {
        skip_whitespace_and_comments(css, &mut pos);
        if pos >= bytes.len() {
            break;
        }

        if css[pos..].starts_with("@media") {
            pos += 6;
            // Read media condition until '{'
            let cond_start = pos;
            while pos < bytes.len() && bytes[pos] != b'{' {
                pos += 1;
            }
            let condition = css[cond_start..pos].trim().to_string();
            if pos < bytes.len() {
                pos += 1; // skip '{'
            }

            // Parse rules inside the media block
            let mut depth = 1;
            let block_start = pos;
            while pos < bytes.len() && depth > 0 {
                if bytes[pos] == b'{' {
                    depth += 1;
                } else if bytes[pos] == b'}' {
                    depth -= 1;
                }
                if depth > 0 {
                    pos += 1;
                }
            }
            let block_css = &css[block_start..pos];
            if pos < bytes.len() {
                pos += 1; // skip closing '}'
            }

            // Recursively parse the inner block
            let inner = parse_stylesheet(block_css);
            for mut rule in inner.rules {
                rule.media_condition = Some(condition.clone());
                rules.push(rule);
            }
        } else if css[pos..].starts_with("@import") {
            // Skip @import rules for now (consume until ';')
            while pos < bytes.len() && bytes[pos] != b';' {
                pos += 1;
            }
            if pos < bytes.len() {
                pos += 1;
            }
        } else if css[pos..].starts_with('@') {
            // Skip unknown at-rules
            let mut depth = 0;
            loop {
                if pos >= bytes.len() {
                    break;
                }
                if bytes[pos] == b'{' {
                    depth += 1;
                } else if bytes[pos] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        pos += 1;
                        break;
                    }
                } else if bytes[pos] == b';' && depth == 0 {
                    pos += 1;
                    break;
                }
                pos += 1;
            }
        } else {
            // Regular rule: selectors { declarations }
            if let Some(rule) = parse_rule(css, &mut pos) {
                rules.push(rule);
            }
        }
    }

    Stylesheet { rules }
}

/// Parse the content of a `style=""` attribute (declarations only, no selector).
pub fn parse_inline_style(style_attr: &str) -> Vec<Declaration> {
    parse_declarations(style_attr)
}

fn parse_rule(css: &str, pos: &mut usize) -> Option<CssRule> {
    let bytes = css.as_bytes();

    // Read selectors until '{'
    let sel_start = *pos;
    while *pos < bytes.len() && bytes[*pos] != b'{' {
        *pos += 1;
    }
    let selector_text = css[sel_start..*pos].trim();
    if selector_text.is_empty() {
        return None;
    }

    if *pos < bytes.len() {
        *pos += 1; // skip '{'
    }

    // Read declarations until '}'
    let decl_start = *pos;
    let mut depth = 1;
    while *pos < bytes.len() && depth > 0 {
        if bytes[*pos] == b'{' {
            depth += 1;
        } else if bytes[*pos] == b'}' {
            depth -= 1;
        }
        if depth > 0 {
            *pos += 1;
        }
    }
    let decl_text = &css[decl_start..*pos];
    if *pos < bytes.len() {
        *pos += 1; // skip '}'
    }

    let selectors: Vec<String> = selector_text
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let declarations = parse_declarations(decl_text);

    Some(CssRule {
        selectors,
        declarations,
        media_condition: None,
    })
}

fn parse_declarations(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();

    for decl_str in text.split(';') {
        let decl_str = decl_str.trim();
        if decl_str.is_empty() {
            continue;
        }

        // Split on first ':'
        if let Some(colon) = decl_str.find(':') {
            let name = decl_str[..colon].trim();
            let mut value = decl_str[colon + 1..].trim();
            let mut important = false;

            if let Some(stripped) = value.strip_suffix("!important") {
                value = stripped.trim();
                important = true;
            }

            let props = parse_declaration(name, value);
            for property in props {
                declarations.push(Declaration { property, important });
            }
        }
    }

    declarations
}

fn skip_whitespace_and_comments(css: &str, pos: &mut usize) {
    let bytes = css.as_bytes();
    while *pos < bytes.len() {
        // Skip whitespace
        if bytes[*pos].is_ascii_whitespace() {
            *pos += 1;
            continue;
        }
        // Skip /* ... */ comments
        if *pos + 1 < bytes.len() && bytes[*pos] == b'/' && bytes[*pos + 1] == b'*' {
            *pos += 2;
            while *pos + 1 < bytes.len() && !(bytes[*pos] == b'*' && bytes[*pos + 1] == b'/') {
                *pos += 1;
            }
            if *pos + 1 < bytes.len() {
                *pos += 2;
            }
            continue;
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::Property;
    use crate::values::Display;

    #[test]
    fn parse_basic_rule() {
        let ss = parse_stylesheet("div { display: block; }");
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors, vec!["div"]);
        assert_eq!(ss.rules[0].declarations.len(), 1);
        assert_eq!(ss.rules[0].declarations[0].property, Property::Display(Display::Block));
    }

    #[test]
    fn parse_multiple_selectors() {
        let ss = parse_stylesheet("h1, h2, h3 { font-weight: bold; }");
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].selectors.len(), 3);
    }

    #[test]
    fn parse_shorthand() {
        let ss = parse_stylesheet("p { margin: 10px 20px; }");
        assert_eq!(ss.rules[0].declarations.len(), 4); // 4 sides
    }

    #[test]
    fn parse_media_query() {
        let css = "@media (max-width: 768px) { .mobile { display: block; } }";
        let ss = parse_stylesheet(css);
        assert_eq!(ss.rules.len(), 1);
        assert_eq!(ss.rules[0].media_condition.as_deref(), Some("(max-width: 768px)"));
    }

    #[test]
    fn parse_inline() {
        let decls = parse_inline_style("color: red; font-size: 16px");
        assert_eq!(decls.len(), 2);
    }

    #[test]
    fn parse_comments() {
        let css = "/* comment */ div { color: blue; }";
        let ss = parse_stylesheet(css);
        assert_eq!(ss.rules.len(), 1);
    }

    #[test]
    fn parse_empty_stylesheet() {
        let ss = parse_stylesheet("");
        assert!(ss.rules.is_empty());
    }

    #[test]
    fn parse_important() {
        let ss = parse_stylesheet(".override { color: red !important; }");
        assert_eq!(ss.rules[0].declarations.len(), 1);
        assert!(ss.rules[0].declarations[0].important);
    }

    #[test]
    fn parse_multiple_rules() {
        let css = "div { display: block; } p { margin: 1em 0; }";
        let ss = parse_stylesheet(css);
        assert_eq!(ss.rules.len(), 2);
    }

    #[test]
    fn parse_real_world_snippet() {
        let css = r#"
            body { margin: 0; font-family: Arial, sans-serif; }
            .container { max-width: 1200px; margin: 0 auto; padding: 0 20px; }
            h1 { font-size: 2em; color: #333; }
            a { color: blue; text-decoration: underline; }
            @media (max-width: 600px) {
                .container { padding: 0 10px; }
            }
        "#;
        let ss = parse_stylesheet(css);
        assert!(ss.rules.len() >= 5);
    }
}
