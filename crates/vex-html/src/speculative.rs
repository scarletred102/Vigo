// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Lightweight speculative preload scanner for streaming HTML.
//!
//! This scanner consumes incoming HTML chunks and emits coarse resource hints
//! before the full DOM is built.

use crate::preload::PreloadKind;

/// A speculative preload hint discovered in streamed bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeculativePreloadHint {
    pub url: String,
    pub kind: PreloadKind,
}

/// Stateful scanner that keeps trailing partial tags between chunks.
#[derive(Debug, Default)]
pub struct SpeculativePreloadScanner {
    pending: String,
}

impl SpeculativePreloadScanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk and return discovered speculative hints.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SpeculativePreloadHint> {
        self.pending.push_str(&String::from_utf8_lossy(chunk));

        let mut out = Vec::new();
        let mut scan_from = 0usize;

        while let Some(rel_lt) = self.pending[scan_from..].find('<') {
            let start = scan_from + rel_lt;
            let Some(rel_gt) = self.pending[start..].find('>') else {
                // Partial tag; keep for next chunk.
                break;
            };
            let end = start + rel_gt;
            let tag = &self.pending[start + 1..end];
            if let Some(hint) = parse_tag_for_hint(tag) {
                out.push(hint);
            }
            scan_from = end + 1;
        }

        if scan_from > 0 {
            self.pending.drain(..scan_from);
        }

        // Prevent unbounded growth on malformed input with no tags.
        if self.pending.len() > 16 * 1024 {
            let keep_from = self.pending.len() - 4 * 1024;
            self.pending.drain(..keep_from);
        }

        out
    }
}

fn parse_tag_for_hint(tag: &str) -> Option<SpeculativePreloadHint> {
    let trimmed = tag.trim();
    if trimmed.starts_with('/') || trimmed.starts_with('!') || trimmed.starts_with('?') {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();

    if lower.starts_with("script") {
        let src = extract_attr(trimmed, "src")?;
        let kind =
            if extract_attr(trimmed, "type").is_some_and(|t| t.eq_ignore_ascii_case("module")) {
                PreloadKind::ModuleScript
            } else {
                PreloadKind::Script
            };
        return Some(SpeculativePreloadHint { url: src, kind });
    }

    if lower.starts_with("img") {
        let src = extract_attr(trimmed, "src")?;
        return Some(SpeculativePreloadHint {
            url: src,
            kind: PreloadKind::Image,
        });
    }

    if lower.starts_with("link") {
        let rel = extract_attr(trimmed, "rel").unwrap_or_default();
        let href = extract_attr(trimmed, "href")?;

        if has_rel_token(&rel, "stylesheet") {
            return Some(SpeculativePreloadHint {
                url: href,
                kind: PreloadKind::Style,
            });
        }

        if has_rel_token(&rel, "modulepreload") {
            return Some(SpeculativePreloadHint {
                url: href,
                kind: PreloadKind::ModuleScript,
            });
        }

        if has_rel_token(&rel, "preload") || has_rel_token(&rel, "prefetch") {
            let kind = match extract_attr(trimmed, "as")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "script" => PreloadKind::Script,
                "style" => PreloadKind::Style,
                "image" => PreloadKind::Image,
                _ => PreloadKind::GenericPreload,
            };
            return Some(SpeculativePreloadHint { url: href, kind });
        }
    }

    None
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    // Minimal ASCII attribute extraction suitable for speculative hints.
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{attr}=");
    let idx = lower.find(&needle)?;
    let value_src = &tag[idx + needle.len()..].trim_start();

    if let Some(rest) = value_src.strip_prefix('"') {
        return Some(rest.split('"').next()?.to_string());
    }
    if let Some(rest) = value_src.strip_prefix('\'') {
        return Some(rest.split('\'').next()?.to_string());
    }

    Some(
        value_src
            .split_ascii_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('>')
            .to_string(),
    )
}

fn has_rel_token(rel: &str, token: &str) -> bool {
    rel.split_ascii_whitespace()
        .any(|part| part.eq_ignore_ascii_case(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_script_and_stylesheet_hints() {
        let mut s = SpeculativePreloadScanner::new();
        let hints = s.feed(b"<script src='/a.js'></script><link rel='stylesheet' href='/a.css'>");
        assert!(hints
            .iter()
            .any(|h| h.url == "/a.js" && h.kind == PreloadKind::Script));
        assert!(hints
            .iter()
            .any(|h| h.url == "/a.css" && h.kind == PreloadKind::Style));
    }

    #[test]
    fn handles_split_tags_across_chunks() {
        let mut s = SpeculativePreloadScanner::new();
        let first = s.feed(b"<scr");
        assert!(first.is_empty());
        let second = s.feed(b"ipt src=\"/b.js\">");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].url, "/b.js");
    }

    #[test]
    fn classifies_preload_as_image() {
        let mut s = SpeculativePreloadScanner::new();
        let hints = s.feed(b"<link rel='preload' href='/hero.webp' as='image'>");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].kind, PreloadKind::Image);
    }
}
