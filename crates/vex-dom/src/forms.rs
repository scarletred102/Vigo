// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Form element state — `<input>`, `<textarea>`, `<select>`.
//!
//! Stores mutable state that isn't in the DOM attribute model: the current
//! value, cursor position, checked state, and selection range.

use std::collections::HashMap;
use std::fmt;

use vex_core::VexId;

/// The `type` attribute of an `<input>` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputType {
    /// `<input type="text">` (default).
    #[default]
    Text,
    /// `<input type="password">`.
    Password,
    /// `<input type="checkbox">`.
    Checkbox,
    /// `<input type="radio">`.
    Radio,
    /// `<input type="submit">`.
    Submit,
    /// `<input type="reset">`.
    Reset,
    /// `<input type="hidden">`.
    Hidden,
    /// `<input type="button">`.
    Button,
    /// `<input type="number">`.
    Number,
    /// `<input type="email">`.
    Email,
    /// `<input type="url">`.
    Url,
    /// `<input type="search">`.
    Search,
    /// `<input type="tel">`.
    Tel,
}

impl InputType {
    /// Parse from an HTML `type` attribute value.
    pub fn from_attr(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "text" => Self::Text,
            "password" => Self::Password,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "submit" => Self::Submit,
            "reset" => Self::Reset,
            "hidden" => Self::Hidden,
            "button" => Self::Button,
            "number" => Self::Number,
            "email" => Self::Email,
            "url" => Self::Url,
            "search" => Self::Search,
            "tel" => Self::Tel,
            _ => Self::Text, // Unknown types default to text.
        }
    }

    /// Whether this input type is a text-entry field.
    pub fn is_text_like(&self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::Password
                | Self::Number
                | Self::Email
                | Self::Url
                | Self::Search
                | Self::Tel
        )
    }

    /// Whether this input type is a toggle (checkbox / radio).
    pub fn is_checkable(&self) -> bool {
        matches!(self, Self::Checkbox | Self::Radio)
    }
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Submit => "submit",
            Self::Reset => "reset",
            Self::Hidden => "hidden",
            Self::Button => "button",
            Self::Number => "number",
            Self::Email => "email",
            Self::Url => "url",
            Self::Search => "search",
            Self::Tel => "tel",
        };
        write!(f, "{s}")
    }
}

/// Kind of form element (input / textarea / select / button).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormElementKind {
    /// `<input>` with the given type.
    Input(InputType),
    /// `<textarea>`.
    Textarea,
    /// `<select>`.
    Select,
    /// `<button>`.
    Button,
}

/// Mutable state for a single form element.
///
/// This exists separately from the DOM attribute model because these values
/// change with user interaction (typing, checking, selecting) and should not
/// round-trip through HTML attributes.
#[derive(Debug, Clone)]
pub struct InputState {
    /// The current input value (what the user sees/typed).
    pub value: String,
    /// Cursor position (byte offset into `value`). For checkbox/radio, unused.
    pub selection_start: usize,
    /// End of selection range (byte offset). Equal to `selection_start` when
    /// no text is selected.
    pub selection_end: usize,
    /// Whether this element is checked (for checkbox/radio).
    pub checked: bool,
    /// Whether this element is disabled.
    pub disabled: bool,
    /// The `name` attribute for form submission.
    pub name: String,
    /// The kind of form element.
    pub kind: FormElementKind,
    /// Placeholder text (for text-like inputs and textarea).
    pub placeholder: String,
}

impl InputState {
    /// Create a new text-like input state.
    pub fn new_text(input_type: InputType) -> Self {
        Self {
            value: String::new(),
            selection_start: 0,
            selection_end: 0,
            checked: false,
            disabled: false,
            name: String::new(),
            kind: FormElementKind::Input(input_type),
            placeholder: String::new(),
        }
    }

    /// Create a new checkbox state.
    pub fn new_checkbox(checked: bool) -> Self {
        Self {
            value: String::new(),
            selection_start: 0,
            selection_end: 0,
            checked,
            disabled: false,
            name: String::new(),
            kind: FormElementKind::Input(InputType::Checkbox),
            placeholder: String::new(),
        }
    }

    /// Create a new radio button state.
    pub fn new_radio(checked: bool) -> Self {
        Self {
            value: String::new(),
            selection_start: 0,
            selection_end: 0,
            checked,
            disabled: false,
            name: String::new(),
            kind: FormElementKind::Input(InputType::Radio),
            placeholder: String::new(),
        }
    }

    /// Create a new textarea state.
    pub fn new_textarea() -> Self {
        Self {
            value: String::new(),
            selection_start: 0,
            selection_end: 0,
            checked: false,
            disabled: false,
            name: String::new(),
            kind: FormElementKind::Textarea,
            placeholder: String::new(),
        }
    }

    /// Create a new select state.
    pub fn new_select() -> Self {
        Self {
            value: String::new(),
            selection_start: 0,
            selection_end: 0,
            checked: false,
            disabled: false,
            name: String::new(),
            kind: FormElementKind::Select,
            placeholder: String::new(),
        }
    }

    /// Whether this element has a non-collapsed text selection.
    pub fn has_selection(&self) -> bool {
        self.selection_start != self.selection_end
    }

    /// Get the selected text range, if any.
    pub fn selected_text(&self) -> &str {
        let start = self.selection_start.min(self.value.len());
        let end = self.selection_end.min(self.value.len());
        if start <= end {
            &self.value[start..end]
        } else {
            &self.value[end..start]
        }
    }

    /// Move the cursor to the given byte offset, collapsing any selection.
    pub fn set_cursor(&mut self, offset: usize) {
        let clamped = offset.min(self.value.len());
        self.selection_start = clamped;
        self.selection_end = clamped;
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        self.selection_start = 0;
        self.selection_end = self.value.len();
    }
}

/// A store for all form element states in a document.
///
/// Keyed by [`VexId`] of the DOM element node. Owned by the [`Document`].
#[derive(Debug, Default, Clone)]
pub struct FormStateMap {
    states: HashMap<VexId, InputState>,
}

impl FormStateMap {
    /// Create a new empty form state map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the form state for a node, if it exists.
    pub fn get(&self, id: VexId) -> Option<&InputState> {
        self.states.get(&id)
    }

    /// Get a mutable reference to the form state for a node.
    pub fn get_mut(&mut self, id: VexId) -> Option<&mut InputState> {
        self.states.get_mut(&id)
    }

    /// Insert or replace the form state for a node.
    pub fn insert(&mut self, id: VexId, state: InputState) {
        self.states.insert(id, state);
    }

    /// Remove form state for a node (e.g., when the node is removed from the DOM).
    pub fn remove(&mut self, id: VexId) -> Option<InputState> {
        self.states.remove(&id)
    }

    /// Check if a node has form state.
    pub fn contains(&self, id: VexId) -> bool {
        self.states.contains_key(&id)
    }

    /// Total number of tracked form elements.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Iterate over all form states.
    pub fn iter(&self) -> impl Iterator<Item = (VexId, &InputState)> {
        self.states.iter().map(|(&id, state)| (id, state))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_type_from_attr() {
        assert_eq!(InputType::from_attr("text"), InputType::Text);
        assert_eq!(InputType::from_attr("PASSWORD"), InputType::Password);
        assert_eq!(InputType::from_attr("checkbox"), InputType::Checkbox);
        assert_eq!(InputType::from_attr("radio"), InputType::Radio);
        assert_eq!(InputType::from_attr("submit"), InputType::Submit);
        assert_eq!(InputType::from_attr("unknown"), InputType::Text);
        assert_eq!(InputType::from_attr(""), InputType::Text);
    }

    #[test]
    fn input_type_traits() {
        assert!(InputType::Text.is_text_like());
        assert!(InputType::Password.is_text_like());
        assert!(InputType::Email.is_text_like());
        assert!(!InputType::Checkbox.is_text_like());
        assert!(!InputType::Radio.is_text_like());
        assert!(!InputType::Submit.is_text_like());

        assert!(InputType::Checkbox.is_checkable());
        assert!(InputType::Radio.is_checkable());
        assert!(!InputType::Text.is_checkable());
    }

    #[test]
    fn input_state_text() {
        let mut state = InputState::new_text(InputType::Text);
        assert_eq!(state.value, "");
        assert_eq!(state.selection_start, 0);
        assert_eq!(state.selection_end, 0);
        assert!(!state.checked);
        assert!(!state.has_selection());

        state.value = "hello".to_string();
        state.set_cursor(3);
        assert_eq!(state.selection_start, 3);
        assert_eq!(state.selection_end, 3);
        assert!(!state.has_selection());

        state.select_all();
        assert_eq!(state.selection_start, 0);
        assert_eq!(state.selection_end, 5);
        assert!(state.has_selection());
        assert_eq!(state.selected_text(), "hello");
    }

    #[test]
    fn input_state_checkbox() {
        let state = InputState::new_checkbox(true);
        assert!(state.checked);
        assert_eq!(state.kind, FormElementKind::Input(InputType::Checkbox));
    }

    #[test]
    fn cursor_clamped_to_value_length() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "hi".to_string();
        state.set_cursor(100);
        assert_eq!(state.selection_start, 2);
        assert_eq!(state.selection_end, 2);
    }

    #[test]
    fn form_state_map_crud() {
        let mut map = FormStateMap::new();
        assert!(map.is_empty());

        let id = VexId::new(5);
        map.insert(id, InputState::new_text(InputType::Text));
        assert_eq!(map.len(), 1);
        assert!(map.contains(id));

        // Mutate through get_mut
        map.get_mut(id).unwrap().value = "typed".to_string();
        assert_eq!(map.get(id).unwrap().value, "typed");

        // Remove
        let removed = map.remove(id);
        assert!(removed.is_some());
        assert!(map.is_empty());
    }

    #[test]
    fn form_state_map_iter() {
        let mut map = FormStateMap::new();
        map.insert(VexId::new(1), InputState::new_text(InputType::Text));
        map.insert(VexId::new(2), InputState::new_checkbox(false));
        map.insert(VexId::new(3), InputState::new_textarea());

        let ids: Vec<u32> = {
            let mut v: Vec<u32> = map.iter().map(|(id, _)| id.index()).collect();
            v.sort();
            v
        };
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn input_type_display() {
        assert_eq!(InputType::Text.to_string(), "text");
        assert_eq!(InputType::Password.to_string(), "password");
        assert_eq!(InputType::Checkbox.to_string(), "checkbox");
    }
}
