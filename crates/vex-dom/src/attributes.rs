// Copyright (c) Vigo Team. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Attribute access helpers.

use vex_core::VexId;

use crate::arena::NodeArena;
use crate::node::{Attribute, NodeData};

/// Get the value of an attribute by name, or `None`.
pub fn get_attribute<'a>(arena: &'a NodeArena, id: VexId, name: &str) -> Option<&'a str> {
    if let NodeData::Element(ref el) = arena.get(id).data {
        el.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    } else {
        None
    }
}

/// Set (or overwrite) an attribute value. No-op on non-elements.
pub fn set_attribute(arena: &mut NodeArena, id: VexId, name: &str, value: &str) {
    if let NodeData::Element(ref mut el) = arena.get_mut(id).data {
        if let Some(attr) = el.attributes.iter_mut().find(|a| a.name == name) {
            attr.value = value.to_string();
        } else {
            el.attributes.push(Attribute {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
    }
}

/// Remove an attribute by name. Returns `true` if it existed.
pub fn remove_attribute(arena: &mut NodeArena, id: VexId, name: &str) -> bool {
    if let NodeData::Element(ref mut el) = arena.get_mut(id).data {
        let before = el.attributes.len();
        el.attributes.retain(|a| a.name != name);
        el.attributes.len() < before
    } else {
        false
    }
}

/// Check whether an element has a given attribute.
pub fn has_attribute(arena: &NodeArena, id: VexId, name: &str) -> bool {
    get_attribute(arena, id, name).is_some()
}

/// Check whether an element has `class_name` in its space-separated `class` list.
pub fn has_class(arena: &NodeArena, id: VexId, class_name: &str) -> bool {
    get_attribute(arena, id, "class")
        .is_some_and(|classes| classes.split_whitespace().any(|c| c == class_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::NodeArena;
    use crate::node::{ElementData, Namespace, NodeData};

    fn el(arena: &mut NodeArena) -> VexId {
        arena.alloc(NodeData::Element(ElementData {
            tag_name: "div".into(),
            namespace: Namespace::Html,
            attributes: vec![],
            template_contents: None,
            mathml_annotation_xml_integration_point: false,
        }))
    }

    #[test]
    fn set_and_get() {
        let mut arena = NodeArena::new();
        let id = el(&mut arena);
        set_attribute(&mut arena, id, "href", "/foo");
        assert_eq!(get_attribute(&arena, id, "href"), Some("/foo"));
    }

    #[test]
    fn overwrite() {
        let mut arena = NodeArena::new();
        let id = el(&mut arena);
        set_attribute(&mut arena, id, "href", "/old");
        set_attribute(&mut arena, id, "href", "/new");
        assert_eq!(get_attribute(&arena, id, "href"), Some("/new"));
    }

    #[test]
    fn remove() {
        let mut arena = NodeArena::new();
        let id = el(&mut arena);
        set_attribute(&mut arena, id, "x", "1");
        assert!(remove_attribute(&mut arena, id, "x"));
        assert!(!has_attribute(&arena, id, "x"));
    }

    #[test]
    fn has_class_space_separated() {
        let mut arena = NodeArena::new();
        let id = el(&mut arena);
        set_attribute(&mut arena, id, "class", "alpha beta gamma");
        assert!(has_class(&arena, id, "beta"));
        assert!(!has_class(&arena, id, "delta"));
    }

    #[test]
    fn non_element_returns_none() {
        let mut arena = NodeArena::new();
        let id = arena.alloc(NodeData::Text("hi".into()));
        assert_eq!(get_attribute(&arena, id, "x"), None);
        assert!(!has_attribute(&arena, id, "x"));
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut arena = NodeArena::new();
        let id = el(&mut arena);
        assert!(!remove_attribute(&mut arena, id, "missing"));
    }
}
