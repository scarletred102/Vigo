// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-js
//!
//! JavaScript engine for the Vex browser engine.
//!
//! Wraps the Boa JS engine with Web API bindings:
//! console, timers, fetch, DOM manipulation, events.

pub mod api;
pub mod context;
pub mod dom_bridge;
pub mod gc_roots;
pub mod lifecycle;
pub mod script_runner;

pub use context::JsRuntime;
pub use dom_bridge::{shared_document, SharedDocument};
pub use gc_roots::GcRootSet;
pub use lifecycle::{fire_dom_content_loaded, fire_load};
pub use script_runner::ExecutionPlan;
