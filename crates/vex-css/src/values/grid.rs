// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Grid Layout value types.
//!
//! Covers `grid-template-columns`, `grid-template-rows`, `grid-auto-flow`,
//! `grid-column-start/end`, `grid-row-start/end`, `gap`,
//! `justify-items`, and `align-items` (for grid context).

/// A single grid track size.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TrackSize {
    /// Fixed pixel size.
    Px(f32),
    /// Fraction of remaining space (`1fr`, `2fr`).
    Fr(f32),
    /// Auto — sized to content.
    #[default]
    Auto,
    /// `minmax(min, max)`.
    MinMax(Box<TrackSize>, Box<TrackSize>),
    /// `min-content`.
    MinContent,
    /// `max-content`.
    MaxContent,
}

/// A list of track sizes for `grid-template-columns` / `grid-template-rows`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackList(pub Vec<TrackSize>);

impl TrackList {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Parse a grid track list from a CSS value string.
    ///
    /// Supports: `px`, `fr`, `auto`, `min-content`, `max-content`, `repeat(N, size)`, `minmax(a, b)`.
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input.is_empty() || input == "none" {
            return Self(Vec::new());
        }

        let mut tracks = Vec::new();
        let mut chars = input.chars().peekable();
        let mut buf = String::new();

        while chars.peek().is_some() {
            // Skip whitespace.
            while chars.peek() == Some(&' ') {
                chars.next();
            }
            if chars.peek().is_none() {
                break;
            }

            buf.clear();
            let mut paren_depth = 0;

            // Collect a token, respecting parentheses.
            while let Some(&c) = chars.peek() {
                if c == '(' {
                    paren_depth += 1;
                    buf.push(c);
                    chars.next();
                } else if c == ')' {
                    paren_depth -= 1;
                    buf.push(c);
                    chars.next();
                    if paren_depth == 0 {
                        break;
                    }
                } else if c == ' ' && paren_depth == 0 {
                    break;
                } else {
                    buf.push(c);
                    chars.next();
                }
            }

            if buf.is_empty() {
                continue;
            }

            if let Some(stripped) = buf
                .strip_prefix("repeat(")
                .and_then(|s| s.strip_suffix(')'))
            {
                // repeat(3, 1fr) or repeat(2, 100px)
                if let Some((count_str, size_str)) = stripped.split_once(',') {
                    if let Ok(count) = count_str.trim().parse::<usize>() {
                        let size = parse_single_track(size_str.trim());
                        for _ in 0..count {
                            tracks.push(size.clone());
                        }
                    }
                }
            } else {
                tracks.push(parse_single_track(&buf));
            }
        }

        Self(tracks)
    }
}

/// Parse a single track size token.
fn parse_single_track(input: &str) -> TrackSize {
    let input = input.trim();

    if input == "auto" {
        return TrackSize::Auto;
    }
    if input == "min-content" {
        return TrackSize::MinContent;
    }
    if input == "max-content" {
        return TrackSize::MaxContent;
    }

    // minmax(a, b)
    if let Some(inner) = input
        .strip_prefix("minmax(")
        .and_then(|s| s.strip_suffix(')'))
    {
        if let Some((a, b)) = inner.split_once(',') {
            return TrackSize::MinMax(
                Box::new(parse_single_track(a.trim())),
                Box::new(parse_single_track(b.trim())),
            );
        }
    }

    // fr
    if let Some(fr_s) = input.strip_suffix("fr") {
        if let Ok(v) = fr_s.trim().parse::<f32>() {
            return TrackSize::Fr(v);
        }
    }

    // px
    if let Some(px_s) = input.strip_suffix("px") {
        if let Ok(v) = px_s.trim().parse::<f32>() {
            return TrackSize::Px(v);
        }
    }

    // Bare number → px.
    if let Ok(v) = input.parse::<f32>() {
        return TrackSize::Px(v);
    }

    TrackSize::Auto
}

/// CSS `grid-auto-flow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

impl GridAutoFlow {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim() {
            "row" => Some(Self::Row),
            "column" => Some(Self::Column),
            "row dense" | "dense row" => Some(Self::RowDense),
            "column dense" | "dense column" => Some(Self::ColumnDense),
            "dense" => Some(Self::RowDense),
            _ => None,
        }
    }

    pub fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::RowDense)
    }
}

/// Grid line placement — `grid-column-start`, `grid-row-start`, etc.
///
/// Values are 1-based line numbers. `Auto` means auto-placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GridLine {
    #[default]
    Auto,
    /// A line number (1-based, can be negative).
    Line(i32),
    /// `span N` — spans N tracks.
    Span(u32),
}

impl GridLine {
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input == "auto" || input.is_empty() {
            return Self::Auto;
        }
        if let Some(span_s) = input.strip_prefix("span") {
            let n = span_s.trim().parse::<u32>().unwrap_or(1);
            return Self::Span(n);
        }
        if let Ok(n) = input.parse::<i32>() {
            return Self::Line(n);
        }
        Self::Auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_track_list_basic() {
        let tl = TrackList::parse("100px 200px 1fr");
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.0[0], TrackSize::Px(100.0));
        assert_eq!(tl.0[1], TrackSize::Px(200.0));
        assert_eq!(tl.0[2], TrackSize::Fr(1.0));
    }

    #[test]
    fn parse_track_list_auto() {
        let tl = TrackList::parse("auto 1fr auto");
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.0[0], TrackSize::Auto);
        assert_eq!(tl.0[1], TrackSize::Fr(1.0));
        assert_eq!(tl.0[2], TrackSize::Auto);
    }

    #[test]
    fn parse_track_list_repeat() {
        let tl = TrackList::parse("repeat(3, 1fr)");
        assert_eq!(tl.len(), 3);
        assert!(tl.0.iter().all(|t| *t == TrackSize::Fr(1.0)));
    }

    #[test]
    fn parse_track_list_minmax() {
        let tl = TrackList::parse("minmax(100px, 1fr) 200px");
        assert_eq!(tl.len(), 2);
        assert_eq!(
            tl.0[0],
            TrackSize::MinMax(Box::new(TrackSize::Px(100.0)), Box::new(TrackSize::Fr(1.0)))
        );
    }

    #[test]
    fn parse_track_list_none() {
        let tl = TrackList::parse("none");
        assert!(tl.is_empty());
    }

    #[test]
    fn parse_grid_auto_flow() {
        assert_eq!(GridAutoFlow::parse("row"), Some(GridAutoFlow::Row));
        assert_eq!(GridAutoFlow::parse("column"), Some(GridAutoFlow::Column));
        assert_eq!(
            GridAutoFlow::parse("row dense"),
            Some(GridAutoFlow::RowDense)
        );
        assert!(GridAutoFlow::Row.is_row());
        assert!(!GridAutoFlow::Column.is_row());
    }

    #[test]
    fn parse_grid_line() {
        assert_eq!(GridLine::parse("auto"), GridLine::Auto);
        assert_eq!(GridLine::parse("2"), GridLine::Line(2));
        assert_eq!(GridLine::parse("-1"), GridLine::Line(-1));
        assert_eq!(GridLine::parse("span 3"), GridLine::Span(3));
    }

    #[test]
    fn track_list_min_max_content() {
        let tl = TrackList::parse("min-content max-content auto");
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.0[0], TrackSize::MinContent);
        assert_eq!(tl.0[1], TrackSize::MaxContent);
        assert_eq!(tl.0[2], TrackSize::Auto);
    }

    #[test]
    fn parse_track_list_mixed_repeat() {
        let tl = TrackList::parse("200px repeat(2, 1fr) 100px");
        assert_eq!(tl.len(), 4);
        assert_eq!(tl.0[0], TrackSize::Px(200.0));
        assert_eq!(tl.0[1], TrackSize::Fr(1.0));
        assert_eq!(tl.0[2], TrackSize::Fr(1.0));
        assert_eq!(tl.0[3], TrackSize::Px(100.0));
    }
}
