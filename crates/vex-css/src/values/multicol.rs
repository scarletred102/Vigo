// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Multi-column Layout types and computation.
//!
//! Implements the CSS Multi-column Layout Module Level 1:
//! <https://www.w3.org/TR/css-multicol-1/>

// ── Types ────────────────────────────────────────────────────────────────────

/// The `column-count` property value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColumnCount {
    /// Automatic (determined by column-width and available width).
    #[default]
    Auto,
    /// An explicit integer count.
    Integer(u32),
}

impl ColumnCount {
    pub fn from_value(s: &str) -> Self {
        let s = s.trim();
        if s.eq_ignore_ascii_case("auto") {
            return Self::Auto;
        }
        s.parse::<u32>().map_or(Self::Auto, Self::Integer)
    }
}

/// The `column-width` property value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColumnWidth {
    /// Automatic (determined by column-count and available width).
    #[default]
    Auto,
    /// An explicit pixel width.
    Px(f32),
}

impl ColumnWidth {
    pub fn from_value(s: &str) -> Self {
        let s = s.trim();
        if s.eq_ignore_ascii_case("auto") {
            return Self::Auto;
        }
        if let Some(stripped) = s.strip_suffix("px") {
            if let Ok(v) = stripped.trim().parse::<f32>() {
                return Self::Px(v);
            }
        }
        Self::Auto
    }
}

/// The `column-fill` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnFill {
    /// Balance content equally across columns.
    #[default]
    Balance,
    /// Fill columns sequentially.
    Auto,
    /// Balance, but only on the last page (fragmented contexts).
    BalanceAll,
}

impl ColumnFill {
    pub fn from_value(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "balance-all" => Self::BalanceAll,
            _ => Self::Balance,
        }
    }
}

/// The `column-span` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnSpan {
    /// Element does not span columns.
    #[default]
    None,
    /// Element spans all columns.
    All,
}

impl ColumnSpan {
    pub fn from_value(s: &str) -> Self {
        if s.trim().eq_ignore_ascii_case("all") {
            Self::All
        } else {
            Self::None
        }
    }
}

/// Resolved column rule (the line between columns).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRule {
    /// Rule width in pixels.
    pub width: f32,
    /// Rule style.
    pub style: ColumnRuleStyle,
    /// Rule color as RGBA.
    pub color: [u8; 4],
}

impl Default for ColumnRule {
    fn default() -> Self {
        Self {
            width: 0.0,
            style: ColumnRuleStyle::None,
            color: [0, 0, 0, 255],
        }
    }
}

/// Border-style values for column rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnRuleStyle {
    #[default]
    None,
    Hidden,
    Solid,
    Dotted,
    Dashed,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

impl ColumnRuleStyle {
    pub fn from_value(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "hidden" => Self::Hidden,
            "solid" => Self::Solid,
            "dotted" => Self::Dotted,
            "dashed" => Self::Dashed,
            "double" => Self::Double,
            "groove" => Self::Groove,
            "ridge" => Self::Ridge,
            "inset" => Self::Inset,
            "outset" => Self::Outset,
            _ => Self::None,
        }
    }
}

// ── Multi-column config ──────────────────────────────────────────────────────

/// All multi-column properties for a container.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiColumnConfig {
    pub column_count: ColumnCount,
    pub column_width: ColumnWidth,
    pub column_gap: f32,
    pub column_fill: ColumnFill,
    pub column_span: ColumnSpan,
    pub column_rule: ColumnRule,
}

impl Default for MultiColumnConfig {
    fn default() -> Self {
        Self {
            column_count: ColumnCount::Auto,
            column_width: ColumnWidth::Auto,
            column_gap: 16.0, // 1em typical default
            column_fill: ColumnFill::Balance,
            column_span: ColumnSpan::None,
            column_rule: ColumnRule::default(),
        }
    }
}

impl MultiColumnConfig {
    /// Whether this element establishes a multi-column formatting context.
    pub fn is_multicol(&self) -> bool {
        self.column_count != ColumnCount::Auto || self.column_width != ColumnWidth::Auto
    }
}

// ── Layout computation ───────────────────────────────────────────────────────

/// Resolved column geometry for a multi-column container.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedColumns {
    /// Number of columns.
    pub count: u32,
    /// Width of each column in pixels.
    pub width: f32,
    /// Gap between columns.
    pub gap: f32,
    /// Inline offsets (x position) for each column.
    pub offsets: Vec<f32>,
}

impl ResolvedColumns {
    /// Resolve column count and width from config and available width.
    ///
    /// Follows the algorithm in CSS Multi-column Layout Module § 3:
    /// - If both column-count and column-width are auto, yields 1 column.
    /// - If only column-count is set, width = (available - gaps) / count.
    /// - If only column-width is set, count = max(1, floor((available + gap) / (width + gap))).
    /// - If both are set, count = min(column-count, floor((available + gap) / (width + gap))).
    pub fn resolve(config: &MultiColumnConfig, available_width: f32) -> Self {
        let gap = config.column_gap;

        let (count, width) = match (config.column_count, config.column_width) {
            (ColumnCount::Auto, ColumnWidth::Auto) => {
                // Single column
                (1, available_width)
            }
            (ColumnCount::Integer(n), ColumnWidth::Auto) => {
                let n = n.max(1);
                let w = (available_width - gap * (n as f32 - 1.0)) / n as f32;
                (n, w.max(0.0))
            }
            (ColumnCount::Auto, ColumnWidth::Px(w)) => {
                let w = w.max(1.0);
                let n = ((available_width + gap) / (w + gap)).floor().max(1.0) as u32;
                let actual_w = (available_width - gap * (n as f32 - 1.0)) / n as f32;
                (n, actual_w.max(0.0))
            }
            (ColumnCount::Integer(n), ColumnWidth::Px(w)) => {
                let w = w.max(1.0);
                let n = n.max(1);
                let n_from_width = ((available_width + gap) / (w + gap)).floor().max(1.0) as u32;
                let actual_n = n.min(n_from_width);
                let actual_w = (available_width - gap * (actual_n as f32 - 1.0)) / actual_n as f32;
                (actual_n, actual_w.max(0.0))
            }
        };

        let offsets = (0..count)
            .map(|i| i as f32 * (width + gap))
            .collect();

        Self {
            count,
            width,
            gap,
            offsets,
        }
    }

    /// Total used width (all columns + gaps).
    pub fn total_width(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        self.count as f32 * self.width + (self.count as f32 - 1.0) * self.gap
    }
}

// ── Content balancing ────────────────────────────────────────────────────────

/// Distribute content heights across columns for balanced fill.
pub fn balance_columns(total_height: f32, column_count: u32) -> Vec<f32> {
    if column_count == 0 {
        return Vec::new();
    }
    let per_column = total_height / column_count as f32;
    vec![per_column; column_count as usize]
}

/// Distribute content heights sequentially (auto fill).
pub fn sequential_fill(total_height: f32, column_count: u32, max_height: f32) -> Vec<f32> {
    if column_count == 0 {
        return Vec::new();
    }
    let mut remaining = total_height;
    let mut heights = Vec::with_capacity(column_count as usize);
    for _ in 0..column_count {
        let h = remaining.min(max_height);
        heights.push(h);
        remaining -= h;
        if remaining <= 0.0 {
            remaining = 0.0;
        }
    }
    heights
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_count_parse() {
        assert_eq!(ColumnCount::from_value("auto"), ColumnCount::Auto);
        assert_eq!(ColumnCount::from_value("3"), ColumnCount::Integer(3));
        assert_eq!(ColumnCount::from_value("invalid"), ColumnCount::Auto);
    }

    #[test]
    fn column_width_parse() {
        assert_eq!(ColumnWidth::from_value("auto"), ColumnWidth::Auto);
        assert_eq!(ColumnWidth::from_value("200px"), ColumnWidth::Px(200.0));
        assert_eq!(ColumnWidth::from_value("bad"), ColumnWidth::Auto);
    }

    #[test]
    fn column_fill_parse() {
        assert_eq!(ColumnFill::from_value("balance"), ColumnFill::Balance);
        assert_eq!(ColumnFill::from_value("auto"), ColumnFill::Auto);
        assert_eq!(ColumnFill::from_value("balance-all"), ColumnFill::BalanceAll);
    }

    #[test]
    fn column_span_parse() {
        assert_eq!(ColumnSpan::from_value("all"), ColumnSpan::All);
        assert_eq!(ColumnSpan::from_value("none"), ColumnSpan::None);
    }

    #[test]
    fn column_rule_style_parse() {
        assert_eq!(ColumnRuleStyle::from_value("solid"), ColumnRuleStyle::Solid);
        assert_eq!(ColumnRuleStyle::from_value("dashed"), ColumnRuleStyle::Dashed);
        assert_eq!(ColumnRuleStyle::from_value("none"), ColumnRuleStyle::None);
    }

    #[test]
    fn is_multicol() {
        let mut config = MultiColumnConfig::default();
        assert!(!config.is_multicol());

        config.column_count = ColumnCount::Integer(3);
        assert!(config.is_multicol());
    }

    #[test]
    fn resolve_single_column() {
        let config = MultiColumnConfig::default();
        let resolved = ResolvedColumns::resolve(&config, 600.0);
        assert_eq!(resolved.count, 1);
        assert!((resolved.width - 600.0).abs() < 0.01);
    }

    #[test]
    fn resolve_count_only() {
        let config = MultiColumnConfig {
            column_count: ColumnCount::Integer(3),
            column_gap: 20.0,
            ..Default::default()
        };
        let resolved = ResolvedColumns::resolve(&config, 600.0);
        assert_eq!(resolved.count, 3);
        // 600 - 2*20 = 560, 560/3 ≈ 186.67
        assert!((resolved.width - 186.666).abs() < 1.0);
        assert_eq!(resolved.offsets.len(), 3);
    }

    #[test]
    fn resolve_width_only() {
        let config = MultiColumnConfig {
            column_width: ColumnWidth::Px(200.0),
            column_gap: 20.0,
            ..Default::default()
        };
        // (600 + 20) / (200 + 20) = 620/220 = 2.8 → floor = 2
        let resolved = ResolvedColumns::resolve(&config, 600.0);
        assert_eq!(resolved.count, 2);
        // actual width = (600 - 20) / 2 = 290
        assert!((resolved.width - 290.0).abs() < 0.01);
    }

    #[test]
    fn resolve_both() {
        let config = MultiColumnConfig {
            column_count: ColumnCount::Integer(4),
            column_width: ColumnWidth::Px(200.0),
            column_gap: 20.0,
            ..Default::default()
        };
        // from width: floor((600+20)/(200+20)) = 2
        // min(4, 2) = 2
        let resolved = ResolvedColumns::resolve(&config, 600.0);
        assert_eq!(resolved.count, 2);
    }

    #[test]
    fn total_width() {
        let config = MultiColumnConfig {
            column_count: ColumnCount::Integer(3),
            column_gap: 20.0,
            ..Default::default()
        };
        let resolved = ResolvedColumns::resolve(&config, 600.0);
        // Should be approximately 600 (3 * width + 2 * 20)
        assert!((resolved.total_width() - 600.0).abs() < 1.0);
    }

    #[test]
    fn balance_columns_even() {
        let heights = balance_columns(300.0, 3);
        assert_eq!(heights.len(), 3);
        assert!((heights[0] - 100.0).abs() < 0.01);
    }

    #[test]
    fn sequential_fill_overflow() {
        let heights = sequential_fill(500.0, 3, 200.0);
        assert_eq!(heights.len(), 3);
        assert!((heights[0] - 200.0).abs() < 0.01);
        assert!((heights[1] - 200.0).abs() < 0.01);
        assert!((heights[2] - 100.0).abs() < 0.01);
    }

    #[test]
    fn column_offsets() {
        let config = MultiColumnConfig {
            column_count: ColumnCount::Integer(3),
            column_gap: 10.0,
            ..Default::default()
        };
        let resolved = ResolvedColumns::resolve(&config, 310.0);
        // width = (310 - 20) / 3 = 96.67
        assert_eq!(resolved.offsets.len(), 3);
        assert!((resolved.offsets[0]).abs() < 0.01);
        assert!((resolved.offsets[1] - (resolved.width + 10.0)).abs() < 0.01);
    }
}
