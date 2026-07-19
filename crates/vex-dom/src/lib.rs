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
pub mod custom_elements;
pub mod document;
pub mod events;
pub mod form_submission;
pub mod forms;
pub mod mutation_observer;
pub mod node;
pub mod observers;
pub mod range;
pub mod selection;
pub mod selector_element;
pub mod selector_impl;
pub mod serialize;
pub mod shadow;
pub mod text_editing;
pub mod traversal;
pub mod tree;
pub mod tree_walker;
pub mod validation;

// Re-exports for convenience.
pub use arena::NodeArena;
pub use custom_elements::{CustomElementDefinition, CustomElementRegistry};
pub use document::Document;
pub use forms::{FormElementKind, FormStateMap, InputState, InputType};
pub use mutation_observer::{MutationObserver, MutationObserverSet, MutationRecord, MutationType};
pub use node::{Attribute, ElementData, ElementState, Namespace, Node, NodeData};
pub use observers::{IntersectionObserver, ResizeObserver};
pub use range::{BoundaryPoint as RangeBoundary, Range as DomRange, RangeComparison, StaticRange};
pub use selection::{BoundaryPoint, Range, Selection, SelectionDirection};
pub use selector_element::{
    matches_selector, matches_selector_list, query_selector, query_selector_all, VexElement,
};
pub use selector_impl::VexSelectorImpl;
pub use shadow::{ShadowDomManager, ShadowRoot, ShadowRootMode};
pub use traversal::{Ancestors, Children, Descendants};
pub use tree_walker::{FilterResult, NodeIterator, TreeWalker, WhatToShow};
