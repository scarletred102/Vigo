// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Lightweight preload candidate extraction.
//!
//! This runs on the parsed DOM to identify high-value subresources that can be
//! scheduled early by the network layer.

use std::collections::HashSet;

use vex_core::VexId;
use vex_dom::attributes::get_attribute;
use vex_dom::traversal::Descendants;
use vex_dom::{Document, NodeData};

/// Preload resource type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreloadKind {
    Script,
    ModuleScript,
    Style,
    Image,
    GenericPreload,
}

/// A discovered preload candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreloadCandidate {
    pub node_id: VexId,
    pub url: String,
    pub kind: PreloadKind,
    pub as_type: Option<String>,
    pub media: Option<String>,
    /// Higher means load sooner.
    pub priority: u8,
}

/// Extract preload candidates from the document in deterministic order.
pub fn extract_preload_candidates(doc: &Document) -> Vec<PreloadCandidate> {
    let arena = doc.arena();
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for nid in Descendants::new(arena, doc.root()) {
        let NodeData::Element(el) = &arena.get(nid).data else {
            continue;
        };

        match el.tag_name.as_str() {
            "script" => {
                if let Some(src) = get_attribute(arena, nid, "src") {
                    let script_type =
                        get_attribute(arena, nid, "type").map(str::to_ascii_lowercase);
                    let kind = if script_type.as_deref() == Some("module") {
                        PreloadKind::ModuleScript
                    } else {
                        PreloadKind::Script
                    };
                    push_candidate(
                        &mut out,
                        &mut seen,
                        PreloadCandidate {
                            node_id: nid,
                            url: src.to_string(),
                            kind,
                            as_type: None,
                            media: None,
                            priority: 100,
                        },
                    );
                }
            }
            "link" => {
                let rel = get_attribute(arena, nid, "rel").unwrap_or_default();
                let href = get_attribute(arena, nid, "href");
                let as_type = get_attribute(arena, nid, "as").map(|v| v.to_ascii_lowercase());
                let media = get_attribute(arena, nid, "media").map(str::to_string);

                if let Some(href) = href {
                    if has_rel_token(rel, "stylesheet") {
                        push_candidate(
                            &mut out,
                            &mut seen,
                            PreloadCandidate {
                                node_id: nid,
                                url: href.to_string(),
                                kind: PreloadKind::Style,
                                as_type: None,
                                media: media.clone(),
                                priority: 90,
                            },
                        );
                    }

                    if has_rel_token(rel, "modulepreload") {
                        push_candidate(
                            &mut out,
                            &mut seen,
                            PreloadCandidate {
                                node_id: nid,
                                url: href.to_string(),
                                kind: PreloadKind::ModuleScript,
                                as_type: Some("script".to_string()),
                                media: media.clone(),
                                priority: 95,
                            },
                        );
                    }

                    if has_rel_token(rel, "preload") || has_rel_token(rel, "prefetch") {
                        let (kind, priority) = match as_type.as_deref() {
                            Some("script") => (PreloadKind::Script, 85),
                            Some("style") => (PreloadKind::Style, 80),
                            Some("image") => (PreloadKind::Image, 60),
                            Some("fetch") => (PreloadKind::GenericPreload, 55),
                            _ => (PreloadKind::GenericPreload, 50),
                        };

                        push_candidate(
                            &mut out,
                            &mut seen,
                            PreloadCandidate {
                                node_id: nid,
                                url: href.to_string(),
                                kind,
                                as_type: as_type.clone(),
                                media: media.clone(),
                                priority,
                            },
                        );
                    }
                }
            }
            "img" => {
                if let Some(src) = get_attribute(arena, nid, "src") {
                    push_candidate(
                        &mut out,
                        &mut seen,
                        PreloadCandidate {
                            node_id: nid,
                            url: src.to_string(),
                            kind: PreloadKind::Image,
                            as_type: Some("image".to_string()),
                            media: None,
                            priority: 60,
                        },
                    );
                }
            }
            "source" => {
                if let Some(srcset) = get_attribute(arena, nid, "srcset") {
                    if let Some(first) = first_srcset_candidate(srcset) {
                        push_candidate(
                            &mut out,
                            &mut seen,
                            PreloadCandidate {
                                node_id: nid,
                                url: first,
                                kind: PreloadKind::Image,
                                as_type: Some("image".to_string()),
                                media: None,
                                priority: 45,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    out.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.url.cmp(&b.url)));
    out
}

fn push_candidate(
    out: &mut Vec<PreloadCandidate>,
    seen: &mut HashSet<(PreloadKind, String)>,
    candidate: PreloadCandidate,
) {
    let key = (candidate.kind.clone(), candidate.url.clone());
    if seen.insert(key) {
        out.push(candidate);
    }
}

fn has_rel_token(rel: &str, token: &str) -> bool {
    rel.split_ascii_whitespace()
        .any(|part| part.eq_ignore_ascii_case(token))
}

fn first_srcset_candidate(srcset: &str) -> Option<String> {
    let first = srcset.split(',').next()?.trim();
    let url = first.split_ascii_whitespace().next()?.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_html;

    #[test]
    fn extracts_core_preload_candidates() {
        let doc = parse_html(
            r#"
            <html><head>
                <link rel="stylesheet" href="/a.css">
                <script src="/a.js"></script>
                <link rel="modulepreload" href="/mod.js">
                <link rel="preload" href="/hero.webp" as="image">
            </head>
            <body><img src="/img.png"></body></html>
            "#,
        );

        let c = extract_preload_candidates(&doc);
        assert!(c
            .iter()
            .any(|x| x.url == "/a.css" && x.kind == PreloadKind::Style));
        assert!(c
            .iter()
            .any(|x| x.url == "/a.js" && x.kind == PreloadKind::Script));
        assert!(c
            .iter()
            .any(|x| x.url == "/mod.js" && x.kind == PreloadKind::ModuleScript));
        assert!(c
            .iter()
            .any(|x| x.url == "/hero.webp" && x.kind == PreloadKind::Image));
        assert!(c
            .iter()
            .any(|x| x.url == "/img.png" && x.kind == PreloadKind::Image));
    }

    #[test]
    fn rel_token_parsing_is_case_insensitive() {
        let doc = parse_html(r#"<link rel="PreLoad STYLESHEET" href="/x.css" as="style">"#);
        let c = extract_preload_candidates(&doc);
        assert!(c
            .iter()
            .any(|x| x.url == "/x.css" && x.kind == PreloadKind::Style));
    }

    #[test]
    fn deduplicates_same_kind_and_url() {
        let doc = parse_html(
            r#"
            <script src="/dup.js"></script>
            <script src="/dup.js"></script>
            "#,
        );
        let c = extract_preload_candidates(&doc);
        let matches = c
            .iter()
            .filter(|x| x.url == "/dup.js" && x.kind == PreloadKind::Script)
            .count();
        assert_eq!(matches, 1);
    }

    #[test]
    fn parses_first_srcset_candidate() {
        let doc = parse_html(r#"<source srcset="/a.webp 1x, /b.webp 2x">"#);
        let c = extract_preload_candidates(&doc);
        assert!(c.iter().any(|x| x.url == "/a.webp"));
    }
}
