// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Embedder message protocol — Servo-inspired message-passing API between
//! the rendering engine and the browser UI (embedder) layer.
//!
//! The engine sends [`EmbedderMsg`] variants to notify the embedder of state
//! changes (title, URL, loading status, history, etc.) so the toolbar and tab
//! bar can update without polling.
//!
//! The embedder sends [`EmbedderCommand`] variants to request actions from
//! the engine (navigate, go back, reload, open tab, etc.).
//!
//! This design is taken from Servo's `components/shared/embedder/lib.rs` and
//! adapted for the Vex engine's type system.

use vex_core::VexUrl;

use std::collections::VecDeque;

use crate::tab::TabId;

/// Servo-style bridge between embedder UI and engine.
///
/// The embedder pushes [`EmbedderCommand`] values for engine execution,
/// and the engine pushes [`EmbedderMsg`] values back for UI updates.
#[derive(Debug, Default)]
pub struct EmbedderBus {
    commands: VecDeque<EmbedderCommand>,
    messages: VecDeque<EmbedderMsg>,
}

impl EmbedderBus {
    /// Create an empty embedder bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a command from UI → engine.
    pub fn push_command(&mut self, command: EmbedderCommand) {
        self.commands.push_back(command);
    }

    /// Queue a message from engine → UI.
    pub fn push_message(&mut self, message: EmbedderMsg) {
        self.messages.push_back(message);
    }

    /// Pop the next queued command (FIFO).
    pub fn pop_command(&mut self) -> Option<EmbedderCommand> {
        self.commands.pop_front()
    }

    /// Pop the next queued message (FIFO).
    pub fn pop_message(&mut self) -> Option<EmbedderMsg> {
        self.messages.pop_front()
    }

    /// Whether any pending commands exist.
    #[must_use]
    pub fn has_commands(&self) -> bool {
        !self.commands.is_empty()
    }

    /// Whether any pending messages exist.
    #[must_use]
    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Total queued item count (commands + messages).
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len() + self.messages.len()
    }

    /// Whether the bus has no queued items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.messages.is_empty()
    }
}

// ──── Engine → Embedder (notifications) ──────────────────────────────

/// Messages the engine sends to the embedder (browser UI) layer.
///
/// Modeled after Servo's `EmbedderMsg` — the engine never touches UI
/// directly; it emits these messages and the embedder reacts.
#[derive(Debug, Clone)]
pub enum EmbedderMsg {
    /// The page title changed (e.g. `<title>` parsed or JS `document.title` set).
    TitleChanged(TabId, Option<String>),

    /// The current URL changed (navigation committed).
    UrlChanged(TabId, VexUrl),

    /// The loading status changed (started / head parsed / complete).
    LoadStatusChanged(TabId, LoadStatus),

    /// The navigation history changed — new list of URLs + current index.
    HistoryChanged(TabId, Vec<VexUrl>, usize),

    /// A new favicon was detected for the tab.
    FaviconChanged(TabId, Vec<u8>),

    /// Status bar text (e.g. hovered link URL).
    StatusText(TabId, Option<String>),

    /// The cursor shape should change.
    CursorChanged(TabId, CursorKind),

    /// The tab was closed by the engine (e.g. `window.close()`).
    TabClosed(TabId),

    /// The engine requests permission from the user (camera, location, etc.).
    PermissionRequest(TabId, PermissionKind),

    /// A JavaScript `alert()`, `confirm()`, or `prompt()` dialog.
    ShowDialog(TabId, DialogRequest),

    /// Console API message (from `console.log` etc.).
    ConsoleMessage(TabId, ConsoleLevel, String),

    /// A content script or extension wants to open a new tab.
    OpenNewTab(VexUrl),

    /// Fullscreen state changed (entered or exited).
    FullscreenChanged(TabId, bool),

    /// The engine has fully shut down.
    ShutdownComplete,
}

// ──── Embedder → Engine (commands) ───────────────────────────────────

/// Commands the embedder sends to the engine.
///
/// Modeled after Servo's embedder → constellation flow.
#[derive(Debug, Clone)]
pub enum EmbedderCommand {
    /// Navigate a tab to a URL.
    Navigate(TabId, VexUrl),

    /// Go back in history.
    GoBack(TabId),

    /// Go forward in history.
    GoForward(TabId),

    /// Reload the current page.
    Reload(TabId),

    /// Stop loading.
    Stop(TabId),

    /// Open a new tab (optionally with a URL).
    NewTab(Option<VexUrl>),

    /// Close a tab.
    CloseTab(TabId),

    /// Switch the active tab.
    SwitchTab(TabId),

    /// Execute JavaScript in the tab's context.
    EvaluateJs(TabId, String),

    /// Set the viewport size (window resize).
    SetViewport(TabId, f32, f32),

    /// Scroll the page content.
    Scroll(TabId, f32, f32),

    /// The embedder is shutting down.
    Shutdown,
}

// ──── Supporting types ───────────────────────────────────────────────

/// Page loading status — mirrors Servo's `LoadStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    /// Network request started.
    Started,
    /// `<head>` has been parsed (first paint possible).
    HeadParsed,
    /// Page fully loaded (DOMContentLoaded + resources).
    Complete,
}

/// Cursor kinds the engine can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKind {
    #[default]
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    Wait,
    Progress,
    ColResize,
    RowResize,
    NResize,
    SResize,
    EResize,
    WResize,
}

/// Permission kinds the engine can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    Geolocation,
    Camera,
    Microphone,
    Notifications,
    Clipboard,
}

/// Simple dialog requests from JavaScript.
#[derive(Debug, Clone)]
pub enum DialogRequest {
    /// `alert(message)` — display only, no return value.
    Alert(String),
    /// `confirm(message)` — expects `true`/`false` response.
    Confirm(String),
    /// `prompt(message, default)` — expects optional string response.
    Prompt(String, Option<String>),
}

/// Console API log levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLevel {
    Log,
    Info,
    Warn,
    Error,
    Debug,
}

impl std::fmt::Display for LoadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => write!(f, "loading"),
            Self::HeadParsed => write!(f, "head parsed"),
            Self::Complete => write!(f, "complete"),
        }
    }
}

impl std::fmt::Display for ConsoleLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "LOG"),
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Debug => write!(f, "DEBUG"),
        }
    }
}

// ──── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedder_msg_variants_debug() {
        let tab = TabId(1);
        let msg = EmbedderMsg::TitleChanged(tab, Some("Hello".into()));
        let dbg = format!("{msg:?}");
        assert!(dbg.contains("TitleChanged"));
        assert!(dbg.contains("Hello"));
    }

    #[test]
    fn embedder_command_variants_debug() {
        let tab = TabId(1);
        let cmd = EmbedderCommand::GoBack(tab);
        let dbg = format!("{cmd:?}");
        assert!(dbg.contains("GoBack"));
    }

    #[test]
    fn load_status_display() {
        assert_eq!(LoadStatus::Started.to_string(), "loading");
        assert_eq!(LoadStatus::HeadParsed.to_string(), "head parsed");
        assert_eq!(LoadStatus::Complete.to_string(), "complete");
    }

    #[test]
    fn console_level_display() {
        assert_eq!(ConsoleLevel::Log.to_string(), "LOG");
        assert_eq!(ConsoleLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn cursor_kind_default() {
        assert_eq!(CursorKind::default(), CursorKind::Default);
    }

    #[test]
    fn dialog_request_variants() {
        let alert = DialogRequest::Alert("test".into());
        assert!(format!("{alert:?}").contains("Alert"));

        let confirm = DialogRequest::Confirm("sure?".into());
        assert!(format!("{confirm:?}").contains("Confirm"));

        let prompt = DialogRequest::Prompt("name?".into(), Some("default".into()));
        assert!(format!("{prompt:?}").contains("Prompt"));
    }

    #[test]
    fn embedder_msg_url_changed() {
        let tab = TabId(2);
        let url = VexUrl::parse("https://example.com").unwrap();
        let msg = EmbedderMsg::UrlChanged(tab, url);
        assert!(format!("{msg:?}").contains("UrlChanged"));
    }

    #[test]
    fn embedder_command_navigate() {
        let tab = TabId(1);
        let url = VexUrl::parse("https://vigo.dev").unwrap();
        let cmd = EmbedderCommand::Navigate(tab, url);
        assert!(format!("{cmd:?}").contains("Navigate"));
    }

    #[test]
    fn embedder_msg_open_new_tab() {
        let url = VexUrl::parse("https://new-tab.com").unwrap();
        let msg = EmbedderMsg::OpenNewTab(url);
        assert!(format!("{msg:?}").contains("OpenNewTab"));
    }

    #[test]
    fn embedder_command_shutdown() {
        let cmd = EmbedderCommand::Shutdown;
        assert!(format!("{cmd:?}").contains("Shutdown"));
    }

    #[test]
    fn bus_command_fifo_order() {
        let mut bus = EmbedderBus::new();
        bus.push_command(EmbedderCommand::Shutdown);
        bus.push_command(EmbedderCommand::NewTab(None));

        assert!(matches!(bus.pop_command(), Some(EmbedderCommand::Shutdown)));
        assert!(matches!(bus.pop_command(), Some(EmbedderCommand::NewTab(None))));
        assert!(bus.pop_command().is_none());
    }

    #[test]
    fn bus_message_fifo_order() {
        let mut bus = EmbedderBus::new();
        let tab = TabId(1);

        bus.push_message(EmbedderMsg::LoadStatusChanged(tab, LoadStatus::Started));
        bus.push_message(EmbedderMsg::LoadStatusChanged(tab, LoadStatus::Complete));

        assert!(matches!(
            bus.pop_message(),
            Some(EmbedderMsg::LoadStatusChanged(_, LoadStatus::Started))
        ));
        assert!(matches!(
            bus.pop_message(),
            Some(EmbedderMsg::LoadStatusChanged(_, LoadStatus::Complete))
        ));
        assert!(bus.pop_message().is_none());
    }

    #[test]
    fn bus_len_and_empty() {
        let mut bus = EmbedderBus::new();
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);

        bus.push_command(EmbedderCommand::Shutdown);
        bus.push_message(EmbedderMsg::ShutdownComplete);

        assert!(!bus.is_empty());
        assert_eq!(bus.len(), 2);
    }
}
