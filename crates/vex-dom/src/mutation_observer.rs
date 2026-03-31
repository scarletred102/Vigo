// Copyright (c) Vigo Contributors
// SPDX-License-Identifier: MPL-2.0

//! MutationObserver — watches for changes to the DOM tree.
//!
//! Implements the MutationObserver API (simplified):
//! - Observe childList, attributes, characterData changes
//! - Filter by attribute names (attributeFilter)
//! - Track old values (attributeOldValue, characterDataOldValue)
//! - Subtree observation
//! - Batched MutationRecord delivery
//!
//! Usage:
//! 1. Create a `MutationObserver` with a unique ID.
//! 2. Call `observe(target, options)` to start watching.
//! 3. When DOM mutations happen, call `notify()` on the `MutationObserverSet`.
//! 4. Call `take_records()` to drain pending records.

use std::collections::HashMap;

use vex_core::VexId;

/// Options for what to observe on a target node.
#[derive(Debug, Clone, Default)]
pub struct MutationObserverInit {
    /// Watch for child additions/removals.
    pub child_list: bool,
    /// Watch for attribute changes.
    pub attributes: bool,
    /// Watch for text node data changes.
    pub character_data: bool,
    /// Also observe all descendant nodes.
    pub subtree: bool,
    /// Record old attribute values.
    pub attribute_old_value: bool,
    /// Record old character data values.
    pub character_data_old_value: bool,
    /// Only observe these specific attribute names (empty = all).
    pub attribute_filter: Vec<String>,
}

impl MutationObserverInit {
    /// Create options for watching child list changes.
    pub fn child_list() -> Self {
        Self {
            child_list: true,
            ..Default::default()
        }
    }

    /// Create options for watching attribute changes.
    pub fn attributes() -> Self {
        Self {
            attributes: true,
            ..Default::default()
        }
    }

    /// Create options for watching everything.
    pub fn all() -> Self {
        Self {
            child_list: true,
            attributes: true,
            character_data: true,
            subtree: true,
            attribute_old_value: true,
            character_data_old_value: true,
            attribute_filter: Vec::new(),
        }
    }
}

/// Type of a mutation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationType {
    /// Child nodes were added or removed.
    ChildList,
    /// An attribute was modified.
    Attributes,
    /// Character data (text content) was modified.
    CharacterData,
}

/// A single mutation record delivered to an observer.
#[derive(Debug, Clone)]
pub struct MutationRecord {
    /// Type of mutation.
    pub mutation_type: MutationType,
    /// The node that was mutated.
    pub target: VexId,
    /// Nodes added (for childList).
    pub added_nodes: Vec<VexId>,
    /// Nodes removed (for childList).
    pub removed_nodes: Vec<VexId>,
    /// Previous sibling of added/removed nodes.
    pub previous_sibling: Option<VexId>,
    /// Next sibling of added/removed nodes.
    pub next_sibling: Option<VexId>,
    /// Name of changed attribute.
    pub attribute_name: Option<String>,
    /// Namespace of changed attribute.
    pub attribute_namespace: Option<String>,
    /// Old value (attribute or character data).
    pub old_value: Option<String>,
}

impl MutationRecord {
    /// Create a childList mutation record.
    pub fn child_list(
        target: VexId,
        added: Vec<VexId>,
        removed: Vec<VexId>,
        prev_sibling: Option<VexId>,
        next_sibling: Option<VexId>,
    ) -> Self {
        Self {
            mutation_type: MutationType::ChildList,
            target,
            added_nodes: added,
            removed_nodes: removed,
            previous_sibling: prev_sibling,
            next_sibling,
            attribute_name: None,
            attribute_namespace: None,
            old_value: None,
        }
    }

    /// Create an attributes mutation record.
    pub fn attributes(
        target: VexId,
        name: &str,
        old_value: Option<String>,
    ) -> Self {
        Self {
            mutation_type: MutationType::Attributes,
            target,
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some(name.to_owned()),
            attribute_namespace: None,
            old_value,
        }
    }

    /// Create a characterData mutation record.
    pub fn character_data(target: VexId, old_value: Option<String>) -> Self {
        Self {
            mutation_type: MutationType::CharacterData,
            target,
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            attribute_namespace: None,
            old_value,
        }
    }
}

/// Unique ID for a mutation observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId(u64);

impl ObserverId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Observer registration on a specific target.
#[derive(Debug, Clone)]
struct Observation {
    target: VexId,
    options: MutationObserverInit,
}

/// A single MutationObserver instance.
#[derive(Debug)]
pub struct MutationObserver {
    _id: ObserverId,
    observations: Vec<Observation>,
    records: Vec<MutationRecord>,
}

impl MutationObserver {
    fn new(id: ObserverId) -> Self {
        Self {
            _id: id,
            observations: Vec::new(),
            records: Vec::new(),
        }
    }

    /// Start observing a target node with the given options.
    pub fn observe(&mut self, target: VexId, options: MutationObserverInit) {
        // Replace existing observation for the same target.
        self.observations.retain(|o| o.target != target);
        self.observations.push(Observation { target, options });
    }

    /// Stop observing a specific target.
    pub fn unobserve(&mut self, target: VexId) {
        self.observations.retain(|o| o.target != target);
    }

    /// Stop observing all targets.
    pub fn disconnect(&mut self) {
        self.observations.clear();
        self.records.clear();
    }

    /// Drain and return all pending mutation records.
    pub fn take_records(&mut self) -> Vec<MutationRecord> {
        std::mem::take(&mut self.records)
    }

    /// How many pending records.
    pub fn pending_count(&self) -> usize {
        self.records.len()
    }
}

/// Manager for all MutationObservers in a document.
///
/// When a DOM mutation occurs, the document calls `notify_*` methods
/// which queue records to matching observers.
#[derive(Debug, Default)]
pub struct MutationObserverSet {
    observers: HashMap<ObserverId, MutationObserver>,
    next_id: u64,
}

impl MutationObserverSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new observer and return its ID.
    pub fn create_observer(&mut self) -> ObserverId {
        let id = ObserverId(self.next_id);
        self.next_id += 1;
        self.observers.insert(id, MutationObserver::new(id));
        id
    }

    /// Get a mutable reference to an observer.
    pub fn observer_mut(&mut self, id: ObserverId) -> Option<&mut MutationObserver> {
        self.observers.get_mut(&id)
    }

    /// Get a reference to an observer.
    pub fn observer(&self, id: ObserverId) -> Option<&MutationObserver> {
        self.observers.get(&id)
    }

    /// Remove an observer.
    pub fn remove_observer(&mut self, id: ObserverId) {
        self.observers.remove(&id);
    }

    /// Notify observers of an attribute change.
    ///
    /// `ancestors` should include the target and all ancestor VexIds
    /// (for subtree matching).
    pub fn notify_attribute(
        &mut self,
        target: VexId,
        ancestors: &[VexId],
        attr_name: &str,
        old_value: Option<String>,
    ) {
        for observer in self.observers.values_mut() {
            for obs in &observer.observations {
                let is_match = if obs.target == target {
                    true
                } else {
                    obs.options.subtree && ancestors.contains(&obs.target)
                };

                if is_match && obs.options.attributes {
                    // Check attribute filter.
                    if !obs.options.attribute_filter.is_empty()
                        && !obs.options.attribute_filter.iter().any(|f| f == attr_name)
                    {
                        continue;
                    }

                    let old = if obs.options.attribute_old_value {
                        old_value.clone()
                    } else {
                        None
                    };

                    observer
                        .records
                        .push(MutationRecord::attributes(target, attr_name, old));
                }
            }
        }
    }

    /// Notify observers of a childList change.
    pub fn notify_child_list(
        &mut self,
        target: VexId,
        ancestors: &[VexId],
        added: Vec<VexId>,
        removed: Vec<VexId>,
        prev_sibling: Option<VexId>,
        next_sibling: Option<VexId>,
    ) {
        for observer in self.observers.values_mut() {
            for obs in &observer.observations {
                let is_match = if obs.target == target {
                    true
                } else {
                    obs.options.subtree && ancestors.contains(&obs.target)
                };

                if is_match && obs.options.child_list {
                    observer.records.push(MutationRecord::child_list(
                        target,
                        added.clone(),
                        removed.clone(),
                        prev_sibling,
                        next_sibling,
                    ));
                }
            }
        }
    }

    /// Notify observers of a characterData change.
    pub fn notify_character_data(
        &mut self,
        target: VexId,
        ancestors: &[VexId],
        old_value: Option<String>,
    ) {
        for observer in self.observers.values_mut() {
            for obs in &observer.observations {
                let is_match = if obs.target == target {
                    true
                } else {
                    obs.options.subtree && ancestors.contains(&obs.target)
                };

                if is_match && obs.options.character_data {
                    let old = if obs.options.character_data_old_value {
                        old_value.clone()
                    } else {
                        None
                    };

                    observer
                        .records
                        .push(MutationRecord::character_data(target, old));
                }
            }
        }
    }

    /// Number of active observers.
    pub fn count(&self) -> usize {
        self.observers.len()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_observer() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();
        assert_eq!(set.count(), 1);
        assert!(set.observer(id).is_some());
    }

    #[test]
    fn observe_and_disconnect() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();
        let obs = set.observer_mut(id).unwrap();

        let target = VexId::new(5);
        obs.observe(target, MutationObserverInit::child_list());
        assert_eq!(obs.observations.len(), 1);

        obs.disconnect();
        assert_eq!(obs.observations.len(), 0);
    }

    #[test]
    fn attribute_notification() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(3);
        set.observer_mut(id)
            .unwrap()
            .observe(target, MutationObserverInit::attributes());

        set.notify_attribute(target, &[target], "class", Some("old-class".into()));

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].mutation_type, MutationType::Attributes);
        assert_eq!(records[0].target, target);
        assert_eq!(records[0].attribute_name.as_deref(), Some("class"));
    }

    #[test]
    fn attribute_old_value() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(3);
        let mut opts = MutationObserverInit::attributes();
        opts.attribute_old_value = true;
        set.observer_mut(id).unwrap().observe(target, opts);

        set.notify_attribute(target, &[target], "href", Some("old-url".into()));

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records[0].old_value.as_deref(), Some("old-url"));
    }

    #[test]
    fn attribute_filter() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(3);
        let opts = MutationObserverInit {
            attributes: true,
            attribute_filter: vec!["class".into(), "id".into()],
            ..Default::default()
        };
        set.observer_mut(id).unwrap().observe(target, opts);

        // "class" should be observed.
        set.notify_attribute(target, &[target], "class", None);
        // "title" should NOT be observed.
        set.notify_attribute(target, &[target], "title", None);

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].attribute_name.as_deref(), Some("class"));
    }

    #[test]
    fn child_list_notification() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let parent = VexId::new(1);
        set.observer_mut(id)
            .unwrap()
            .observe(parent, MutationObserverInit::child_list());

        let child = VexId::new(10);
        set.notify_child_list(parent, &[parent], vec![child], vec![], None, None);

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].mutation_type, MutationType::ChildList);
        assert_eq!(records[0].added_nodes, vec![child]);
    }

    #[test]
    fn character_data_notification() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(5);
        let mut opts = MutationObserverInit::default();
        opts.character_data = true;
        opts.character_data_old_value = true;
        set.observer_mut(id).unwrap().observe(target, opts);

        set.notify_character_data(target, &[target], Some("old text".into()));

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].mutation_type, MutationType::CharacterData);
        assert_eq!(records[0].old_value.as_deref(), Some("old text"));
    }

    #[test]
    fn subtree_observation() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let root = VexId::new(0);
        let mut opts = MutationObserverInit::all();
        opts.subtree = true;
        set.observer_mut(id).unwrap().observe(root, opts);

        // Mutation on a descendant should be caught.
        let descendant = VexId::new(5);
        set.notify_attribute(descendant, &[descendant, VexId::new(2), root], "class", None);

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target, descendant);
    }

    #[test]
    fn no_match_without_subtree() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let parent = VexId::new(1);
        // Observe parent WITHOUT subtree.
        set.observer_mut(id)
            .unwrap()
            .observe(parent, MutationObserverInit::attributes());

        // Mutation on child should NOT be caught.
        let child = VexId::new(5);
        set.notify_attribute(child, &[child, parent], "class", None);

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn multiple_observers() {
        let mut set = MutationObserverSet::new();
        let id1 = set.create_observer();
        let id2 = set.create_observer();

        let target = VexId::new(3);
        set.observer_mut(id1)
            .unwrap()
            .observe(target, MutationObserverInit::attributes());
        set.observer_mut(id2)
            .unwrap()
            .observe(target, MutationObserverInit::child_list());

        // Attribute change: only id1 should get a record.
        set.notify_attribute(target, &[target], "src", None);

        assert_eq!(set.observer_mut(id1).unwrap().pending_count(), 1);
        assert_eq!(set.observer_mut(id2).unwrap().pending_count(), 0);

        // Child change: only id2 should get a record.
        set.notify_child_list(target, &[target], vec![VexId::new(10)], vec![], None, None);

        assert_eq!(set.observer_mut(id1).unwrap().pending_count(), 1);
        assert_eq!(set.observer_mut(id2).unwrap().pending_count(), 1);
    }

    #[test]
    fn take_records_clears() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(3);
        set.observer_mut(id)
            .unwrap()
            .observe(target, MutationObserverInit::attributes());

        set.notify_attribute(target, &[target], "class", None);
        set.notify_attribute(target, &[target], "id", None);

        let records = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records.len(), 2);

        // Second call should be empty.
        let records2 = set.observer_mut(id).unwrap().take_records();
        assert_eq!(records2.len(), 0);
    }

    #[test]
    fn remove_observer() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();
        assert_eq!(set.count(), 1);
        set.remove_observer(id);
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn unobserve_specific_target() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let t1 = VexId::new(1);
        let t2 = VexId::new(2);
        let obs = set.observer_mut(id).unwrap();
        obs.observe(t1, MutationObserverInit::attributes());
        obs.observe(t2, MutationObserverInit::attributes());
        assert_eq!(obs.observations.len(), 2);

        obs.unobserve(t1);
        assert_eq!(obs.observations.len(), 1);
        assert_eq!(obs.observations[0].target, t2);
    }

    #[test]
    fn replace_observation_for_same_target() {
        let mut set = MutationObserverSet::new();
        let id = set.create_observer();

        let target = VexId::new(1);
        let obs = set.observer_mut(id).unwrap();
        obs.observe(target, MutationObserverInit::attributes());
        obs.observe(target, MutationObserverInit::child_list());

        // Should replace, not accumulate.
        assert_eq!(obs.observations.len(), 1);
        assert!(obs.observations[0].options.child_list);
        assert!(!obs.observations[0].options.attributes);
    }

    #[test]
    fn mutation_record_constructors() {
        let r1 = MutationRecord::child_list(
            VexId::new(1),
            vec![VexId::new(2)],
            vec![VexId::new(3)],
            Some(VexId::new(4)),
            Some(VexId::new(5)),
        );
        assert_eq!(r1.mutation_type, MutationType::ChildList);
        assert_eq!(r1.added_nodes.len(), 1);
        assert_eq!(r1.removed_nodes.len(), 1);
        assert_eq!(r1.previous_sibling, Some(VexId::new(4)));
        assert_eq!(r1.next_sibling, Some(VexId::new(5)));

        let r2 = MutationRecord::attributes(VexId::new(1), "href", Some("old".into()));
        assert_eq!(r2.mutation_type, MutationType::Attributes);
        assert_eq!(r2.attribute_name.as_deref(), Some("href"));
        assert_eq!(r2.old_value.as_deref(), Some("old"));

        let r3 = MutationRecord::character_data(VexId::new(1), Some("old text".into()));
        assert_eq!(r3.mutation_type, MutationType::CharacterData);
        assert_eq!(r3.old_value.as_deref(), Some("old text"));
    }

    #[test]
    fn observer_init_constructors() {
        let cl = MutationObserverInit::child_list();
        assert!(cl.child_list);
        assert!(!cl.attributes);

        let attr = MutationObserverInit::attributes();
        assert!(attr.attributes);
        assert!(!attr.child_list);

        let all = MutationObserverInit::all();
        assert!(all.child_list);
        assert!(all.attributes);
        assert!(all.character_data);
        assert!(all.subtree);
        assert!(all.attribute_old_value);
        assert!(all.character_data_old_value);
    }
}
