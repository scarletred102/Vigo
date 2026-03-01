// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Crash isolation and recovery for renderer processes.
//!
//! When a renderer process crashes, the browser process shows a "page
//! crashed" error page in the affected tab and allows the user to reload.
//! Other tabs are unaffected.

use crate::error_pages;
use crate::process::ProcessManager;
use crate::tab::TabId;

/// Result of handling a crashed renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrashRecoveryAction {
    /// Show a crash error page in the tab.
    ShowCrashPage { tab_id: TabId, html: String },
    /// The tab was already gone — no action needed.
    TabNotFound,
}

/// Handle a renderer crash for a given tab.
///
/// 1. Marks the renderer as crashed in [`ProcessManager`].
/// 2. Generates a "page crashed" error page.
/// 3. Returns the recovery action so the caller can display it.
pub fn handle_renderer_crash(
    pm: &mut ProcessManager,
    tab_id: TabId,
    reason: &str,
) -> CrashRecoveryAction {
    match pm.get(tab_id) {
        Some(_) => {
            pm.mark_crashed(tab_id);
            let html = error_pages::crash_error_page(tab_id, reason);
            CrashRecoveryAction::ShowCrashPage { tab_id, html }
        }
        None => CrashRecoveryAction::TabNotFound,
    }
}

/// Attempt to restart a crashed renderer by removing the old entry and
/// spawning a fresh one.
///
/// Returns `Some(new_pid)` if recovery succeeded, `None` if the tab had
/// no crashed renderer.
pub fn restart_renderer(
    pm: &mut ProcessManager,
    tab_id: TabId,
) -> Option<crate::process::ProcessId> {
    if !pm.is_crashed(tab_id) {
        return None;
    }
    pm.remove_renderer(tab_id);
    let pid = pm.spawn_renderer(tab_id);
    pm.mark_running(tab_id);
    Some(pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessStatus;

    fn make_tab_id(n: u32) -> TabId {
        TabId(n)
    }

    #[test]
    fn handle_crash_shows_page() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(1);
        pm.spawn_renderer(tid);
        pm.mark_running(tid);

        let action = handle_renderer_crash(&mut pm, tid, "segfault");
        match action {
            CrashRecoveryAction::ShowCrashPage { tab_id, html } => {
                assert_eq!(tab_id, tid);
                assert!(html.contains("crashed"));
            }
            _ => panic!("expected ShowCrashPage"),
        }
        assert!(pm.is_crashed(tid));
    }

    #[test]
    fn handle_crash_unknown_tab() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(999);
        let action = handle_renderer_crash(&mut pm, tid, "segfault");
        assert_eq!(action, CrashRecoveryAction::TabNotFound);
    }

    #[test]
    fn restart_crashed_renderer() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(5);
        pm.spawn_renderer(tid);
        pm.mark_crashed(tid);

        let new_pid = restart_renderer(&mut pm, tid);
        assert!(new_pid.is_some());

        let info = pm.get(tid).unwrap();
        assert_eq!(info.status, ProcessStatus::Running);
    }

    #[test]
    fn restart_non_crashed_returns_none() {
        let mut pm = ProcessManager::new();
        let tid = make_tab_id(6);
        pm.spawn_renderer(tid);
        pm.mark_running(tid);
        assert!(restart_renderer(&mut pm, tid).is_none());
    }
}
