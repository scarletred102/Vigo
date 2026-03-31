// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Shared DOM state bridge between Rust and JavaScript.
//!
//! The [`DomBridge`] holds an `Rc<RefCell<Document>>` that is shared
//! between the Rust-side page pipeline and the JS runtime's DOM bindings.

use std::cell::RefCell;
use std::rc::Rc;

use vex_dom::Document;

/// Shared reference to the DOM document.
///
/// Clone this to hand out more references — all clones point to the
/// same underlying `Document`.
pub type SharedDocument = Rc<RefCell<Document>>;

/// Create a new shared document wrapper.
pub fn shared_document(doc: Document) -> SharedDocument {
    Rc::new(RefCell::new(doc))
}
