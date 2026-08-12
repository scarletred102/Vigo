// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Web API implementations registered on the JS context.

pub mod browser_bridge;
pub mod console;
pub mod document;
pub(crate) mod dom_dirty;
pub mod element;
pub mod eme;
pub mod events;
pub mod extensions;
pub mod fetch;
pub mod indexed_db;
pub mod local_storage;
pub mod session_storage;
pub mod style_proxy;
pub mod timers;
pub mod window;
