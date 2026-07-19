// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! HTML5 constraint validation API.
//!
//! Implements `ValidityState` and `checkValidity()` logic per the spec:
//! <https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#the-constraint-validation-api>

use crate::forms::{FormElementKind, InputState, InputType};

/// The validity state of a form control, matching the Web `ValidityState` interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ValidityState {
    /// The element's value is missing when required.
    pub value_missing: bool,
    /// The value doesn't match the `type` (e.g., not a valid email).
    pub type_mismatch: bool,
    /// The value doesn't match the `pattern` attribute regex.
    pub pattern_mismatch: bool,
    /// The value is shorter than `minlength`.
    pub too_short: bool,
    /// The value is longer than `maxlength`.
    pub too_long: bool,
    /// For numeric inputs: below `min`.
    pub range_underflow: bool,
    /// For numeric inputs: above `max`.
    pub range_overflow: bool,
    /// For numeric inputs: doesn't match `step`.
    pub step_mismatch: bool,
    /// Custom validity message set by `setCustomValidity()`.
    pub custom_error: bool,
}

impl ValidityState {
    /// Whether the element satisfies all constraints (the overall `valid` flag).
    pub fn valid(&self) -> bool {
        !self.value_missing
            && !self.type_mismatch
            && !self.pattern_mismatch
            && !self.too_short
            && !self.too_long
            && !self.range_underflow
            && !self.range_overflow
            && !self.step_mismatch
            && !self.custom_error
    }
}

/// Constraints from HTML attributes on a form control.
#[derive(Debug, Clone, Default)]
pub struct ValidationConstraints {
    /// `required` attribute present.
    pub required: bool,
    /// `pattern` attribute (regex string).
    pub pattern: Option<String>,
    /// `minlength` attribute.
    pub minlength: Option<usize>,
    /// `maxlength` attribute.
    pub maxlength: Option<usize>,
    /// `min` attribute (for number inputs).
    pub min: Option<f64>,
    /// `max` attribute (for number inputs).
    pub max: Option<f64>,
    /// `step` attribute (for number inputs, defaults to 1).
    pub step: Option<f64>,
    /// Custom validity message (from `setCustomValidity()`).
    pub custom_message: Option<String>,
}

/// Check validity of a form element, returning the `ValidityState`.
pub fn check_validity(state: &InputState, constraints: &ValidationConstraints) -> ValidityState {
    let mut validity = ValidityState::default();

    // Custom validity always wins.
    if let Some(ref msg) = constraints.custom_message {
        if !msg.is_empty() {
            validity.custom_error = true;
            return validity;
        }
    }

    // Non-submittable/non-validatable types skip validation.
    let input_type = match state.kind {
        FormElementKind::Input(t) => t,
        FormElementKind::Textarea => InputType::Text, // Textarea validates like text.
        FormElementKind::Select | FormElementKind::Button => {
            // Select: only required check.
            if constraints.required && state.value.is_empty() {
                validity.value_missing = true;
            }
            return validity;
        }
    };

    // Hidden, submit, reset, button don't participate in validation.
    if matches!(
        input_type,
        InputType::Hidden | InputType::Submit | InputType::Reset | InputType::Button
    ) {
        return validity;
    }

    let value = &state.value;

    // Required check.
    if constraints.required {
        let is_empty = match input_type {
            InputType::Checkbox => !state.checked,
            InputType::Radio => !state.checked,
            _ => value.is_empty(),
        };
        if is_empty {
            validity.value_missing = true;
        }
    }

    // Skip further checks for unchecked checkboxes/radios.
    if input_type.is_checkable() {
        return validity;
    }

    // Type mismatch.
    if !value.is_empty() {
        match input_type {
            InputType::Email => {
                if !is_valid_email(value) {
                    validity.type_mismatch = true;
                }
            }
            InputType::Url if !is_valid_url(value) => validity.type_mismatch = true,
            _ => {}
        }
    }

    // Pattern mismatch.
    if !value.is_empty() {
        if let Some(ref pattern) = constraints.pattern {
            if !matches_pattern(value, pattern) {
                validity.pattern_mismatch = true;
            }
        }
    }

    // Length constraints (for text-like inputs).
    if input_type.is_text_like() && !value.is_empty() {
        let char_count = value.chars().count();
        if let Some(min_len) = constraints.minlength {
            if char_count < min_len {
                validity.too_short = true;
            }
        }
        if let Some(max_len) = constraints.maxlength {
            if char_count > max_len {
                validity.too_long = true;
            }
        }
    }

    // Range constraints (for number inputs).
    if input_type == InputType::Number && !value.is_empty() {
        if let Ok(num) = value.parse::<f64>() {
            if let Some(min) = constraints.min {
                if num < min {
                    validity.range_underflow = true;
                }
            }
            if let Some(max) = constraints.max {
                if num > max {
                    validity.range_overflow = true;
                }
            }
            if let Some(step) = constraints.step {
                let base = constraints.min.unwrap_or(0.0);
                let diff = num - base;
                if step > 0.0 && (diff % step).abs() > 1e-10 {
                    validity.step_mismatch = true;
                }
            }
        } else {
            // Non-parseable number → type mismatch.
            validity.type_mismatch = true;
        }
    }

    validity
}

/// Build a human-readable validation message for the first error.
pub fn validation_message(validity: &ValidityState) -> String {
    if validity.value_missing {
        return "Please fill out this field.".to_string();
    }
    if validity.type_mismatch {
        return "Please enter a valid value.".to_string();
    }
    if validity.pattern_mismatch {
        return "Please match the requested format.".to_string();
    }
    if validity.too_short {
        return "Please use at least the minimum number of characters.".to_string();
    }
    if validity.too_long {
        return "Please shorten this text.".to_string();
    }
    if validity.range_underflow {
        return "Value must be greater than or equal to the minimum.".to_string();
    }
    if validity.range_overflow {
        return "Value must be less than or equal to the maximum.".to_string();
    }
    if validity.step_mismatch {
        return "Please enter a valid value.".to_string();
    }
    if validity.custom_error {
        return "Invalid value.".to_string();
    }
    String::new()
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Basic email validation (RFC 5321 simplified).
fn is_valid_email(value: &str) -> bool {
    let parts: Vec<&str> = value.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

/// Basic URL validation.
fn is_valid_url(value: &str) -> bool {
    value.contains("://") && value.len() > 8
}

/// Check if value matches a `pattern` attribute regex (anchored).
fn matches_pattern(value: &str, pattern: &str) -> bool {
    // The HTML spec says patterns are anchored to ^...$​.
    // We do a simple prefix/suffix match for common patterns,
    // and fall back to full string equality for complex ones.
    // A real implementation would use a regex engine.
    if pattern.is_empty() {
        return true;
    }

    // Simple literal pattern.
    if !pattern.contains(|c: char| ".*+?[](){}^$|\\".contains(c)) {
        return value == pattern;
    }

    // Dot-star matches anything.
    if pattern == ".*" {
        return true;
    }

    // Simple [0-9]+ pattern for numeric fields.
    if pattern == "[0-9]+" || pattern == "\\d+" {
        return !value.is_empty() && value.chars().all(|c| c.is_ascii_digit());
    }

    // Simple [a-zA-Z]+ pattern.
    if pattern == "[a-zA-Z]+" {
        return !value.is_empty() && value.chars().all(|c| c.is_ascii_alphabetic());
    }

    // For other patterns, accept the value (permissive fallback).
    true
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn text_state(value: &str) -> InputState {
        let mut s = InputState::new_text(InputType::Text);
        s.value = value.to_string();
        s
    }

    fn email_state(value: &str) -> InputState {
        let mut s = InputState::new_text(InputType::Email);
        s.value = value.to_string();
        s
    }

    fn number_state(value: &str) -> InputState {
        let mut s = InputState::new_text(InputType::Number);
        s.value = value.to_string();
        s
    }

    #[test]
    fn valid_with_no_constraints() {
        let state = text_state("hello");
        let constraints = ValidationConstraints::default();
        let v = check_validity(&state, &constraints);
        assert!(v.valid());
    }

    #[test]
    fn required_empty_text() {
        let state = text_state("");
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.value_missing);
        assert!(!v.valid());
    }

    #[test]
    fn required_filled_text() {
        let state = text_state("hello");
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(!v.value_missing);
        assert!(v.valid());
    }

    #[test]
    fn required_checkbox_unchecked() {
        let state = InputState::new_checkbox(false);
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.value_missing);
    }

    #[test]
    fn required_checkbox_checked() {
        let state = InputState::new_checkbox(true);
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(!v.value_missing);
        assert!(v.valid());
    }

    #[test]
    fn email_type_mismatch() {
        let state = email_state("notanemail");
        let constraints = ValidationConstraints::default();
        let v = check_validity(&state, &constraints);
        assert!(v.type_mismatch);
        assert!(!v.valid());
    }

    #[test]
    fn email_valid() {
        let state = email_state("user@example.com");
        let constraints = ValidationConstraints::default();
        let v = check_validity(&state, &constraints);
        assert!(!v.type_mismatch);
        assert!(v.valid());
    }

    #[test]
    fn minlength_too_short() {
        let state = text_state("ab");
        let constraints = ValidationConstraints {
            minlength: Some(5),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.too_short);
        assert!(!v.valid());
    }

    #[test]
    fn maxlength_too_long() {
        let state = text_state("a very long string indeed");
        let constraints = ValidationConstraints {
            maxlength: Some(5),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.too_long);
        assert!(!v.valid());
    }

    #[test]
    fn number_range_underflow() {
        let state = number_state("3");
        let constraints = ValidationConstraints {
            min: Some(5.0),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.range_underflow);
        assert!(!v.valid());
    }

    #[test]
    fn number_range_overflow() {
        let state = number_state("15");
        let constraints = ValidationConstraints {
            max: Some(10.0),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.range_overflow);
    }

    #[test]
    fn number_step_mismatch() {
        let state = number_state("7");
        let constraints = ValidationConstraints {
            min: Some(0.0),
            step: Some(5.0),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.step_mismatch);
    }

    #[test]
    fn number_step_valid() {
        let state = number_state("10");
        let constraints = ValidationConstraints {
            min: Some(0.0),
            step: Some(5.0),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(!v.step_mismatch);
        assert!(v.valid());
    }

    #[test]
    fn pattern_mismatch_digits_only() {
        let state = text_state("abc");
        let constraints = ValidationConstraints {
            pattern: Some("[0-9]+".to_string()),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.pattern_mismatch);
    }

    #[test]
    fn pattern_match_digits_only() {
        let state = text_state("123");
        let constraints = ValidationConstraints {
            pattern: Some("[0-9]+".to_string()),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(!v.pattern_mismatch);
        assert!(v.valid());
    }

    #[test]
    fn custom_validity_overrides() {
        let state = text_state("valid data");
        let constraints = ValidationConstraints {
            custom_message: Some("Nope".to_string()),
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.custom_error);
        assert!(!v.valid());
    }

    #[test]
    fn validation_message_for_required() {
        let state = text_state("");
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        let msg = validation_message(&v);
        assert!(msg.contains("fill out"));
    }

    #[test]
    fn hidden_input_skips_validation() {
        let mut state = InputState::new_text(InputType::Hidden);
        state.value = String::new();
        let constraints = ValidationConstraints {
            required: true,
            ..Default::default()
        };
        let v = check_validity(&state, &constraints);
        assert!(v.valid()); // Hidden inputs skip validation.
    }

    #[test]
    fn validity_state_default_is_valid() {
        let v = ValidityState::default();
        assert!(v.valid());
    }
}
