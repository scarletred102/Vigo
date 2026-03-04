// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Form handling — connects keyboard events to input editing, manages
//! focus state across form elements, and handles form submission.

use url::Url;
use vex_core::VexId;
use vex_dom::forms::{FormElementKind, FormStateMap, InputType};
use vex_dom::{Document, NodeArena, NodeData};

/// Result of processing a keyboard event on a form element.
#[derive(Debug, Clone, PartialEq)]
pub enum FormEditResult {
    /// Input text changed — re-render needed.
    Changed,
    /// Nothing changed (arrow key at edge, etc.).
    NoChange,
    /// Form submission triggered (Enter in text input).
    Submit { action: String, method: FormMethod },
}

/// HTTP method for form submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMethod {
    Get,
    Post,
}

impl FormMethod {
    /// Parse from HTML `method` attribute value.
    pub fn from_attr(s: &str) -> Self {
        if s.eq_ignore_ascii_case("post") {
            Self::Post
        } else {
            Self::Get
        }
    }
}

/// Manages focused form element and routes input to the correct state.
pub struct FormHandler {
    /// Currently focused element, if any.
    focused: Option<VexId>,
}

impl FormHandler {
    /// Create a new form handler with no focus.
    pub fn new() -> Self {
        Self { focused: None }
    }

    /// Set focus to a form element.
    pub fn set_focus(&mut self, id: VexId) {
        self.focused = Some(id);
    }

    /// Clear focus.
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Return the currently focused element ID.
    pub fn focused(&self) -> Option<VexId> {
        self.focused
    }

    /// Process a character input for the focused element.
    ///
    /// Returns `Changed` if the input was modified, `NoChange` if there
    /// is no focused text-like element, or `Submit` if Enter was pressed.
    pub fn handle_char(
        &self,
        ch: char,
        form_states: &mut FormStateMap,
        doc: &Document,
    ) -> FormEditResult {
        let id = match self.focused {
            Some(id) => id,
            None => return FormEditResult::NoChange,
        };
        let state = match form_states.get_mut(id) {
            Some(s) => s,
            None => return FormEditResult::NoChange,
        };

        // Only text-like inputs accept character input.
        if !is_text_like(&state.kind) {
            return FormEditResult::NoChange;
        }

        // Enter triggers form submission.
        if ch == '\r' || ch == '\n' {
            return find_form_action(id, doc);
        }

        // Tab is not a printable character for input fields.
        if ch == '\t' {
            return FormEditResult::NoChange;
        }

        state.insert_char(ch);
        FormEditResult::Changed
    }

    /// Process a special key for the focused element.
    pub fn handle_key(&self, key: FormKey, form_states: &mut FormStateMap) -> FormEditResult {
        let id = match self.focused {
            Some(id) => id,
            None => return FormEditResult::NoChange,
        };
        let state = match form_states.get_mut(id) {
            Some(s) => s,
            None => return FormEditResult::NoChange,
        };

        if !is_text_like(&state.kind) {
            // Toggle checkbox/radio on Space.
            if matches!(key, FormKey::Space) && state.kind == FormElementKind::Input(InputType::Checkbox) {
                state.checked = !state.checked;
                return FormEditResult::Changed;
            }
            return FormEditResult::NoChange;
        }

        match key {
            FormKey::Backspace => {
                state.delete_backward();
                FormEditResult::Changed
            }
            FormKey::Delete => {
                state.delete_forward();
                FormEditResult::Changed
            }
            FormKey::Left => {
                state.move_cursor_left();
                FormEditResult::Changed
            }
            FormKey::Right => {
                state.move_cursor_right();
                FormEditResult::Changed
            }
            FormKey::Home => {
                state.move_cursor_home();
                FormEditResult::Changed
            }
            FormKey::End => {
                state.move_cursor_end();
                FormEditResult::Changed
            }
            FormKey::SelectAll => {
                state.select_all();
                FormEditResult::Changed
            }
            FormKey::Space => {
                state.insert_char(' ');
                FormEditResult::Changed
            }
        }
    }
}

impl Default for FormHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Keys that have special handling in form inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKey {
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    SelectAll,
    Space,
}

/// Check if a form element kind accepts text input.
fn is_text_like(kind: &FormElementKind) -> bool {
    match kind {
        FormElementKind::Input(t) => t.is_text_like(),
        FormElementKind::Textarea => true,
        FormElementKind::Select | FormElementKind::Button => false,
    }
}

/// Find the `<form>` ancestor of an element and extract its action and method.
fn find_form_action(element_id: VexId, doc: &Document) -> FormEditResult {
    // Walk up the tree looking for a <form> ancestor.
    let arena = doc.arena();
    let mut current = Some(element_id);

    while let Some(id) = current {
        let node = arena.get(id);
        if let NodeData::Element(ref el) = node.data {
            if el.tag_name == "form" {
                let action = el
                    .attributes
                    .iter()
                    .find(|a| a.name == "action")
                    .map(|a| a.value.clone())
                    .unwrap_or_default();
                let method = el
                    .attributes
                    .iter()
                    .find(|a| a.name == "method")
                    .map(|a| FormMethod::from_attr(&a.value))
                    .unwrap_or(FormMethod::Get);
                return FormEditResult::Submit { action, method };
            }
        }
        current = node.parent;
    }

    // No form found — just treat Enter as no-op.
    FormEditResult::NoChange
}

/// Collect all form input values into a URL-encoded query string.
///
/// Returns pairs of `(name, value)` for each named input inside the form.
pub fn collect_form_values(
    form_id: VexId,
    doc: &Document,
    form_states: &FormStateMap,
) -> Vec<(String, String)> {
    let mut values = Vec::new();
    let arena = doc.arena();

    // Simple DFS to find all input descendants of the form.
    collect_inputs_recursive(form_id, arena, form_states, &mut values);

    values
}

fn collect_inputs_recursive(
    node_id: VexId,
    arena: &NodeArena,
    form_states: &FormStateMap,
    out: &mut Vec<(String, String)>,
) {
    let node = arena.get(node_id);

    // Check if this node has a form state entry with a name.
    if let Some(state) = form_states.get(node_id) {
        if !state.name.is_empty() {
            // Skip unchecked checkboxes/radios.
            let include = match state.kind {
                FormElementKind::Input(InputType::Checkbox | InputType::Radio) => state.checked,
                FormElementKind::Input(InputType::Submit | InputType::Button | InputType::Hidden) => {
                    // Submit buttons only included if they are the submitter.
                    // For simplicity, include hidden fields always.
                    matches!(state.kind, FormElementKind::Input(InputType::Hidden))
                }
                _ => true,
            };
            if include {
                let value = if state.kind == FormElementKind::Input(InputType::Checkbox) && state.checked {
                    if state.value.is_empty() { "on".to_string() } else { state.value.clone() }
                } else {
                    state.value.clone()
                };
                out.push((state.name.clone(), value));
            }
        }
    }

    // Recurse into children via linked-list traversal.
    let mut child = node.first_child;
    while let Some(child_id) = child {
        collect_inputs_recursive(child_id, arena, form_states, out);
        child = arena.get(child_id).next_sibling;
    }
}

/// Build a URL-encoded query string from name-value pairs.
pub fn encode_form_data(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                percent_encode(k),
                percent_encode(v)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Build a form submission URL from the action, method, and collected values.
///
/// For GET: appends query string to the action URL.
/// For POST: returns the action URL as-is (body handled separately).
pub fn build_submission_url(
    action: &str,
    base_url: &Url,
    method: FormMethod,
    pairs: &[(String, String)],
) -> Option<Url> {
    let resolved = base_url.join(action).ok()?;
    match method {
        FormMethod::Get => {
            let mut url = resolved;
            let query = encode_form_data(pairs);
            if !query.is_empty() {
                url.set_query(Some(&query));
            }
            Some(url)
        }
        FormMethod::Post => Some(resolved),
    }
}

/// Simple percent-encoding for form data.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push('+'),
            _ => {
                let mut buf = [0u8; 4];
                let encoded = ch.encode_utf8(&mut buf);
                for &b in encoded.as_bytes() {
                    out.push('%');
                    out.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
                    out.push(char::from_digit((b & 0xF) as u32, 16).unwrap_or('0'));
                }
            }
        }
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use vex_dom::forms::{InputState, InputType};

    #[test]
    fn form_method_from_attr() {
        assert_eq!(FormMethod::from_attr("get"), FormMethod::Get);
        assert_eq!(FormMethod::from_attr("GET"), FormMethod::Get);
        assert_eq!(FormMethod::from_attr("post"), FormMethod::Post);
        assert_eq!(FormMethod::from_attr("POST"), FormMethod::Post);
        assert_eq!(FormMethod::from_attr("anything"), FormMethod::Get);
    }

    #[test]
    fn percent_encode_simple() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello+world");
        assert_eq!(percent_encode("a&b=c"), "a%26b%3dc");
    }

    #[test]
    fn encode_form_data_pairs() {
        let pairs = vec![
            ("name".to_string(), "John Doe".to_string()),
            ("age".to_string(), "30".to_string()),
        ];
        let encoded = encode_form_data(&pairs);
        assert_eq!(encoded, "name=John+Doe&age=30");
    }

    #[test]
    fn encode_empty_pairs() {
        let pairs: Vec<(String, String)> = vec![];
        assert_eq!(encode_form_data(&pairs), "");
    }

    #[test]
    fn form_handler_no_focus() {
        let handler = FormHandler::new();
        let mut states = FormStateMap::new();
        let doc = Document::new();
        assert_eq!(handler.handle_char('a', &mut states, &doc), FormEditResult::NoChange);
    }

    #[test]
    fn form_handler_char_input() {
        let mut handler = FormHandler::new();
        let mut states = FormStateMap::new();
        let doc = Document::new();

        let id = VexId::new(1);
        states.insert(id, InputState::new_text(InputType::Text));
        handler.set_focus(id);

        let result = handler.handle_char('a', &mut states, &doc);
        assert_eq!(result, FormEditResult::Changed);
        assert_eq!(states.get(id).map(|s| s.value.as_str()), Some("a"));
    }

    #[test]
    fn form_handler_key_backspace() {
        let mut handler = FormHandler::new();
        let mut states = FormStateMap::new();

        let id = VexId::new(1);
        let mut state = InputState::new_text(InputType::Text);
        state.value = "hello".to_string();
        state.set_cursor(5);
        states.insert(id, state);
        handler.set_focus(id);

        let result = handler.handle_key(FormKey::Backspace, &mut states);
        assert_eq!(result, FormEditResult::Changed);
        assert_eq!(states.get(id).map(|s| s.value.as_str()), Some("hell"));
    }

    #[test]
    fn form_handler_checkbox_toggle() {
        let mut handler = FormHandler::new();
        let mut states = FormStateMap::new();

        let id = VexId::new(1);
        states.insert(id, InputState::new_checkbox(false));
        handler.set_focus(id);

        let result = handler.handle_key(FormKey::Space, &mut states);
        assert_eq!(result, FormEditResult::Changed);
        assert!(states.get(id).map(|s| s.checked).unwrap_or(false));
    }

    #[test]
    fn build_get_submission_url() {
        let base = Url::parse("https://example.com/page").unwrap();
        let pairs = vec![("q".to_string(), "rust".to_string())];
        let url = build_submission_url("/search", &base, FormMethod::Get, &pairs).unwrap();
        assert_eq!(url.as_str(), "https://example.com/search?q=rust");
    }

    #[test]
    fn build_post_submission_url() {
        let base = Url::parse("https://example.com/page").unwrap();
        let pairs = vec![("q".to_string(), "rust".to_string())];
        let url = build_submission_url("/submit", &base, FormMethod::Post, &pairs).unwrap();
        // POST URL should not have query string.
        assert_eq!(url.as_str(), "https://example.com/submit");
    }
}
