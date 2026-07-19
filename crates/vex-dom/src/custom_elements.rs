// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! Custom Elements Registry for Web Components.
//!
//! Provides the `CustomElementRegistry` (analogous to `customElements`)
//! for defining, getting, and upgrading custom elements.

use std::collections::HashMap;

// ── Types ────────────────────────────────────────────────────────────────────

/// A custom element definition.
#[derive(Debug, Clone)]
pub struct CustomElementDefinition {
    /// The tag name (must contain a hyphen, e.g. "my-component").
    pub name: String,
    /// The extended built-in element (for `is=""` usage), if any.
    pub extends: Option<String>,
    /// Observed attributes for `attributeChangedCallback`.
    pub observed_attributes: Vec<String>,
    /// Whether the element uses form-associated lifecycle callbacks.
    pub form_associated: bool,
    /// Whether the element should disable shadow DOM internals.
    pub disable_shadow: bool,
    /// Whether the element supports form internals.
    pub disable_internals: bool,
}

impl CustomElementDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            extends: None,
            observed_attributes: Vec::new(),
            form_associated: false,
            disable_shadow: false,
            disable_internals: false,
        }
    }

    pub fn with_extends(mut self, extends: &str) -> Self {
        self.extends = Some(extends.to_string());
        self
    }

    pub fn with_observed_attributes(mut self, attrs: Vec<String>) -> Self {
        self.observed_attributes = attrs;
        self
    }
}

/// Error returned when custom element registration fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomElementError {
    /// The name is not valid (must contain a hyphen, must not start with a dash).
    InvalidName(String),
    /// An element with this name is already defined.
    AlreadyDefined(String),
    /// A reserved tag name was used.
    ReservedName(String),
}

impl std::fmt::Display for CustomElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(n) => write!(f, "Invalid custom element name: '{n}'"),
            Self::AlreadyDefined(n) => write!(f, "Custom element '{n}' already defined"),
            Self::ReservedName(n) => write!(f, "Reserved element name: '{n}'"),
        }
    }
}

/// Lifecycle callback type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleCallback {
    Connected,
    Disconnected,
    Adopted,
    AttributeChanged,
    FormAssociated,
    FormDisabled,
    FormReset,
    FormStateRestore,
}

/// Reserved custom element names (per spec).
const RESERVED_NAMES: &[&str] = &[
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-src",
    "font-face-uri",
    "font-face-format",
    "font-face-name",
    "missing-glyph",
];

// ── CustomElementRegistry ────────────────────────────────────────────────────

/// The custom elements registry.
#[derive(Debug, Default)]
pub struct CustomElementRegistry {
    /// Defined elements: name → definition.
    definitions: HashMap<String, CustomElementDefinition>,
    /// Waiters for `whenDefined`: name → list of callback IDs.
    when_defined_waiters: HashMap<String, Vec<u64>>,
    /// Next waiter ID.
    next_waiter_id: u64,
}

impl CustomElementRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate a custom element name per the spec.
    fn validate_name(name: &str) -> Result<(), CustomElementError> {
        // Must contain a hyphen
        if !name.contains('-') {
            return Err(CustomElementError::InvalidName(name.to_string()));
        }

        // Must not start with a hyphen
        if name.starts_with('-') {
            return Err(CustomElementError::InvalidName(name.to_string()));
        }

        // Must be lowercase
        if name != name.to_ascii_lowercase() {
            return Err(CustomElementError::InvalidName(name.to_string()));
        }

        // Must not be a reserved name
        if RESERVED_NAMES.contains(&name) {
            return Err(CustomElementError::ReservedName(name.to_string()));
        }

        Ok(())
    }

    /// Define a custom element.
    pub fn define(
        &mut self,
        definition: CustomElementDefinition,
    ) -> Result<(), CustomElementError> {
        Self::validate_name(&definition.name)?;

        if self.definitions.contains_key(&definition.name) {
            return Err(CustomElementError::AlreadyDefined(definition.name.clone()));
        }

        let name = definition.name.clone();
        self.definitions.insert(name.clone(), definition);

        // Resolve any waiters
        self.when_defined_waiters.remove(&name);

        Ok(())
    }

    /// Get a custom element definition.
    pub fn get(&self, name: &str) -> Option<&CustomElementDefinition> {
        self.definitions.get(name)
    }

    /// Check if a custom element is defined.
    pub fn is_defined(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }

    /// Register a waiter for when a custom element is defined.
    /// Returns a waiter ID and whether the element is already defined.
    pub fn when_defined(&mut self, name: &str) -> (u64, bool) {
        if self.is_defined(name) {
            return (0, true);
        }

        let id = self.next_waiter_id;
        self.next_waiter_id += 1;
        self.when_defined_waiters
            .entry(name.to_string())
            .or_default()
            .push(id);
        (id, false)
    }

    /// Get the number of defined custom elements.
    pub fn count(&self) -> usize {
        self.definitions.len()
    }

    /// Get all defined element names.
    pub fn names(&self) -> Vec<&str> {
        self.definitions.keys().map(|s| s.as_str()).collect()
    }

    /// Upgrade an element (check if its tag name matches a defined custom element).
    /// Returns the matching definition if found.
    pub fn lookup_definition(&self, tag_name: &str) -> Option<&CustomElementDefinition> {
        self.definitions.get(tag_name)
    }

    /// Check whether a built-in element extension applies.
    pub fn lookup_extends(
        &self,
        builtin_name: &str,
        is_value: &str,
    ) -> Option<&CustomElementDefinition> {
        self.definitions
            .get(is_value)
            .filter(|def| def.extends.as_deref() == Some(builtin_name))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_custom_element() {
        let mut registry = CustomElementRegistry::new();
        let def = CustomElementDefinition::new("my-element");
        registry.define(def).unwrap();
        assert!(registry.is_defined("my-element"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn get_definition() {
        let mut registry = CustomElementRegistry::new();
        let def = CustomElementDefinition::new("my-button")
            .with_observed_attributes(vec!["disabled".to_string()]);
        registry.define(def).unwrap();

        let d = registry.get("my-button").unwrap();
        assert_eq!(d.name, "my-button");
        assert_eq!(d.observed_attributes, vec!["disabled"]);
    }

    #[test]
    fn already_defined() {
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new("my-element"))
            .unwrap();
        let err = registry
            .define(CustomElementDefinition::new("my-element"))
            .unwrap_err();
        assert!(matches!(err, CustomElementError::AlreadyDefined(_)));
    }

    #[test]
    fn invalid_name_no_hyphen() {
        let mut registry = CustomElementRegistry::new();
        let err = registry
            .define(CustomElementDefinition::new("myelement"))
            .unwrap_err();
        assert!(matches!(err, CustomElementError::InvalidName(_)));
    }

    #[test]
    fn invalid_name_starts_with_hyphen() {
        let mut registry = CustomElementRegistry::new();
        let err = registry
            .define(CustomElementDefinition::new("-my-element"))
            .unwrap_err();
        assert!(matches!(err, CustomElementError::InvalidName(_)));
    }

    #[test]
    fn invalid_name_uppercase() {
        let mut registry = CustomElementRegistry::new();
        let err = registry
            .define(CustomElementDefinition::new("My-Element"))
            .unwrap_err();
        assert!(matches!(err, CustomElementError::InvalidName(_)));
    }

    #[test]
    fn reserved_name() {
        let mut registry = CustomElementRegistry::new();
        let err = registry
            .define(CustomElementDefinition::new("annotation-xml"))
            .unwrap_err();
        assert!(matches!(err, CustomElementError::ReservedName(_)));
    }

    #[test]
    fn when_defined_already_exists() {
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new("my-element"))
            .unwrap();
        let (_, already) = registry.when_defined("my-element");
        assert!(already);
    }

    #[test]
    fn when_defined_not_yet() {
        let mut registry = CustomElementRegistry::new();
        let (id, already) = registry.when_defined("my-element");
        assert!(!already);
        assert_eq!(id, 0);
    }

    #[test]
    fn extends_builtin() {
        let mut registry = CustomElementRegistry::new();
        let def = CustomElementDefinition::new("fancy-button").with_extends("button");
        registry.define(def).unwrap();

        let found = registry.lookup_extends("button", "fancy-button");
        assert!(found.is_some());

        // Wrong builtin
        assert!(registry.lookup_extends("div", "fancy-button").is_none());
    }

    #[test]
    fn names_list() {
        let mut registry = CustomElementRegistry::new();
        registry
            .define(CustomElementDefinition::new("x-foo"))
            .unwrap();
        registry
            .define(CustomElementDefinition::new("x-bar"))
            .unwrap();
        let names = registry.names();
        assert_eq!(names.len(), 2);
    }
}
