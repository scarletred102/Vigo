// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Form control rendering — generates display commands for `<input>`,
//! `<button>`, `<textarea>`, and `<select>` elements.
//!
//! These are simplified "native-style" controls — not pixel-perfect replicas
//! of OS widgets, but recognisable and functional.

use vex_core::color::Color;
use vex_core::geometry::{Insets, Point, Rect};
use vex_dom::forms::{FormElementKind, InputState, InputType};
use vex_layout::TextEngine;

use crate::display_list::{DisplayCommand, DisplayList, RenderBorderStyle};

/// Default colors for form controls (light theme).
const INPUT_BACKGROUND: Color = Color::rgb(255, 255, 255);
const INPUT_BORDER: Color = Color::rgb(169, 169, 169);
const INPUT_TEXT: Color = Color::rgb(0, 0, 0);
const PLACEHOLDER_COLOR: Color = Color::rgb(169, 169, 169);
const CURSOR_COLOR: Color = Color::rgb(0, 0, 0);
const CHECKBOX_CHECK: Color = Color::rgb(0, 120, 212);
const BUTTON_BACKGROUND: Color = Color::rgb(240, 240, 240);
const BUTTON_BORDER: Color = Color::rgb(204, 204, 204);
const FOCUS_BORDER: Color = Color::rgb(0, 120, 212);

/// Default dimensions for form controls.
const INPUT_FONT_SIZE: f32 = 14.0;
const CHECKBOX_SIZE: f32 = 16.0;
const RADIO_SIZE: f32 = 16.0;
const CURSOR_WIDTH: f32 = 1.0;
const BORDER_WIDTH: f32 = 1.0;
const INPUT_PADDING: f32 = 4.0;

/// Paint a form control into the display list based on its state.
///
/// The `rect` is the layout box content area for the form element.
/// Pass a `TextEngine` for accurate cursor positioning; if `None`, a
/// monospace heuristic is used.
pub fn paint_form_control(
    state: &InputState,
    rect: Rect,
    focused: bool,
    dl: &mut DisplayList,
    text_engine: Option<&mut TextEngine>,
) {
    match state.kind {
        FormElementKind::Input(input_type) => {
            paint_input(state, input_type, rect, focused, dl, text_engine);
        }
        FormElementKind::Textarea => {
            paint_textarea(state, rect, focused, dl, text_engine);
        }
        FormElementKind::Select => {
            paint_select(state, rect, focused, dl);
        }
        FormElementKind::Button => {
            paint_button(state, rect, focused, dl);
        }
    }
}

/// Paint an `<input>` element.
fn paint_input(
    state: &InputState,
    input_type: InputType,
    rect: Rect,
    focused: bool,
    dl: &mut DisplayList,
    text_engine: Option<&mut TextEngine>,
) {
    match input_type {
        InputType::Checkbox => paint_checkbox(state, rect, focused, dl),
        InputType::Radio => paint_radio(state, rect, focused, dl),
        InputType::Submit | InputType::Reset | InputType::Button => {
            paint_button(state, rect, focused, dl);
        }
        InputType::Hidden => {} // No visual output.
        _ => paint_text_input(state, rect, focused, dl, text_engine),
    }
}

/// Paint a text-like input (`text`, `password`, `email`, `url`, etc.).
fn paint_text_input(
    state: &InputState,
    rect: Rect,
    focused: bool,
    dl: &mut DisplayList,
    text_engine: Option<&mut TextEngine>,
) {
    let border_color = if focused { FOCUS_BORDER } else { INPUT_BORDER };

    // Background.
    dl.push(DisplayCommand::FillRect {
        rect,
        color: INPUT_BACKGROUND,
        border_radius: 0.0,
    });

    // Border.
    dl.push(DisplayCommand::DrawBorder {
        rect,
        widths: Insets::uniform(BORDER_WIDTH),
        colors: [border_color; 4],
        styles: [RenderBorderStyle::Solid; 4],
    });

    // Clip to content area.
    let content_rect = Rect::new(
        rect.origin.x + INPUT_PADDING,
        rect.origin.y,
        (rect.size.width - INPUT_PADDING * 2.0).max(0.0),
        rect.size.height,
    );
    dl.push(DisplayCommand::PushClip { rect: content_rect });

    // Text or placeholder.
    let (text, color) = if state.value.is_empty() && !state.placeholder.is_empty() {
        (state.placeholder.as_str(), PLACEHOLDER_COLOR)
    } else if matches!(state.kind, FormElementKind::Input(InputType::Password)) {
        // Password masking is handled by the display text.
        ("", INPUT_TEXT) // We'll push masked text below.
    } else {
        (state.value.as_str(), INPUT_TEXT)
    };

    // For password fields, show dots.
    let display_text: String;
    let final_text = if matches!(state.kind, FormElementKind::Input(InputType::Password)) {
        display_text = "•".repeat(state.value.chars().count());
        display_text.as_str()
    } else {
        text
    };

    if !final_text.is_empty() {
        let text_y = content_rect.origin.y + (rect.size.height + INPUT_FONT_SIZE) / 2.0
            - INPUT_FONT_SIZE * 0.2;
        dl.push(DisplayCommand::DrawText {
            position: Point::new(content_rect.origin.x, text_y),
            text: final_text.to_string(),
            color,
            font_size: INPUT_FONT_SIZE,
            line_height: INPUT_FONT_SIZE * 1.2,
        });
    }

    // Cursor (only when focused and no selection).
    if focused && !state.has_selection() {
        let cursor_x =
            content_rect.origin.x + measure_cursor_x(state, INPUT_FONT_SIZE, text_engine);
        let cursor_y = content_rect.origin.y + 2.0;
        let cursor_h = rect.size.height - 4.0;

        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(cursor_x, cursor_y, CURSOR_WIDTH, cursor_h.max(0.0)),
            color: CURSOR_COLOR,
            border_radius: 0.0,
        });
    }

    dl.push(DisplayCommand::PopClip);
}

/// Paint a checkbox.
fn paint_checkbox(state: &InputState, rect: Rect, focused: bool, dl: &mut DisplayList) {
    let size = CHECKBOX_SIZE.min(rect.size.width).min(rect.size.height);
    let x = rect.origin.x + (rect.size.width - size) / 2.0;
    let y = rect.origin.y + (rect.size.height - size) / 2.0;
    let box_rect = Rect::new(x, y, size, size);
    let border_color = if focused { FOCUS_BORDER } else { INPUT_BORDER };

    // Box background.
    dl.push(DisplayCommand::FillRect {
        rect: box_rect,
        color: if state.checked {
            CHECKBOX_CHECK
        } else {
            INPUT_BACKGROUND
        },
        border_radius: 0.0,
    });

    // Box border.
    dl.push(DisplayCommand::DrawBorder {
        rect: box_rect,
        widths: Insets::uniform(BORDER_WIDTH),
        colors: [border_color; 4],
        styles: [RenderBorderStyle::Solid; 4],
    });

    // Checkmark (simple: two lines approximated as thin rects).
    if state.checked {
        let check_color = Color::WHITE;
        let inset = size * 0.25;
        let cx = x + inset;
        let cy = y + size * 0.5;

        // Short stroke (down-left).
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(cx, cy, size * 0.2, size * 0.3),
            color: check_color,
            border_radius: 0.0,
        });
        // Long stroke (up-right).
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(cx + size * 0.15, cy - size * 0.1, size * 0.35, size * 0.15),
            color: check_color,
            border_radius: 0.0,
        });
    }
}

/// Paint a radio button.
fn paint_radio(state: &InputState, rect: Rect, focused: bool, dl: &mut DisplayList) {
    let size = RADIO_SIZE.min(rect.size.width).min(rect.size.height);
    let x = rect.origin.x + (rect.size.width - size) / 2.0;
    let y = rect.origin.y + (rect.size.height - size) / 2.0;
    let radio_rect = Rect::new(x, y, size, size);
    let border_color = if focused { FOCUS_BORDER } else { INPUT_BORDER };

    // Outer circle (using border_radius = half size for a perfect circle).
    dl.push(DisplayCommand::FillRect {
        rect: radio_rect,
        color: INPUT_BACKGROUND,
        border_radius: size / 2.0,
    });
    dl.push(DisplayCommand::DrawBorder {
        rect: radio_rect,
        widths: Insets::uniform(BORDER_WIDTH),
        colors: [border_color; 4],
        styles: [RenderBorderStyle::Solid; 4],
    });

    // Inner dot when selected.
    if state.checked {
        let dot_inset = size * 0.3;
        let dot_size = size - dot_inset * 2.0;
        dl.push(DisplayCommand::FillRect {
            rect: Rect::new(x + dot_inset, y + dot_inset, dot_size, dot_size),
            color: CHECKBOX_CHECK,
            border_radius: dot_size / 2.0,
        });
    }
}

/// Paint a `<button>` or submit/reset input.
fn paint_button(state: &InputState, rect: Rect, focused: bool, dl: &mut DisplayList) {
    let border_color = if focused { FOCUS_BORDER } else { BUTTON_BORDER };

    // Background.
    dl.push(DisplayCommand::FillRect {
        rect,
        color: BUTTON_BACKGROUND,
        border_radius: 0.0,
    });

    // Border.
    dl.push(DisplayCommand::DrawBorder {
        rect,
        widths: Insets::uniform(BORDER_WIDTH),
        colors: [border_color; 4],
        styles: [RenderBorderStyle::Solid; 4],
    });

    // Button label.
    let label = if state.value.is_empty() {
        match state.kind {
            FormElementKind::Input(InputType::Submit) => "Submit",
            FormElementKind::Input(InputType::Reset) => "Reset",
            _ => "Button",
        }
    } else {
        &state.value
    };

    let text_y = rect.origin.y + (rect.size.height + INPUT_FONT_SIZE) / 2.0 - INPUT_FONT_SIZE * 0.2;
    dl.push(DisplayCommand::DrawText {
        position: Point::new(rect.origin.x + INPUT_PADDING, text_y),
        text: label.to_string(),
        color: INPUT_TEXT,
        font_size: INPUT_FONT_SIZE,
        line_height: INPUT_FONT_SIZE * 1.2,
    });
}

/// Paint a `<textarea>`.
fn paint_textarea(
    state: &InputState,
    rect: Rect,
    focused: bool,
    dl: &mut DisplayList,
    text_engine: Option<&mut TextEngine>,
) {
    // Same as text input but taller.
    paint_text_input(state, rect, focused, dl, text_engine);
}

/// Paint a `<select>` (simplified dropdown stub).
fn paint_select(state: &InputState, rect: Rect, focused: bool, dl: &mut DisplayList) {
    let border_color = if focused { FOCUS_BORDER } else { INPUT_BORDER };

    // Background.
    dl.push(DisplayCommand::FillRect {
        rect,
        color: INPUT_BACKGROUND,
        border_radius: 0.0,
    });

    // Border.
    dl.push(DisplayCommand::DrawBorder {
        rect,
        widths: Insets::uniform(BORDER_WIDTH),
        colors: [border_color; 4],
        styles: [RenderBorderStyle::Solid; 4],
    });

    // Current value.
    let text = if state.value.is_empty() {
        &state.placeholder
    } else {
        &state.value
    };

    if !text.is_empty() {
        let text_y =
            rect.origin.y + (rect.size.height + INPUT_FONT_SIZE) / 2.0 - INPUT_FONT_SIZE * 0.2;
        dl.push(DisplayCommand::DrawText {
            position: Point::new(rect.origin.x + INPUT_PADDING, text_y),
            text: text.to_string(),
            color: INPUT_TEXT,
            font_size: INPUT_FONT_SIZE,
            line_height: INPUT_FONT_SIZE * 1.2,
        });
    }

    // Dropdown arrow (small triangle approximated as ▼ text).
    let arrow_x = rect.origin.x + rect.size.width - 16.0;
    let arrow_y =
        rect.origin.y + (rect.size.height + INPUT_FONT_SIZE) / 2.0 - INPUT_FONT_SIZE * 0.2;
    dl.push(DisplayCommand::DrawText {
        position: Point::new(arrow_x, arrow_y),
        text: "▾".to_string(),
        color: INPUT_TEXT,
        font_size: INPUT_FONT_SIZE,
        line_height: INPUT_FONT_SIZE,
    });
}

/// Measure cursor X offset using the text shaper when available.
///
/// Falls back to a monospace heuristic (~60% of font size per char) if no
/// `TextEngine` is provided.
fn measure_cursor_x(
    state: &InputState,
    font_size: f32,
    text_engine: Option<&mut TextEngine>,
) -> f32 {
    let text_before_cursor = if matches!(state.kind, FormElementKind::Input(InputType::Password)) {
        "•".repeat(state.value.chars().count())
    } else {
        state
            .value
            .get(..state.selection_start)
            .unwrap_or(&state.value)
            .to_string()
    };

    match text_engine {
        Some(engine) => {
            let (width, _) =
                engine.measure(&text_before_cursor, font_size, font_size * 1.2, f32::MAX);
            width
        }
        None => {
            // Heuristic fallback: ~60% of font size per character.
            text_before_cursor.chars().count() as f32 * font_size * 0.6
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn text_input(value: &str) -> InputState {
        let mut s = InputState::new_text(InputType::Text);
        s.value = value.to_string();
        s
    }

    fn rect() -> Rect {
        Rect::new(10.0, 20.0, 200.0, 24.0)
    }

    #[test]
    fn text_input_renders_background_border_text() {
        let state = text_input("hello");
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        // Background, border, clip, text, pop_clip = 5 commands
        assert!(dl.len() >= 4, "got {} commands", dl.len());

        // First command should be FillRect for background
        assert!(matches!(dl.commands()[0], DisplayCommand::FillRect { .. }));
    }

    #[test]
    fn text_input_focused_shows_cursor() {
        let mut state = text_input("hi");
        state.set_cursor(2);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), true, &mut dl, None);

        // Should have a cursor FillRect somewhere
        let cursor_fills: Vec<_> = dl
            .commands()
            .iter()
            .filter(|cmd| {
                matches!(
                    cmd,
                    DisplayCommand::FillRect {
                        color,
                        ..
                    } if color == &CURSOR_COLOR
                )
            })
            .collect();
        assert!(!cursor_fills.is_empty(), "expected cursor fill rect");
    }

    #[test]
    fn password_renders_dots() {
        let mut state = InputState::new_text(InputType::Password);
        state.value = "secret".to_string();
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        // Find DrawText command
        let text_cmds: Vec<_> = dl
            .commands()
            .iter()
            .filter_map(|cmd| {
                if let DisplayCommand::DrawText { text, .. } = cmd {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect();
        // Should show dots, not the actual password
        assert!(text_cmds.iter().any(|t| t.contains('•')));
        assert!(!text_cmds.iter().any(|t| t.contains("secret")));
    }

    #[test]
    fn checkbox_unchecked() {
        let state = InputState::new_checkbox(false);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        // Background + border = 2 minimum
        assert!(dl.len() >= 2);
    }

    #[test]
    fn checkbox_checked_has_check_mark() {
        let state = InputState::new_checkbox(true);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        // Checked: background(blue) + border + 2 check rects = 4
        assert!(dl.len() >= 4, "got {} commands", dl.len());
    }

    #[test]
    fn radio_selected_has_dot() {
        let state = InputState::new_radio(true);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        // Outer + border + inner dot = 3 minimum
        assert!(dl.len() >= 3);
    }

    #[test]
    fn button_renders_label() {
        let mut state = InputState::new_text(InputType::Submit);
        state.kind = FormElementKind::Input(InputType::Submit);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        let text_cmds: Vec<_> = dl
            .commands()
            .iter()
            .filter_map(|cmd| {
                if let DisplayCommand::DrawText { text, .. } = cmd {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(text_cmds.iter().any(|t| t == "Submit"));
    }

    #[test]
    fn select_renders_dropdown_arrow() {
        let mut state = InputState::new_select();
        state.value = "Option A".to_string();
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        let text_cmds: Vec<_> = dl
            .commands()
            .iter()
            .filter_map(|cmd| {
                if let DisplayCommand::DrawText { text, .. } = cmd {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(text_cmds.iter().any(|t| t.contains("▾")));
        assert!(text_cmds.iter().any(|t| t.contains("Option A")));
    }

    #[test]
    fn hidden_input_emits_nothing() {
        let mut state = InputState::new_text(InputType::Hidden);
        state.kind = FormElementKind::Input(InputType::Hidden);
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);
        assert!(dl.is_empty());
    }

    #[test]
    fn placeholder_shown_when_empty() {
        let mut state = text_input("");
        state.placeholder = "Enter text...".to_string();
        let mut dl = DisplayList::default();

        paint_form_control(&state, rect(), false, &mut dl, None);

        let text_cmds: Vec<_> = dl
            .commands()
            .iter()
            .filter_map(|cmd| {
                if let DisplayCommand::DrawText { text, color, .. } = cmd {
                    Some((text.clone(), *color))
                } else {
                    None
                }
            })
            .collect();
        assert!(text_cmds
            .iter()
            .any(|(t, c)| t == "Enter text..." && *c == PLACEHOLDER_COLOR));
    }
}
