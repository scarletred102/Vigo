// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Sources panel — page source viewer with syntax highlighting.
//!
//! Displays the page's HTML source and any external resources (scripts,
//! stylesheets) as separate tabs. Provides basic syntax highlighting
//! by classifying spans of source text into token types.

use vex_core::geometry::Rect;

/// Unique identifier for a source tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u64);

/// Type of source resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// The main page HTML.
    Html,
    /// An external JavaScript file.
    Script,
    /// An external CSS stylesheet.
    Stylesheet,
}

impl SourceKind {
    /// Display label for the source kind.
    pub fn label(self) -> &'static str {
        match self {
            Self::Html => "HTML",
            Self::Script => "JS",
            Self::Stylesheet => "CSS",
        }
    }
}

/// A source document tracked in the Sources panel.
#[derive(Debug, Clone)]
pub struct SourceDocument {
    /// Unique identifier.
    pub id: SourceId,
    /// Display label (e.g. filename or URL path).
    pub label: String,
    /// Full URL of the resource.
    pub url: String,
    /// Type of source.
    pub kind: SourceKind,
    /// Raw source text.
    pub content: String,
    /// Pre-computed line offsets (byte offset of each line start).
    line_offsets: Vec<usize>,
}

impl SourceDocument {
    /// Create a new source document and compute line offsets.
    pub fn new(
        id: SourceId,
        label: String,
        url: String,
        kind: SourceKind,
        content: String,
    ) -> Self {
        let line_offsets = compute_line_offsets(&content);
        Self {
            id,
            label,
            url,
            kind,
            content,
            line_offsets,
        }
    }

    /// Number of lines in the source.
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    /// Get a specific line (1-indexed). Returns `None` for out-of-range.
    pub fn line(&self, number: usize) -> Option<&str> {
        if number == 0 || number > self.line_offsets.len() {
            return None;
        }
        let start = self.line_offsets[number - 1];
        let end = if number < self.line_offsets.len() {
            self.line_offsets[number]
        } else {
            self.content.len()
        };
        // Trim trailing newline characters from the line.
        let line = &self.content[start..end];
        Some(line.trim_end_matches(['\n', '\r']))
    }

    /// Get a range of lines (1-indexed, inclusive). Clamps to valid range.
    pub fn lines(&self, start: usize, end: usize) -> Vec<(usize, &str)> {
        let start = start.max(1);
        let end = end.min(self.line_count());
        (start..=end)
            .filter_map(|n| self.line(n).map(|l| (n, l)))
            .collect()
    }
}

/// Compute byte offsets of each line start in the source text.
fn compute_line_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, byte) in content.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Token type for basic syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// HTML/XML tag names and angle brackets.
    Tag,
    /// Attribute names.
    Attribute,
    /// Attribute values (quoted strings).
    StringLiteral,
    /// Comments (`<!-- -->` or `/* */` or `//`).
    Comment,
    /// Keywords (JS: `function`, `var`, `let`, `const`, `return`, etc.).
    Keyword,
    /// Numbers.
    Number,
    /// CSS property names.
    Property,
    /// Punctuation / operators.
    Punctuation,
    /// Regular text / identifiers.
    Plain,
}

/// A highlighted span within a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Start column (0-indexed byte offset within the line).
    pub start: usize,
    /// End column (exclusive byte offset within the line).
    pub end: usize,
    /// Token kind for coloring.
    pub kind: TokenKind,
}

/// Basic HTML syntax highlighting for a single line.
///
/// This is a simple heuristic-based highlighter, not a full parser.
pub fn highlight_html_line(line: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            // Start of a tag or comment.
            if i + 3 < len && &bytes[i..i + 4] == b"<!--" {
                // HTML comment.
                let end = find_subsequence(&bytes[i..], b"-->")
                    .map(|pos| i + pos + 3)
                    .unwrap_or(len);
                spans.push(HighlightSpan {
                    start: i,
                    end,
                    kind: TokenKind::Comment,
                });
                i = end;
            } else {
                // Tag: mark the `<TagName` or `</TagName` as Tag.
                let tag_end = bytes[i..]
                    .iter()
                    .position(|&b| b == b'>' || b == b' ' || b == b'\t')
                    .map(|p| i + p)
                    .unwrap_or(len);
                spans.push(HighlightSpan {
                    start: i,
                    end: tag_end,
                    kind: TokenKind::Tag,
                });
                i = tag_end;

                // Scan for attributes until >.
                while i < len && bytes[i] != b'>' {
                    // Skip whitespace.
                    if bytes[i].is_ascii_whitespace() {
                        i += 1;
                        continue;
                    }
                    // Check for attribute value (quoted string).
                    if bytes[i] == b'"' || bytes[i] == b'\'' {
                        let quote = bytes[i];
                        let str_end = bytes[i + 1..]
                            .iter()
                            .position(|&b| b == quote)
                            .map(|p| i + p + 2)
                            .unwrap_or(len);
                        spans.push(HighlightSpan {
                            start: i,
                            end: str_end,
                            kind: TokenKind::StringLiteral,
                        });
                        i = str_end;
                        continue;
                    }
                    // Check for `=`.
                    if bytes[i] == b'=' {
                        spans.push(HighlightSpan {
                            start: i,
                            end: i + 1,
                            kind: TokenKind::Punctuation,
                        });
                        i += 1;
                        continue;
                    }
                    // Attribute name.
                    let attr_end = bytes[i..]
                        .iter()
                        .position(|&b| {
                            b == b'=' || b == b'>' || b == b' ' || b == b'\t' || b == b'"'
                        })
                        .map(|p| i + p)
                        .unwrap_or(len);
                    if attr_end > i {
                        spans.push(HighlightSpan {
                            start: i,
                            end: attr_end,
                            kind: TokenKind::Attribute,
                        });
                    }
                    i = attr_end;
                }

                // Closing `>`.
                if i < len && bytes[i] == b'>' {
                    spans.push(HighlightSpan {
                        start: i,
                        end: i + 1,
                        kind: TokenKind::Tag,
                    });
                    i += 1;
                }
            }
        } else {
            // Plain text until next `<`.
            let text_end = bytes[i..]
                .iter()
                .position(|&b| b == b'<')
                .map(|p| i + p)
                .unwrap_or(len);
            if text_end > i {
                spans.push(HighlightSpan {
                    start: i,
                    end: text_end,
                    kind: TokenKind::Plain,
                });
            }
            i = text_end;
        }
    }

    spans
}

/// Find a byte subsequence within a slice.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// State for the Sources panel.
#[derive(Debug, Clone)]
pub struct SourcesState {
    /// All source documents.
    sources: Vec<SourceDocument>,
    /// Auto-incrementing ID.
    next_id: u64,
    /// Currently active tab (by SourceId).
    active_tab: Option<SourceId>,
    /// Scroll offset (line number, 1-indexed).
    scroll_line: usize,
    /// Search query (for find-in-source).
    search_text: String,
    /// Search match positions: (source_id, line_number).
    search_matches: Vec<(SourceId, usize)>,
    /// Current search match index.
    search_index: usize,
}

impl Default for SourcesState {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            next_id: 1,
            active_tab: None,
            scroll_line: 1,
            search_text: String::new(),
            search_matches: Vec::new(),
            search_index: 0,
        }
    }
}

impl SourcesState {
    /// Create a new empty sources state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source document. Returns its ID.
    pub fn add_source(
        &mut self,
        label: String,
        url: String,
        kind: SourceKind,
        content: String,
    ) -> SourceId {
        let id = SourceId(self.next_id);
        self.next_id += 1;
        let doc = SourceDocument::new(id, label, url, kind, content);
        self.sources.push(doc);

        // Auto-select first source.
        if self.active_tab.is_none() {
            self.active_tab = Some(id);
        }

        id
    }

    /// Remove all sources (e.g. on navigation).
    pub fn clear(&mut self) {
        self.sources.clear();
        self.active_tab = None;
        self.scroll_line = 1;
        self.search_text.clear();
        self.search_matches.clear();
        self.next_id = 1;
    }

    /// All source documents.
    pub fn sources(&self) -> &[SourceDocument] {
        &self.sources
    }

    /// Get a source document by ID.
    pub fn get_source(&self, id: SourceId) -> Option<&SourceDocument> {
        self.sources.iter().find(|s| s.id == id)
    }

    /// The currently active source document.
    pub fn active_source(&self) -> Option<&SourceDocument> {
        self.active_tab.and_then(|id| self.get_source(id))
    }

    /// Switch to a different source tab.
    pub fn set_active_tab(&mut self, id: SourceId) {
        if self.sources.iter().any(|s| s.id == id) {
            self.active_tab = Some(id);
            self.scroll_line = 1;
        }
    }

    /// Currently active tab ID.
    pub fn active_tab(&self) -> Option<SourceId> {
        self.active_tab
    }

    /// Number of source documents.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    // ── Scrolling ──

    /// Current scroll line (1-indexed).
    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    /// Set scroll line (clamped to valid range).
    pub fn set_scroll_line(&mut self, line: usize) {
        let max = self.active_source().map(|s| s.line_count()).unwrap_or(1);
        self.scroll_line = line.max(1).min(max);
    }

    /// Scroll down by N lines.
    pub fn scroll_down(&mut self, lines: usize) {
        self.set_scroll_line(self.scroll_line.saturating_add(lines));
    }

    /// Scroll up by N lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.set_scroll_line(self.scroll_line.saturating_sub(lines));
    }

    // ── Search ──

    /// Set search text and compute matches in the active source.
    pub fn search(&mut self, text: String) {
        self.search_text = text;
        self.search_matches.clear();
        self.search_index = 0;

        if self.search_text.is_empty() {
            return;
        }

        let search_lower = self.search_text.to_ascii_lowercase();

        // Find the active source by tab ID.
        let active_idx = self
            .active_tab
            .and_then(|id| self.sources.iter().position(|s| s.id == id));

        if let Some(idx) = active_idx {
            let source = &self.sources[idx];
            let sid = source.id;
            let line_count = source.line_count();
            for line_num in 1..=line_count {
                if let Some(line) = source.line(line_num) {
                    if line.to_ascii_lowercase().contains(&search_lower) {
                        self.search_matches.push((sid, line_num));
                    }
                }
            }
        }
    }

    /// Number of search matches.
    pub fn match_count(&self) -> usize {
        self.search_matches.len()
    }

    /// Current search match (source_id, line_number).
    pub fn current_match(&self) -> Option<(SourceId, usize)> {
        self.search_matches.get(self.search_index).copied()
    }

    /// Jump to next search match.
    pub fn next_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.search_index = (self.search_index + 1) % self.search_matches.len();
            if let Some((_, line)) = self.current_match() {
                self.scroll_line = line;
            }
        }
    }

    /// Jump to previous search match.
    pub fn prev_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.search_index = if self.search_index == 0 {
                self.search_matches.len() - 1
            } else {
                self.search_index - 1
            };
            if let Some((_, line)) = self.current_match() {
                self.scroll_line = line;
            }
        }
    }

    /// Current search text.
    pub fn search_text(&self) -> &str {
        &self.search_text
    }
}

/// Layout regions for the sources panel.
#[derive(Debug, Clone)]
pub struct SourcesLayout {
    /// Tab bar for switching between sources.
    pub tab_bar: Rect,
    /// Line number gutter.
    pub gutter: Rect,
    /// Source code content area.
    pub content: Rect,
    /// Search bar (at bottom).
    pub search_bar: Rect,
}

/// Tab bar height.
const TAB_HEIGHT: f32 = 28.0;
/// Gutter width for line numbers.
const GUTTER_WIDTH: f32 = 48.0;
/// Search bar height.
const SEARCH_BAR_HEIGHT: f32 = 28.0;

/// Compute the sources panel layout.
pub fn compute_sources_layout(body: Rect) -> SourcesLayout {
    let tab_bar = Rect::new(body.origin.x, body.origin.y, body.size.width, TAB_HEIGHT);
    let search_bar = Rect::new(
        body.origin.x,
        body.origin.y + body.size.height - SEARCH_BAR_HEIGHT,
        body.size.width,
        SEARCH_BAR_HEIGHT,
    );
    let code_top = tab_bar.origin.y + TAB_HEIGHT;
    let code_height = (search_bar.origin.y - code_top).max(0.0);
    let gutter = Rect::new(body.origin.x, code_top, GUTTER_WIDTH, code_height);
    let content = Rect::new(
        body.origin.x + GUTTER_WIDTH,
        code_top,
        (body.size.width - GUTTER_WIDTH).max(0.0),
        code_height,
    );

    SourcesLayout {
        tab_bar,
        gutter,
        content,
        search_bar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SourceDocument ──

    #[test]
    fn line_count_and_access() {
        let doc = SourceDocument::new(
            SourceId(1),
            "index.html".to_owned(),
            "https://example.com/".to_owned(),
            SourceKind::Html,
            "<html>\n<body>\nHello\n</body>\n</html>".to_owned(),
        );
        assert_eq!(doc.line_count(), 5);
        assert_eq!(doc.line(1), Some("<html>"));
        assert_eq!(doc.line(3), Some("Hello"));
        assert_eq!(doc.line(5), Some("</html>"));
        assert_eq!(doc.line(0), None);
        assert_eq!(doc.line(6), None);
    }

    #[test]
    fn lines_range() {
        let doc = SourceDocument::new(
            SourceId(1),
            "test.js".to_owned(),
            "https://x.com/test.js".to_owned(),
            SourceKind::Script,
            "line1\nline2\nline3\nline4\nline5".to_owned(),
        );
        let lines = doc.lines(2, 4);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], (2, "line2"));
        assert_eq!(lines[2], (4, "line4"));
    }

    #[test]
    fn single_line_document() {
        let doc = SourceDocument::new(
            SourceId(1),
            "inline".to_owned(),
            String::new(),
            SourceKind::Script,
            "alert('hi')".to_owned(),
        );
        assert_eq!(doc.line_count(), 1);
        assert_eq!(doc.line(1), Some("alert('hi')"));
    }

    // ── Highlighting ──

    #[test]
    fn highlight_html_tag() {
        let spans = highlight_html_line("<div>");
        assert!(spans.iter().any(|s| s.kind == TokenKind::Tag));
    }

    #[test]
    fn highlight_html_attribute() {
        let spans = highlight_html_line(r#"<a href="url">"#);
        assert!(spans.iter().any(|s| s.kind == TokenKind::Attribute));
        assert!(spans.iter().any(|s| s.kind == TokenKind::StringLiteral));
    }

    #[test]
    fn highlight_html_comment() {
        let spans = highlight_html_line("<!-- comment -->");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, TokenKind::Comment);
    }

    #[test]
    fn highlight_plain_text() {
        let spans = highlight_html_line("Hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, TokenKind::Plain);
    }

    // ── SourcesState ──

    #[test]
    fn add_and_get_source() {
        let mut state = SourcesState::new();
        let id = state.add_source(
            "index.html".to_owned(),
            "https://x.com/".to_owned(),
            SourceKind::Html,
            "<html></html>".to_owned(),
        );
        assert_eq!(state.source_count(), 1);
        assert!(state.get_source(id).is_some());
        assert_eq!(state.active_tab(), Some(id));
    }

    #[test]
    fn tab_switching() {
        let mut state = SourcesState::new();
        let id1 = state.add_source(
            "page.html".to_owned(),
            "https://x.com/".to_owned(),
            SourceKind::Html,
            "<html></html>".to_owned(),
        );
        let id2 = state.add_source(
            "app.js".to_owned(),
            "https://x.com/app.js".to_owned(),
            SourceKind::Script,
            "console.log('hi')".to_owned(),
        );
        assert_eq!(state.active_tab(), Some(id1));
        state.set_active_tab(id2);
        assert_eq!(state.active_tab(), Some(id2));
    }

    #[test]
    fn clear_sources() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "content".to_owned(),
        );
        state.clear();
        assert_eq!(state.source_count(), 0);
        assert_eq!(state.active_tab(), None);
    }

    #[test]
    fn scroll_clamped() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "line1\nline2\nline3".to_owned(),
        );
        state.set_scroll_line(100);
        assert_eq!(state.scroll_line(), 3);
        state.set_scroll_line(0);
        assert_eq!(state.scroll_line(), 1);
    }

    #[test]
    fn scroll_up_down() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "a\nb\nc\nd\ne".to_owned(),
        );
        state.scroll_down(2);
        assert_eq!(state.scroll_line(), 3);
        state.scroll_up(1);
        assert_eq!(state.scroll_line(), 2);
    }

    // ── Search ──

    #[test]
    fn search_finds_matches() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "apple\nbanana\napple pie\norange".to_owned(),
        );
        state.search("apple".to_owned());
        assert_eq!(state.match_count(), 2);
    }

    #[test]
    fn search_case_insensitive() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Script,
            "Function\nfunction\nFUNCTION".to_owned(),
        );
        state.search("function".to_owned());
        assert_eq!(state.match_count(), 3);
    }

    #[test]
    fn search_navigation() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "x\nmatch\ny\nmatch\nz".to_owned(),
        );
        state.search("match".to_owned());
        assert_eq!(state.match_count(), 2);

        // First match at line 2.
        let (_, line) = state.current_match().unwrap();
        assert_eq!(line, 2);

        // Next → line 4.
        state.next_match();
        let (_, line) = state.current_match().unwrap();
        assert_eq!(line, 4);

        // Next wraps to line 2.
        state.next_match();
        let (_, line) = state.current_match().unwrap();
        assert_eq!(line, 2);

        // Prev wraps to line 4.
        state.prev_match();
        let (_, line) = state.current_match().unwrap();
        assert_eq!(line, 4);
    }

    #[test]
    fn empty_search_clears_matches() {
        let mut state = SourcesState::new();
        state.add_source(
            "test".to_owned(),
            String::new(),
            SourceKind::Html,
            "content".to_owned(),
        );
        state.search("content".to_owned());
        assert_eq!(state.match_count(), 1);
        state.search(String::new());
        assert_eq!(state.match_count(), 0);
    }

    // ── Layout ──

    #[test]
    fn sources_layout() {
        let body = Rect::new(0.0, 0.0, 800.0, 400.0);
        let layout = compute_sources_layout(body);
        assert_eq!(layout.tab_bar.size.height, TAB_HEIGHT);
        assert_eq!(layout.gutter.size.width, GUTTER_WIDTH);
        assert!(layout.content.size.width > 0.0);
        assert!(layout.content.size.height > 0.0);
    }

    #[test]
    fn source_kind_labels() {
        assert_eq!(SourceKind::Html.label(), "HTML");
        assert_eq!(SourceKind::Script.label(), "JS");
        assert_eq!(SourceKind::Stylesheet.label(), "CSS");
    }
}
