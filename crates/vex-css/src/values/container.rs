// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS Container Queries and Scroll Snap types.
//!
//! Container queries allow styles to respond to the size of a containing
//! element rather than the viewport. Scroll snap provides precise control
//! over scroll positions.

// ── Container Queries ────────────────────────────────────────────────────────

/// Container type for container queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContainerType {
    /// No containment (default).
    #[default]
    Normal,
    /// Size containment on inline axis.
    InlineSize,
    /// Size containment on both axes.
    Size,
}

impl ContainerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::InlineSize => "inline-size",
            Self::Size => "size",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "normal" => Some(Self::Normal),
            "inline-size" => Some(Self::InlineSize),
            "size" => Some(Self::Size),
            _ => None,
        }
    }
}

/// A container query condition.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerCondition {
    /// `min-width: <value>`
    MinWidth(f32),
    /// `max-width: <value>`
    MaxWidth(f32),
    /// `min-height: <value>`
    MinHeight(f32),
    /// `max-height: <value>`
    MaxHeight(f32),
    /// `width: <value>` (exact)
    Width(f32),
    /// `height: <value>` (exact)
    Height(f32),
    /// Logical AND of conditions.
    And(Vec<ContainerCondition>),
    /// Logical OR of conditions.
    Or(Vec<ContainerCondition>),
    /// Logical NOT.
    Not(Box<ContainerCondition>),
}

impl ContainerCondition {
    /// Evaluate the condition against a container's dimensions.
    pub fn evaluate(&self, width: f32, height: f32) -> bool {
        match self {
            Self::MinWidth(v) => width >= *v,
            Self::MaxWidth(v) => width <= *v,
            Self::MinHeight(v) => height >= *v,
            Self::MaxHeight(v) => height <= *v,
            Self::Width(v) => (width - *v).abs() < 0.01,
            Self::Height(v) => (height - *v).abs() < 0.01,
            Self::And(conds) => conds.iter().all(|c| c.evaluate(width, height)),
            Self::Or(conds) => conds.iter().any(|c| c.evaluate(width, height)),
            Self::Not(cond) => !cond.evaluate(width, height),
        }
    }
}

/// A `@container` rule.
#[derive(Debug, Clone)]
pub struct ContainerRule {
    /// Optional container name (matches `container-name`).
    pub name: Option<String>,
    /// The container query condition.
    pub condition: ContainerCondition,
}

impl ContainerRule {
    /// Evaluate this rule against a named container with given dimensions.
    pub fn matches(&self, container_name: Option<&str>, width: f32, height: f32) -> bool {
        // If the rule specifies a name, it must match
        if let Some(ref required_name) = self.name {
            if container_name != Some(required_name.as_str()) {
                return false;
            }
        }
        self.condition.evaluate(width, height)
    }
}

// ── Scroll Snap ──────────────────────────────────────────────────────────────

/// `scroll-snap-type` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollSnapType {
    #[default]
    None,
    /// Snap on x-axis.
    X(ScrollSnapStrictness),
    /// Snap on y-axis.
    Y(ScrollSnapStrictness),
    /// Snap on both axes.
    Both(ScrollSnapStrictness),
    /// Snap on the block axis.
    Block(ScrollSnapStrictness),
    /// Snap on the inline axis.
    Inline(ScrollSnapStrictness),
}

/// How strictly the browser should snap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollSnapStrictness {
    /// Snap if the scroll position is close enough.
    #[default]
    Proximity,
    /// Always snap after scrolling.
    Mandatory,
}

impl ScrollSnapType {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.as_slice() {
            ["none"] => Some(Self::None),
            [axis] => {
                let strictness = ScrollSnapStrictness::Proximity;
                Self::from_axis_strictness(axis, strictness)
            }
            [axis, strict] => {
                let strictness = match *strict {
                    "mandatory" => ScrollSnapStrictness::Mandatory,
                    "proximity" => ScrollSnapStrictness::Proximity,
                    _ => return None,
                };
                Self::from_axis_strictness(axis, strictness)
            }
            _ => None,
        }
    }

    fn from_axis_strictness(axis: &str, strict: ScrollSnapStrictness) -> Option<Self> {
        match axis {
            "x" => Some(Self::X(strict)),
            "y" => Some(Self::Y(strict)),
            "both" => Some(Self::Both(strict)),
            "block" => Some(Self::Block(strict)),
            "inline" => Some(Self::Inline(strict)),
            _ => None,
        }
    }
}

/// `scroll-snap-align` value for a snap child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollSnapAlign {
    #[default]
    None,
    Start,
    End,
    Center,
}

impl ScrollSnapAlign {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "none" => Some(Self::None),
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}

/// `scroll-snap-stop` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollSnapStop {
    /// The browser may skip the snap position.
    #[default]
    Normal,
    /// The browser must stop at this snap position.
    Always,
}

impl ScrollSnapStop {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "normal" => Some(Self::Normal),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

/// `scroll-behavior` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollBehavior {
    /// Instant scroll (no animation).
    #[default]
    Auto,
    /// Smooth scrolling animation.
    Smooth,
}

impl ScrollBehavior {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "auto" => Some(Self::Auto),
            "smooth" => Some(Self::Smooth),
            _ => None,
        }
    }
}

/// `overscroll-behavior` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OverscrollBehavior {
    /// Default browser behavior.
    #[default]
    Auto,
    /// Contain overscroll within the element.
    Contain,
    /// Prevent overscroll entirely.
    None,
}

impl OverscrollBehavior {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "auto" => Some(Self::Auto),
            "contain" => Some(Self::Contain),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Container Queries ───────────────────────────────────────

    #[test]
    fn container_type_parse() {
        assert_eq!(ContainerType::parse("normal"), Some(ContainerType::Normal));
        assert_eq!(
            ContainerType::parse("inline-size"),
            Some(ContainerType::InlineSize)
        );
        assert_eq!(ContainerType::parse("size"), Some(ContainerType::Size));
        assert_eq!(ContainerType::parse("invalid"), None);
    }

    #[test]
    fn container_condition_min_width() {
        let cond = ContainerCondition::MinWidth(300.0);
        assert!(cond.evaluate(400.0, 200.0));
        assert!(!cond.evaluate(200.0, 200.0));
    }

    #[test]
    fn container_condition_max_width() {
        let cond = ContainerCondition::MaxWidth(600.0);
        assert!(cond.evaluate(400.0, 200.0));
        assert!(!cond.evaluate(700.0, 200.0));
    }

    #[test]
    fn container_condition_and() {
        let cond = ContainerCondition::And(vec![
            ContainerCondition::MinWidth(300.0),
            ContainerCondition::MaxWidth(600.0),
        ]);
        assert!(cond.evaluate(400.0, 200.0));
        assert!(!cond.evaluate(200.0, 200.0));
        assert!(!cond.evaluate(700.0, 200.0));
    }

    #[test]
    fn container_condition_or() {
        let cond = ContainerCondition::Or(vec![
            ContainerCondition::MinWidth(500.0),
            ContainerCondition::MinHeight(500.0),
        ]);
        assert!(cond.evaluate(600.0, 100.0));
        assert!(cond.evaluate(100.0, 600.0));
        assert!(!cond.evaluate(100.0, 100.0));
    }

    #[test]
    fn container_condition_not() {
        let cond = ContainerCondition::Not(Box::new(ContainerCondition::MinWidth(300.0)));
        assert!(cond.evaluate(200.0, 200.0));
        assert!(!cond.evaluate(400.0, 200.0));
    }

    #[test]
    fn container_rule_with_name() {
        let rule = ContainerRule {
            name: Some("sidebar".to_string()),
            condition: ContainerCondition::MinWidth(200.0),
        };
        assert!(rule.matches(Some("sidebar"), 300.0, 100.0));
        assert!(!rule.matches(Some("main"), 300.0, 100.0));
        assert!(!rule.matches(None, 300.0, 100.0));
    }

    #[test]
    fn container_rule_without_name() {
        let rule = ContainerRule {
            name: None,
            condition: ContainerCondition::MinWidth(200.0),
        };
        assert!(rule.matches(Some("anything"), 300.0, 100.0));
        assert!(rule.matches(None, 300.0, 100.0));
    }

    // ── Scroll Snap ─────────────────────────────────────────────

    #[test]
    fn scroll_snap_type_parse() {
        assert_eq!(ScrollSnapType::parse("none"), Some(ScrollSnapType::None));
        assert_eq!(
            ScrollSnapType::parse("y mandatory"),
            Some(ScrollSnapType::Y(ScrollSnapStrictness::Mandatory))
        );
        assert_eq!(
            ScrollSnapType::parse("x"),
            Some(ScrollSnapType::X(ScrollSnapStrictness::Proximity))
        );
        assert_eq!(
            ScrollSnapType::parse("both proximity"),
            Some(ScrollSnapType::Both(ScrollSnapStrictness::Proximity))
        );
    }

    #[test]
    fn scroll_snap_align_parse() {
        assert_eq!(
            ScrollSnapAlign::parse("start"),
            Some(ScrollSnapAlign::Start)
        );
        assert_eq!(
            ScrollSnapAlign::parse("center"),
            Some(ScrollSnapAlign::Center)
        );
        assert_eq!(ScrollSnapAlign::parse("invalid"), None);
    }

    #[test]
    fn scroll_snap_stop_parse() {
        assert_eq!(
            ScrollSnapStop::parse("normal"),
            Some(ScrollSnapStop::Normal)
        );
        assert_eq!(
            ScrollSnapStop::parse("always"),
            Some(ScrollSnapStop::Always)
        );
    }

    #[test]
    fn scroll_behavior_parse() {
        assert_eq!(ScrollBehavior::parse("auto"), Some(ScrollBehavior::Auto));
        assert_eq!(
            ScrollBehavior::parse("smooth"),
            Some(ScrollBehavior::Smooth)
        );
    }

    #[test]
    fn overscroll_behavior_parse() {
        assert_eq!(
            OverscrollBehavior::parse("auto"),
            Some(OverscrollBehavior::Auto)
        );
        assert_eq!(
            OverscrollBehavior::parse("contain"),
            Some(OverscrollBehavior::Contain)
        );
        assert_eq!(
            OverscrollBehavior::parse("none"),
            Some(OverscrollBehavior::None)
        );
    }
}
