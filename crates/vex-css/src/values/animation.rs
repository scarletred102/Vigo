// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! CSS animation and transition value types.
//!
//! Implements:
//! - `TimingFunction` — easing curves (ease, linear, cubic-bezier, steps).
//! - `AnimationDirection` — normal, reverse, alternate, alternate-reverse.
//! - `AnimationFillMode` — none, forwards, backwards, both.
//! - `AnimationPlayState` — running, paused.
//! - `Keyframe` / `KeyframeRule` — @keyframes data.
//! - `TransitionProperty` — which CSS properties to transition.

use crate::properties::Declaration;

// ── TimingFunction ───────────────────────────────────────────────────────────

/// CSS easing function.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TimingFunction {
    /// Linear interpolation.
    Linear,
    /// Ease (cubic-bezier 0.25, 0.1, 0.25, 1.0).
    #[default]
    Ease,
    /// Ease-in (0.42, 0, 1, 1).
    EaseIn,
    /// Ease-out (0, 0, 0.58, 1).
    EaseOut,
    /// Ease-in-out (0.42, 0, 0.58, 1).
    EaseInOut,
    /// Arbitrary cubic-bezier (x1, y1, x2, y2).
    CubicBezier(f32, f32, f32, f32),
    /// Steps(count, position).
    Steps(u32, StepPosition),
}

impl TimingFunction {
    /// Evaluate the easing at progress `t` ∈ [0, 1], returning the output value.
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0, t),
            Self::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0, t),
            Self::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0, t),
            Self::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0, t),
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier(*x1, *y1, *x2, *y2, t),
            Self::Steps(count, position) => step_function(*count, *position, t),
        }
    }

    /// Parse a timing function from CSS value.
    pub fn parse(value: &str) -> Option<Self> {
        let v = value.trim().to_ascii_lowercase();
        match v.as_str() {
            "linear" => Some(Self::Linear),
            "ease" => Some(Self::Ease),
            "ease-in" => Some(Self::EaseIn),
            "ease-out" => Some(Self::EaseOut),
            "ease-in-out" => Some(Self::EaseInOut),
            "step-start" => Some(Self::Steps(1, StepPosition::Start)),
            "step-end" => Some(Self::Steps(1, StepPosition::End)),
            _ if v.starts_with("cubic-bezier(") => parse_cubic_bezier(&v),
            _ if v.starts_with("steps(") => parse_steps(&v),
            _ => None,
        }
    }
}

/// Position for `steps()` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepPosition {
    /// Jump at the start of each interval.
    Start,
    /// Jump at the end of each interval (default).
    #[default]
    End,
    /// No jump at start or end.
    JumpNone,
    /// Jump at both start and end.
    JumpBoth,
}

// ── AnimationDirection ───────────────────────────────────────────────────────

/// CSS `animation-direction` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationDirection {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl AnimationDirection {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "reverse" => Some(Self::Reverse),
            "alternate" => Some(Self::Alternate),
            "alternate-reverse" => Some(Self::AlternateReverse),
            _ => None,
        }
    }
}

// ── AnimationFillMode ────────────────────────────────────────────────────────

/// CSS `animation-fill-mode` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationFillMode {
    #[default]
    None,
    Forwards,
    Backwards,
    Both,
}

impl AnimationFillMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "forwards" => Some(Self::Forwards),
            "backwards" => Some(Self::Backwards),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

// ── AnimationPlayState ───────────────────────────────────────────────────────

/// CSS `animation-play-state` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationPlayState {
    #[default]
    Running,
    Paused,
}

impl AnimationPlayState {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }
}

// ── AnimationIterationCount ──────────────────────────────────────────────────

/// CSS `animation-iteration-count`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationIterationCount {
    /// Finite number of iterations.
    Number(f32),
    /// Infinite iterations.
    Infinite,
}

impl Default for AnimationIterationCount {
    fn default() -> Self {
        Self::Number(1.0)
    }
}

impl AnimationIterationCount {
    pub fn parse(value: &str) -> Option<Self> {
        let v = value.trim().to_ascii_lowercase();
        if v == "infinite" {
            Some(Self::Infinite)
        } else {
            v.parse::<f32>().ok().map(Self::Number)
        }
    }
}

// ── Keyframes ────────────────────────────────────────────────────────────────

/// A single keyframe (percentage → declarations).
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    /// Percentage in [0.0, 1.0] (0% = 0.0, 100% = 1.0).
    pub offset: f32,
    /// The declarations at this keyframe.
    pub declarations: Vec<Declaration>,
}

/// A `@keyframes` rule with a name and a list of keyframes.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeRule {
    pub name: String,
    pub keyframes: Vec<Keyframe>,
}

impl KeyframeRule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            keyframes: Vec::new(),
        }
    }

    /// Add a keyframe at the given offset.
    pub fn add_keyframe(&mut self, offset: f32, declarations: Vec<Declaration>) {
        self.keyframes.push(Keyframe {
            offset: offset.clamp(0.0, 1.0),
            declarations,
        });
        self.keyframes.sort_by(|a, b| {
            a.offset
                .partial_cmp(&b.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Get the two keyframes surrounding a given progress value.
    /// Returns (before, after, local_progress).
    pub fn interpolation_pair(&self, progress: f32) -> Option<(&Keyframe, &Keyframe, f32)> {
        if self.keyframes.is_empty() {
            return None;
        }
        let progress = progress.clamp(0.0, 1.0);

        // Find the pair
        for i in 0..self.keyframes.len() - 1 {
            let a = &self.keyframes[i];
            let b = &self.keyframes[i + 1];
            if progress >= a.offset && progress <= b.offset {
                let range = b.offset - a.offset;
                let local = if range > 0.0 {
                    (progress - a.offset) / range
                } else {
                    0.0
                };
                return Some((a, b, local));
            }
        }

        // Past the end — return the last keyframe
        let last = self.keyframes.last()?;
        Some((last, last, 1.0))
    }
}

// ── TransitionProperty ───────────────────────────────────────────────────────

/// Which CSS property (or all) to transition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TransitionProperty {
    /// Transition all transitionable properties.
    #[default]
    All,
    /// Transition no properties.
    None,
    /// Transition a specific named property.
    Property(String),
}

impl TransitionProperty {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "all" => Self::All,
            "none" => Self::None,
            other => Self::Property(other.to_string()),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Approximate cubic-bezier using De Casteljau subdivision (Newton's method).
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    // We need to find the `t_bezier` parameter such that bezier_x(t_bezier) = t,
    // then return bezier_y(t_bezier).
    // Use Newton-Raphson iteration.
    let mut guess = t;
    for _ in 0..8 {
        let x = bezier_component(x1, x2, guess) - t;
        let dx = bezier_derivative(x1, x2, guess);
        if dx.abs() < 1e-7 {
            break;
        }
        guess -= x / dx;
        guess = guess.clamp(0.0, 1.0);
    }
    bezier_component(y1, y2, guess)
}

/// A single component of a cubic bezier: B(t) = 3(1-t)²·t·p1 + 3(1-t)·t²·p2 + t³
fn bezier_component(p1: f32, p2: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

/// Derivative of the bezier component.
fn bezier_derivative(p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

/// Step easing function.
fn step_function(count: u32, position: StepPosition, t: f32) -> f32 {
    if count == 0 {
        return t;
    }
    let n = count as f32;
    match position {
        StepPosition::Start => ((t * n).ceil() / n).min(1.0),
        StepPosition::End => ((t * n).floor() / n).min(1.0),
        StepPosition::JumpNone => {
            if n <= 1.0 {
                return t;
            }
            ((t * (n - 1.0)).floor() / (n - 1.0)).min(1.0)
        }
        StepPosition::JumpBoth => ((t * (n + 1.0)).ceil() / (n + 1.0)).min(1.0),
    }
}

/// Parse `cubic-bezier(x1, y1, x2, y2)`.
fn parse_cubic_bezier(s: &str) -> Option<TimingFunction> {
    let inner = s.strip_prefix("cubic-bezier(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let x1: f32 = parts[0].trim().parse().ok()?;
    let y1: f32 = parts[1].trim().parse().ok()?;
    let x2: f32 = parts[2].trim().parse().ok()?;
    let y2: f32 = parts[3].trim().parse().ok()?;
    Some(TimingFunction::CubicBezier(x1, y1, x2, y2))
}

/// Parse `steps(count, position)`.
fn parse_steps(s: &str) -> Option<TimingFunction> {
    let inner = s.strip_prefix("steps(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').collect();
    let count: u32 = parts[0].trim().parse().ok()?;
    let position = if parts.len() > 1 {
        match parts[1].trim() {
            "start" | "jump-start" => StepPosition::Start,
            "end" | "jump-end" => StepPosition::End,
            "jump-none" => StepPosition::JumpNone,
            "jump-both" => StepPosition::JumpBoth,
            _ => StepPosition::End,
        }
    } else {
        StepPosition::End
    };
    Some(TimingFunction::Steps(count, position))
}

/// Parse a CSS time value (e.g., "0.3s", "300ms", "1s").
/// Returns the duration in seconds.
pub fn parse_time(value: &str) -> Option<f32> {
    let v = value.trim().to_ascii_lowercase();
    if let Some(ms) = v.strip_suffix("ms") {
        ms.trim().parse::<f32>().ok().map(|m| m / 1000.0)
    } else if let Some(s) = v.strip_suffix('s') {
        s.trim().parse::<f32>().ok()
    } else {
        None
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TimingFunction ──────────────────────────────────────────

    #[test]
    fn timing_linear() {
        let f = TimingFunction::Linear;
        assert!((f.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((f.evaluate(0.5) - 0.5).abs() < 0.001);
        assert!((f.evaluate(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn timing_ease_endpoints() {
        let f = TimingFunction::Ease;
        assert!((f.evaluate(0.0) - 0.0).abs() < 0.01);
        assert!((f.evaluate(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn timing_ease_in() {
        let f = TimingFunction::EaseIn;
        // Ease-in should be slow at start
        assert!(f.evaluate(0.5) < 0.5);
    }

    #[test]
    fn timing_ease_out() {
        let f = TimingFunction::EaseOut;
        // Ease-out should be fast at start
        assert!(f.evaluate(0.5) > 0.5);
    }

    #[test]
    fn timing_cubic_bezier_parse() {
        let f = TimingFunction::parse("cubic-bezier(0.25, 0.1, 0.25, 1.0)");
        assert!(matches!(f, Some(TimingFunction::CubicBezier(_, _, _, _))));
    }

    #[test]
    fn timing_steps_parse() {
        let f = TimingFunction::parse("steps(4, start)");
        assert_eq!(f, Some(TimingFunction::Steps(4, StepPosition::Start)));
    }

    #[test]
    fn timing_steps_end() {
        let f = TimingFunction::Steps(4, StepPosition::End);
        assert!((f.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((f.evaluate(0.24) - 0.0).abs() < 0.001);
        assert!((f.evaluate(0.25) - 0.25).abs() < 0.001);
    }

    #[test]
    fn timing_step_start_shorthand() {
        assert_eq!(
            TimingFunction::parse("step-start"),
            Some(TimingFunction::Steps(1, StepPosition::Start))
        );
    }

    #[test]
    fn timing_parse_keywords() {
        assert_eq!(
            TimingFunction::parse("linear"),
            Some(TimingFunction::Linear)
        );
        assert_eq!(TimingFunction::parse("ease"), Some(TimingFunction::Ease));
        assert_eq!(
            TimingFunction::parse("ease-in-out"),
            Some(TimingFunction::EaseInOut)
        );
    }

    #[test]
    fn timing_clamp() {
        let f = TimingFunction::Linear;
        assert!((f.evaluate(-0.5) - 0.0).abs() < 0.001);
        assert!((f.evaluate(1.5) - 1.0).abs() < 0.001);
    }

    // ── AnimationDirection ──────────────────────────────────────

    #[test]
    fn direction_parse() {
        assert_eq!(
            AnimationDirection::parse("alternate-reverse"),
            Some(AnimationDirection::AlternateReverse)
        );
        assert_eq!(
            AnimationDirection::parse("normal"),
            Some(AnimationDirection::Normal)
        );
    }

    // ── AnimationFillMode ───────────────────────────────────────

    #[test]
    fn fill_mode_parse() {
        assert_eq!(
            AnimationFillMode::parse("both"),
            Some(AnimationFillMode::Both)
        );
        assert_eq!(
            AnimationFillMode::parse("none"),
            Some(AnimationFillMode::None)
        );
    }

    // ── AnimationPlayState ──────────────────────────────────────

    #[test]
    fn play_state_parse() {
        assert_eq!(
            AnimationPlayState::parse("paused"),
            Some(AnimationPlayState::Paused)
        );
        assert_eq!(
            AnimationPlayState::parse("running"),
            Some(AnimationPlayState::Running)
        );
    }

    // ── AnimationIterationCount ─────────────────────────────────

    #[test]
    fn iteration_count_parse() {
        assert_eq!(
            AnimationIterationCount::parse("infinite"),
            Some(AnimationIterationCount::Infinite)
        );
        assert_eq!(
            AnimationIterationCount::parse("3"),
            Some(AnimationIterationCount::Number(3.0))
        );
        assert_eq!(
            AnimationIterationCount::parse("2.5"),
            Some(AnimationIterationCount::Number(2.5))
        );
    }

    // ── Keyframes ───────────────────────────────────────────────

    #[test]
    fn keyframe_rule_build() {
        let mut rule = KeyframeRule::new("fadeIn");
        rule.add_keyframe(0.0, vec![]);
        rule.add_keyframe(1.0, vec![]);
        assert_eq!(rule.name, "fadeIn");
        assert_eq!(rule.keyframes.len(), 2);
    }

    #[test]
    fn keyframe_sorted_order() {
        let mut rule = KeyframeRule::new("test");
        rule.add_keyframe(1.0, vec![]);
        rule.add_keyframe(0.0, vec![]);
        rule.add_keyframe(0.5, vec![]);
        assert!((rule.keyframes[0].offset - 0.0).abs() < 0.001);
        assert!((rule.keyframes[1].offset - 0.5).abs() < 0.001);
        assert!((rule.keyframes[2].offset - 1.0).abs() < 0.001);
    }

    #[test]
    fn keyframe_interpolation_pair() {
        let mut rule = KeyframeRule::new("slide");
        rule.add_keyframe(0.0, vec![]);
        rule.add_keyframe(0.5, vec![]);
        rule.add_keyframe(1.0, vec![]);

        let (a, b, local) = rule.interpolation_pair(0.25).unwrap();
        assert!((a.offset - 0.0).abs() < 0.001);
        assert!((b.offset - 0.5).abs() < 0.001);
        assert!((local - 0.5).abs() < 0.001);

        let (a2, b2, local2) = rule.interpolation_pair(0.75).unwrap();
        assert!((a2.offset - 0.5).abs() < 0.001);
        assert!((b2.offset - 1.0).abs() < 0.001);
        assert!((local2 - 0.5).abs() < 0.001);
    }

    #[test]
    fn keyframe_interpolation_at_boundary() {
        let mut rule = KeyframeRule::new("test");
        rule.add_keyframe(0.0, vec![]);
        rule.add_keyframe(1.0, vec![]);

        let (_, _, local) = rule.interpolation_pair(0.0).unwrap();
        assert!((local - 0.0).abs() < 0.001);

        let (_, _, local) = rule.interpolation_pair(1.0).unwrap();
        assert!((local - 1.0).abs() < 0.001);
    }

    // ── TransitionProperty ──────────────────────────────────────

    #[test]
    fn transition_property_parse() {
        assert_eq!(TransitionProperty::parse("all"), TransitionProperty::All);
        assert_eq!(TransitionProperty::parse("none"), TransitionProperty::None);
        assert_eq!(
            TransitionProperty::parse("opacity"),
            TransitionProperty::Property("opacity".to_string())
        );
    }

    // ── parse_time ──────────────────────────────────────────────

    #[test]
    fn parse_time_seconds() {
        assert_eq!(parse_time("1s"), Some(1.0));
        assert_eq!(parse_time("0.5s"), Some(0.5));
        assert_eq!(parse_time("2s"), Some(2.0));
    }

    #[test]
    fn parse_time_milliseconds() {
        assert_eq!(parse_time("300ms"), Some(0.3));
        assert_eq!(parse_time("1000ms"), Some(1.0));
    }

    #[test]
    fn parse_time_invalid() {
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time(""), None);
    }
}
