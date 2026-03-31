// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Extension platform for the Vigo browser engine.
//!
//! Extensions are loaded from the user's profile directory, each containing
//! a `manifest.json`. The platform handles manifest parsing, content script
//! injection, background script lifecycle, browser action, and permissions.

pub mod action;
pub mod content;
pub mod loader;
pub mod manifest;
pub mod permissions;

pub use action::BrowserAction;
pub use content::{ContentScript, InjectionTime, MatchPattern};
pub use loader::ExtensionLoader;
pub use manifest::ExtensionManifest;
pub use permissions::{Permission, PermissionSet};
