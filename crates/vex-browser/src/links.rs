// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Link handling — resolves clicked `<a>` elements to navigation actions.

use vex_core::{VexId, VexResult, VexUrl};
use vex_dom::attributes::get_attribute;
use vex_dom::node::NodeData;
use vex_dom::Document;

/// The action to take when a link is clicked.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    /// Navigate the current tab to this URL.
    Navigate(VexUrl),
    /// Open URL in a new tab.
    NewTab(VexUrl),
    /// Execute the given JavaScript.
    RunScript(String),
    /// Nothing to do (not a valid link).
    None,
}

/// Resolve a clicked element to a navigation action.
///
/// Walks up from the clicked element to find the nearest `<a>` ancestor,
/// then inspects its `href` and `target` attributes.
pub fn resolve_link_click(
    document: &Document,
    clicked_id: VexId,
    current_url: &VexUrl,
) -> LinkAction {
    // Walk up from the clicked element to find an <a> tag.
    let arena = document.arena();
    let mut node_id = Some(clicked_id);
    while let Some(id) = node_id {
        let node = arena.get(id);
        if let NodeData::Element(ref el) = node.data {
            if el.tag_name.eq_ignore_ascii_case("a") {
                return resolve_anchor(document, id, current_url);
            }
        }
        node_id = node.parent;
    }
    LinkAction::None
}

/// Resolve an `<a>` element's href to a link action.
fn resolve_anchor(document: &Document, anchor_id: VexId, current_url: &VexUrl) -> LinkAction {
    let arena = document.arena();
    let href = match get_attribute(arena, anchor_id, "href") {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => return LinkAction::None,
    };

    // Handle javascript: URLs.
    if href.starts_with("javascript:") {
        let script = href.strip_prefix("javascript:").unwrap_or("");
        return LinkAction::RunScript(script.to_string());
    }

    // Resolve relative URL against current page.
    let resolved = match current_url.join(&href) {
        Ok(url) => url,
        Err(_) => return LinkAction::None,
    };

    // Check target attribute.
    let target = get_attribute(arena, anchor_id, "target").unwrap_or("");

    if target == "_blank" {
        LinkAction::NewTab(resolved)
    } else {
        LinkAction::Navigate(resolved)
    }
}

/// Ensure a URL string has a scheme; prepend `https://` if missing.
pub fn normalize_url_input(input: &str) -> VexResult<VexUrl> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return VexUrl::parse("vex://newtab");
    }

    // If it already has a scheme, parse directly.
    if trimmed.contains("://") || trimmed.starts_with("vex:") || trimmed.starts_with("about:") {
        return VexUrl::parse(trimmed);
    }

    // If it looks like a domain name, add https://.
    if trimmed.contains('.') || trimmed.starts_with("localhost") {
        return VexUrl::parse(&format!("https://{trimmed}"));
    }

    // Otherwise, treat as a search query (stub — just prepend https://).
    VexUrl::parse(&format!("https://{trimmed}"))
}

/// Normalize user input into a URL, using a search engine for non-URL text.
///
/// `search_template` is a URL template such as
/// `"https://duckduckgo.com/?q={query}"` where `{query}` will be replaced
/// with the percent-encoded query text.
pub fn normalize_or_search(input: &str, search_template: &str) -> VexResult<VexUrl> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return VexUrl::parse("vex://newtab");
    }

    // Already has a scheme — parse directly.
    if trimmed.contains("://") || trimmed.starts_with("vex:") || trimmed.starts_with("about:") {
        return VexUrl::parse(trimmed);
    }

    // Looks like a domain name — add https://.
    if trimmed.contains('.') || trimmed.starts_with("localhost") {
        return VexUrl::parse(&format!("https://{trimmed}"));
    }

    // Search query — percent-encode and substitute into the template.
    let encoded = simple_url_encode(trimmed);
    let search_url = search_template.replace("{query}", &encoded);
    VexUrl::parse(&search_url)
}

/// Minimal percent-encoding for search query strings.
fn simple_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_click_on_non_link_returns_none() {
        let doc = vex_html::parse_html("<html><body><p>No links here</p></body></html>");
        let body_ids = doc.get_elements_by_tag_name("p");
        let p_id = body_ids[0];
        let url = VexUrl::parse("https://example.com").unwrap();
        assert_eq!(resolve_link_click(&doc, p_id, &url), LinkAction::None);
    }

    #[test]
    fn resolve_click_on_anchor() {
        let doc = vex_html::parse_html("<html><body><a href=\"/page\">link</a></body></html>");
        let a_ids = doc.get_elements_by_tag_name("a");
        let a_id = a_ids[0];
        let url = VexUrl::parse("https://example.com").unwrap();

        match resolve_link_click(&doc, a_id, &url) {
            LinkAction::Navigate(resolved) => {
                assert_eq!(resolved.as_ref(), "https://example.com/page");
            }
            other => panic!("Expected Navigate, got {other:?}"),
        }
    }

    #[test]
    fn resolve_blank_target_opens_new_tab() {
        let doc = vex_html::parse_html(
            "<html><body><a href=\"https://other.com\" target=\"_blank\">x</a></body></html>",
        );
        let a_ids = doc.get_elements_by_tag_name("a");
        let url = VexUrl::parse("https://example.com").unwrap();

        match resolve_link_click(&doc, a_ids[0], &url) {
            LinkAction::NewTab(u) => {
                assert_eq!(u.as_ref(), "https://other.com/");
            }
            other => panic!("Expected NewTab, got {other:?}"),
        }
    }

    #[test]
    fn normalize_adds_https() {
        let url = normalize_url_input("example.com").unwrap();
        assert_eq!(url.as_ref(), "https://example.com/");
    }

    #[test]
    fn normalize_preserves_existing_scheme() {
        let url = normalize_url_input("http://example.com").unwrap();
        assert_eq!(url.as_ref(), "http://example.com/");
    }

    #[test]
    fn normalize_empty_returns_newtab() {
        let url = normalize_url_input("").unwrap();
        assert_eq!(url.as_ref(), "vex://newtab");
    }

    #[test]
    fn search_query_uses_template() {
        let url = normalize_or_search("rust browser", "https://search.example/?q={query}").unwrap();
        assert_eq!(url.as_ref(), "https://search.example/?q=rust+browser");
    }

    #[test]
    fn search_preserves_existing_scheme() {
        let url = normalize_or_search("https://example.com", "https://search.example/?q={query}")
            .unwrap();
        assert_eq!(url.as_ref(), "https://example.com/");
    }

    #[test]
    fn search_domain_gets_https() {
        let url = normalize_or_search("example.com", "https://search.example/?q={query}").unwrap();
        assert_eq!(url.as_ref(), "https://example.com/");
    }

    #[test]
    fn search_empty_returns_newtab() {
        let url = normalize_or_search("", "https://search.example/?q={query}").unwrap();
        assert_eq!(url.as_ref(), "vex://newtab");
    }
}
