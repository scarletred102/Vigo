// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Form submission — collecting form data and encoding for GET/POST requests.
//!
//! The [`collect_form_data`] function walks a `<form>` element's descendants and
//! gathers all submittable `<input>`, `<textarea>`, and `<select>` values into
//! name-value pairs. [`encode_form_data`] builds a `application/x-www-form-urlencoded`
//! string from those pairs.

use vex_core::VexId;

use crate::arena::NodeArena;
use crate::attributes::get_attribute;
use crate::forms::{FormStateMap, InputType};
use crate::node::NodeData;
use crate::traversal::Descendants;

/// HTTP method for form submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMethod {
    Get,
    Post,
}

impl FormMethod {
    /// Parse from the `method` attribute (defaults to GET).
    pub fn from_attr(s: &str) -> Self {
        if s.eq_ignore_ascii_case("post") {
            Self::Post
        } else {
            Self::Get
        }
    }
}

/// A name-value pair from a form element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormEntry {
    /// The `name` attribute of the form control.
    pub name: String,
    /// The current value (from InputState for interactive elements,
    /// or from the `value` attribute for hidden/submit types).
    pub value: String,
}

/// Parsed metadata from a `<form>` element.
#[derive(Debug, Clone)]
pub struct FormData {
    /// The `action` URL (may be empty, meaning same-page).
    pub action: String,
    /// The submission method.
    pub method: FormMethod,
    /// Collected name-value pairs, in document order.
    pub entries: Vec<FormEntry>,
}

/// Collect all submittable form data under a `<form>` element.
///
/// Walks descendants in document order, collecting values from:
/// - `<input>` (text-like → value, checkbox/radio → "on" if checked, hidden → value attr)
/// - `<textarea>` → value
/// - `<select>` → value
///
/// Elements without a `name` attribute are skipped.
/// Disabled elements are skipped.
pub fn collect_form_data(
    arena: &NodeArena,
    form_states: &FormStateMap,
    form_id: VexId,
) -> FormData {
    let action = get_attribute(arena, form_id, "action")
        .unwrap_or("")
        .to_string();
    let method = get_attribute(arena, form_id, "method")
        .map(FormMethod::from_attr)
        .unwrap_or(FormMethod::Get);

    let mut entries = Vec::new();

    for desc_id in Descendants::new(arena, form_id) {
        let node = arena.get(desc_id);
        let NodeData::Element(ref el) = node.data else {
            continue;
        };

        let tag = el.tag_name.as_str();
        if !matches!(tag, "input" | "textarea" | "select") {
            continue;
        }

        // Must have a name attribute.
        let Some(name) = get_attribute(arena, desc_id, "name") else {
            continue;
        };
        let name = name.to_string();

        // Check for disabled attribute.
        if get_attribute(arena, desc_id, "disabled").is_some() {
            continue;
        }

        // Determine the value.
        let value = if let Some(state) = form_states.get(desc_id) {
            // Has runtime form state (user has interacted or state was initialized).
            if state.disabled {
                continue;
            }

            let input_type = match state.kind {
                crate::forms::FormElementKind::Input(t) => t,
                _ => InputType::Text,
            };

            match input_type {
                InputType::Checkbox | InputType::Radio => {
                    if !state.checked {
                        continue; // Unchecked checkboxes/radios are not submitted.
                    }
                    // Use the `value` attribute, or default "on".
                    if state.value.is_empty() {
                        get_attribute(arena, desc_id, "value")
                            .unwrap_or("on")
                            .to_string()
                    } else {
                        state.value.clone()
                    }
                }
                InputType::Submit | InputType::Button | InputType::Reset => {
                    continue; // Submit buttons only submit if they triggered submission.
                }
                _ => state.value.clone(),
            }
        } else {
            // No runtime state — fall back to the `value` attribute.
            match tag {
                "input" => {
                    let input_type = get_attribute(arena, desc_id, "type")
                        .map(InputType::from_attr)
                        .unwrap_or(InputType::Text);

                    match input_type {
                        InputType::Checkbox | InputType::Radio => {
                            // Without runtime state, check if `checked` attribute exists.
                            if get_attribute(arena, desc_id, "checked").is_none() {
                                continue;
                            }
                            get_attribute(arena, desc_id, "value")
                                .unwrap_or("on")
                                .to_string()
                        }
                        InputType::Submit | InputType::Button | InputType::Reset => {
                            continue;
                        }
                        _ => get_attribute(arena, desc_id, "value")
                            .unwrap_or("")
                            .to_string(),
                    }
                }
                _ => get_attribute(arena, desc_id, "value")
                    .unwrap_or("")
                    .to_string(),
            }
        };

        entries.push(FormEntry { name, value });
    }

    FormData {
        action,
        method,
        entries,
    }
}

/// Encode form entries as `application/x-www-form-urlencoded`.
///
/// Follows the URL Standard encoding:
/// - Space → `+`
/// - Non-alphanumeric/safe chars → `%XX`
/// - Entries joined by `&`
pub fn encode_form_data(entries: &[FormEntry]) -> String {
    entries
        .iter()
        .map(|e| format!("{}={}", url_encode(&e.name), url_encode(&e.value),))
        .collect::<Vec<_>>()
        .join("&")
}

/// Build a full submission URL for GET requests.
///
/// Appends the encoded form data as a query string to the action URL.
pub fn build_get_url(action: &str, entries: &[FormEntry]) -> String {
    let encoded = encode_form_data(entries);
    if encoded.is_empty() {
        return action.to_string();
    }
    if action.contains('?') {
        format!("{action}&{encoded}")
    } else {
        format!("{action}?{encoded}")
    }
}

/// Minimal URL percent-encoding for form data.
fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(HEX_UPPER[(byte >> 4) as usize] as char);
                out.push(HEX_UPPER[(byte & 0x0F) as usize] as char);
            }
        }
    }
    out
}

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::{InputState, InputType};
    use crate::node::Namespace;
    use crate::tree;

    fn make_doc_with_form() -> (NodeArena, FormStateMap, VexId) {
        let mut arena = NodeArena::new();
        let mut states = FormStateMap::new();

        // Create <form action="/submit" method="post">
        let form = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "form".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "action".to_string(),
                    value: "/submit".to_string(),
                },
                crate::node::Attribute {
                    name: "method".to_string(),
                    value: "post".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));

        // <input type="text" name="username">
        let input_text = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "input".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "type".to_string(),
                    value: "text".to_string(),
                },
                crate::node::Attribute {
                    name: "name".to_string(),
                    value: "username".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, form, input_text);

        let mut text_state = InputState::new_text(InputType::Text);
        text_state.value = "alice".to_string();
        text_state.name = "username".to_string();
        states.insert(input_text, text_state);

        // <input type="checkbox" name="agree" checked>
        let input_cb = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "input".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "type".to_string(),
                    value: "checkbox".to_string(),
                },
                crate::node::Attribute {
                    name: "name".to_string(),
                    value: "agree".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, form, input_cb);

        let mut cb_state = InputState::new_checkbox(true);
        cb_state.name = "agree".to_string();
        states.insert(input_cb, cb_state);

        (arena, states, form)
    }

    #[test]
    fn collect_basic_form() {
        let (arena, states, form) = make_doc_with_form();
        let data = collect_form_data(&arena, &states, form);

        assert_eq!(data.action, "/submit");
        assert_eq!(data.method, FormMethod::Post);
        assert_eq!(data.entries.len(), 2);
        assert_eq!(data.entries[0].name, "username");
        assert_eq!(data.entries[0].value, "alice");
        assert_eq!(data.entries[1].name, "agree");
        assert_eq!(data.entries[1].value, "on");
    }

    #[test]
    fn unchecked_checkbox_excluded() {
        let mut arena = NodeArena::new();
        let mut states = FormStateMap::new();

        let form = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "form".to_string(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));

        let cb = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "input".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "type".to_string(),
                    value: "checkbox".to_string(),
                },
                crate::node::Attribute {
                    name: "name".to_string(),
                    value: "agree".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, form, cb);

        let mut cb_state = InputState::new_checkbox(false);
        cb_state.name = "agree".to_string();
        states.insert(cb, cb_state);

        let data = collect_form_data(&arena, &states, form);
        assert!(data.entries.is_empty());
    }

    #[test]
    fn disabled_inputs_excluded() {
        let mut arena = NodeArena::new();
        let states = FormStateMap::new();

        let form = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "form".to_string(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));

        let input = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "input".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "name".to_string(),
                    value: "field".to_string(),
                },
                crate::node::Attribute {
                    name: "disabled".to_string(),
                    value: "".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, form, input);

        let data = collect_form_data(&arena, &states, form);
        assert!(data.entries.is_empty());
    }

    #[test]
    fn encode_form_data_basic() {
        let entries = vec![
            FormEntry {
                name: "q".to_string(),
                value: "hello world".to_string(),
            },
            FormEntry {
                name: "lang".to_string(),
                value: "en".to_string(),
            },
        ];
        assert_eq!(encode_form_data(&entries), "q=hello+world&lang=en");
    }

    #[test]
    fn encode_form_data_special_chars() {
        let entries = vec![FormEntry {
            name: "data".to_string(),
            value: "a=b&c".to_string(),
        }];
        assert_eq!(encode_form_data(&entries), "data=a%3Db%26c");
    }

    #[test]
    fn build_get_url_no_query() {
        let entries = vec![FormEntry {
            name: "q".to_string(),
            value: "test".to_string(),
        }];
        assert_eq!(build_get_url("/search", &entries), "/search?q=test");
    }

    #[test]
    fn build_get_url_existing_query() {
        let entries = vec![FormEntry {
            name: "page".to_string(),
            value: "2".to_string(),
        }];
        assert_eq!(
            build_get_url("/search?q=test", &entries),
            "/search?q=test&page=2"
        );
    }

    #[test]
    fn build_get_url_empty_entries() {
        assert_eq!(build_get_url("/search", &[]), "/search");
    }

    #[test]
    fn form_method_parsing() {
        assert_eq!(FormMethod::from_attr("POST"), FormMethod::Post);
        assert_eq!(FormMethod::from_attr("post"), FormMethod::Post);
        assert_eq!(FormMethod::from_attr("get"), FormMethod::Get);
        assert_eq!(FormMethod::from_attr("GET"), FormMethod::Get);
        assert_eq!(FormMethod::from_attr(""), FormMethod::Get);
        assert_eq!(FormMethod::from_attr("weird"), FormMethod::Get);
    }

    #[test]
    fn fallback_to_value_attribute() {
        let mut arena = NodeArena::new();
        let states = FormStateMap::new(); // No runtime states

        let form = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "form".to_string(),
            namespace: Namespace::Html,
            attributes: Vec::new(),
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));

        // Hidden input with value attribute only
        let hidden = arena.alloc(NodeData::Element(crate::node::ElementData {
            tag_name: "input".to_string(),
            namespace: Namespace::Html,
            attributes: vec![
                crate::node::Attribute {
                    name: "type".to_string(),
                    value: "hidden".to_string(),
                },
                crate::node::Attribute {
                    name: "name".to_string(),
                    value: "token".to_string(),
                },
                crate::node::Attribute {
                    name: "value".to_string(),
                    value: "abc123".to_string(),
                },
            ],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }));
        tree::append_child(&mut arena, form, hidden);

        let data = collect_form_data(&arena, &states, form);
        assert_eq!(data.entries.len(), 1);
        assert_eq!(data.entries[0].name, "token");
        assert_eq!(data.entries[0].value, "abc123");
    }
}
