// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Text input editing operations for form elements.
//!
//! Handles keyboard-driven text manipulation: character insertion, deletion,
//! cursor movement, and selection. Each operation mutates an [`InputState`]
//! and returns which DOM events should be fired.

use crate::forms::InputState;

/// A DOM event that should be dispatched after an editing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditEvent {
    /// The `input` event — value changed during editing.
    Input,
    /// The `change` event — value committed (e.g., on blur or Enter).
    Change,
}

/// The result of an editing operation.
#[derive(Debug, Default)]
pub struct EditResult {
    /// Whether the value was modified.
    pub value_changed: bool,
    /// DOM events to fire after the edit.
    pub events: Vec<EditEvent>,
}

impl EditResult {
    fn input_changed() -> Self {
        Self {
            value_changed: true,
            events: vec![EditEvent::Input],
        }
    }

    fn no_change() -> Self {
        Self::default()
    }
}

/// Insert a character at the current cursor position.
///
/// If text is selected, the selection is replaced by the character.
pub fn insert_char(state: &mut InputState, ch: char) -> EditResult {
    delete_selection(state);

    let pos = state.selection_start.min(state.value.len());
    state.value.insert(pos, ch);
    let new_pos = pos + ch.len_utf8();
    state.set_cursor(new_pos);

    EditResult::input_changed()
}

/// Insert a string at the current cursor position.
///
/// If text is selected, the selection is replaced.
pub fn insert_text(state: &mut InputState, text: &str) -> EditResult {
    if text.is_empty() {
        return EditResult::no_change();
    }

    delete_selection(state);

    let pos = state.selection_start.min(state.value.len());
    state.value.insert_str(pos, text);
    let new_pos = pos + text.len();
    state.set_cursor(new_pos);

    EditResult::input_changed()
}

/// Delete the character before the cursor (Backspace).
pub fn backspace(state: &mut InputState) -> EditResult {
    // If there's a selection, delete it.
    if state.has_selection() {
        delete_selection(state);
        return EditResult::input_changed();
    }

    if state.selection_start == 0 {
        return EditResult::no_change();
    }

    // Find the previous character boundary.
    let pos = state.selection_start.min(state.value.len());
    let prev = prev_char_boundary(&state.value, pos);
    state.value.drain(prev..pos);
    state.set_cursor(prev);

    EditResult::input_changed()
}

/// Delete the character after the cursor (Delete key).
pub fn delete_forward(state: &mut InputState) -> EditResult {
    if state.has_selection() {
        delete_selection(state);
        return EditResult::input_changed();
    }

    let pos = state.selection_start.min(state.value.len());
    if pos >= state.value.len() {
        return EditResult::no_change();
    }

    let next = next_char_boundary(&state.value, pos);
    state.value.drain(pos..next);
    state.set_cursor(pos);

    EditResult::input_changed()
}

/// Move cursor one character to the left.
///
/// If `extend_selection` is true, extends the selection instead of collapsing it.
pub fn move_left(state: &mut InputState, extend_selection: bool) -> EditResult {
    if extend_selection {
        let prev = prev_char_boundary(&state.value, state.selection_end);
        state.selection_end = prev;
    } else if state.has_selection() {
        // Collapse to the start of the selection.
        let start = state.selection_start.min(state.selection_end);
        state.set_cursor(start);
    } else {
        let prev = prev_char_boundary(&state.value, state.selection_start);
        state.set_cursor(prev);
    }
    EditResult::no_change()
}

/// Move cursor one character to the right.
///
/// If `extend_selection` is true, extends the selection instead of collapsing it.
pub fn move_right(state: &mut InputState, extend_selection: bool) -> EditResult {
    if extend_selection {
        let next = next_char_boundary(&state.value, state.selection_end);
        state.selection_end = next;
    } else if state.has_selection() {
        // Collapse to the end of the selection.
        let end = state.selection_start.max(state.selection_end);
        state.set_cursor(end);
    } else {
        let next = next_char_boundary(&state.value, state.selection_start);
        state.set_cursor(next);
    }
    EditResult::no_change()
}

/// Move cursor to the beginning of the value (Home key).
pub fn move_home(state: &mut InputState, extend_selection: bool) -> EditResult {
    if extend_selection {
        state.selection_end = 0;
    } else {
        state.set_cursor(0);
    }
    EditResult::no_change()
}

/// Move cursor to the end of the value (End key).
pub fn move_end(state: &mut InputState, extend_selection: bool) -> EditResult {
    let end = state.value.len();
    if extend_selection {
        state.selection_end = end;
    } else {
        state.set_cursor(end);
    }
    EditResult::no_change()
}

/// Select all text (Ctrl+A).
pub fn select_all(state: &mut InputState) -> EditResult {
    state.select_all();
    EditResult::no_change()
}

/// Commit the current value — fires a `change` event.
///
/// Called on blur or Enter key.
pub fn commit(_state: &InputState) -> EditResult {
    EditResult {
        value_changed: false,
        events: vec![EditEvent::Change],
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Delete the selected text, collapsing the selection to a cursor.
fn delete_selection(state: &mut InputState) {
    if !state.has_selection() {
        return;
    }

    let start = state
        .selection_start
        .min(state.selection_end)
        .min(state.value.len());
    let end = state
        .selection_start
        .max(state.selection_end)
        .min(state.value.len());
    state.value.drain(start..end);
    state.set_cursor(start);
}

/// Find the byte offset of the previous character boundary.
fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut i = pos.min(s.len()) - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the byte offset of the next character boundary.
fn next_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut i = pos + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::InputType;

    fn text_state(value: &str) -> InputState {
        let mut s = InputState::new_text(InputType::Text);
        s.value = value.to_string();
        s.set_cursor(value.len()); // cursor at end
        s
    }

    #[test]
    fn insert_char_at_end() {
        let mut state = text_state("hel");
        let result = insert_char(&mut state, 'l');
        assert!(result.value_changed);
        assert_eq!(state.value, "hell");
        assert_eq!(state.selection_start, 4);
    }

    #[test]
    fn insert_char_at_beginning() {
        let mut state = text_state("ello");
        state.set_cursor(0);
        let result = insert_char(&mut state, 'h');
        assert!(result.value_changed);
        assert_eq!(state.value, "hello");
        assert_eq!(state.selection_start, 1);
    }

    #[test]
    fn insert_replaces_selection() {
        let mut state = text_state("hello world");
        state.selection_start = 5;
        state.selection_end = 11;
        let result = insert_char(&mut state, '!');
        assert!(result.value_changed);
        assert_eq!(state.value, "hello!");
    }

    #[test]
    fn backspace_deletes_char() {
        let mut state = text_state("hello");
        let result = backspace(&mut state);
        assert!(result.value_changed);
        assert_eq!(state.value, "hell");
        assert_eq!(state.selection_start, 4);
    }

    #[test]
    fn backspace_at_beginning_is_noop() {
        let mut state = text_state("hello");
        state.set_cursor(0);
        let result = backspace(&mut state);
        assert!(!result.value_changed);
        assert_eq!(state.value, "hello");
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut state = text_state("hello");
        state.selection_start = 1;
        state.selection_end = 4;
        let result = backspace(&mut state);
        assert!(result.value_changed);
        assert_eq!(state.value, "ho");
        assert_eq!(state.selection_start, 1);
    }

    #[test]
    fn delete_forward_deletes_char() {
        let mut state = text_state("hello");
        state.set_cursor(0);
        let result = delete_forward(&mut state);
        assert!(result.value_changed);
        assert_eq!(state.value, "ello");
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut state = text_state("hello");
        let result = delete_forward(&mut state);
        assert!(!result.value_changed);
    }

    #[test]
    fn move_left_collapses_selection() {
        let mut state = text_state("hello");
        state.selection_start = 1;
        state.selection_end = 4;
        move_left(&mut state, false);
        assert_eq!(state.selection_start, 1);
        assert_eq!(state.selection_end, 1);
    }

    #[test]
    fn move_right_past_selection() {
        let mut state = text_state("hello");
        state.selection_start = 1;
        state.selection_end = 4;
        move_right(&mut state, false);
        assert_eq!(state.selection_start, 4);
        assert_eq!(state.selection_end, 4);
    }

    #[test]
    fn move_left_with_shift_extends_selection() {
        let mut state = text_state("hello");
        state.set_cursor(3);
        move_left(&mut state, true);
        assert_eq!(state.selection_start, 3);
        assert_eq!(state.selection_end, 2);
    }

    #[test]
    fn home_and_end() {
        let mut state = text_state("hello");
        state.set_cursor(3);
        move_home(&mut state, false);
        assert_eq!(state.selection_start, 0);

        move_end(&mut state, false);
        assert_eq!(state.selection_start, 5);
    }

    #[test]
    fn select_all_selects_entire_value() {
        let mut state = text_state("hello");
        state.set_cursor(2);
        select_all(&mut state);
        assert_eq!(state.selection_start, 0);
        assert_eq!(state.selection_end, 5);
    }

    #[test]
    fn commit_fires_change_event() {
        let state = text_state("hello");
        let result = commit(&state);
        assert!(!result.value_changed);
        assert_eq!(result.events, vec![EditEvent::Change]);
    }

    #[test]
    fn insert_text_multiple_chars() {
        let mut state = text_state("");
        let result = insert_text(&mut state, "hello");
        assert!(result.value_changed);
        assert_eq!(state.value, "hello");
        assert_eq!(state.selection_start, 5);
    }

    #[test]
    fn insert_empty_is_noop() {
        let mut state = text_state("hello");
        let result = insert_text(&mut state, "");
        assert!(!result.value_changed);
        assert_eq!(state.value, "hello");
    }

    #[test]
    fn unicode_char_boundaries() {
        let mut state = text_state("café");
        // "café" = 5 bytes: c(1) a(1) f(1) é(2)
        backspace(&mut state);
        assert_eq!(state.value, "caf");

        insert_char(&mut state, '☺');
        assert_eq!(state.value, "caf☺");
    }
}
