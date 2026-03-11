// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! DOM node types and data structures.

use std::fmt;
use vex_core::VexId;

/// HTML/SVG/MathML namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Namespace {
    #[default]
    Html,
    Svg,
    MathMl,
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Html => write!(f, "html"),
            Self::Svg => write!(f, "svg"),
            Self::MathMl => write!(f, "mathml"),
        }
    }
}

/// An attribute on an element (e.g., `class="foo"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

/// Bit-flags tracking dynamic interaction / form state on an element.
///
/// Used by the CSS selector engine to match pseudo-classes like `:hover`,
/// `:focus`, `:checked`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ElementState(pub u32);

impl ElementState {
    pub const HOVER: u32 = 1 << 0;
    pub const ACTIVE: u32 = 1 << 1;
    pub const FOCUS: u32 = 1 << 2;
    pub const FOCUS_WITHIN: u32 = 1 << 3;
    pub const ENABLED: u32 = 1 << 4;
    pub const DISABLED: u32 = 1 << 5;
    pub const CHECKED: u32 = 1 << 6;
    pub const INDETERMINATE: u32 = 1 << 7;
    pub const READ_ONLY: u32 = 1 << 8;
    pub const READ_WRITE: u32 = 1 << 9;
    pub const PLACEHOLDER_SHOWN: u32 = 1 << 10;
    pub const DEFAULT: u32 = 1 << 11;
    pub const REQUIRED: u32 = 1 << 12;
    pub const OPTIONAL: u32 = 1 << 13;
    pub const VALID: u32 = 1 << 14;
    pub const INVALID: u32 = 1 << 15;
    pub const IN_RANGE: u32 = 1 << 16;
    pub const OUT_OF_RANGE: u32 = 1 << 17;
    pub const VISITED: u32 = 1 << 18;
    pub const TARGET: u32 = 1 << 19;
    pub const FOCUS_VISIBLE: u32 = 1 << 20;
    pub const OPEN: u32 = 1 << 21;
    pub const DEFINED: u32 = 1 << 22;
    pub const FULLSCREEN: u32 = 1 << 23;
    pub const AUTOFILL: u32 = 1 << 24;

    /// Returns `true` if this state contains the given flag.
    #[inline]
    pub fn contains(self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Insert (set) a flag.
    #[inline]
    pub fn insert(&mut self, flag: u32) {
        self.0 |= flag;
    }

    /// Remove (clear) a flag.
    #[inline]
    pub fn remove(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    /// Set or clear a flag depending on `value`.
    #[inline]
    pub fn set(&mut self, flag: u32, value: bool) {
        if value {
            self.insert(flag);
        } else {
            self.remove(flag);
        }
    }
}

/// Data specific to element nodes.
#[derive(Debug, Clone)]
pub struct ElementData {
    /// Lowercase tag name (e.g., `"div"`, `"p"`, `"html"`).
    pub tag_name: String,
    /// Namespace of the element.
    pub namespace: Namespace,
    /// Attributes on the element.
    pub attributes: Vec<Attribute>,
    /// For `<template>` elements, the document fragment holding template contents.
    pub template_contents: Option<VexId>,
    /// MathML annotation-xml integration point flag (used by html5ever).
    pub mathml_annotation_xml_integration_point: bool,
    /// Dynamic interaction / form state flags (`:hover`, `:focus`, etc.).
    pub state: ElementState,
}

/// The payload carried by each DOM node.
#[derive(Debug, Clone)]
pub enum NodeData {
    /// The root document node.
    Document,
    /// An element like `<div>` or `<p>`.
    Element(ElementData),
    /// A run of text.
    Text(String),
    /// A comment `<!-- ... -->`.
    Comment(String),
    /// A `<!DOCTYPE ...>` node.
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
}

/// A single node in the DOM tree, linked to its neighbours via [`VexId`].
#[derive(Debug)]
pub struct Node {
    pub id: VexId,
    pub parent: Option<VexId>,
    pub first_child: Option<VexId>,
    pub last_child: Option<VexId>,
    pub next_sibling: Option<VexId>,
    pub prev_sibling: Option<VexId>,
    pub data: NodeData,
}
