// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! # vex-dom
//!
//! DOM tree implementation for the Vex browser engine.
//!
//! Arena-allocated node storage, tree manipulation, traversal iterators,
//! and HTML serialization.

pub mod arena;
pub mod attributes;
pub mod document;
pub mod events;
pub mod form_submission;
pub mod forms;
pub mod node;
pub mod selector_element;
pub mod selector_impl;
pub mod serialize;
pub mod text_editing;
pub mod traversal;
pub mod tree;

// Re-exports for convenience.
pub use arena::NodeArena;
pub use document::Document;
pub use forms::{FormElementKind, FormStateMap, InputState, InputType};
pub use node::{Attribute, ElementData, Namespace, Node, NodeData};
pub use selector_element::{query_selector, query_selector_all, VexElement};
pub use selector_impl::VexSelectorImpl;
pub use traversal::{Ancestors, Children, Descendants};
