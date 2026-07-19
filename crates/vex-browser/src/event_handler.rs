// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Platform event → DOM event pipeline.
//!
//! Connects mouse/keyboard platform events to the DOM event dispatch
//! system via hit-testing and the JS callback bridge.
//!
//! ## Click pipeline
//!
//! 1. Platform reports mouse click at `(x, y)`.
//! 2. `hit_test()` finds the deepest element at that position.
//! 3. `MouseDown`, `MouseUp`, and `Click` events are dispatched through
//!    the DOM (capture → target → bubble), invoking JS callbacks.
//! 4. If the `Click` event was not `preventDefault()`-ed and the target
//!    is inside an `<a href>`, `resolve_link_click()` determines the nav action.

use vex_core::{VexId, VexUrl};
use vex_dom::events::{Event, EventType};
use vex_dom::node::NodeData;
use vex_dom::Document;
use vex_dom::{ElementState, FormElementKind, InputState, InputType};
use vex_js::{JsRuntime, SharedDocument};
use vex_layout::LayoutBox;
use vex_render::scroll::ScrollState;

use crate::links::{self, LinkAction};

/// Result of processing a mouse click.
#[derive(Debug, Clone, PartialEq)]
pub enum ClickResult {
    /// A link navigation was triggered.
    Navigate(LinkAction),
    /// The click was handled but did not trigger navigation.
    Handled,
    /// No element was hit at the given position.
    Miss,
}

/// Process a mouse click at viewport coordinates `(x, y)`.
///
/// Runs the full pipeline: hit-test → dispatch MouseDown/MouseUp/Click →
/// check link default action. Returns what navigation (if any) should happen.
#[allow(clippy::too_many_arguments)]
pub fn process_click(
    x: f32,
    y: f32,
    layout: &LayoutBox,
    scroll: &ScrollState,
    shared_doc: &SharedDocument,
    current_url: &VexUrl,
    runtime: &mut JsRuntime,
) -> ClickResult {
    // Convert viewport coords to content coords (account for scroll).
    let content_x = x + scroll.offset_x;
    let content_y = y + scroll.offset_y;

    let Some(target_id) = vex_layout::hit_test(layout, content_x, content_y) else {
        return ClickResult::Miss;
    };

    // Dispatch MouseDown
    let mut mousedown = Event::new(EventType::MouseDown, target_id);
    runtime.dispatch_dom_event(shared_doc, &mut mousedown);

    // Dispatch MouseUp
    let mut mouseup = Event::new(EventType::MouseUp, target_id);
    runtime.dispatch_dom_event(shared_doc, &mut mouseup);

    // Dispatch Click
    let mut click = Event::new(EventType::Click, target_id);
    let prevented = runtime.dispatch_dom_event(shared_doc, &mut click);

    if prevented {
        return ClickResult::Handled;
    }

    // Default form control behavior (checkbox/radio toggles).
    if apply_default_form_click(shared_doc, target_id) {
        let mut input_evt = Event::new(EventType::Input, target_id);
        runtime.dispatch_dom_event(shared_doc, &mut input_evt);
        let mut change_evt = Event::new(EventType::Change, target_id);
        runtime.dispatch_dom_event(shared_doc, &mut change_evt);
    }

    // Default action — check if the target is inside an <a> element.
    let link_action = {
        let doc = shared_doc.borrow();
        links::resolve_link_click(&doc, target_id, current_url)
    };
    if link_action != LinkAction::None {
        return ClickResult::Navigate(link_action);
    }

    ClickResult::Handled
}

/// Process focus change when clicking on an element.
///
/// Returns the newly focused `VexId` if focus changed, `None` if same.
pub fn process_focus_change(
    new_target: VexId,
    current_focus: Option<VexId>,
    shared_doc: &SharedDocument,
    runtime: &mut JsRuntime,
) -> Option<VexId> {
    if current_focus == Some(new_target) {
        return None;
    }

    // Fire blur on old element.
    if let Some(old_id) = current_focus {
        {
            let mut doc = shared_doc.borrow_mut();
            clear_focus_chain(&mut doc, old_id);
        }
        let mut blur = Event::new(EventType::Blur, old_id);
        runtime.dispatch_dom_event(shared_doc, &mut blur);
    }

    {
        let mut doc = shared_doc.borrow_mut();
        set_focus_chain(&mut doc, new_target);
    }

    // Fire focus on new element.
    let mut focus = Event::new(EventType::Focus, new_target);
    runtime.dispatch_dom_event(shared_doc, &mut focus);

    Some(new_target)
}

fn clear_focus_chain(doc: &mut Document, start: VexId) {
    let mut current = Some(start);
    let mut first = true;
    while let Some(id) = current {
        let parent = {
            let node = doc.arena().get(id);
            node.parent
        };

        {
            let node = doc.arena_mut().get_mut(id);
            if let NodeData::Element(ref mut el) = node.data {
                if first {
                    el.state.remove(ElementState::FOCUS);
                    first = false;
                }
                el.state.remove(ElementState::FOCUS_WITHIN);
            }
        }

        current = parent;
    }
}

fn set_focus_chain(doc: &mut Document, target: VexId) {
    let mut current = Some(target);
    let mut first = true;
    while let Some(id) = current {
        let parent = {
            let node = doc.arena().get(id);
            node.parent
        };

        {
            let node = doc.arena_mut().get_mut(id);
            if let NodeData::Element(ref mut el) = node.data {
                if first {
                    el.state.insert(ElementState::FOCUS);
                    first = false;
                }
                el.state.insert(ElementState::FOCUS_WITHIN);
            }
        }

        current = parent;
    }
}

fn apply_default_form_click(shared_doc: &SharedDocument, target_id: VexId) -> bool {
    let mut doc = shared_doc.borrow_mut();
    let Some(input_id) = find_ancestor_input(&doc, target_id) else {
        return false;
    };

    let (input_type, input_name) = {
        let node = doc.arena().get(input_id);
        let NodeData::Element(ref el) = node.data else {
            return false;
        };
        (
            element_input_type(el),
            el.attributes
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case("name"))
                .map(|a| a.value.clone())
                .unwrap_or_default(),
        )
    };

    match input_type {
        InputType::Checkbox => {
            ensure_input_state(&mut doc, input_id, input_type, &input_name);
            let checked = {
                let state = match doc.form_states_mut().get_mut(input_id) {
                    Some(s) => s,
                    None => return false,
                };
                state.checked = !state.checked;
                state.checked
            };
            set_checked_state_flag(&mut doc, input_id, checked);
            true
        }
        InputType::Radio => {
            // Uncheck all radios in the same group, then check the clicked one.
            let mut group_ids: Vec<VexId> = Vec::new();
            for idx in 0..doc.arena().len() {
                let id = VexId::new(idx as u32);
                let node = doc.arena().get(id);
                let NodeData::Element(ref el) = node.data else {
                    continue;
                };
                if !el.tag_name.eq_ignore_ascii_case("input")
                    || element_input_type(el) != InputType::Radio
                {
                    continue;
                }
                let name = el
                    .attributes
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case("name"))
                    .map(|a| a.value.as_str())
                    .unwrap_or("");
                if name == input_name {
                    group_ids.push(id);
                }
            }

            for id in &group_ids {
                ensure_input_state(&mut doc, *id, InputType::Radio, &input_name);
                if let Some(state) = doc.form_states_mut().get_mut(*id) {
                    state.checked = *id == input_id;
                }
                set_checked_state_flag(&mut doc, *id, *id == input_id);
            }

            if group_ids.is_empty() {
                ensure_input_state(&mut doc, input_id, InputType::Radio, &input_name);
                if let Some(state) = doc.form_states_mut().get_mut(input_id) {
                    state.checked = true;
                }
                set_checked_state_flag(&mut doc, input_id, true);
            }

            true
        }
        _ => false,
    }
}

fn find_ancestor_input(doc: &Document, start: VexId) -> Option<VexId> {
    let mut current = Some(start);
    while let Some(id) = current {
        let node = doc.arena().get(id);
        if let NodeData::Element(ref el) = node.data {
            if el.tag_name.eq_ignore_ascii_case("input") {
                return Some(id);
            }
        }
        current = node.parent;
    }
    None
}

fn element_input_type(el: &vex_dom::node::ElementData) -> InputType {
    el.attributes
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("type"))
        .map(|a| InputType::from_attr(&a.value))
        .unwrap_or(InputType::Text)
}

fn ensure_input_state(doc: &mut Document, id: VexId, input_type: InputType, name: &str) {
    if doc.form_states().contains(id) {
        return;
    }

    let mut state = match input_type {
        InputType::Checkbox => InputState::new_checkbox(false),
        InputType::Radio => InputState::new_radio(false),
        _ => InputState::new_text(input_type),
    };
    state.kind = FormElementKind::Input(input_type);
    state.name = name.to_string();
    doc.form_states_mut().insert(id, state);
}

fn set_checked_state_flag(doc: &mut Document, id: VexId, checked: bool) {
    let node = doc.arena_mut().get_mut(id);
    if let NodeData::Element(ref mut el) = node.data {
        el.state.set(ElementState::CHECKED, checked);
    }
}

// ── Keyboard Event Handling ──────────────────────────────────────────

/// Virtual key codes used by the platform layer (Win32 VK constants).
mod vk {
    pub const BACK: u32 = 0x08;
    pub const TAB: u32 = 0x09;
    pub const RETURN: u32 = 0x0D;
    pub const DELETE: u32 = 0x2E;
    pub const LEFT: u32 = 0x25;
    pub const RIGHT: u32 = 0x27;
    pub const HOME: u32 = 0x24;
    pub const END: u32 = 0x23;
}

/// Result of processing a keyboard event.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyResult {
    /// The focused input's value was modified — re-render needed.
    InputChanged,
    /// The key was handled (dispatched to JS) but no value change.
    Handled,
    /// No element is focused — nothing happened.
    NoFocus,
}

/// Process a key-down event for the currently focused element.
///
/// Pipeline:
/// 1. Dispatch `KeyDown` DOM event through JS.
/// 2. If `preventDefault()` was called, stop.
/// 3. If the focused element is a text-like input or textarea, update
///    its [`InputState`] for printable characters, backspace, delete, and
///    arrow keys.
/// 4. Fire an `Input` event after text changes.
///
/// `keycode` is a Win32 VK code. `char_value` is the Unicode character
/// for printable keys (from `WM_CHAR`), or `None` for control keys.
pub fn process_key_down(
    focused: VexId,
    keycode: u32,
    char_value: Option<char>,
    shared_doc: &SharedDocument,
    runtime: &mut JsRuntime,
) -> KeyResult {
    // 1. Dispatch KeyDown to JS.
    let mut keydown = Event::new(EventType::KeyDown, focused);
    let prevented = runtime.dispatch_dom_event(shared_doc, &mut keydown);

    if prevented {
        return KeyResult::Handled;
    }

    // 2. Check if the focused element is a text-like input / textarea.
    let mut doc = shared_doc.borrow_mut();
    let is_text_input = is_text_input_element(&doc, focused);

    if !is_text_input {
        return KeyResult::Handled;
    }

    // 3. Apply the keystroke to the InputState.
    let changed = apply_key_to_input(&mut doc, focused, keycode, char_value);

    if changed {
        drop(doc); // Release borrow before dispatching another event.
        let mut input_evt = Event::new(EventType::Input, focused);
        runtime.dispatch_dom_event(shared_doc, &mut input_evt);
        KeyResult::InputChanged
    } else {
        KeyResult::Handled
    }
}

/// Check whether a DOM node is a text-accepting form element.
fn is_text_input_element(doc: &Document, id: VexId) -> bool {
    let node = doc.arena().get(id);
    if let NodeData::Element(ref el) = node.data {
        match el.tag_name.as_str() {
            "textarea" => true,
            "input" => {
                // Check the type attribute — only text-like inputs accept keyboard.
                let input_type = el
                    .attributes
                    .iter()
                    .find(|a| a.name == "type")
                    .map(|a| vex_dom::InputType::from_attr(&a.value))
                    .unwrap_or_default();
                input_type.is_text_like()
            }
            _ => false,
        }
    } else {
        false
    }
}

/// Apply a keystroke to the focused element's `InputState`.
///
/// Returns `true` if the value was modified (i.e., re-render needed).
fn apply_key_to_input(
    doc: &mut Document,
    id: VexId,
    keycode: u32,
    char_value: Option<char>,
) -> bool {
    // Ensure the element has an InputState — create one if missing.
    if !doc.form_states().contains(id) {
        let node = doc.arena().get(id);
        if let NodeData::Element(ref el) = node.data {
            let state = match el.tag_name.as_str() {
                "textarea" => vex_dom::InputState::new_textarea(),
                "input" => {
                    let it = el
                        .attributes
                        .iter()
                        .find(|a| a.name == "type")
                        .map(|a| vex_dom::InputType::from_attr(&a.value))
                        .unwrap_or_default();
                    vex_dom::InputState::new_text(it)
                }
                _ => return false,
            };
            doc.form_states_mut().insert(id, state);
        }
    }

    let Some(state) = doc.form_states_mut().get_mut(id) else {
        return false;
    };

    match keycode {
        vk::BACK => state.delete_backward(),
        vk::DELETE => state.delete_forward(),
        vk::LEFT => {
            state.move_cursor_left();
            false
        }
        vk::RIGHT => {
            state.move_cursor_right();
            false
        }
        vk::HOME => {
            state.move_cursor_home();
            false
        }
        vk::END => {
            state.move_cursor_end();
            false
        }
        vk::RETURN | vk::TAB => false,
        _ => {
            // Printable character — insert it.
            if let Some(ch) = char_value {
                if !ch.is_control() {
                    state.insert_char(ch);
                    return true;
                }
            }
            false
        }
    }
}

// ── Page Lifecycle Events ────────────────────────────────────────────

/// Fire `DOMContentLoaded` and `load` lifecycle events for a page load.
///
/// Should be called after:
/// 1. HTML parsing → DOM tree built
/// 2. Blocking/deferred scripts executed
/// 3. CSS styles computed and layout done
///
/// Fires `DOMContentLoaded` first (DOM ready), then `load` (all resources).
pub fn fire_page_lifecycle(shared_doc: &SharedDocument, runtime: &mut JsRuntime) {
    runtime.fire_dom_content_loaded(shared_doc);
    runtime.fire_load(shared_doc);
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::{Insets, Point, Rect, Size};
    use vex_layout::box_model::{BoxType, Dimensions};

    fn make_box(id: Option<VexId>, x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        LayoutBox {
            node_id: id,
            box_type: BoxType::Block,
            dimensions: Dimensions {
                content: Rect {
                    origin: Point::new(x, y),
                    size: Size::new(w, h),
                },
                padding: Insets::default(),
                border: Insets::default(),
                margin: Insets::default(),
            },
            children: Vec::new(),
            clip_rect: None,
            scroll_offset: Point::default(),
        }
    }

    #[test]
    fn click_miss_returns_miss() {
        let root = make_box(Some(VexId::new(1)), 0.0, 0.0, 100.0, 100.0);
        let doc = vex_html::parse_html("<html><body>hello</body></html>");
        let shared = vex_js::shared_document(doc);
        let url = VexUrl::parse("https://example.com").unwrap();
        let scroll = ScrollState::new(800.0, 600.0);
        let mut rt = JsRuntime::new();

        // Read doc from shared before passing another borrow
        let doc_ref = shared.borrow();
        drop(doc_ref);
        let result = process_click(200.0, 200.0, &root, &scroll, &shared, &url, &mut rt);
        assert_eq!(result, ClickResult::Miss);
    }

    #[test]
    fn click_on_non_link_returns_handled() {
        let doc = vex_html::parse_html("<html><body><p>hi</p></body></html>");
        let body_ids = doc.get_elements_by_tag_name("body");
        let body_id = body_ids[0];
        let root = make_box(Some(body_id), 0.0, 0.0, 500.0, 500.0);
        let shared = vex_js::shared_document(doc);
        let url = VexUrl::parse("https://example.com").unwrap();
        let scroll = ScrollState::new(800.0, 600.0);
        let mut rt = JsRuntime::new();

        let result = process_click(50.0, 50.0, &root, &scroll, &shared, &url, &mut rt);
        assert_eq!(result, ClickResult::Handled);
    }

    #[test]
    fn focus_change_sets_element_state_flags() {
        let mut doc = vex_dom::Document::new();
        let html = doc.create_element("html", vex_dom::Namespace::Html);
        let body = doc.create_element("body", vex_dom::Namespace::Html);
        let input = doc.create_element("input", vex_dom::Namespace::Html);
        doc.append_child(doc.root(), html);
        doc.append_child(html, body);
        doc.append_child(body, input);

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();
        let changed = process_focus_change(input, None, &shared, &mut rt);
        assert_eq!(changed, Some(input));

        let doc_ref = shared.borrow();
        let input_node = doc_ref.arena().get(input);
        let body_node = doc_ref.arena().get(body);
        let html_node = doc_ref.arena().get(html);

        let NodeData::Element(ref input_el) = input_node.data else {
            panic!("input must be element");
        };
        let NodeData::Element(ref body_el) = body_node.data else {
            panic!("body must be element");
        };
        let NodeData::Element(ref html_el) = html_node.data else {
            panic!("html must be element");
        };

        assert!(input_el.state.contains(ElementState::FOCUS));
        assert!(input_el.state.contains(ElementState::FOCUS_WITHIN));
        assert!(body_el.state.contains(ElementState::FOCUS_WITHIN));
        assert!(html_el.state.contains(ElementState::FOCUS_WITHIN));
    }

    #[test]
    fn checkbox_click_toggles_checked_state() {
        let mut doc = vex_dom::Document::new();
        let input = doc.create_element("input", vex_dom::Namespace::Html);
        {
            let node = doc.arena_mut().get_mut(input);
            if let NodeData::Element(ref mut el) = node.data {
                el.attributes.push(vex_dom::node::Attribute {
                    name: "type".to_string(),
                    value: "checkbox".to_string(),
                });
            }
        }
        doc.append_child(doc.root(), input);

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();
        let mut root = make_box(Some(input), 0.0, 0.0, 100.0, 40.0);
        root.box_type = BoxType::Inline;
        let scroll = ScrollState::new(800.0, 600.0);
        let url = VexUrl::parse("https://example.com").unwrap();

        let _ = process_click(10.0, 10.0, &root, &scroll, &shared, &url, &mut rt);

        let doc_ref = shared.borrow();
        let checked = doc_ref
            .form_states()
            .get(input)
            .map(|s| s.checked)
            .unwrap_or(false);
        assert!(checked);
    }

    #[test]
    fn key_on_text_input_updates_value() {
        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let input = doc.create_element("input", vex_dom::Namespace::Html);
        doc.append_child(root, input);

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();

        // Type 'A', 'B'
        let r1 = process_key_down(input, 0x41, Some('A'), &shared, &mut rt);
        assert_eq!(r1, KeyResult::InputChanged);

        let r2 = process_key_down(input, 0x42, Some('B'), &shared, &mut rt);
        assert_eq!(r2, KeyResult::InputChanged);

        let doc_ref = shared.borrow();
        let value = doc_ref.form_states().get(input).unwrap().value.clone();
        assert_eq!(value, "AB");
    }

    #[test]
    fn key_backspace_deletes_char() {
        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let input = doc.create_element("input", vex_dom::Namespace::Html);
        doc.append_child(root, input);

        // Pre-fill some text.
        doc.form_states_mut().insert(input, {
            let mut s = vex_dom::InputState::new_text(vex_dom::InputType::Text);
            s.value = "abc".to_string();
            s.set_cursor(3);
            s
        });

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();

        let r = process_key_down(input, 0x08, None, &shared, &mut rt);
        assert_eq!(r, KeyResult::InputChanged);

        let doc_ref = shared.borrow();
        assert_eq!(doc_ref.form_states().get(input).unwrap().value, "ab");
    }

    #[test]
    fn key_on_non_input_is_handled_only() {
        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let div = doc.create_element("div", vex_dom::Namespace::Html);
        doc.append_child(root, div);

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();

        let r = process_key_down(div, 0x41, Some('A'), &shared, &mut rt);
        assert_eq!(r, KeyResult::Handled);
    }

    #[test]
    fn arrow_keys_move_cursor_without_value_change() {
        let mut doc = vex_dom::Document::new();
        let root = doc.root();
        let input = doc.create_element("input", vex_dom::Namespace::Html);
        doc.append_child(root, input);

        doc.form_states_mut().insert(input, {
            let mut s = vex_dom::InputState::new_text(vex_dom::InputType::Text);
            s.value = "abc".to_string();
            s.set_cursor(2);
            s
        });

        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();

        // Left arrow — no value change.
        let r = process_key_down(input, 0x25, None, &shared, &mut rt);
        assert_eq!(r, KeyResult::Handled);

        let doc_ref = shared.borrow();
        assert_eq!(doc_ref.form_states().get(input).unwrap().selection_start, 1);
    }

    #[test]
    fn fire_page_lifecycle_does_not_panic() {
        let doc = vex_html::parse_html("<html><body>hello</body></html>");
        let shared = vex_js::shared_document(doc);
        let mut rt = JsRuntime::new();
        rt.register_document(&shared);

        // Should fire DOMContentLoaded + load without panicking.
        fire_page_lifecycle(&shared, &mut rt);
    }
}
