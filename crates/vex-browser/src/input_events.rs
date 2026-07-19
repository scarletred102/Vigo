// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Input Events Level 2 and Keyboard Events implementation.
//!
//! Covers `beforeinput`, `input`, `keydown`, `keyup`, `keypress` events
//! and the InputEvent/KeyboardEvent interfaces.
//! <https://www.w3.org/TR/input-events-2/>
//! <https://www.w3.org/TR/uievents/#events-keyboardevents>

use std::time::Instant;

// ── Input Event ──────────────────────────────────────────────────────────────

/// The `inputType` values from Input Events Level 2.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputType {
    /// Typing a character.
    InsertText,
    /// Inserting a paragraph break (Enter).
    InsertParagraph,
    /// Inserting a line break (Shift+Enter).
    InsertLineBreak,
    /// Inserting from paste.
    InsertFromPaste,
    /// Inserting from drop.
    InsertFromDrop,
    /// Inserting from yank (Ctrl+Y).
    InsertFromYank,
    /// Insert a link.
    InsertLink,
    /// Replace content.
    InsertReplacementText,
    /// Delete content backward (Backspace).
    DeleteContentBackward,
    /// Delete content forward (Delete).
    DeleteContentForward,
    /// Delete a word backward.
    DeleteWordBackward,
    /// Delete a word forward.
    DeleteWordForward,
    /// Delete a soft line backward.
    DeleteSoftLineBackward,
    /// Delete a soft line forward.
    DeleteSoftLineForward,
    /// Delete to end of line.
    DeleteHardLineForward,
    /// Delete to start of line.
    DeleteHardLineBackward,
    /// Delete all.
    DeleteContent,
    /// Delete by drag.
    DeleteByDrag,
    /// Delete by cut.
    DeleteByCut,
    /// Undo.
    HistoryUndo,
    /// Redo.
    HistoryRedo,
    /// Bold formatting.
    FormatBold,
    /// Italic formatting.
    FormatItalic,
    /// Underline formatting.
    FormatUnderline,
    /// Strikethrough formatting.
    FormatStrikeThrough,
    /// Set text to superscript.
    FormatSuperscript,
    /// Set text to subscript.
    FormatSubscript,
    /// Custom / unknown input type.
    Other(String),
}

impl InputType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::InsertText => "insertText",
            Self::InsertParagraph => "insertParagraph",
            Self::InsertLineBreak => "insertLineBreak",
            Self::InsertFromPaste => "insertFromPaste",
            Self::InsertFromDrop => "insertFromDrop",
            Self::InsertFromYank => "insertFromYank",
            Self::InsertLink => "insertLink",
            Self::InsertReplacementText => "insertReplacementText",
            Self::DeleteContentBackward => "deleteContentBackward",
            Self::DeleteContentForward => "deleteContentForward",
            Self::DeleteWordBackward => "deleteWordBackward",
            Self::DeleteWordForward => "deleteWordForward",
            Self::DeleteSoftLineBackward => "deleteSoftLineBackward",
            Self::DeleteSoftLineForward => "deleteSoftLineForward",
            Self::DeleteHardLineForward => "deleteHardLineForward",
            Self::DeleteHardLineBackward => "deleteHardLineBackward",
            Self::DeleteContent => "deleteContent",
            Self::DeleteByDrag => "deleteByDrag",
            Self::DeleteByCut => "deleteByCut",
            Self::HistoryUndo => "historyUndo",
            Self::HistoryRedo => "historyRedo",
            Self::FormatBold => "formatBold",
            Self::FormatItalic => "formatItalic",
            Self::FormatUnderline => "formatUnderline",
            Self::FormatStrikeThrough => "formatStrikeThrough",
            Self::FormatSuperscript => "formatSuperscript",
            Self::FormatSubscript => "formatSubscript",
            Self::Other(s) => s.as_str(),
        }
    }

    pub fn from_name(s: &str) -> Self {
        match s {
            "insertText" => Self::InsertText,
            "insertParagraph" => Self::InsertParagraph,
            "insertLineBreak" => Self::InsertLineBreak,
            "insertFromPaste" => Self::InsertFromPaste,
            "insertFromDrop" => Self::InsertFromDrop,
            "insertFromYank" => Self::InsertFromYank,
            "insertLink" => Self::InsertLink,
            "insertReplacementText" => Self::InsertReplacementText,
            "deleteContentBackward" => Self::DeleteContentBackward,
            "deleteContentForward" => Self::DeleteContentForward,
            "deleteWordBackward" => Self::DeleteWordBackward,
            "deleteWordForward" => Self::DeleteWordForward,
            "deleteSoftLineBackward" => Self::DeleteSoftLineBackward,
            "deleteSoftLineForward" => Self::DeleteSoftLineForward,
            "deleteHardLineForward" => Self::DeleteHardLineForward,
            "deleteHardLineBackward" => Self::DeleteHardLineBackward,
            "deleteContent" => Self::DeleteContent,
            "deleteByDrag" => Self::DeleteByDrag,
            "deleteByCut" => Self::DeleteByCut,
            "historyUndo" => Self::HistoryUndo,
            "historyRedo" => Self::HistoryRedo,
            "formatBold" => Self::FormatBold,
            "formatItalic" => Self::FormatItalic,
            "formatUnderline" => Self::FormatUnderline,
            "formatStrikeThrough" => Self::FormatStrikeThrough,
            "formatSuperscript" => Self::FormatSuperscript,
            "formatSubscript" => Self::FormatSubscript,
            other => Self::Other(other.to_string()),
        }
    }

    /// Whether this is a deletion input type.
    pub fn is_deletion(&self) -> bool {
        matches!(
            self,
            Self::DeleteContentBackward
                | Self::DeleteContentForward
                | Self::DeleteWordBackward
                | Self::DeleteWordForward
                | Self::DeleteSoftLineBackward
                | Self::DeleteSoftLineForward
                | Self::DeleteHardLineForward
                | Self::DeleteHardLineBackward
                | Self::DeleteContent
                | Self::DeleteByDrag
                | Self::DeleteByCut
        )
    }

    /// Whether this is an insertion input type.
    pub fn is_insertion(&self) -> bool {
        matches!(
            self,
            Self::InsertText
                | Self::InsertParagraph
                | Self::InsertLineBreak
                | Self::InsertFromPaste
                | Self::InsertFromDrop
                | Self::InsertFromYank
                | Self::InsertLink
                | Self::InsertReplacementText
        )
    }

    /// Whether this is a formatting command.
    pub fn is_format(&self) -> bool {
        matches!(
            self,
            Self::FormatBold
                | Self::FormatItalic
                | Self::FormatUnderline
                | Self::FormatStrikeThrough
                | Self::FormatSuperscript
                | Self::FormatSubscript
        )
    }
}

/// An InputEvent as dispatched to editable content.
#[derive(Debug, Clone)]
pub struct InputEvent {
    /// The type of input.
    pub input_type: InputType,
    /// The data being inserted (e.g., the character typed). Null for deletions.
    pub data: Option<String>,
    /// Whether the event is composing (IME).
    pub is_composing: bool,
    /// Whether the event has been cancelled (via `preventDefault()`).
    pub cancelled: bool,
    /// Timestamp.
    pub timestamp: Instant,
}

impl InputEvent {
    pub fn new(input_type: InputType, data: Option<String>) -> Self {
        Self {
            input_type,
            data,
            is_composing: false,
            cancelled: false,
            timestamp: Instant::now(),
        }
    }

    /// Create a beforeinput event for typing a character.
    pub fn before_insert_text(text: &str) -> Self {
        Self::new(InputType::InsertText, Some(text.to_string()))
    }

    /// Create a beforeinput event for deletion.
    pub fn before_delete(input_type: InputType) -> Self {
        Self::new(input_type, None)
    }

    pub fn prevent_default(&mut self) {
        self.cancelled = true;
    }
}

// ── Keyboard Events ──────────────────────────────────────────────────────────

/// Modifier keys state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModifierState {
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
}

impl ModifierState {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_ctrl() -> Self {
        Self {
            ctrl: true,
            ..Default::default()
        }
    }

    pub fn with_shift() -> Self {
        Self {
            shift: true,
            ..Default::default()
        }
    }

    pub fn with_alt() -> Self {
        Self {
            alt: true,
            ..Default::default()
        }
    }

    /// Check if any modifier is pressed.
    pub fn any(&self) -> bool {
        self.alt || self.ctrl || self.meta || self.shift
    }

    /// Get modifier state for a named modifier.
    pub fn get_modifier(&self, name: &str) -> bool {
        match name {
            "Alt" => self.alt,
            "Control" => self.ctrl,
            "Meta" => self.meta,
            "Shift" => self.shift,
            _ => false,
        }
    }
}

/// Location of a key on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyLocation {
    #[default]
    Standard,
    Left,
    Right,
    Numpad,
}

impl KeyLocation {
    pub fn value(&self) -> u32 {
        match self {
            Self::Standard => 0,
            Self::Left => 1,
            Self::Right => 2,
            Self::Numpad => 3,
        }
    }
}

/// Type of keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    KeyDown,
    KeyUp,
    KeyPress,
}

/// A KeyboardEvent.
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    /// Event type (keydown, keyup, keypress).
    pub event_type: KeyEventType,
    /// The key value (e.g., "a", "Enter", "ArrowDown").
    pub key: String,
    /// The key code (e.g., "KeyA", "Enter", "ArrowDown").
    pub code: String,
    /// Modifier state.
    pub modifiers: ModifierState,
    /// Key location.
    pub location: KeyLocation,
    /// Whether this is a repeat event.
    pub repeat: bool,
    /// Whether this is during IME composition.
    pub is_composing: bool,
    /// Whether the event has been cancelled.
    pub cancelled: bool,
    /// Timestamp.
    pub timestamp: Instant,
}

impl KeyboardEvent {
    pub fn new(event_type: KeyEventType, key: &str, code: &str) -> Self {
        Self {
            event_type,
            key: key.to_string(),
            code: code.to_string(),
            modifiers: ModifierState::none(),
            location: KeyLocation::Standard,
            repeat: false,
            is_composing: false,
            cancelled: false,
            timestamp: Instant::now(),
        }
    }

    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn with_location(mut self, location: KeyLocation) -> Self {
        self.location = location;
        self
    }

    pub fn with_repeat(mut self) -> Self {
        self.repeat = true;
        self
    }

    pub fn prevent_default(&mut self) {
        self.cancelled = true;
    }

    /// Whether this is a printable character key.
    pub fn is_printable(&self) -> bool {
        self.key.len() == 1 && !self.modifiers.ctrl && !self.modifiers.meta
    }

    /// Convert a keyboard shortcut to an input type (for rich text editing).
    pub fn to_input_type(&self) -> Option<InputType> {
        if !self.modifiers.ctrl {
            return None;
        }
        match self.key.as_str() {
            "b" | "B" => Some(InputType::FormatBold),
            "i" | "I" => Some(InputType::FormatItalic),
            "u" | "U" => Some(InputType::FormatUnderline),
            "z" | "Z" if !self.modifiers.shift => Some(InputType::HistoryUndo),
            "z" | "Z" if self.modifiers.shift => Some(InputType::HistoryRedo),
            "y" | "Y" => Some(InputType::HistoryRedo),
            _ => None,
        }
    }
}

// ── Composition Events ───────────────────────────────────────────────────────

/// Type of composition event (IME).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionEventType {
    CompositionStart,
    CompositionUpdate,
    CompositionEnd,
}

/// A CompositionEvent (for IME input).
#[derive(Debug, Clone)]
pub struct CompositionEvent {
    pub event_type: CompositionEventType,
    pub data: String,
    pub timestamp: Instant,
}

impl CompositionEvent {
    pub fn new(event_type: CompositionEventType, data: &str) -> Self {
        Self {
            event_type,
            data: data.to_string(),
            timestamp: Instant::now(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_type_roundtrip() {
        for (s, expected) in [
            ("insertText", InputType::InsertText),
            ("deleteContentBackward", InputType::DeleteContentBackward),
            ("formatBold", InputType::FormatBold),
            ("historyUndo", InputType::HistoryUndo),
        ] {
            let parsed = InputType::from_name(s);
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), s);
        }
    }

    #[test]
    fn input_type_classification() {
        assert!(InputType::InsertText.is_insertion());
        assert!(!InputType::InsertText.is_deletion());
        assert!(InputType::DeleteContentBackward.is_deletion());
        assert!(!InputType::DeleteContentBackward.is_insertion());
        assert!(InputType::FormatBold.is_format());
        assert!(!InputType::FormatBold.is_deletion());
    }

    #[test]
    fn input_event_creation() {
        let event = InputEvent::before_insert_text("a");
        assert_eq!(event.input_type, InputType::InsertText);
        assert_eq!(event.data, Some("a".to_string()));
        assert!(!event.is_composing);
        assert!(!event.cancelled);
    }

    #[test]
    fn input_event_prevent_default() {
        let mut event = InputEvent::before_delete(InputType::DeleteContentBackward);
        assert!(!event.cancelled);
        event.prevent_default();
        assert!(event.cancelled);
    }

    #[test]
    fn keyboard_event_create() {
        let event = KeyboardEvent::new(KeyEventType::KeyDown, "a", "KeyA");
        assert_eq!(event.key, "a");
        assert_eq!(event.code, "KeyA");
        assert_eq!(event.event_type, KeyEventType::KeyDown);
        assert!(!event.repeat);
    }

    #[test]
    fn keyboard_event_modifiers() {
        let event = KeyboardEvent::new(KeyEventType::KeyDown, "a", "KeyA")
            .with_modifiers(ModifierState::with_ctrl());
        assert!(event.modifiers.ctrl);
        assert!(!event.modifiers.alt);
    }

    #[test]
    fn keyboard_event_printable() {
        let plain = KeyboardEvent::new(KeyEventType::KeyDown, "a", "KeyA");
        assert!(plain.is_printable());

        let control = KeyboardEvent::new(KeyEventType::KeyDown, "a", "KeyA")
            .with_modifiers(ModifierState::with_ctrl());
        assert!(!control.is_printable());

        let enter = KeyboardEvent::new(KeyEventType::KeyDown, "Enter", "Enter");
        assert!(!enter.is_printable());
    }

    #[test]
    fn keyboard_shortcut_to_input_type() {
        let bold = KeyboardEvent::new(KeyEventType::KeyDown, "b", "KeyB")
            .with_modifiers(ModifierState::with_ctrl());
        assert_eq!(bold.to_input_type(), Some(InputType::FormatBold));

        let undo = KeyboardEvent::new(KeyEventType::KeyDown, "z", "KeyZ")
            .with_modifiers(ModifierState::with_ctrl());
        assert_eq!(undo.to_input_type(), Some(InputType::HistoryUndo));

        let redo =
            KeyboardEvent::new(KeyEventType::KeyDown, "z", "KeyZ").with_modifiers(ModifierState {
                ctrl: true,
                shift: true,
                ..Default::default()
            });
        assert_eq!(redo.to_input_type(), Some(InputType::HistoryRedo));

        let plain = KeyboardEvent::new(KeyEventType::KeyDown, "a", "KeyA");
        assert_eq!(plain.to_input_type(), None);
    }

    #[test]
    fn modifier_state() {
        let none = ModifierState::none();
        assert!(!none.any());

        let ctrl = ModifierState::with_ctrl();
        assert!(ctrl.any());
        assert!(ctrl.get_modifier("Control"));
        assert!(!ctrl.get_modifier("Alt"));
    }

    #[test]
    fn key_location_values() {
        assert_eq!(KeyLocation::Standard.value(), 0);
        assert_eq!(KeyLocation::Left.value(), 1);
        assert_eq!(KeyLocation::Right.value(), 2);
        assert_eq!(KeyLocation::Numpad.value(), 3);
    }

    #[test]
    fn composition_event() {
        let event = CompositionEvent::new(CompositionEventType::CompositionStart, "");
        assert_eq!(event.event_type, CompositionEventType::CompositionStart);

        let update = CompositionEvent::new(CompositionEventType::CompositionUpdate, "日");
        assert_eq!(update.data, "日");
    }
}
