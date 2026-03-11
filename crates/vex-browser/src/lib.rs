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
pub mod broadcast;
pub mod clipboard;
pub mod crash_recovery;
pub mod devtools;
pub mod downloads;
pub mod drag_drop;
pub mod drm_overlay;
pub mod encoding;
pub mod error_pages;
pub mod event_handler;
pub mod event_source;
pub mod find;
pub mod forms;
pub mod fullscreen;
pub mod gamepad;
pub mod geolocation;
pub mod history;
pub mod image_loading;
pub mod internal_pages;
pub mod links;
pub mod media_session;
pub mod navigation;
pub mod notifications;
pub mod performance;
pub mod permissions;
pub mod pointer_events;
pub mod process;
pub mod sandbox;
pub mod secure_fetch;
pub mod service_worker;
pub mod session;
pub mod session_history;
pub mod settings;
pub mod tab;
pub mod tab_manager;
pub mod touch_events;
pub mod ui;
pub mod url_api;
pub mod visibility;
pub mod web_crypto;
pub mod webview_fallback;
pub mod workers;
pub mod wpt;
pub mod abort_controller;
pub mod fetch_api;
pub mod input_events;
pub mod navigation_api;
pub mod streams;
pub mod structured_clone;
pub mod zoom;

pub mod extensions;

pub use abort_controller::{AbortController, AbortSignal, AbortReason};
pub use bookmarks::{Bookmark, BookmarkId, BookmarkManager};
pub use broadcast::{BroadcastChannel, BroadcastHub, BroadcastMessage, ChannelEndpointId};
pub use clipboard::{Clipboard, ClipboardItem, DataTransfer, DataTransferItem};
pub use devtools::{DevToolsAction, DevToolsLayout, DevToolsPanel, DevToolsState, DockPosition};
pub use downloads::{DownloadManager, DownloadState};
pub use drag_drop::{DragController, DragEvent, DragEventType, DropEffect, EffectAllowed};
pub use drm_overlay::OverlayManager;
pub use encoding::{Encoding, TextDecoder, TextEncoder};
pub use fetch_api::{BodyContent, FetchHeaders, FetchRequest, FetchResponse, RequestCache, RequestCredentials, RequestMethod, RequestMode, RequestRedirect, ReferrerPolicy, ResponseType};
pub use event_source::{EventSource, EventSourceState, SseEvent};
pub use find::FindState;
pub use fullscreen::{FullscreenManager, FullscreenError, ScreenOrientation, OrientationType};
pub use gamepad::{Gamepad, GamepadButton, GamepadManager, StandardButton};
pub use geolocation::{GeolocationService, GeoPermission, GeoPosition};
pub use history::BrowsingHistory;
pub use input_events::{CompositionEvent, CompositionEventType, InputEvent, InputType, KeyEventType, KeyLocation, KeyboardEvent, ModifierState};
pub use image_loading::{AsyncImageLoader, ImageLoadResult};
pub use links::LinkAction;
pub use media_session::{MediaMetadata, MediaSession, MediaSessionAction, MediaSessionPlaybackState};
pub use navigation::{HistoryEntry, NavigationHistory};
pub use navigation_api::{NavigateEvent, Navigation, NavigationHistoryEntry, NavigationTransition, NavigationType};
pub use notifications::{NotificationCenter, NotificationPermission};
pub use performance::{EntryType, NavigationTiming, Performance, PerformanceEntry, ResourceTiming};
pub use permissions::{PermissionManager, PermissionName, PermissionState};
pub use pointer_events::{PointerCaptureManager, PointerEvent, PointerEventType, PointerType};
pub use process::{IpcMessage, ProcessId, ProcessManager, ProcessRole, ProcessStatus};
pub use sandbox::{SandboxError, SandboxPolicy};
pub use secure_fetch::SecurityContext;
pub use service_worker::{SwManager, SwRegistration, SwState};
pub use session::SessionState;
pub use session_history::SessionHistory;
pub use streams::{ReadableStream, ReadableStreamState, StreamChunk, TransformStream, WritableStream, WritableStreamState};
pub use structured_clone::{CloneError, StructuredValue, Transferable, TransferableType, TypedArrayKind};
pub use settings::BrowserSettings;
pub use tab::{LoadingState, Tab, TabId};
pub use tab_manager::TabManager;
pub use ui::chrome::ChromeLayout;
pub use ui::nav_bar::NavBarAction;
pub use ui::shortcuts::{BrowserAction, Key, Shortcut};
pub use ui::tab_bar::TabBarAction;
pub use touch_events::{Touch, TouchEvent, TouchEventType, TouchList, GestureRecognizer, Gesture};
pub use url_api::{UrlSearchParams, WebUrl};
pub use visibility::{VisibilityManager, VisibilityState};
pub use web_crypto::{CryptoKey, SubtleCrypto, HashAlgorithm, CryptoAlgorithm};
pub use webview_fallback::{WebViewError, WebViewFallback, WebViewState};
pub use wpt::{WptReport, WptRunner};
pub use zoom::ZoomState;

pub use extensions::action::{ActionBar, ActionClick, BrowserAction as ExtBrowserAction};
pub use extensions::content::{ContentScript, InjectionTime, MatchPattern};
pub use extensions::loader::ExtensionLoader;
pub use extensions::manifest::ExtensionManifest;
pub use extensions::permissions::{Permission, PermissionSet};
