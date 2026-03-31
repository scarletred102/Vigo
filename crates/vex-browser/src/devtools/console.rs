// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Console panel — log messages, REPL input, object inspection.
//!
//! Captures output from `console.log/warn/error` calls in the JS runtime.
//! Provides a REPL for interactive JavaScript evaluation.
//! Objects/DOM nodes in output can be expanded into tree views.

use std::collections::VecDeque;
use std::time::SystemTime;

use vex_core::id::VexId;

/// Maximum number of log entries kept in the console buffer.
const MAX_LOG_ENTRIES: usize = 5000;

/// Severity level of a console message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// `console.log()`.
    Log,
    /// `console.info()`.
    Info,
    /// `console.warn()`.
    Warn,
    /// `console.error()`.
    Error,
    /// `console.debug()`.
    Debug,
    /// System message (e.g. navigation events, errors).
    System,
}

impl LogLevel {
    /// CSS-style color hint for rendering the log level.
    pub fn label(self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Debug => "debug",
            Self::System => "system",
        }
    }
}

/// A value that was logged or returned from the REPL.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsoleValue {
    /// Primitive or string value.
    String(String),
    /// A numeric value.
    Number(f64),
    /// Boolean.
    Bool(bool),
    /// null.
    Null,
    /// undefined.
    Undefined,
    /// An object with named properties (expandable in the inspector).
    Object {
        /// Type label (e.g. "Object", "Array", "HTMLDivElement").
        type_name: String,
        /// Property name → value pairs.
        properties: Vec<(String, ConsoleValue)>,
    },
    /// A reference to a DOM node.
    DomNode {
        /// The VexId of the node.
        node_id: VexId,
        /// Short label (e.g. `<div class="main">`).
        label: String,
    },
    /// An array of values.
    Array(Vec<ConsoleValue>),
}

impl ConsoleValue {
    /// One-line summary for display in the log list.
    pub fn summary(&self) -> String {
        match self {
            Self::String(s) => {
                if s.len() > 80 {
                    format!("\"{}…\"", &s[..80])
                } else {
                    format!("\"{s}\"")
                }
            }
            Self::Number(n) => format!("{n}"),
            Self::Bool(b) => format!("{b}"),
            Self::Null => "null".to_owned(),
            Self::Undefined => "undefined".to_owned(),
            Self::Object {
                type_name,
                properties,
            } => {
                format!("{type_name} {{{} props}}", properties.len())
            }
            Self::DomNode { label, .. } => label.clone(),
            Self::Array(items) => format!("Array({})", items.len()),
        }
    }

    /// Whether this value can be expanded in the object inspector.
    pub fn is_expandable(&self) -> bool {
        matches!(self, Self::Object { .. } | Self::Array(_))
    }
}

/// A single entry in the console log.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Unique sequential ID.
    pub id: u64,
    /// Timestamp when the message was logged.
    pub timestamp: SystemTime,
    /// Severity level.
    pub level: LogLevel,
    /// The logged values (one `console.log()` call can have multiple args).
    pub values: Vec<ConsoleValue>,
    /// Source location (optional — e.g. "script.js:42").
    pub source: Option<String>,
}

impl LogEntry {
    /// Formatted one-line summary of the entry.
    pub fn summary(&self) -> String {
        self.values
            .iter()
            .map(|v| v.summary())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// State for the Console panel.
#[derive(Debug, Clone)]
pub struct ConsoleState {
    /// Log entries in chronological order.
    entries: VecDeque<LogEntry>,
    /// Auto-incrementing entry ID.
    next_id: u64,
    /// REPL input history (most recent last).
    input_history: Vec<String>,
    /// Current history navigation index (None = typing new input).
    history_index: Option<usize>,
    /// Current REPL input text.
    input_text: String,
    /// Scroll offset (from bottom, in entries).
    scroll_offset: usize,
    /// Filter by log level (None = show all).
    level_filter: Option<LogLevel>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 1,
            input_history: Vec::new(),
            history_index: None,
            input_text: String::new(),
            scroll_offset: 0,
            level_filter: None,
        }
    }
}

impl ConsoleState {
    /// Create a new empty console state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a log entry from JS `console.*()` calls.
    pub fn log(&mut self, level: LogLevel, values: Vec<ConsoleValue>, source: Option<String>) {
        let entry = LogEntry {
            id: self.next_id,
            timestamp: SystemTime::now(),
            level,
            values,
            source,
        };
        self.next_id += 1;
        self.entries.push_back(entry);

        // Trim old entries if over capacity.
        while self.entries.len() > MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
    }

    /// Convenience: log a simple string message.
    pub fn log_message(&mut self, level: LogLevel, message: &str) {
        self.log(level, vec![ConsoleValue::String(message.to_owned())], None);
    }

    /// All log entries (filtered if a level filter is active).
    pub fn visible_entries(&self) -> Vec<&LogEntry> {
        match self.level_filter {
            Some(level) => self.entries.iter().filter(|e| e.level == level).collect(),
            None => self.entries.iter().collect(),
        }
    }

    /// Total number of entries (unfiltered).
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all log entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Set level filter.
    pub fn set_filter(&mut self, filter: Option<LogLevel>) {
        self.level_filter = filter;
    }

    /// Current level filter.
    pub fn filter(&self) -> Option<LogLevel> {
        self.level_filter
    }

    // ── REPL ──

    /// Current REPL input text.
    pub fn input_text(&self) -> &str {
        &self.input_text
    }

    /// Set the REPL input text.
    pub fn set_input_text(&mut self, text: String) {
        self.input_text = text;
        self.history_index = None;
    }

    /// Submit the current input for evaluation.
    /// Returns the input text and resets the input field.
    pub fn submit_input(&mut self) -> String {
        let text = std::mem::take(&mut self.input_text);
        if !text.trim().is_empty() {
            self.input_history.push(text.clone());
        }
        self.history_index = None;
        text
    }

    /// Navigate to the previous command in history (Up arrow).
    pub fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) if i > 0 => i - 1,
            Some(_) => 0,
            None => self.input_history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input_text = self.input_history[idx].clone();
    }

    /// Navigate to the next command in history (Down arrow).
    pub fn history_next(&mut self) {
        match self.history_index {
            Some(i) if i + 1 < self.input_history.len() => {
                let idx = i + 1;
                self.history_index = Some(idx);
                self.input_text = self.input_history[idx].clone();
            }
            Some(_) => {
                // Past end of history — clear input.
                self.history_index = None;
                self.input_text.clear();
            }
            None => {}
        }
    }

    /// Input history (for display/debugging).
    pub fn history(&self) -> &[String] {
        &self.input_history
    }

    /// Log a REPL input echo (shows the command the user typed).
    pub fn log_repl_input(&mut self, input: &str) {
        self.log(
            LogLevel::Log,
            vec![ConsoleValue::String(format!("> {input}"))],
            Some("REPL".to_owned()),
        );
    }

    /// Log a REPL result.
    pub fn log_repl_result(&mut self, value: ConsoleValue) {
        self.log(LogLevel::Log, vec![value], Some("REPL".to_owned()));
    }

    /// Log a REPL error.
    pub fn log_repl_error(&mut self, message: &str) {
        self.log(
            LogLevel::Error,
            vec![ConsoleValue::String(message.to_owned())],
            Some("REPL".to_owned()),
        );
    }

    /// Scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_console_is_empty() {
        let console = ConsoleState::new();
        assert_eq!(console.entry_count(), 0);
        assert!(console.input_text().is_empty());
    }

    #[test]
    fn log_adds_entries() {
        let mut console = ConsoleState::new();
        console.log_message(LogLevel::Log, "hello");
        console.log_message(LogLevel::Warn, "careful");
        assert_eq!(console.entry_count(), 2);
    }

    #[test]
    fn log_entries_have_sequential_ids() {
        let mut console = ConsoleState::new();
        console.log_message(LogLevel::Log, "a");
        console.log_message(LogLevel::Log, "b");
        let entries = console.visible_entries();
        assert_eq!(entries[0].id, 1);
        assert_eq!(entries[1].id, 2);
    }

    #[test]
    fn clear_removes_all() {
        let mut console = ConsoleState::new();
        console.log_message(LogLevel::Log, "test");
        console.clear();
        assert_eq!(console.entry_count(), 0);
    }

    #[test]
    fn level_filter() {
        let mut console = ConsoleState::new();
        console.log_message(LogLevel::Log, "normal");
        console.log_message(LogLevel::Error, "bad");
        console.log_message(LogLevel::Warn, "warning");

        console.set_filter(Some(LogLevel::Error));
        let visible = console.visible_entries();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].level, LogLevel::Error);
    }

    #[test]
    fn repl_submit_and_history() {
        let mut console = ConsoleState::new();
        console.set_input_text("1 + 1".to_owned());
        let input = console.submit_input();
        assert_eq!(input, "1 + 1");
        assert!(console.input_text().is_empty());
        assert_eq!(console.history().len(), 1);
    }

    #[test]
    fn repl_history_navigation() {
        let mut console = ConsoleState::new();
        console.set_input_text("first".to_owned());
        console.submit_input();
        console.set_input_text("second".to_owned());
        console.submit_input();

        // Up arrow → last command.
        console.history_prev();
        assert_eq!(console.input_text(), "second");
        // Up again → first command.
        console.history_prev();
        assert_eq!(console.input_text(), "first");
        // Down → back to second.
        console.history_next();
        assert_eq!(console.input_text(), "second");
        // Down → past end, clears input.
        console.history_next();
        assert!(console.input_text().is_empty());
    }

    #[test]
    fn empty_input_not_added_to_history() {
        let mut console = ConsoleState::new();
        console.set_input_text("  ".to_owned());
        console.submit_input();
        assert!(console.history().is_empty());
    }

    #[test]
    fn max_entries_trimmed() {
        let mut console = ConsoleState::new();
        for i in 0..6000 {
            console.log_message(LogLevel::Log, &format!("msg {i}"));
        }
        assert!(console.entry_count() <= MAX_LOG_ENTRIES);
    }

    // ── ConsoleValue Tests ──

    #[test]
    fn string_value_summary() {
        let v = ConsoleValue::String("hello".to_owned());
        assert_eq!(v.summary(), "\"hello\"");
    }

    #[test]
    fn object_value_expandable() {
        let v = ConsoleValue::Object {
            type_name: "Object".to_owned(),
            properties: vec![
                ("x".to_owned(), ConsoleValue::Number(1.0)),
                ("y".to_owned(), ConsoleValue::Number(2.0)),
            ],
        };
        assert!(v.is_expandable());
        assert!(v.summary().contains("2 props"));
    }

    #[test]
    fn array_value_summary() {
        let v = ConsoleValue::Array(vec![
            ConsoleValue::Number(1.0),
            ConsoleValue::Number(2.0),
            ConsoleValue::Number(3.0),
        ]);
        assert_eq!(v.summary(), "Array(3)");
        assert!(v.is_expandable());
    }

    #[test]
    fn dom_node_value() {
        let v = ConsoleValue::DomNode {
            node_id: VexId::new(42),
            label: "<div class=\"main\">".to_owned(),
        };
        assert!(!v.is_expandable());
        assert!(v.summary().contains("<div"));
    }

    #[test]
    fn null_and_undefined() {
        assert_eq!(ConsoleValue::Null.summary(), "null");
        assert_eq!(ConsoleValue::Undefined.summary(), "undefined");
        assert!(!ConsoleValue::Null.is_expandable());
    }

    #[test]
    fn entry_summary_joins_values() {
        let entry = LogEntry {
            id: 1,
            timestamp: SystemTime::now(),
            level: LogLevel::Log,
            values: vec![
                ConsoleValue::String("count:".to_owned()),
                ConsoleValue::Number(42.0),
            ],
            source: None,
        };
        let summary = entry.summary();
        assert!(summary.contains("count:"));
        assert!(summary.contains("42"));
    }

    #[test]
    fn log_level_labels() {
        assert_eq!(LogLevel::Log.label(), "log");
        assert_eq!(LogLevel::Warn.label(), "warn");
        assert_eq!(LogLevel::Error.label(), "error");
        assert_eq!(LogLevel::System.label(), "system");
    }

    #[test]
    fn repl_log_methods() {
        let mut console = ConsoleState::new();
        console.log_repl_input("2 + 2");
        console.log_repl_result(ConsoleValue::Number(4.0));
        console.log_repl_error("ReferenceError: x is not defined");
        assert_eq!(console.entry_count(), 3);
    }
}
