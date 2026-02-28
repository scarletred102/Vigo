// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! CSS media query evaluation.

use vex_core::Size;

/// A media query condition.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaCondition {
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    Screen,
    Print,
    All,
    And(Vec<MediaCondition>),
    Or(Vec<MediaCondition>),
    Not(Box<MediaCondition>),
}

/// Evaluate a media condition against the current viewport.
/// We always treat ourselves as `screen` type.
pub fn evaluate_media(condition: &MediaCondition, viewport: Size) -> bool {
    match condition {
        MediaCondition::MinWidth(w) => viewport.width >= *w,
        MediaCondition::MaxWidth(w) => viewport.width <= *w,
        MediaCondition::MinHeight(h) => viewport.height >= *h,
        MediaCondition::MaxHeight(h) => viewport.height <= *h,
        MediaCondition::Screen => true,
        MediaCondition::Print => false,
        MediaCondition::All => true,
        MediaCondition::And(conditions) => conditions.iter().all(|c| evaluate_media(c, viewport)),
        MediaCondition::Or(conditions) => conditions.iter().any(|c| evaluate_media(c, viewport)),
        MediaCondition::Not(inner) => !evaluate_media(inner, viewport),
    }
}

/// Parse a simple media condition string.
/// Handles: `(min-width: Npx)`, `(max-width: Npx)`, `screen`, `print`, `all`,
/// `not (...)`, and `and` combinations.
pub fn parse_media_condition(input: &str) -> Option<MediaCondition> {
    let input = input.trim();

    if input.is_empty() || input == "all" {
        return Some(MediaCondition::All);
    }

    if input == "screen" {
        return Some(MediaCondition::Screen);
    }

    if input == "print" {
        return Some(MediaCondition::Print);
    }

    // Handle "not (...)"
    if let Some(inner) = input.strip_prefix("not ") {
        return parse_media_condition(inner.trim()).map(|c| MediaCondition::Not(Box::new(c)));
    }

    // Handle "and" combinations: split on " and "
    if input.contains(" and ") {
        let parts: Vec<&str> = input.split(" and ").collect();
        let conditions: Vec<MediaCondition> = parts
            .iter()
            .filter_map(|p| parse_media_condition(p.trim()))
            .collect();
        if conditions.len() > 1 {
            return Some(MediaCondition::And(conditions));
        }
    }

    // Parse single parenthesized condition: (min-width: 768px)
    let inner = input.trim_start_matches('(').trim_end_matches(')').trim();
    if let Some((name, value)) = inner.split_once(':') {
        let name = name.trim();
        let value = value.trim();
        let px = value
            .strip_suffix("px")
            .and_then(|v| v.trim().parse::<f32>().ok())?;

        return match name {
            "min-width" => Some(MediaCondition::MinWidth(px)),
            "max-width" => Some(MediaCondition::MaxWidth(px)),
            "min-height" => Some(MediaCondition::MinHeight(px)),
            "max-height" => Some(MediaCondition::MaxHeight(px)),
            _ => None,
        };
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const VP: Size = Size { width: 1024.0, height: 768.0 };

    #[test]
    fn min_width() {
        let c = MediaCondition::MinWidth(768.0);
        assert!(evaluate_media(&c, VP));
        assert!(!evaluate_media(&c, Size::new(600.0, 400.0)));
    }

    #[test]
    fn max_width() {
        let c = MediaCondition::MaxWidth(1280.0);
        assert!(evaluate_media(&c, VP));
        assert!(!evaluate_media(&c, Size::new(1920.0, 1080.0)));
    }

    #[test]
    fn screen_type() {
        assert!(evaluate_media(&MediaCondition::Screen, VP));
        assert!(!evaluate_media(&MediaCondition::Print, VP));
    }

    #[test]
    fn and_combo() {
        let c = MediaCondition::And(vec![
            MediaCondition::MinWidth(600.0),
            MediaCondition::MaxWidth(1280.0),
        ]);
        assert!(evaluate_media(&c, VP));
        assert!(!evaluate_media(&c, Size::new(400.0, 300.0)));
    }

    #[test]
    fn not_condition() {
        let c = MediaCondition::Not(Box::new(MediaCondition::Print));
        assert!(evaluate_media(&c, VP));
    }

    #[test]
    fn parse_min_width() {
        let c = parse_media_condition("(min-width: 768px)").unwrap();
        assert_eq!(c, MediaCondition::MinWidth(768.0));
    }

    #[test]
    fn parse_and() {
        let c = parse_media_condition("(min-width: 600px) and (max-width: 1200px)").unwrap();
        assert!(evaluate_media(&c, VP));
    }
}
