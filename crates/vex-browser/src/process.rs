// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Multi-process model for per-tab isolation.
//!
//! The browser runs as two process roles:
//!
//! - **Browser process** (the main process): manages tabs, UI, navigation,
//!   and the network stack. One per browser instance.
//! - **Renderer process**: handles DOM, layout, CSS, JS and painting for a
//!   single tab. One per tab.
//!
//! Communication uses [`IpcMessage`] passed over named pipes (Windows) or
//! Unix domain sockets.

use std::collections::HashMap;
use std::fmt;

use vex_core::VexUrl;

use crate::tab::TabId;

// ── Process identity ───────────────────────────────────────────────────

/// Which role a process fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessRole {
    /// The main browser process (singleton).
    Browser,
    /// A renderer process for a single tab.
    Renderer,
}

impl fmt::Display for ProcessRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Browser => f.write_str("browser"),
            Self::Renderer => f.write_str("renderer"),
        }
    }
}

/// Opaque identifier for a child renderer process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(pub u32);

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pid:{}", self.0)
    }
}

// ── IPC messages ───────────────────────────────────────────────────────

/// Messages sent between the browser process and renderer processes.
#[derive(Debug, Clone)]
pub enum IpcMessage {
    // ── Browser → Renderer ──
    /// Navigate the tab to a URL.
    LoadUrl { tab_id: TabId, url: VexUrl },

    /// Forward a user input event to the renderer.
    InputEvent {
        tab_id: TabId,
        event: InputEventData,
    },

    /// Request the renderer to produce a new display list / frame.
    RequestFrame { tab_id: TabId },

    /// Tell the renderer to shut down gracefully.
    Shutdown,

    // ── Renderer → Browser ──
    /// The renderer finished loading a page.
    NavigationComplete {
        tab_id: TabId,
        url: VexUrl,
        title: String,
    },

    /// A freshly painted frame is ready (handle/ID for shared texture).
    RenderFrame {
        tab_id: TabId,
        frame_id: u64,
        width: u32,
        height: u32,
    },

    /// JavaScript initiated a callback / DOM event that the browser
    /// process needs to know about (e.g. `window.close()`, title change).
    JsCallback { tab_id: TabId, kind: JsCallbackKind },

    /// The renderer encountered a fatal error or crash.
    RendererCrashed { tab_id: TabId, reason: String },
}

/// Simplified input event data for IPC serialisation.
#[derive(Debug, Clone)]
pub enum InputEventData {
    KeyDown { key: String, modifiers: u8 },
    KeyUp { key: String, modifiers: u8 },
    MouseDown { x: f32, y: f32, button: u8 },
    MouseUp { x: f32, y: f32, button: u8 },
    MouseMove { x: f32, y: f32 },
    Scroll { dx: f32, dy: f32 },
    TextInput { text: String },
}

/// Kinds of JS callbacks the renderer can send to the browser process.
#[derive(Debug, Clone)]
pub enum JsCallbackKind {
    /// `document.title` changed.
    TitleChanged(String),
    /// JS called `window.close()`.
    WindowClose,
    /// JS called `window.location.href = ...` or similar navigation.
    Navigate(VexUrl),
    /// JS called `window.open(url)`.
    OpenNewTab(VexUrl),
}

// ── Process state tracking ─────────────────────────────────────────────

/// Status of a renderer process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Starting up, not yet ready for messages.
    Starting,
    /// Running and accepting messages.
    Running,
    /// Shutting down gracefully.
    ShuttingDown,
    /// Crashed or terminated unexpectedly.
    Crashed,
    /// Exited cleanly.
    Exited,
}

/// Metadata about a running renderer process.
#[derive(Debug, Clone)]
pub struct RendererInfo {
    pub pid: ProcessId,
    pub tab_id: TabId,
    pub status: ProcessStatus,
}

/// Manages the set of active renderer processes.
///
/// The browser process owns a single `ProcessManager` which tracks all
/// child renderer processes.
#[derive(Debug, Default)]
pub struct ProcessManager {
    renderers: HashMap<TabId, RendererInfo>,
    next_fake_pid: u32,
}

impl ProcessManager {
    /// Create a new (empty) process manager.
    pub fn new() -> Self {
        Self {
            renderers: HashMap::new(),
            next_fake_pid: 1000,
        }
    }

    /// Register a new renderer for a tab.
    ///
    /// In a real implementation this would `CreateProcess` on Windows.
    /// For now we allocate a logical [`ProcessId`] and track the state.
    pub fn spawn_renderer(&mut self, tab_id: TabId) -> ProcessId {
        let pid = ProcessId(self.next_fake_pid);
        self.next_fake_pid += 1;

        self.renderers.insert(
            tab_id,
            RendererInfo {
                pid,
                tab_id,
                status: ProcessStatus::Starting,
            },
        );
        tracing::info!(%tab_id, %pid, "spawned renderer process");
        pid
    }

    /// Mark a renderer as running (ready to accept messages).
    pub fn mark_running(&mut self, tab_id: TabId) {
        if let Some(info) = self.renderers.get_mut(&tab_id) {
            info.status = ProcessStatus::Running;
        }
    }

    /// Mark a renderer as crashed.
    pub fn mark_crashed(&mut self, tab_id: TabId) {
        if let Some(info) = self.renderers.get_mut(&tab_id) {
            info.status = ProcessStatus::Crashed;
            tracing::error!(%tab_id, pid = %info.pid, "renderer crashed");
        }
    }

    /// Remove a renderer (after clean exit or crash recovery).
    pub fn remove_renderer(&mut self, tab_id: TabId) -> Option<RendererInfo> {
        self.renderers.remove(&tab_id)
    }

    /// Look up the renderer for a tab.
    pub fn get(&self, tab_id: TabId) -> Option<&RendererInfo> {
        self.renderers.get(&tab_id)
    }

    /// Iterate over all active renderers.
    pub fn all(&self) -> impl Iterator<Item = &RendererInfo> {
        self.renderers.values()
    }

    /// Number of active renderer processes.
    pub fn count(&self) -> usize {
        self.renderers.len()
    }

    /// Check if a tab has a crashed renderer.
    pub fn is_crashed(&self, tab_id: TabId) -> bool {
        self.renderers
            .get(&tab_id)
            .is_some_and(|info| info.status == ProcessStatus::Crashed)
    }

    /// Spawn a real OS renderer process via `CreateProcessW` (Windows).
    ///
    /// **Current status:** Returns a logical [`ProcessId`] without creating a
    /// real OS process. The browser operates in single-process mode — all DOM,
    /// layout, rendering, and JS run on the main thread. This method exists so
    /// the [`ProcessManager`] API is ready for the multi-process milestone.
    ///
    /// When multi-process mode is implemented, this will:
    /// 1. Build a command line: `vigo.exe --renderer --tab-id={id} --pipe={name}`
    /// 2. Create a named pipe for IPC
    /// 3. Call `CreateProcessW` with a restricted token (via [`SandboxPolicy`])
    /// 4. Store the real PID from `PROCESS_INFORMATION.dwProcessId`
    ///
    /// [`SandboxPolicy`]: crate::sandbox::SandboxPolicy
    pub fn spawn_renderer_os(&mut self, tab_id: TabId) -> ProcessId {
        // For now, delegate to the logical PID path.
        let pid = self.spawn_renderer(tab_id);
        tracing::debug!(
            %tab_id, %pid,
            "spawn_renderer_os: single-process mode, using logical PID"
        );
        pid
    }

    /// Send an IPC message to a renderer process.
    ///
    /// **Current status:** Logs the message. When multi-process mode is active,
    /// this will serialize the message and write it to the named pipe.
    pub fn send_message(&self, tab_id: TabId, msg: &IpcMessage) {
        if let Some(info) = self.renderers.get(&tab_id) {
            tracing::debug!(
                %tab_id, pid = %info.pid,
                "IPC send: {msg:?}"
            );
        } else {
            tracing::warn!(%tab_id, "send_message: no renderer for tab");
        }
    }

    /// Terminate a renderer process, optionally with a timeout.
    ///
    /// In single-process mode this just removes the renderer tracking entry.
    /// In multi-process mode this would send `IpcMessage::Shutdown`, wait up to
    /// `timeout_ms`, then call `TerminateProcess` if the renderer hasn't exited.
    pub fn terminate_renderer(&mut self, tab_id: TabId, _timeout_ms: u32) -> Option<RendererInfo> {
        if let Some(mut info) = self.renderers.remove(&tab_id) {
            info.status = ProcessStatus::Exited;
            tracing::info!(%tab_id, pid = %info.pid, "renderer terminated");
            Some(info)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab_id(n: u32) -> TabId {
        TabId(n)
    }

    #[test]
    fn spawn_and_get_renderer() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(1);
        let pid = pm.spawn_renderer(tid);
        assert_eq!(pm.count(), 1);
        let info = pm.get(tid).unwrap();
        assert_eq!(info.pid, pid);
        assert_eq!(info.status, ProcessStatus::Starting);
    }

    #[test]
    fn mark_running_and_crashed() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(2);
        pm.spawn_renderer(tid);
        pm.mark_running(tid);
        assert_eq!(pm.get(tid).unwrap().status, ProcessStatus::Running);
        pm.mark_crashed(tid);
        assert!(pm.is_crashed(tid));
    }

    #[test]
    fn remove_renderer() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(3);
        pm.spawn_renderer(tid);
        let removed = pm.remove_renderer(tid);
        assert!(removed.is_some());
        assert_eq!(pm.count(), 0);
        assert!(pm.get(tid).is_none());
    }

    #[test]
    fn multiple_tabs_tracked() {
        let mut pm = ProcessManager::new();
        let t1 = make_tab_id(10);
        let t2 = make_tab_id(20);
        let t3 = make_tab_id(30);
        pm.spawn_renderer(t1);
        pm.spawn_renderer(t2);
        pm.spawn_renderer(t3);
        assert_eq!(pm.count(), 3);

        pm.mark_running(t1);
        pm.mark_running(t2);
        pm.mark_crashed(t3);

        assert_eq!(pm.get(t1).unwrap().status, ProcessStatus::Running);
        assert_eq!(pm.get(t2).unwrap().status, ProcessStatus::Running);
        assert!(pm.is_crashed(t3));
    }

    #[test]
    fn ipc_message_variants() {
        // Ensure all message variants are constructible.
        let url = VexUrl::parse("https://example.com").unwrap();
        let tid = make_tab_id(1);

        let msgs = [
            IpcMessage::LoadUrl {
                tab_id: tid,
                url: url.clone(),
            },
            IpcMessage::InputEvent {
                tab_id: tid,
                event: InputEventData::KeyDown {
                    key: "Enter".into(),
                    modifiers: 0,
                },
            },
            IpcMessage::RequestFrame { tab_id: tid },
            IpcMessage::Shutdown,
            IpcMessage::NavigationComplete {
                tab_id: tid,
                url: url.clone(),
                title: "Hello".into(),
            },
            IpcMessage::RenderFrame {
                tab_id: tid,
                frame_id: 42,
                width: 1280,
                height: 720,
            },
            IpcMessage::JsCallback {
                tab_id: tid,
                kind: JsCallbackKind::TitleChanged("New Title".into()),
            },
            IpcMessage::RendererCrashed {
                tab_id: tid,
                reason: "segfault".into(),
            },
        ];
        assert_eq!(msgs.len(), 8);
    }

    #[test]
    fn spawn_renderer_os_uses_logical_pid() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(50);
        let pid = pm.spawn_renderer_os(tid);
        assert_eq!(pm.count(), 1);
        let info = pm.get(tid).unwrap();
        assert_eq!(info.pid, pid);
        assert_eq!(info.status, ProcessStatus::Starting);
    }

    #[test]
    fn send_message_no_panic() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(60);
        pm.spawn_renderer(tid);
        pm.mark_running(tid);
        let msg = IpcMessage::RequestFrame { tab_id: tid };
        pm.send_message(tid, &msg);
        // No panic = success.
    }

    #[test]
    fn send_message_missing_tab() {
        let pm = ProcessManager::new();
        let tid = make_tab_id(99);
        let msg = IpcMessage::Shutdown;
        pm.send_message(tid, &msg);
        // Logs a warning, no panic.
    }

    #[test]
    fn terminate_renderer_marks_exited() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(70);
        pm.spawn_renderer(tid);
        pm.mark_running(tid);
        let info = pm.terminate_renderer(tid, 5000).unwrap();
        assert_eq!(info.status, ProcessStatus::Exited);
        assert_eq!(pm.count(), 0);
    }

    #[test]
    fn terminate_missing_renderer() {
        let mut pm = ProcessManager::new();
        assert!(pm.terminate_renderer(make_tab_id(999), 0).is_none());
    }
}
