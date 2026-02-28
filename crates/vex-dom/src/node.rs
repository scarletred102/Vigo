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
