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

    // ── Text editing operations ──────────────────────────────────────

    /// Insert a character at the current cursor position (or replace selection).
    ///
    /// Moves the cursor to just after the inserted character.
    pub fn insert_char(&mut self, ch: char) {
        let (start, end) = self.ordered_selection();
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.value.replace_range(start..end, s);
        let new_pos = start + s.len();
        self.selection_start = new_pos;
        self.selection_end = new_pos;
    }

    /// Insert a string at the current cursor position (or replace selection).
    pub fn insert_str(&mut self, text: &str) {
        let (start, end) = self.ordered_selection();
        self.value.replace_range(start..end, text);
        let new_pos = start + text.len();
        self.selection_start = new_pos;
        self.selection_end = new_pos;
    }

    /// Delete the character before the cursor (Backspace).
    ///
    /// If text is selected, deletes the selection instead.
    /// Returns `true` if the value changed.
    pub fn delete_backward(&mut self) -> bool {
        if self.has_selection() {
            let (start, end) = self.ordered_selection();
            self.value.replace_range(start..end, "");
            self.selection_start = start;
            self.selection_end = start;
            return true;
        }
        if self.selection_start == 0 {
            return false;
        }
        // Find the previous character boundary.
        let prev = self.value[..self.selection_start]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.value.replace_range(prev..self.selection_start, "");
        self.selection_start = prev;
        self.selection_end = prev;
        true
    }

    /// Delete the character after the cursor (Delete key).
    ///
    /// If text is selected, deletes the selection instead.
    /// Returns `true` if the value changed.
    pub fn delete_forward(&mut self) -> bool {
        if self.has_selection() {
            let (start, end) = self.ordered_selection();
            self.value.replace_range(start..end, "");
            self.selection_start = start;
            self.selection_end = start;
            return true;
        }
        if self.selection_start >= self.value.len() {
            return false;
        }
        // Find the next character boundary.
        let next = self.value[self.selection_start..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.selection_start + i)
            .unwrap_or(self.value.len());
        self.value.replace_range(self.selection_start..next, "");
        true
    }

    /// Move the cursor one character to the left.
    pub fn move_cursor_left(&mut self) {
        if self.has_selection() {
            let (start, _) = self.ordered_selection();
            self.selection_start = start;
            self.selection_end = start;
            return;
        }
        if self.selection_start > 0 {
            let prev = self.value[..self.selection_start]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.selection_start = prev;
            self.selection_end = prev;
        }
    }

    /// Move the cursor one character to the right.
    pub fn move_cursor_right(&mut self) {
        if self.has_selection() {
            let (_, end) = self.ordered_selection();
            self.selection_start = end;
            self.selection_end = end;
            return;
        }
        if self.selection_start < self.value.len() {
            let next = self.value[self.selection_start..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.selection_start + i)
                .unwrap_or(self.value.len());
            self.selection_start = next;
            self.selection_end = next;
        }
    }

    /// Move the cursor to the beginning of the value (Home key).
    pub fn move_cursor_home(&mut self) {
        self.selection_start = 0;
        self.selection_end = 0;
    }

    /// Move the cursor to the end of the value (End key).
    pub fn move_cursor_end(&mut self) {
        let end = self.value.len();
        self.selection_start = end;
        self.selection_end = end;
    }

    /// Get the ordered (start, end) of the current selection/cursor range,
    /// clamped to the value length.
    fn ordered_selection(&self) -> (usize, usize) {
        let s = self.selection_start.min(self.value.len());
        let e = self.selection_end.min(self.value.len());
        if s <= e {
            (s, e)
        } else {
            (e, s)
        }
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

    #[test]
    fn insert_char_at_cursor() {
        let mut state = InputState::new_text(InputType::Text);
        state.insert_char('H');
        state.insert_char('i');
        assert_eq!(state.value, "Hi");
        assert_eq!(state.selection_start, 2);
    }

    #[test]
    fn insert_char_replaces_selection() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "Hello".to_string();
        state.selection_start = 1;
        state.selection_end = 4;
        state.insert_char('a');
        assert_eq!(state.value, "Hao");
        assert_eq!(state.selection_start, 2);
    }

    #[test]
    fn delete_backward_removes_char() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(3);
        assert!(state.delete_backward());
        assert_eq!(state.value, "ab");
        assert_eq!(state.selection_start, 2);
    }

    #[test]
    fn delete_backward_at_start_is_noop() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(0);
        assert!(!state.delete_backward());
        assert_eq!(state.value, "abc");
    }

    #[test]
    fn delete_forward_removes_char() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(1);
        assert!(state.delete_forward());
        assert_eq!(state.value, "ac");
        assert_eq!(state.selection_start, 1);
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(3);
        assert!(!state.delete_forward());
        assert_eq!(state.value, "abc");
    }

    #[test]
    fn move_cursor_left_right() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(2);
        state.move_cursor_left();
        assert_eq!(state.selection_start, 1);
        state.move_cursor_right();
        assert_eq!(state.selection_start, 2);
    }

    #[test]
    fn move_cursor_home_end() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "abc".to_string();
        state.set_cursor(1);
        state.move_cursor_end();
        assert_eq!(state.selection_start, 3);
        state.move_cursor_home();
        assert_eq!(state.selection_start, 0);
    }

    #[test]
    fn insert_str_replaces_selection() {
        let mut state = InputState::new_text(InputType::Text);
        state.value = "Hello World".to_string();
        state.selection_start = 5;
        state.selection_end = 11;
        state.insert_str(" Vex");
        assert_eq!(state.value, "Hello Vex");
        assert_eq!(state.selection_start, 9);
    }
}
