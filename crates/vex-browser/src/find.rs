// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Find-in-page — text search within the current page's DOM.

use vex_core::VexId;
use vex_dom::node::NodeData;
use vex_dom::Document;

/// A match found during page search.
#[derive(Debug, Clone, PartialEq)]
pub struct FindMatch {
    /// The text node containing the match.
    pub node_id: VexId,
    /// Byte offset within the text node's content.
    pub offset: usize,
    /// Length of the matched text in bytes.
    pub length: usize,
}

/// Find-in-page search state.
#[derive(Debug)]
pub struct FindState {
    /// The search query.
    pub query: String,
    /// All matches found.
    pub matches: Vec<FindMatch>,
    /// Index of the currently highlighted match (if any).
    pub current_index: Option<usize>,
    /// Whether the search is case-sensitive.
    pub case_sensitive: bool,
}

impl FindState {
    /// Create a new find state (no active search).
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current_index: None,
            case_sensitive: false,
        }
    }

    /// Search the document for all occurrences of the query.
    pub fn search(&mut self, document: &Document, query: &str) {
        self.query = query.to_string();
        self.matches.clear();
        self.current_index = None;

        if query.is_empty() {
            return;
        }

        let search_query = if self.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        // Walk all text nodes in document order.
        let arena = document.arena();
        if let Some(root_element) = document.root_element() {
            collect_text_matches(
                arena,
                root_element,
                &search_query,
                self.case_sensitive,
                &mut self.matches,
            );
        }

        if !self.matches.is_empty() {
            self.current_index = Some(0);
        }
    }

    /// Move to the next match. Wraps around.
    pub fn next_match(&mut self) {
        if let Some(ref mut idx) = self.current_index {
            *idx = (*idx + 1) % self.matches.len();
        }
    }

    /// Move to the previous match. Wraps around.
    pub fn prev_match(&mut self) {
        if let Some(ref mut idx) = self.current_index {
            if *idx == 0 {
                *idx = self.matches.len() - 1;
            } else {
                *idx -= 1;
            }
        }
    }

    /// Get the currently highlighted match.
    pub fn current_match(&self) -> Option<&FindMatch> {
        self.current_index.and_then(|i| self.matches.get(i))
    }

    /// Match count.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Status string like "1 of 5".
    pub fn status(&self) -> String {
        if self.matches.is_empty() {
            if self.query.is_empty() {
                String::new()
            } else {
                "No matches".to_string()
            }
        } else if let Some(idx) = self.current_index {
            format!("{} of {}", idx + 1, self.matches.len())
        } else {
            format!("{} matches", self.matches.len())
        }
    }

    /// Clear the search.
    pub fn clear(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.current_index = None;
    }

    /// Whether a search is active.
    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }
}

impl Default for FindState {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively collect text matches in document order.
fn collect_text_matches(
    arena: &vex_dom::arena::NodeArena,
    node_id: VexId,
    query: &str,
    case_sensitive: bool,
    matches: &mut Vec<FindMatch>,
) {
    let node = arena.get(node_id);
    if let NodeData::Text(ref text) = node.data {
        let haystack = if case_sensitive {
            text.clone()
        } else {
            text.to_lowercase()
        };

        let mut start = 0;
        while let Some(pos) = haystack[start..].find(query) {
            matches.push(FindMatch {
                node_id,
                offset: start + pos,
                length: query.len(),
            });
            start += pos + 1;
        }
    }

    // Recurse into children.
    let mut child = node.first_child;
    while let Some(child_id) = child {
        collect_text_matches(arena, child_id, query, case_sensitive, matches);
        child = arena.get(child_id).next_sibling;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(html: &str) -> Document {
        vex_html::parse_html(html)
    }

    #[test]
    fn find_text_in_page() {
        let doc = make_doc("<html><body><p>Hello world. Hello again.</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "Hello");
        assert_eq!(find.match_count(), 2);
        assert_eq!(find.status(), "1 of 2");
    }

    #[test]
    fn find_case_insensitive() {
        let doc = make_doc("<html><body><p>Hello HELLO hello</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "hello");
        assert_eq!(find.match_count(), 3);
    }

    #[test]
    fn find_no_matches() {
        let doc = make_doc("<html><body><p>Hello world</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "xyz");
        assert_eq!(find.match_count(), 0);
        assert_eq!(find.status(), "No matches");
    }

    #[test]
    fn next_prev_match_wraps() {
        let doc = make_doc("<html><body><p>a a a</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "a");
        assert!(find.match_count() >= 3);

        let first = find.current_index;
        find.next_match();
        assert_ne!(find.current_index, first);

        // Go back to first.
        find.prev_match();
        assert_eq!(find.current_index, first);

        // Wrap around backward.
        find.prev_match();
        assert_eq!(find.current_index, Some(find.match_count() - 1));
    }

    #[test]
    fn clear_resets_state() {
        let doc = make_doc("<html><body><p>Hello</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "Hello");
        assert!(find.is_active());
        find.clear();
        assert!(!find.is_active());
        assert_eq!(find.match_count(), 0);
    }

    #[test]
    fn find_across_multiple_text_nodes() {
        let doc =
            make_doc("<html><body><p>Hello</p><p>Hello again</p><div>Hello!</div></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "Hello");
        assert_eq!(find.match_count(), 3);
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let doc = make_doc("<html><body><p>Hello</p></body></html>");
        let mut find = FindState::new();
        find.search(&doc, "");
        assert_eq!(find.match_count(), 0);
        assert!(!find.is_active());
    }
}
