// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extract `<script>` and `<style>` / `<link rel="stylesheet">` references
//! from a parsed DOM tree.

use vex_core::VexId;
use vex_dom::attributes::get_attribute;
use vex_dom::traversal::Descendants;
use vex_dom::{Document, NodeData};

/// Information about a `<script>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptInfo {
    /// The node id in the DOM.
    pub node_id: VexId,
    /// `src` attribute value, if external.
    pub src: Option<String>,
    /// Inline script content (text children).
    pub inline_content: Option<String>,
    /// `type` attribute (e.g., `"module"`).
    pub script_type: Option<String>,
    /// Whether `async` attribute is present.
    pub is_async: bool,
    /// Whether `defer` attribute is present.
    pub is_defer: bool,
}

/// Information about a stylesheet reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleInfo {
    /// An inline `<style>` element.
    Inline { node_id: VexId, content: String },
    /// An external `<link rel="stylesheet">`.
    External {
        node_id: VexId,
        href: String,
        media: Option<String>,
    },
}

/// Extract all script references from the document.
pub fn extract_scripts(doc: &Document) -> Vec<ScriptInfo> {
    let arena = doc.arena();
    let mut scripts = Vec::new();

    for nid in Descendants::new(arena, doc.root()) {
        if let NodeData::Element(ref el) = arena.get(nid).data {
            if el.tag_name == "script" {
                let src = get_attribute(arena, nid, "src").map(|s| s.to_string());
                let script_type = get_attribute(arena, nid, "type").map(|s| s.to_string());
                let is_async = get_attribute(arena, nid, "async").is_some();
                let is_defer = get_attribute(arena, nid, "defer").is_some();

                let inline_content = if src.is_none() {
                    let text = doc.text_content(nid);
                    let trimmed = text.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                } else {
                    None
                };

                scripts.push(ScriptInfo {
                    node_id: nid,
                    src,
                    inline_content,
                    script_type,
                    is_async,
                    is_defer,
                });
            }
        }
    }

    scripts
}

/// Extract all stylesheet references from the document.
pub fn extract_styles(doc: &Document) -> Vec<StyleInfo> {
    let arena = doc.arena();
    let mut styles = Vec::new();

    for nid in Descendants::new(arena, doc.root()) {
        if let NodeData::Element(ref el) = arena.get(nid).data {
            match el.tag_name.as_str() {
                "style" => {
                    let content = doc.text_content(nid);
                    if !content.is_empty() {
                        styles.push(StyleInfo::Inline {
                            node_id: nid,
                            content,
                        });
                    }
                }
                "link" => {
                    let rel = get_attribute(arena, nid, "rel");
                    let href = get_attribute(arena, nid, "href");
                    if rel.is_some_and(|v| has_rel_token(v, "stylesheet")) {
                        if let Some(href) = href {
                            styles.push(StyleInfo::External {
                                node_id: nid,
                                href: href.to_string(),
                                media: get_attribute(arena, nid, "media").map(|s| s.to_string()),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    styles
}

fn has_rel_token(rel: &str, token: &str) -> bool {
    rel.split_ascii_whitespace()
        .any(|part| part.eq_ignore_ascii_case(token))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_html;

    #[test]
    fn extract_inline_script() {
        let doc =
            parse_html(r#"<html><head><script>alert("hi")</script></head><body></body></html>"#);
        let scripts = extract_scripts(&doc);
        assert_eq!(scripts.len(), 1);
        assert!(scripts[0].src.is_none());
        assert_eq!(scripts[0].inline_content.as_deref(), Some(r#"alert("hi")"#));
    }

    #[test]
    fn extract_external_script() {
        let doc = parse_html(
            r#"<html><head><script src="/app.js" defer></script></head><body></body></html>"#,
        );
        let scripts = extract_scripts(&doc);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].src.as_deref(), Some("/app.js"));
        assert!(scripts[0].is_defer);
        assert!(scripts[0].inline_content.is_none());
    }

    #[test]
    fn extract_module_script() {
        let doc = parse_html(r#"<script type="module" src="/mod.js" async></script>"#);
        let scripts = extract_scripts(&doc);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].script_type.as_deref(), Some("module"));
        assert!(scripts[0].is_async);
    }

    #[test]
    fn extract_inline_style() {
        let doc = parse_html(
            r#"<html><head><style>body { margin: 0; }</style></head><body></body></html>"#,
        );
        let styles = extract_styles(&doc);
        assert_eq!(styles.len(), 1);
        match &styles[0] {
            StyleInfo::Inline { content, .. } => {
                assert!(content.contains("margin: 0"));
            }
            _ => panic!("expected inline style"),
        }
    }

    #[test]
    fn extract_external_stylesheet() {
        let doc = parse_html(
            r#"<html><head><link rel="stylesheet" href="/style.css" media="screen"></head><body></body></html>"#,
        );
        let styles = extract_styles(&doc);
        assert_eq!(styles.len(), 1);
        match &styles[0] {
            StyleInfo::External { href, media, .. } => {
                assert_eq!(href, "/style.css");
                assert_eq!(media.as_deref(), Some("screen"));
            }
            _ => panic!("expected external stylesheet"),
        }
    }

    #[test]
    fn ignores_non_stylesheet_links() {
        let doc = parse_html(r#"<link rel="icon" href="/favicon.ico">"#);
        let styles = extract_styles(&doc);
        assert!(styles.is_empty());
    }

    #[test]
    fn rel_token_parsing_for_stylesheet_is_case_insensitive() {
        let doc = parse_html(r#"<link rel="preload STYLESHEET" href="/x.css">"#);
        let styles = extract_styles(&doc);
        assert_eq!(styles.len(), 1);
    }

    #[test]
    fn inline_script_content_is_trimmed() {
        let doc = parse_html("<script>\n  console.log('x')\n</script>");
        let scripts = extract_scripts(&doc);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].inline_content.as_deref(), Some("console.log('x')"));
    }

    #[test]
    fn multiple_scripts_and_styles() {
        let doc = parse_html(
            r#"
            <html>
            <head>
                <script src="/a.js"></script>
                <style>h1 { color: red }</style>
                <link rel="stylesheet" href="/b.css">
            </head>
            <body>
                <script>console.log("hi")</script>
            </body>
            </html>
        "#,
        );
        assert_eq!(extract_scripts(&doc).len(), 2);
        assert_eq!(extract_styles(&doc).len(), 2);
    }
}
