// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-browser
//!
//! Browser shell for the Vex browser engine.
//!
//! Tab management, navigation history, bookmarks, browsing history,
//! find-in-page, downloads, zoom, settings, keyboard shortcuts,
//! and the full page-load pipeline.

pub mod bookmarks;
pub mod crash_recovery;
pub mod devtools;
pub mod downloads;
pub mod drm_overlay;
pub mod error_pages;
pub mod forms;
pub mod event_handler;
pub mod find;
pub mod history;
pub mod image_loading;
pub mod internal_pages;
pub mod links;
pub mod navigation;
pub mod process;
pub mod sandbox;
pub mod secure_fetch;
pub mod session;
pub mod settings;
pub mod tab;
pub mod tab_manager;
pub mod ui;
pub mod webview_fallback;
pub mod wpt;
pub mod zoom;

pub mod extensions;

pub use bookmarks::{Bookmark, BookmarkId, BookmarkManager};
pub use devtools::{DevToolsAction, DevToolsLayout, DevToolsPanel, DevToolsState, DockPosition};
pub use downloads::{DownloadManager, DownloadState};
pub use drm_overlay::OverlayManager;
pub use find::FindState;
pub use history::BrowsingHistory;
pub use image_loading::{AsyncImageLoader, ImageLoadResult};
pub use links::LinkAction;
pub use navigation::{HistoryEntry, NavigationHistory};
pub use process::{IpcMessage, ProcessId, ProcessManager, ProcessRole, ProcessStatus};
pub use sandbox::{SandboxError, SandboxPolicy};
pub use secure_fetch::SecurityContext;
pub use session::SessionState;
pub use settings::BrowserSettings;
pub use tab::{LoadingState, Tab, TabId};
pub use tab_manager::TabManager;
pub use ui::chrome::ChromeLayout;
pub use ui::nav_bar::NavBarAction;
pub use ui::shortcuts::{BrowserAction, Key, Shortcut};
pub use ui::tab_bar::TabBarAction;
pub use webview_fallback::{WebViewError, WebViewFallback, WebViewState};
pub use wpt::{WptReport, WptRunner};
pub use zoom::ZoomState;

pub use extensions::action::{ActionBar, ActionClick, BrowserAction as ExtBrowserAction};
pub use extensions::content::{ContentScript, InjectionTime, MatchPattern};
pub use extensions::loader::ExtensionLoader;
pub use extensions::manifest::ExtensionManifest;
pub use extensions::permissions::{Permission, PermissionSet};
